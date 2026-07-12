    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    use crate::ast::BinaryOp;
    use crate::hir::{HDeclaration, HExpr, HImportDecl, HModule, HParam, HResolvedIdent, HStatement};
    use crate::lower::lower_module;
    use crate::manifest::{CompilerConfig, CrateBinding, ExternalManifest};
    use crate::parser::parse_module;
    use crate::scanner::scan;
    use crate::semantic::analyze;
    use crate::symbols::SymbolKind;

    use super::{generate_cargo_toml, generate_main_rs, generate_rust_project};

    fn ident(id: usize, name: &str, kind: SymbolKind) -> HResolvedIdent {
        HResolvedIdent {
            id,
            name: name.to_string(),
            kind,
        }
    }

    #[test]
    fn emits_procedure_function_and_call_from_main() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![HDeclaration::Procedure {
                id: 1,
                name: "AddAndPrint".to_string(),
                params: vec![HParam {
                    id: 2,
                    name: "a".to_string(),
                    declared_type: None,
                    is_var: false,
                }],
                local_vars: vec![ident(3, "x", SymbolKind::Variable)],
                body: vec![
                    HStatement::Assign {
                        target: ident(3, "x", SymbolKind::Variable),
                        value: HExpr::Name(ident(2, "a", SymbolKind::Parameter)),
                    },
                    HStatement::Call {
                        name: ident(4, "WriteInt", SymbolKind::Procedure),
                        args: vec![HExpr::Name(ident(3, "x", SymbolKind::Variable))],
                    },
                ],
                end_name: "AddAndPrint".to_string(),
            }],
            statements: vec![HStatement::Call {
                name: ident(1, "AddAndPrint", SymbolKind::Procedure),
                args: vec![HExpr::Integer(7)],
            }],
        };

        let generated = generate_main_rs(&module, true);

        assert!(generated.contains("// Generated from Oberon0 module `Main`."));
        assert!(generated.contains("/// Returns the current value of a module-level Oberon0 variable."));
        assert!(generated.contains("/// Implements the Oberon0 procedure `AddAndPrint`."));
        assert!(generated.contains("/// - `param_2` corresponds to the Oberon0 parameter `a`."));
        assert!(generated.contains("fn AddAndPrint(vars: &mut BTreeMap<String, i64>, mut param_2: i64)"));
        assert!(generated.contains("set_procedure_var(vars, \"AddAndPrint\", \"a\", param_2);"));
        assert!(generated.contains("// Local variable backing the Oberon0 `x` binding."));
        assert!(generated.contains("let mut local_3: i64 = 0;"));
        assert!(generated.contains("set_procedure_var(vars, \"AddAndPrint\", \"x\", local_3);"));
        assert!(generated.contains("local_3 = param_2;"));
        assert!(generated.contains("print!(\"{}\", local_3);"));
        assert!(generated.contains("/// Executes the Oberon0 module `Main`."));
        assert!(generated.contains("// Runtime state keeps module variables and optional procedure-local snapshots."));
        assert!(generated.contains("AddAndPrint(&mut vars, 7);"));
    }

    #[test]
    fn emits_dependency_entries_with_package_and_features() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![HImportDecl {
                local_name: "IO".to_string(),
                external_name: "IO".to_string(),
            }],
            declarations: vec![],
            statements: vec![],
        };

        let mut dependencies = BTreeMap::new();
        dependencies.insert(
            "IO".to_string(),
            CrateBinding {
                crate_name: "termcolor".to_string(),
                version: "1.4".to_string(),
                package: Some("termcolor".to_string()),
                features: vec!["std".to_string()],
            },
        );
        let manifest = ExternalManifest {
            dependencies,
            compiler: CompilerConfig::default(),
        };

        let cargo_toml = generate_cargo_toml(&module, Some(&manifest))
            .expect("cargo toml generation should succeed");

        assert!(cargo_toml.contains("io = { version = \"1.4\", package = \"termcolor\", features = [\"std\"] }"));
    }

    #[test]
    fn binary_top_level_expr_is_not_wrapped_with_outer_parentheses() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![],
            statements: vec![HStatement::Assign {
                target: ident(10, "x", SymbolKind::Variable),
                value: HExpr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(HExpr::Integer(1)),
                    right: Box::new(HExpr::Binary {
                        op: BinaryOp::Mul,
                        left: Box::new(HExpr::Integer(2)),
                        right: Box::new(HExpr::Integer(3)),
                    }),
                },
            }],
        };

        let generated = generate_main_rs(&module, false);
        assert!(generated.contains("vars.insert(\"x\".to_string(), 1 + (2 * 3));"));
    }

    #[test]
    fn emits_write_string_builtin_as_print_macro() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![],
            statements: vec![HStatement::Call {
                name: ident(20, "WriteString", SymbolKind::Procedure),
                args: vec![HExpr::String("Hello, \"Oberon\"".to_string())],
            }],
        };

        let generated = generate_main_rs(&module, false);
        assert!(generated.contains("print!(\"{}\", \"Hello, \\\"Oberon\\\"\");"));
        assert!(!generated.contains("let mut vars: BTreeMap<String, i64> = BTreeMap::new();"));
        assert!(!generated.contains("State: {:?}"));
    }

    #[test]
    fn emits_writeln_builtin_as_newline_println() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![],
            statements: vec![HStatement::Call {
                name: ident(21, "WriteLn", SymbolKind::Procedure),
                args: vec![],
            }],
        };

        let generated = generate_main_rs(&module, false);
        assert!(generated.contains("println!();"));
    }

    #[test]
    fn emits_readint_and_eof_call_expressions() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![HDeclaration::Var {
                id: 1,
                name: "x".to_string(),
                declared_type: None,
            }],
            statements: vec![
                HStatement::Assign {
                    target: ident(1, "x", SymbolKind::Variable),
                    value: HExpr::Call {
                        name: ident(2, "ReadInt", SymbolKind::Procedure),
                        args: vec![],
                    },
                },
                HStatement::If {
                    condition: HExpr::Call {
                        name: ident(3, "EOF", SymbolKind::Procedure),
                        args: vec![],
                    },
                    then_branch: vec![HStatement::Call {
                        name: ident(4, "WriteLn", SymbolKind::Procedure),
                        args: vec![],
                    }],
                    else_branch: None,
                },
            ],
        };

        let generated = generate_main_rs(&module, false);
        assert!(generated.contains("fn read_int() -> i64"));
        assert!(generated.contains("fn eof() -> i64"));
        assert!(generated.contains("vars.insert(\"x\".to_string(), read_int());"));
        assert!(generated.contains("if eof() != 0 {"));
    }

    #[test]
    fn runtime_readint_and_eof_follow_input_contract() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![HDeclaration::Var {
                id: 1,
                name: "x".to_string(),
                declared_type: None,
            }],
            statements: vec![
                HStatement::Assign {
                    target: ident(1, "x", SymbolKind::Variable),
                    value: HExpr::Call {
                        name: ident(2, "ReadInt", SymbolKind::Procedure),
                        args: vec![],
                    },
                },
                HStatement::If {
                    condition: HExpr::Call {
                        name: ident(3, "EOF", SymbolKind::Procedure),
                        args: vec![],
                    },
                    then_branch: vec![HStatement::Call {
                        name: ident(4, "WriteInt", SymbolKind::Procedure),
                        args: vec![HExpr::Integer(1)],
                    }],
                    else_branch: Some(vec![HStatement::Call {
                        name: ident(5, "WriteInt", SymbolKind::Procedure),
                        args: vec![HExpr::Integer(0)],
                    }]),
                },
                HStatement::Call {
                    name: ident(6, "WriteInt", SymbolKind::Procedure),
                    args: vec![HExpr::Name(ident(1, "x", SymbolKind::Variable))],
                },
            ],
        };

        let out_root = temp_codegen_dir("readint_eof_runtime");
        let project_dir = generate_rust_project(&module, None, &out_root, false)
            .expect("project generation should succeed");

        let mut child = std::process::Command::new("cargo")
            .arg("run")
            .current_dir(&project_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("generated project should start");

        {
            use std::io::Write;
            let stdin = child.stdin.as_mut().expect("stdin should be piped");
            stdin
                .write_all(b"42\n")
                .expect("should write stdin for generated program");
        }

        let output = child
            .wait_with_output()
            .expect("generated project should finish");
        assert!(
            output.status.success(),
            "generated project failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert_eq!(stdout, "142");

        std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
    }

    #[test]
    fn runtime_readint_fails_after_eof_is_reached() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![HDeclaration::Var {
                id: 1,
                name: "x".to_string(),
                declared_type: None,
            }],
            statements: vec![
                HStatement::If {
                    condition: HExpr::Call {
                        name: ident(2, "EOF", SymbolKind::Procedure),
                        args: vec![],
                    },
                    then_branch: vec![HStatement::Call {
                        name: ident(3, "WriteInt", SymbolKind::Procedure),
                        args: vec![HExpr::Integer(1)],
                    }],
                    else_branch: Some(vec![HStatement::Call {
                        name: ident(4, "WriteInt", SymbolKind::Procedure),
                        args: vec![HExpr::Integer(0)],
                    }]),
                },
                HStatement::Assign {
                    target: ident(1, "x", SymbolKind::Variable),
                    value: HExpr::Call {
                        name: ident(5, "ReadInt", SymbolKind::Procedure),
                        args: vec![],
                    },
                },
            ],
        };

        let out_root = temp_codegen_dir("readint_after_eof_runtime");
        let project_dir = generate_rust_project(&module, None, &out_root, false)
            .expect("project generation should succeed");

        let output = std::process::Command::new("cargo")
            .arg("run")
            .current_dir(&project_dir)
            .output()
            .expect("generated project should run");

        assert!(
            !output.status.success(),
            "generated project should fail when ReadInt() is called after EOF"
        );

        let stderr = String::from_utf8(output.stderr).expect("stderr should be utf-8");
        assert!(
            stderr.contains("ReadInt() reached EOF"),
            "expected runtime EOF message, got: {stderr}"
        );

        std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
    }

    #[test]
    fn resolves_module_constants_in_generated_expressions() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![HDeclaration::Const {
                id: 30,
                name: "BASE".to_string(),
                value: 10,
            }],
            statements: vec![HStatement::Assign {
                target: ident(31, "x", SymbolKind::Variable),
                value: HExpr::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(HExpr::Name(ident(30, "BASE", SymbolKind::Constant))),
                    right: Box::new(HExpr::Integer(2)),
                },
            }],
        };

        let generated = generate_main_rs(&module, true);
        assert!(generated.contains("vars.insert(\"x\".to_string(), 10 + 2);"));
        assert!(!generated.contains("get_var(&vars, \"BASE\")"));
    }

    #[test]
    fn emits_state_output_only_when_explicitly_enabled() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![],
            statements: vec![HStatement::Assign {
                target: ident(40, "x", SymbolKind::Variable),
                value: HExpr::Integer(7),
            }],
        };

        let disabled = generate_main_rs(&module, false);
        assert!(!disabled.contains("State: {:?}"));
        assert!(disabled.contains("let mut vars: BTreeMap<String, i64> = BTreeMap::new();"));

        let enabled = generate_main_rs(&module, true);
        assert!(enabled.contains("let mut vars: BTreeMap<String, i64> = BTreeMap::new();"));
        assert!(enabled.contains("println!(\"State: {:?}\", vars);"));
    }

    #[test]
    fn emits_state_map_for_procedure_locals_without_module_variables() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![HDeclaration::Procedure {
                id: 1,
                name: "P".to_string(),
                params: vec![],
                local_vars: vec![ident(2, "local", SymbolKind::Variable)],
                body: vec![HStatement::Assign {
                    target: ident(2, "local", SymbolKind::Variable),
                    value: HExpr::Integer(9),
                }],
                end_name: "P".to_string(),
            }],
            statements: vec![HStatement::Call {
                name: ident(1, "P", SymbolKind::Procedure),
                args: vec![],
            }],
        };

        let generated = generate_main_rs(&module, true);
        assert!(generated.contains("let mut vars: BTreeMap<String, i64> = BTreeMap::new();"));
        assert!(generated.contains("set_procedure_var(vars, \"P\", \"local\", local_2);"));
    }

    #[test]
    fn emits_state_map_for_procedure_parameters_when_enabled() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![HDeclaration::Procedure {
                id: 1,
                name: "P".to_string(),
                params: vec![HParam {
                    id: 2,
                    name: "x".to_string(),
                    declared_type: None,
                    is_var: false,
                }],
                local_vars: vec![],
                body: vec![],
                end_name: "P".to_string(),
            }],
            statements: vec![HStatement::Call {
                name: ident(1, "P", SymbolKind::Procedure),
                args: vec![HExpr::Integer(9)],
            }],
        };

        let generated = generate_main_rs(&module, true);
        assert!(generated.contains("fn P(vars: &mut BTreeMap<String, i64>, mut param_2: i64)"));
        assert!(generated.contains("set_procedure_var(vars, \"P\", \"x\", param_2);"));
    }

    #[test]
    fn runtime_state_output_supports_reassigned_procedure_parameters() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![
                HDeclaration::Var {
                    id: 1,
                    name: "x".to_string(),
                    declared_type: None,
                },
                HDeclaration::Procedure {
                    id: 2,
                    name: "Walk".to_string(),
                    params: vec![HParam {
                        id: 3,
                        name: "x".to_string(),
                        declared_type: None,
                        is_var: false,
                    }],
                    local_vars: vec![],
                    body: vec![HStatement::While {
                        condition: HExpr::Name(ident(3, "x", SymbolKind::Parameter)),
                        body: vec![HStatement::Assign {
                            target: ident(3, "x", SymbolKind::Parameter),
                            value: HExpr::Binary {
                                op: BinaryOp::Sub,
                                left: Box::new(HExpr::Name(ident(3, "x", SymbolKind::Parameter))),
                                right: Box::new(HExpr::Integer(1)),
                            },
                        }],
                    }],
                    end_name: "Walk".to_string(),
                },
            ],
            statements: vec![
                HStatement::Assign {
                    target: ident(1, "x", SymbolKind::Variable),
                    value: HExpr::Integer(3),
                },
                HStatement::Call {
                    name: ident(2, "Walk", SymbolKind::Procedure),
                    args: vec![HExpr::Integer(2)],
                },
            ],
        };

        let out_root = temp_codegen_dir("mutable_param_state");
        let project_dir = generate_rust_project(&module, None, &out_root, true)
            .expect("project generation should succeed");

        let output = std::process::Command::new("cargo")
            .arg("run")
            .current_dir(&project_dir)
            .output()
            .expect("generated project should run");
        assert!(output.status.success(), "generated project failed: {}", String::from_utf8_lossy(&output.stderr));

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert!(stdout.contains("State: {\"Walk.x\": 0, \"x\": 3}"));

        std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
    }

    #[test]
    fn runtime_state_output_shows_shadowed_module_and_procedure_parameter_values() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![
                HDeclaration::Var {
                    id: 1,
                    name: "x".to_string(),
                    declared_type: None,
                },
                HDeclaration::Procedure {
                    id: 2,
                    name: "Show".to_string(),
                    params: vec![HParam {
                        id: 3,
                        name: "x".to_string(),
                        declared_type: None,
                        is_var: false,
                    }],
                    local_vars: vec![],
                    body: vec![HStatement::Call {
                        name: ident(4, "WriteInt", SymbolKind::Procedure),
                        args: vec![HExpr::Name(ident(3, "x", SymbolKind::Parameter))],
                    }],
                    end_name: "Show".to_string(),
                },
            ],
            statements: vec![
                HStatement::Assign {
                    target: ident(1, "x", SymbolKind::Variable),
                    value: HExpr::Integer(7),
                },
                HStatement::Call {
                    name: ident(2, "Show", SymbolKind::Procedure),
                    args: vec![HExpr::Integer(42)],
                },
            ],
        };

        let out_root = temp_codegen_dir("shadowed_param_state");
        let project_dir = generate_rust_project(&module, None, &out_root, true)
            .expect("project generation should succeed");

        let output = std::process::Command::new("cargo")
            .arg("run")
            .current_dir(&project_dir)
            .output()
            .expect("generated project should run");
        assert!(output.status.success(), "generated project failed: {}", String::from_utf8_lossy(&output.stderr));

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert!(stdout.contains("42"));
        assert!(stdout.contains("State: {\"Show.x\": 42, \"x\": 7}"));

        std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
    }

    #[test]
    fn runtime_state_output_shows_only_module_variables_when_enabled() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![HDeclaration::Procedure {
                id: 1,
                name: "P".to_string(),
                params: vec![],
                local_vars: vec![ident(2, "local", SymbolKind::Variable)],
                body: vec![
                    HStatement::Assign {
                        target: ident(2, "local", SymbolKind::Variable),
                        value: HExpr::Integer(9),
                    },
                    HStatement::Assign {
                        target: ident(3, "x", SymbolKind::Variable),
                        value: HExpr::Integer(7),
                    },
                ],
                end_name: "P".to_string(),
            }],
            statements: vec![HStatement::Call {
                name: ident(1, "P", SymbolKind::Procedure),
                args: vec![],
            }],
        };

        let out_root = temp_codegen_dir("state_enabled");
        let manifest = ExternalManifest {
            dependencies: BTreeMap::new(),
            compiler: CompilerConfig { emit_state: true },
        };
        let project_dir = generate_rust_project(&module, Some(&manifest), &out_root, true)
            .expect("project generation should succeed");

        let output = std::process::Command::new("cargo")
            .arg("run")
            .current_dir(&project_dir)
            .output()
            .expect("generated project should run");
        assert!(output.status.success(), "generated project failed: {}", String::from_utf8_lossy(&output.stderr));

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert!(stdout.contains("State: {\"P.local\": 9, \"x\": 7}"));

        std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
    }

    #[test]
    fn runtime_state_output_can_be_forced_on_without_manifest() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![HDeclaration::Procedure {
                id: 1,
                name: "P".to_string(),
                params: vec![],
                local_vars: vec![ident(2, "local", SymbolKind::Variable)],
                body: vec![HStatement::Assign {
                    target: ident(2, "local", SymbolKind::Variable),
                    value: HExpr::Integer(9),
                }],
                end_name: "P".to_string(),
            }],
            statements: vec![HStatement::Call {
                name: ident(1, "P", SymbolKind::Procedure),
                args: vec![],
            }],
        };

        let out_root = temp_codegen_dir("forced_state");
        let project_dir = generate_rust_project(&module, None, &out_root, true)
            .expect("project generation should succeed");

        let output = std::process::Command::new("cargo")
            .arg("run")
            .current_dir(&project_dir)
            .output()
            .expect("generated project should run");
        assert!(output.status.success(), "generated project failed: {}", String::from_utf8_lossy(&output.stderr));

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert!(stdout.contains("State: {\"P.local\": 9}"));

        std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
    }

    #[test]
    fn runtime_state_output_is_suppressed_by_default() {
        let module = HModule {
            name: "Main".to_string(),
            end_name: "Main".to_string(),
            imports: vec![],
            declarations: vec![],
            statements: vec![HStatement::Assign {
                target: ident(50, "x", SymbolKind::Variable),
                value: HExpr::Integer(7),
            }],
        };

        let out_root = temp_codegen_dir("state_disabled");
        let project_dir = generate_rust_project(&module, None, &out_root, false)
            .expect("project generation should succeed");

        let output = std::process::Command::new("cargo")
            .arg("run")
            .current_dir(&project_dir)
            .output()
            .expect("generated project should run");
        assert!(output.status.success(), "generated project failed: {}", String::from_utf8_lossy(&output.stderr));

        let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
        assert!(!stdout.contains("State: {"));

        std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
    }

    fn run_golden_case(case_name: &str, emit_state: bool) {
        let case_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("codegen_golden")
            .join(case_name);

        let source = fs::read_to_string(case_dir.join("source.ob0"))
            .expect("golden case source.ob0 should exist");
        let expected_stdout_path = case_dir.join("expected_stdout.txt");
        let expected_stdout = if expected_stdout_path.exists() {
            Some(
                fs::read_to_string(&expected_stdout_path)
                    .expect("golden case expected_stdout.txt should be readable"),
            )
        } else {
            None
        };

        let expected_exit_code_path = case_dir.join("expected_exit_code.txt");
        let expected_exit_code = if expected_exit_code_path.exists() {
            fs::read_to_string(&expected_exit_code_path)
                .expect("golden case expected_exit_code.txt should be readable")
                .trim()
                .parse::<i32>()
                .expect("golden expected exit code must be a valid i32")
        } else {
            0
        };

        scan(&source).expect("golden source should scan");
        let module = parse_module(&source).expect("golden source should parse");
        analyze(&module, None).expect("golden source should pass semantic analysis");
        let hir = lower_module(&module).expect("golden source should lower");

        let out_root = temp_codegen_dir(&format!("golden_{}", case_name));
        let project_dir = generate_rust_project(&hir, None, &out_root, emit_state)
            .expect("golden project generation should succeed");

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("run")
            .current_dir(&project_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        let mut child = cmd.spawn().expect("golden generated project should start");

        let stdin_path = case_dir.join("stdin.txt");
        if stdin_path.exists() {
            let input = fs::read(stdin_path).expect("golden stdin.txt should be readable");
            use std::io::Write;
            let stdin = child.stdin.as_mut().expect("stdin should be piped");
            stdin
                .write_all(&input)
                .expect("should write stdin for golden case");
        }

        let output = child
            .wait_with_output()
            .expect("golden generated project should finish");
        let actual_exit_code = output.status.code().unwrap_or(-1);
        assert_eq!(
            actual_exit_code, expected_exit_code,
            "golden exit code mismatch for case {case_name}; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).expect("golden stdout should be utf-8");
        if let Some(expected_stdout) = expected_stdout {
            assert_eq!(stdout, expected_stdout, "golden stdout mismatch for case {case_name}");
        }

        let stderr = String::from_utf8(output.stderr).expect("golden stderr should be utf-8");

        let expected_stdout_contains_path = case_dir.join("expected_stdout_contains.txt");
        if expected_stdout_contains_path.exists() {
            let expected_stdout_contains = fs::read_to_string(&expected_stdout_contains_path)
                .expect("golden case expected_stdout_contains.txt should be readable")
                .trim()
                .to_string();
            assert!(
                stdout.contains(&expected_stdout_contains),
                "golden stdout does not contain expected substring for case {case_name}: {expected_stdout_contains}"
            );
        }

        let expected_stderr_contains_path = case_dir.join("expected_stderr_contains.txt");
        if expected_stderr_contains_path.exists() {
            let expected_stderr_contains = fs::read_to_string(&expected_stderr_contains_path)
                .expect("golden case expected_stderr_contains.txt should be readable")
                .trim()
                .to_string();
            assert!(
                stderr.contains(&expected_stderr_contains),
                "golden stderr does not contain expected substring for case {case_name}: {expected_stderr_contains}"
            );
        }

        let expected_main_path = case_dir.join("expected_main.rs");
        if expected_main_path.exists() {
            let expected_main = fs::read_to_string(&expected_main_path)
                .expect("golden expected_main.rs should be readable");
            let generated_main = fs::read_to_string(project_dir.join("src").join("main.rs"))
                .expect("generated main.rs should be readable");
            assert_eq!(
                generated_main, expected_main,
                "golden generated main.rs mismatch for case {case_name}"
            );
        }

        std::fs::remove_dir_all(&out_root).expect("golden temp codegen dir should be removable");
    }

    #[test]
    fn golden_case_writeint_hello_matches_runtime_and_codegen_output() {
        run_golden_case("writeint_hello", false);
    }

    #[test]
    fn golden_case_readint_eof_matches_runtime_output() {
        run_golden_case("readint_eof", false);
    }

    #[test]
    fn golden_case_state_shadowing_emit_state_matches_runtime_output() {
        run_golden_case("state_shadowing_emit_state", true);
    }

    #[test]
    fn golden_case_readint_invalid_token_fails_with_parse_error() {
        run_golden_case("readint_invalid_token", false);
    }

    #[test]
    fn golden_case_readint_after_eof_fails() {
        run_golden_case("readint_after_eof", false);
    }

    fn temp_codegen_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("oberon0_codegen_{}_{}", name, nanos))
    }

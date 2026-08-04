use std::collections::BTreeMap;

use crate::ast::{BinaryOp, Expr, TypeRef, UnaryOp};
use crate::hir::{
    HDeclaration, HExpr, HImportDecl, HModule, HParam, HResolvedIdent, HStatement, HTarget,
};
use crate::lower::lower_module;
use crate::manifest::{CompilerConfig, CrateBinding, ExternalManifest};
use crate::parser::parse_module;
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

fn assign_target(id: usize, name: &str, kind: SymbolKind) -> HTarget {
    HTarget::Name(ident(id, name, kind))
}

fn indexed_target(id: usize, name: &str, kind: SymbolKind, index: HExpr) -> HTarget {
    HTarget::Indexed {
        name: ident(id, name, kind),
        index,
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
                    target: assign_target(3, "x", SymbolKind::Variable),
                    value: HExpr::Name(ident(2, "a", SymbolKind::Parameter)),
                },
                HStatement::Call {
                    module: None,
                    name: ident(4, "WriteInt", SymbolKind::Procedure),
                    args: vec![HExpr::Name(ident(3, "x", SymbolKind::Variable))],
                },
            ],
            end_name: "AddAndPrint".to_string(),
        }],
        statements: vec![HStatement::Call {
            module: None,
            name: ident(1, "AddAndPrint", SymbolKind::Procedure),
            args: vec![HExpr::Integer(7)],
        }],
    };

    let generated = generate_main_rs(&module, true);

    assert!(generated.contains("// Generated from Oberon0 module `Main`."));
    assert!(
        generated.contains("/// Returns the current value of a module-level Oberon0 variable.")
    );
    assert!(generated.contains("/// Implements the Oberon0 procedure `AddAndPrint`."));
    assert!(generated.contains("/// - `param_2` corresponds to the Oberon0 parameter `a`."));
    assert!(
        generated.contains("fn AddAndPrint(vars: &mut BTreeMap<String, Value>, param_2: Value)")
    );
    assert!(generated.contains("set_procedure_var(vars, \"AddAndPrint\", \"a\", &param_2);"));
    assert!(generated.contains("// Local variable backing the Oberon0 `x` binding."));
    assert!(generated.contains("let mut local_3: Value = value_integer(0);"));
    assert!(generated.contains("set_procedure_var(vars, \"AddAndPrint\", \"x\", &local_3);"));
    assert!(generated.contains("local_3 = param_2;"));
    assert!(generated.contains("print_value(&local_3);"));
    assert!(generated.contains("/// Executes the Oberon0 module `Main`."));
    assert!(generated.contains(
        "// Runtime state keeps module variables and optional procedure-local snapshots."
    ));
    assert!(generated.contains("let call_arg_0 = value_integer(7);"));
    assert!(generated.contains("AddAndPrint(&mut vars, call_arg_0);"));
}

#[test]
fn emits_dependency_entries_with_package_and_features() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![HImportDecl {
            local_name: "Color".to_string(),
            external_name: "TermColor".to_string(),
        }],
        declarations: vec![],
        statements: vec![],
    };

    let mut dependencies = BTreeMap::new();
    dependencies.insert(
        "TermColor".to_string(),
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

    assert!(
        cargo_toml
            .contains("color = { version = \"1.4\", package = \"termcolor\", features = [\"std\"] }")
    );
}

#[test]
fn emits_runtime_call_for_indexed_assignment_target() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![],
        statements: vec![HStatement::Assign {
            target: indexed_target(10, "arr", SymbolKind::Variable, HExpr::Integer(1)),
            value: HExpr::Integer(42),
        }],
    };

    let generated = generate_main_rs(&module, false);
    assert!(generated.contains("let indexed_idx_10 = (value_integer(1)).clone();"));
    assert!(generated.contains("let indexed_value_10 = (value_integer(42)).clone();"));
    assert!(
        generated.contains("set_var_index(&mut vars, \"arr\", &indexed_idx_10, indexed_value_10);")
    );
}

#[test]
fn emits_runtime_call_for_indexed_expression_read() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![],
        statements: vec![HStatement::Assign {
            target: assign_target(11, "x", SymbolKind::Variable),
            value: HExpr::Indexed {
                name: ident(12, "arr", SymbolKind::Variable),
                index: Box::new(HExpr::Integer(0)),
            },
        }],
    };

    let generated = generate_main_rs(&module, false);
    assert!(generated.contains("value_index(&get_var(&vars, \"arr\"), &value_integer(0))"));
}

#[test]
fn binary_top_level_expr_is_not_wrapped_with_outer_parentheses() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![],
        statements: vec![HStatement::Assign {
            target: assign_target(10, "x", SymbolKind::Variable),
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
    assert!(generated.contains("vars.insert(\"x\".to_string(),"));
    assert!(generated.contains("value_add(&value_integer(1),"));
    assert!(generated.contains("value_mul(&value_integer(2), &value_integer(3))"));
}

#[test]
fn emits_local_binding_branches_for_indexed_assignments_var_calls_and_builtin_exprs() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![
            HDeclaration::Procedure {
                id: 1,
                name: "Bump".to_string(),
                params: vec![HParam {
                    id: 2,
                    name: "target".to_string(),
                    declared_type: Some(TypeRef::Integer),
                    is_var: true,
                }],
                local_vars: vec![],
                body: vec![],
                end_name: "Bump".to_string(),
            },
            HDeclaration::Procedure {
                id: 3,
                name: "Worker".to_string(),
                params: vec![HParam {
                    id: 4,
                    name: "input".to_string(),
                    declared_type: Some(TypeRef::Integer),
                    is_var: true,
                }],
                local_vars: vec![
                    ident(5, "arr", SymbolKind::Variable),
                    ident(6, "x", SymbolKind::Variable),
                ],
                body: vec![
                    HStatement::Assign {
                        target: indexed_target(5, "arr", SymbolKind::Variable, HExpr::Integer(0)),
                        value: HExpr::Name(ident(4, "input", SymbolKind::Parameter)),
                    },
                    HStatement::Assign {
                        target: assign_target(6, "x", SymbolKind::Variable),
                        value: HExpr::Indexed {
                            name: ident(5, "arr", SymbolKind::Variable),
                            index: Box::new(HExpr::Integer(1)),
                        },
                    },
                    HStatement::Assign {
                        target: assign_target(6, "x", SymbolKind::Variable),
                        value: HExpr::Call {
                            name: ident(7, "FLT", SymbolKind::Procedure),
                            args: vec![HExpr::Name(ident(6, "x", SymbolKind::Variable))],
                        },
                    },
                    HStatement::Assign {
                        target: assign_target(6, "x", SymbolKind::Variable),
                        value: HExpr::Call {
                            name: ident(8, "FLOOR", SymbolKind::Procedure),
                            args: vec![HExpr::Real(1.5)],
                        },
                    },
                    HStatement::Call {
                        module: None,
                        name: ident(1, "Bump", SymbolKind::Procedure),
                        args: vec![HExpr::Name(ident(6, "x", SymbolKind::Variable))],
                    },
                ],
                end_name: "Worker".to_string(),
            },
        ],
        statements: vec![],
    };

    let generated = generate_main_rs(&module, true);

    assert!(generated.contains("value_set_index(&mut local_5, &indexed_idx_5, indexed_value_5);"));
    assert!(generated.contains("set_procedure_var(vars, \"Worker\", \"arr\", &local_5);"));
    assert!(generated.contains("(*param_4).clone()"));
    assert!(generated.contains("value_index(&local_5, &value_integer(1))"));
    assert!(generated.contains("value_as_real(&local_6)"));
    assert!(generated.contains("value_as_integer(&value_real(1.5))"));
    assert!(generated.contains("Bump(vars, &mut local_6);"));
}

#[test]
fn nested_binary_expressions_do_not_add_unnecessary_parentheses() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![],
        statements: vec![HStatement::Assign {
            target: assign_target(10, "x", SymbolKind::Variable),
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
    assert!(generated.contains(
        "value_add(&value_integer(1), &value_mul(&value_integer(2), &value_integer(3)))"
    ));
    assert!(!generated.contains("value_add(value_integer(1), (value_mul"));
}

#[test]
fn generated_runtime_helpers_are_marked_for_dead_code_suppression() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![],
        statements: vec![],
    };

    let generated = generate_main_rs(&module, false);
    assert!(generated.contains("#![allow(dead_code)]"));
    assert!(generated.contains("#[allow(dead_code)]\n#[derive(Clone, Debug, PartialEq)]"));
}

#[test]
fn emits_extended_unary_and_binary_operators() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![],
        statements: vec![HStatement::Assign {
            target: assign_target(10, "x", SymbolKind::Variable),
            value: HExpr::Binary {
                op: BinaryOp::And,
                left: Box::new(HExpr::Unary {
                    op: UnaryOp::Not,
                    value: Box::new(HExpr::Binary {
                        op: BinaryOp::Or,
                        left: Box::new(HExpr::Integer(1)),
                        right: Box::new(HExpr::Integer(0)),
                    }),
                }),
                right: Box::new(HExpr::Binary {
                    op: BinaryOp::Mod,
                    left: Box::new(HExpr::Binary {
                        op: BinaryOp::IntDiv,
                        left: Box::new(HExpr::Unary {
                            op: UnaryOp::Minus,
                            value: Box::new(HExpr::Integer(7)),
                        }),
                        right: Box::new(HExpr::Integer(2)),
                    }),
                    right: Box::new(HExpr::Integer(3)),
                }),
            },
        }],
    };

    let generated = generate_main_rs(&module, false);
    assert!(generated.contains("value_not("));
    assert!(generated.contains("value_or(&value_integer(1), &value_integer(0))"));
    assert!(generated.contains("value_mod("));
    assert!(generated.contains("value_div(&value_neg(&value_integer(7)), &value_integer(2))"));
    assert!(generated.contains("value_and("));
}

#[test]
fn emits_relational_operators_as_boolean_i64() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![],
        statements: vec![
            HStatement::Assign {
                target: assign_target(11, "x", SymbolKind::Variable),
                value: HExpr::Binary {
                    op: BinaryOp::Eq,
                    left: Box::new(HExpr::Integer(1)),
                    right: Box::new(HExpr::Integer(1)),
                },
            },
            HStatement::Assign {
                target: assign_target(11, "x", SymbolKind::Variable),
                value: HExpr::Binary {
                    op: BinaryOp::Ne,
                    left: Box::new(HExpr::Integer(1)),
                    right: Box::new(HExpr::Integer(2)),
                },
            },
            HStatement::Assign {
                target: assign_target(11, "x", SymbolKind::Variable),
                value: HExpr::Binary {
                    op: BinaryOp::Lt,
                    left: Box::new(HExpr::Integer(2)),
                    right: Box::new(HExpr::Integer(3)),
                },
            },
            HStatement::Assign {
                target: assign_target(11, "x", SymbolKind::Variable),
                value: HExpr::Binary {
                    op: BinaryOp::Le,
                    left: Box::new(HExpr::Integer(2)),
                    right: Box::new(HExpr::Integer(2)),
                },
            },
            HStatement::Assign {
                target: assign_target(11, "x", SymbolKind::Variable),
                value: HExpr::Binary {
                    op: BinaryOp::Gt,
                    left: Box::new(HExpr::Integer(3)),
                    right: Box::new(HExpr::Integer(2)),
                },
            },
            HStatement::Assign {
                target: assign_target(11, "x", SymbolKind::Variable),
                value: HExpr::Binary {
                    op: BinaryOp::Ge,
                    left: Box::new(HExpr::Integer(3)),
                    right: Box::new(HExpr::Integer(3)),
                },
            },
        ],
    };

    let generated = generate_main_rs(&module, false);
    assert!(
        generated
            .contains("value_bool_from_cmp(&value_integer(1), &value_integer(1), |a, b| a == b)")
    );
    assert!(
        generated
            .contains("value_bool_from_cmp(&value_integer(1), &value_integer(2), |a, b| a != b)")
    );
    assert!(
        generated
            .contains("value_bool_from_cmp(&value_integer(2), &value_integer(3), |a, b| a < b)")
    );
    assert!(
        generated
            .contains("value_bool_from_cmp(&value_integer(2), &value_integer(2), |a, b| a <= b)")
    );
    assert!(
        generated
            .contains("value_bool_from_cmp(&value_integer(3), &value_integer(2), |a, b| a > b)")
    );
    assert!(
        generated
            .contains("value_bool_from_cmp(&value_integer(3), &value_integer(3), |a, b| a >= b)")
    );
}

#[test]
fn expr_needs_state_map_tracks_variable_reads_in_nested_expressions() {
    assert!(super::expr_needs_state_map(&HExpr::Name(ident(
        2,
        "x",
        SymbolKind::Variable
    ))));
    assert!(!super::expr_needs_state_map(&HExpr::Name(ident(
        3,
        "C",
        SymbolKind::Constant
    ))));
    assert!(super::expr_needs_state_map(&HExpr::Binary {
        op: BinaryOp::Add,
        left: Box::new(HExpr::Name(ident(2, "x", SymbolKind::Variable))),
        right: Box::new(HExpr::Integer(7)),
    }));
    assert!(super::expr_needs_state_map(&HExpr::Unary {
        op: UnaryOp::Minus,
        value: Box::new(HExpr::Name(ident(2, "x", SymbolKind::Variable))),
    }));
    assert!(!super::expr_needs_state_map(&HExpr::Binary {
        op: BinaryOp::Add,
        left: Box::new(HExpr::Name(ident(3, "C", SymbolKind::Constant))),
        right: Box::new(HExpr::Integer(7)),
    }));
}

#[test]
fn statement_assigns_id_recurses_through_nested_if_and_while_blocks() {
    let stmt = HStatement::If {
        condition: HExpr::Integer(1),
        then_branch: vec![HStatement::Assign {
            target: assign_target(4, "x", SymbolKind::Variable),
            value: HExpr::Integer(1),
        }],
        else_branch: Some(vec![HStatement::While {
            condition: HExpr::Boolean(true),
            body: vec![HStatement::Assign {
                target: assign_target(5, "y", SymbolKind::Variable),
                value: HExpr::Integer(2),
            }],
        }]),
    };

    assert!(super::statement_assigns_id(&stmt, 4));
    assert!(super::statement_assigns_id(&stmt, 5));
    assert!(!super::statement_assigns_id(&stmt, 6));
}

#[test]
fn emits_write_string_builtin_as_print_macro() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![],
        statements: vec![HStatement::Call {
            module: None,
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
            module: None,
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
                target: assign_target(1, "x", SymbolKind::Variable),
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
                    module: None,
                    name: ident(4, "WriteLn", SymbolKind::Procedure),
                    args: vec![],
                }],
                else_branch: None,
            },
        ],
    };

    let generated = generate_main_rs(&module, false);
    assert!(generated.contains("fn read_int() -> Value"));
    assert!(generated.contains("fn eof() -> Value"));
    assert!(generated.contains("vars.insert(\"x\".to_string(), read_int());"));
    assert!(generated.contains("if value_truthy(&eof()) {"));
}

#[test]
fn emits_real_and_longreal_runtime_builtins() {
    let source = r#"
MODULE Main;
IMPORT IO;
VAR x: REAL;
    y: LONGREAL;
BEGIN
    x := IO.ReadReal();
    y := IO.ReadLongReal();
    IO.WriteReal(x);
    IO.WriteLn;
    IO.WriteLongReal(y)
END Main.
"#;

    let module = parse_module(source).expect("source should parse for codegen regression test");
    analyze(&module, None).expect("source should pass semantic analysis for codegen regression");
    let hir = lower_module(&module).expect("source should lower for codegen regression");

    let generated = generate_main_rs(&hir, false);
    assert!(generated.contains("fn read_real() -> Value"));
    assert!(generated.contains("fn read_longreal() -> Value"));
    assert!(generated.contains("fn write_real(value: &Value)"));
    assert!(generated.contains("fn write_longreal(value: &Value)"));
    assert!(generated.contains("write_real(&x);") || generated.contains("write_real(&get_var"));
    assert!(
        generated.contains("write_longreal(&y);") || generated.contains("write_longreal(&get_var")
    );
}

#[test]
fn evaluates_procedure_call_arguments_before_mutable_vars_borrow() {
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
                    name: "value".to_string(),
                    declared_type: None,
                    is_var: false,
                }],
                local_vars: vec![],
                body: vec![HStatement::Call {
                    module: None,
                    name: ident(4, "WriteInt", SymbolKind::Procedure),
                    args: vec![HExpr::Name(ident(3, "value", SymbolKind::Parameter))],
                }],
                end_name: "Show".to_string(),
            },
        ],
        statements: vec![
            HStatement::Assign {
                target: assign_target(1, "x", SymbolKind::Variable),
                value: HExpr::Integer(7),
            },
            HStatement::Call {
                module: None,
                name: ident(2, "Show", SymbolKind::Procedure),
                args: vec![HExpr::Name(ident(1, "x", SymbolKind::Variable))],
            },
        ],
    };

    let generated = generate_main_rs(&module, false);
    assert!(generated.contains("let call_arg_0 = get_var(&vars, \"x\");"));
    assert!(generated.contains("Show(&mut vars, call_arg_0);"));
}

#[test]
fn omits_io_runtime_helpers_when_not_used() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![],
        statements: vec![HStatement::Call {
            module: None,
            name: ident(1, "WriteLn", SymbolKind::Procedure),
            args: vec![],
        }],
    };

    let generated = generate_main_rs(&module, false);
    assert!(!generated.contains("use std::io::Read;"));
    assert!(!generated.contains("struct InputState"));
    assert!(!generated.contains("fn read_int() -> i64"));
    assert!(!generated.contains("fn eof() -> i64"));
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
                target: assign_target(1, "x", SymbolKind::Variable),
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
                    module: None,
                    name: ident(4, "WriteInt", SymbolKind::Procedure),
                    args: vec![HExpr::Integer(1)],
                }],
                else_branch: Some(vec![HStatement::Call {
                    module: None,
                    name: ident(5, "WriteInt", SymbolKind::Procedure),
                    args: vec![HExpr::Integer(0)],
                }]),
            },
            HStatement::Call {
                module: None,
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
                    module: None,
                    name: ident(3, "WriteInt", SymbolKind::Procedure),
                    args: vec![HExpr::Integer(1)],
                }],
                else_branch: Some(vec![HStatement::Call {
                    module: None,
                    name: ident(4, "WriteInt", SymbolKind::Procedure),
                    args: vec![HExpr::Integer(0)],
                }]),
            },
            HStatement::Assign {
                target: assign_target(1, "x", SymbolKind::Variable),
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
            value: HExpr::Integer(10),
        }],
        statements: vec![HStatement::Assign {
            target: assign_target(31, "x", SymbolKind::Variable),
            value: HExpr::Binary {
                op: BinaryOp::Add,
                left: Box::new(HExpr::Name(ident(30, "BASE", SymbolKind::Constant))),
                right: Box::new(HExpr::Integer(2)),
            },
        }],
    };

    let generated = generate_main_rs(&module, true);
    assert!(generated.contains(
        "vars.insert(\"x\".to_string(), value_add(&value_integer(10), &value_integer(2)));"
    ));
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
            target: assign_target(40, "x", SymbolKind::Variable),
            value: HExpr::Integer(7),
        }],
    };

    let disabled = generate_main_rs(&module, false);
    assert!(!disabled.contains("State: {"));
    assert!(disabled.contains("let mut vars: BTreeMap<String, Value> = BTreeMap::new();"));

    let enabled = generate_main_rs(&module, true);
    assert!(enabled.contains("let mut vars: BTreeMap<String, Value> = BTreeMap::new();"));
    assert!(enabled.contains("println!(\"State: {}\", runtime_state_string(&vars));"));
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
                target: assign_target(2, "local", SymbolKind::Variable),
                value: HExpr::Integer(9),
            }],
            end_name: "P".to_string(),
        }],
        statements: vec![HStatement::Call {
            module: None,
            name: ident(1, "P", SymbolKind::Procedure),
            args: vec![],
        }],
    };

    let generated = generate_main_rs(&module, true);
    assert!(generated.contains("let mut vars: BTreeMap<String, Value> = BTreeMap::new();"));
    assert!(generated.contains("set_procedure_var(vars, \"P\", \"local\", &local_2);"));
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
            module: None,
            name: ident(1, "P", SymbolKind::Procedure),
            args: vec![HExpr::Integer(9)],
        }],
    };

    let generated = generate_main_rs(&module, true);
    assert!(generated.contains("fn P(vars: &mut BTreeMap<String, Value>, param_2: Value)"));
    assert!(generated.contains("set_procedure_var(vars, \"P\", \"x\", &param_2);"));
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
                        target: assign_target(3, "x", SymbolKind::Parameter),
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
                target: assign_target(1, "x", SymbolKind::Variable),
                value: HExpr::Integer(3),
            },
            HStatement::Call {
                module: None,
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
    assert!(
        output.status.success(),
        "generated project failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

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
                    module: None,
                    name: ident(4, "WriteInt", SymbolKind::Procedure),
                    args: vec![HExpr::Name(ident(3, "x", SymbolKind::Parameter))],
                }],
                end_name: "Show".to_string(),
            },
        ],
        statements: vec![
            HStatement::Assign {
                target: assign_target(1, "x", SymbolKind::Variable),
                value: HExpr::Integer(7),
            },
            HStatement::Call {
                module: None,
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
    assert!(
        output.status.success(),
        "generated project failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

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
                    target: assign_target(2, "local", SymbolKind::Variable),
                    value: HExpr::Integer(9),
                },
                HStatement::Assign {
                    target: assign_target(3, "x", SymbolKind::Variable),
                    value: HExpr::Integer(7),
                },
            ],
            end_name: "P".to_string(),
        }],
        statements: vec![HStatement::Call {
            module: None,
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
    assert!(
        output.status.success(),
        "generated project failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

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
                target: assign_target(2, "local", SymbolKind::Variable),
                value: HExpr::Integer(9),
            }],
            end_name: "P".to_string(),
        }],
        statements: vec![HStatement::Call {
            module: None,
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
    assert!(
        output.status.success(),
        "generated project failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

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
            target: assign_target(50, "x", SymbolKind::Variable),
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
    assert!(
        output.status.success(),
        "generated project failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(!stdout.contains("State: {"));

    std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
}

#[test]
fn runtime_state_output_should_include_array_values() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![HDeclaration::Var {
            id: 60,
            name: "arr".to_string(),
            declared_type: Some(TypeRef::Array {
                length: Expr::Integer(3),
                element_type: Box::new(TypeRef::Integer),
            }),
        }],
        statements: vec![
            HStatement::Assign {
                target: indexed_target(60, "arr", SymbolKind::Variable, HExpr::Integer(0)),
                value: HExpr::Integer(10),
            },
            HStatement::Assign {
                target: indexed_target(60, "arr", SymbolKind::Variable, HExpr::Integer(1)),
                value: HExpr::Integer(20),
            },
            HStatement::Assign {
                target: indexed_target(60, "arr", SymbolKind::Variable, HExpr::Integer(2)),
                value: HExpr::Integer(30),
            },
        ],
    };

    let out_root = temp_codegen_dir("state_array_missing");
    let project_dir = generate_rust_project(&module, None, &out_root, true)
        .expect("project generation should succeed");

    let output = std::process::Command::new("cargo")
        .arg("run")
        .current_dir(&project_dir)
        .output()
        .expect("generated project should run");
    assert!(
        output.status.success(),
        "generated project failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("State: {\"arr\": [10, 20, 30]}"),
        "expected rendered array in state output, got: {stdout}"
    );

    std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
}

#[test]
fn var_array_parameter_updates_module_variable() {
    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![
            HDeclaration::Type {
                id: 70,
                name: "IntArray".to_string(),
                target: TypeRef::Array {
                    length: Expr::Integer(4),
                    element_type: Box::new(TypeRef::Integer),
                },
            },
            HDeclaration::Var {
                id: 71,
                name: "arr".to_string(),
                declared_type: Some(TypeRef::Named("IntArray".to_string())),
            },
            HDeclaration::Procedure {
                id: 72,
                name: "BumpFirst".to_string(),
                params: vec![HParam {
                    id: 73,
                    name: "a".to_string(),
                    declared_type: Some(TypeRef::Named("IntArray".to_string())),
                    is_var: true,
                }],
                local_vars: vec![],
                body: vec![HStatement::Assign {
                    target: indexed_target(73, "a", SymbolKind::Parameter, HExpr::Integer(0)),
                    value: HExpr::Binary {
                        op: BinaryOp::Add,
                        left: Box::new(HExpr::Indexed {
                            name: ident(73, "a", SymbolKind::Parameter),
                            index: Box::new(HExpr::Integer(0)),
                        }),
                        right: Box::new(HExpr::Integer(1)),
                    },
                }],
                end_name: "BumpFirst".to_string(),
            },
        ],
        statements: vec![
            HStatement::Assign {
                target: indexed_target(71, "arr", SymbolKind::Variable, HExpr::Integer(0)),
                value: HExpr::Integer(41),
            },
            HStatement::Call {
                module: None,
                name: ident(72, "BumpFirst", SymbolKind::Procedure),
                args: vec![HExpr::Name(ident(71, "arr", SymbolKind::Variable))],
            },
            HStatement::Call {
                module: None,
                name: ident(74, "WriteInt", SymbolKind::Procedure),
                args: vec![HExpr::Indexed {
                    name: ident(71, "arr", SymbolKind::Variable),
                    index: Box::new(HExpr::Integer(0)),
                }],
            },
        ],
    };

    let out_root = temp_codegen_dir("var_array_roundtrip");
    let project_dir = generate_rust_project(&module, None, &out_root, true)
        .expect("project generation should succeed");

    let output = std::process::Command::new("cargo")
        .arg("run")
        .current_dir(&project_dir)
        .output()
        .expect("generated project should run");
    assert!(
        output.status.success(),
        "generated project failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf-8");
    assert!(
        stdout.contains("42"),
        "expected updated first element, got: {stdout}"
    );
    assert!(
        stdout.contains("\"arr\": [42]"),
        "expected propagated array state, got: {stdout}"
    );

    std::fs::remove_dir_all(&out_root).expect("temp codegen dir should be removable");
}

#[test]
fn runtime_type_helpers_cover_boolean_array_and_alias_resolution() {
    let mut aliases = std::collections::HashMap::new();
    aliases.insert("AliasReal".to_string(), TypeRef::Real);
    aliases.insert(
        "AliasArray".to_string(),
        TypeRef::Array {
            length: Expr::Integer(2),
            element_type: Box::new(TypeRef::Integer),
        },
    );

    let module = HModule {
        name: "Main".to_string(),
        end_name: "Main".to_string(),
        imports: vec![],
        declarations: vec![
            HDeclaration::Type {
                id: 1,
                name: "AliasReal".to_string(),
                target: TypeRef::Real,
            },
            HDeclaration::Type {
                id: 2,
                name: "AliasArray".to_string(),
                target: TypeRef::Array {
                    length: Expr::Integer(2),
                    element_type: Box::new(TypeRef::Integer),
                },
            },
        ],
        statements: vec![],
    };

    let collected_aliases = super::collect_type_aliases(&module);
    assert_eq!(
        super::runtime_type_from_type_ref(Some(&TypeRef::Boolean)),
        Some(super::RuntimeType::Boolean)
    );
    assert_eq!(
        super::runtime_type_from_type_ref(Some(&TypeRef::Array {
            length: Expr::Integer(1),
            element_type: Box::new(TypeRef::Integer),
        })),
        Some(super::RuntimeType::Array)
    );
    assert_eq!(
        super::resolve_runtime_type(Some(&TypeRef::Named("AliasReal".to_string())), &aliases),
        Some(super::RuntimeType::Real)
    );
    assert_eq!(
        super::resolve_runtime_type(Some(&TypeRef::Named("AliasArray".to_string())), &aliases),
        Some(super::RuntimeType::Array)
    );
    assert_eq!(
        super::resolve_runtime_type(
            Some(&TypeRef::Named("AliasArray".to_string())),
            &collected_aliases
        ),
        Some(super::RuntimeType::Array)
    );
    assert_eq!(
        super::resolve_named_type(&TypeRef::Named("Missing".to_string()), &aliases),
        &TypeRef::Named("Missing".to_string())
    );
    assert_eq!(super::default_literal(super::RuntimeType::Real), "value_real(0.0)");
    assert_eq!(
        super::default_literal(super::RuntimeType::LongReal),
        "value_longreal(0.0)"
    );
    assert_eq!(
        super::default_literal(super::RuntimeType::Array),
        "value_array(0)"
    );
}

#[test]
fn generate_cargo_toml_allows_internal_builtin_module_imports_without_manifest_binding() {
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

    let manifest = ExternalManifest {
        dependencies: BTreeMap::new(),
        compiler: CompilerConfig::default(),
    };

    let cargo_toml = generate_cargo_toml(&module, Some(&manifest))
        .expect("internal builtin module imports should not need manifest bindings");
    assert!(
        !cargo_toml.contains("io = {"),
        "internal builtin modules should not generate Cargo dependencies"
    );
}

#[test]
fn statement_needs_state_map_covers_while_and_indexed_condition_paths() {
    let procedures = std::collections::HashSet::new();
    let while_stmt = HStatement::While {
        condition: HExpr::Indexed {
            name: ident(10, "arr", SymbolKind::Variable),
            index: Box::new(HExpr::Name(ident(11, "i", SymbolKind::Variable))),
        },
        body: vec![HStatement::Call {
            module: None,
            name: ident(12, "WriteLn", SymbolKind::Procedure),
            args: vec![],
        }],
    };

    assert!(super::statement_needs_state_map(&while_stmt, &procedures));
}

#[test]
fn expr_io_usage_tracks_nested_call_arguments() {
    let expr = HExpr::Call {
        name: ident(20, "SomeCall", SymbolKind::Procedure),
        args: vec![HExpr::Call {
            name: ident(21, "ReadLongReal", SymbolKind::Procedure),
            args: vec![HExpr::Call {
                name: ident(22, "ReadReal", SymbolKind::Procedure),
                args: vec![],
            }],
        }],
    };

    let usage = super::expr_io_usage(&expr);
    assert!(usage.uses_read_real);
    assert!(usage.uses_read_longreal);
}

#[test]
fn format_statement_covers_by_ref_locals_and_var_argument_temporaries() {
    let mut locals = std::collections::HashMap::new();
    locals.insert(31, "param_31".to_string());
    locals.insert(32, "param_32".to_string());

    let mut by_ref_locals = std::collections::HashSet::new();
    by_ref_locals.insert(31);
    by_ref_locals.insert(32);

    let mut procedures = std::collections::HashSet::new();
    procedures.insert("Bump".to_string());

    let mut procedure_param_modes = std::collections::HashMap::new();
    procedure_param_modes.insert("Bump".to_string(), vec![true, true]);

    let ctx = super::FormatContext {
        locals,
        by_ref_locals,
        constants: std::collections::HashMap::new(),
        procedures: &procedures,
        procedure_param_modes: &procedure_param_modes,
        vars_arg: "vars",
        procedure_name: Some("P"),
        track_procedure_locals: true,
        types: std::collections::HashMap::new(),
    };

    let assign_name = HStatement::Assign {
        target: assign_target(31, "x", SymbolKind::Parameter),
        value: HExpr::Integer(1),
    };
    let assign_indexed = HStatement::Assign {
        target: indexed_target(32, "arr", SymbolKind::Parameter, HExpr::Integer(0)),
        value: HExpr::Integer(9),
    };
    let call_var_proc = HStatement::Call {
        module: None,
        name: ident(40, "Bump", SymbolKind::Procedure),
        args: vec![
            HExpr::Name(ident(31, "x", SymbolKind::Parameter)),
            HExpr::Binary {
                op: BinaryOp::Add,
                left: Box::new(HExpr::Integer(2)),
                right: Box::new(HExpr::Integer(3)),
            },
        ],
    };

    let rendered_assign_name = super::format_statement(&assign_name, "    ", &ctx);
    assert!(rendered_assign_name.contains("*param_31 = value_integer(1);"));
    assert!(rendered_assign_name.contains("set_procedure_var(vars, \"P\", \"x\", &*param_31);"));

    let rendered_assign_indexed = super::format_statement(&assign_indexed, "    ", &ctx);
    assert!(rendered_assign_indexed.contains("value_set_index(&mut *param_32"));
    assert!(
        rendered_assign_indexed.contains("set_procedure_var(vars, \"P\", \"arr\", &*param_32);")
    );

    let rendered_call = super::format_statement(&call_var_proc, "    ", &ctx);
    assert!(rendered_call.contains("Bump(vars, &mut *param_31, &mut call_arg_1);"));
    assert!(
        rendered_call.contains("let mut call_arg_1 = value_add(&value_integer(2), &value_integer(3));")
    );
}

#[test]
fn format_statement_defaults_and_format_expr_extended_paths() {
    let procedures = std::collections::HashSet::new();
    let procedure_param_modes = std::collections::HashMap::new();
    let ctx = super::FormatContext {
        locals: std::collections::HashMap::new(),
        by_ref_locals: std::collections::HashSet::new(),
        constants: std::collections::HashMap::new(),
        procedures: &procedures,
        procedure_param_modes: &procedure_param_modes,
        vars_arg: "vars",
        procedure_name: None,
        track_procedure_locals: false,
        types: std::collections::HashMap::new(),
    };

    let write_int = HStatement::Call {
        module: None,
        name: ident(50, "WriteInt", SymbolKind::Procedure),
        args: vec![],
    };
    let write_real = HStatement::Call {
        module: None,
        name: ident(51, "WriteReal", SymbolKind::Procedure),
        args: vec![],
    };
    let write_longreal = HStatement::Call {
        module: None,
        name: ident(52, "WriteLongReal", SymbolKind::Procedure),
        args: vec![],
    };
    let write_string = HStatement::Call {
        module: None,
        name: ident(53, "WriteString", SymbolKind::Procedure),
        args: vec![],
    };

    assert!(super::format_statement(&write_int, "    ", &ctx).contains("print_value(&value_integer(0));"));
    assert!(super::format_statement(&write_real, "    ", &ctx).contains("write_real(0.0);"));
    assert!(
        super::format_statement(&write_longreal, "    ", &ctx).contains("write_longreal(0.0);")
    );
    assert!(super::format_statement(&write_string, "    ", &ctx).contains("print!(\"\");"));

    assert_eq!(super::format_expr(&HExpr::Boolean(true), &ctx), "value_integer(1)");
    assert_eq!(super::format_expr(&HExpr::LongReal(1.5), &ctx), "value_longreal(1.5)");
    assert_eq!(super::format_expr(&HExpr::Real(2.5), &ctx), "value_real(2.5)");
    assert_eq!(
        super::format_expr(
            &HExpr::Unary {
                op: UnaryOp::Plus,
                value: Box::new(HExpr::Integer(7)),
            },
            &ctx
        ),
        "value_integer(7)"
    );

    assert_eq!(
        super::format_expr(
            &HExpr::Call {
                name: ident(54, "FLT", SymbolKind::Procedure),
                args: vec![]
            },
            &ctx
        ),
        "value_real(0.0)"
    );
    assert_eq!(
        super::format_expr(
            &HExpr::Call {
                name: ident(55, "FLOOR", SymbolKind::Procedure),
                args: vec![]
            },
            &ctx
        ),
        "value_integer(0)"
    );
    assert!(
        super::format_expr(
            &HExpr::Call {
                name: ident(56, "Unknown", SymbolKind::Procedure),
                args: vec![HExpr::Integer(1), HExpr::Integer(2)]
            },
            &ctx
        )
        .contains("/* unsupported call expr Unknown(value_integer(1), value_integer(2)) */ 0")
    );
    assert_eq!(
        super::format_binary_expr(BinaryOp::Div, "lhs", "rhs", false),
        "value_div(&lhs, &rhs)"
    );
    assert_eq!(
        super::format_binary_expr(BinaryOp::Add, "lhs", "rhs", true),
        "(value_add(&lhs, &rhs))"
    );
}

fn temp_codegen_dir(name: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("oberon0_codegen_{}_{}", name, nanos))
}

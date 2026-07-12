use std::fs;
use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};

use crate::ast::BinaryOp;
use crate::hir::{HDeclaration, HExpr, HModule, HParam, HResolvedIdent, HStatement};
use crate::manifest::{CrateBinding, ExternalManifest};

pub fn generate_rust_project(
    module: &HModule,
    manifest: Option<&ExternalManifest>,
    out_root: &Path,
    emit_state: bool,
) -> Result<PathBuf> {
    let project_dir = out_root.join(&module.name);
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)
        .with_context(|| format!("Could not create directory: {}", src_dir.display()))?;

    let cargo_toml = generate_cargo_toml(module, manifest)?;
    let main_rs = generate_main_rs(module, emit_state);

    fs::write(project_dir.join("Cargo.toml"), cargo_toml)
        .with_context(|| format!("Could not write Cargo.toml: {}", project_dir.display()))?;
    fs::write(src_dir.join("main.rs"), main_rs)
        .with_context(|| format!("Could not write main.rs: {}", src_dir.display()))?;

    Ok(project_dir)
}

fn generate_cargo_toml(module: &HModule, manifest: Option<&ExternalManifest>) -> Result<String> {
    let mut out = String::new();
    out.push_str("[package]\n");
    out.push_str(&format!("name = \"{}\"\n", module.name.to_lowercase()));
    out.push_str("version = \"0.1.0\"\n");
    out.push_str("edition = \"2024\"\n\n");

    out.push_str("[dependencies]\n");

    if let Some(manifest) = manifest {
        for import in &module.imports {
            let binding = manifest.resolve(&import.external_name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Import '{}' was not found in the manifest",
                    import.external_name
                )
            })?;
            out.push_str(&dependency_line(&import.local_name, binding));
            out.push('\n');
        }
    }

    Ok(out)
}

fn dependency_line(local_name: &str, binding: &CrateBinding) -> String {
    let dep_name = local_name.to_lowercase();
    let mut fields = vec![format!("version = \"{}\"", binding.version)];

    let package_name = binding
        .package
        .clone()
        .unwrap_or_else(|| binding.crate_name.clone());

    if dep_name != package_name {
        fields.push(format!("package = \"{}\"", package_name));
    }

    if !binding.features.is_empty() {
        let features = binding
            .features
            .iter()
            .map(|f| format!("\"{}\"", f))
            .collect::<Vec<_>>()
            .join(", ");
        fields.push(format!("features = [{}]", features));
    }

    format!("{} = {{ {} }}", dep_name, fields.join(", "))
}

fn generate_main_rs(module: &HModule, emit_state: bool) -> String {
    let mut out = String::new();
    let procedure_names = collect_procedure_names(module);
    let module_constants = collect_module_constants(module);
    let needs_module_state = statements_need_state_map(&module.statements, &procedure_names);
    let tracks_procedure_state = emit_state && module_has_procedure_locals(module);
    let needs_runtime_state = needs_module_state || tracks_procedure_state;
    let show_state = emit_state && needs_runtime_state;

    out.push_str(&format!(
        "// Generated from Oberon0 module `{}`.\n",
        module.name
    ));
    out.push_str("// Comments preserve the mapping between Oberon0 names and generated Rust bindings.\n\n");
    out.push_str("use std::collections::BTreeMap;\n\n");
    out.push_str("/// Returns the current value of a module-level Oberon0 variable.\n");
    out.push_str("///\n");
    out.push_str("/// Generated programs keep module state in `vars`, keyed by the original Oberon0 name.\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn get_var(vars: &BTreeMap<String, i64>, name: &str) -> i64 {\n");
    out.push_str("    *vars.get(name).unwrap_or(&0)\n");
    out.push_str("}\n\n");

    out.push_str("/// Records the current value of a procedure-scoped Oberon0 variable.\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn set_procedure_var(vars: &mut BTreeMap<String, i64>, procedure: &str, name: &str, value: i64) {\n");
    out.push_str("    vars.insert(format!(\"{}.{}\", procedure, name), value);\n");
    out.push_str("}\n\n");

    for declaration in &module.declarations {
        if let HDeclaration::Procedure {
            name,
            params,
            local_vars,
            body,
            ..
        } = declaration
        {
            out.push_str(&format_procedure(
                name,
                params,
                local_vars,
                body,
                &module_constants,
                &procedure_names,
                emit_state,
            ));
            out.push('\n');
        }
    }

    out.push_str(&format!("/// Executes the Oberon0 module `{}`.\n", module.name));
    out.push_str("fn main() {\n");
    if needs_runtime_state {
        out.push_str("    // Runtime state keeps module variables and optional procedure-local snapshots.\n");
        out.push_str("    let mut vars: BTreeMap<String, i64> = BTreeMap::new();\n");
    }

    let main_ctx = FormatContext {
        locals: HashMap::new(),
        constants: module_constants,
        procedures: &procedure_names,
        vars_arg: if needs_runtime_state { "&mut vars" } else { "&mut BTreeMap::new()" },
        procedure_name: None,
        track_procedure_locals: false,
    };

    for stmt in &module.statements {
        out.push_str(&format_statement(stmt, "    ", &main_ctx));
    }

    if show_state {
        out.push_str("    if !vars.is_empty() {\n");
        out.push_str("        println!(\"State: {:?}\", vars);\n");
        out.push_str("    }\n");
    }
    out.push_str("}\n");

    out
}

struct FormatContext<'a> {
    locals: HashMap<usize, String>,
    constants: HashMap<usize, i64>,
    procedures: &'a HashSet<String>,
    vars_arg: &'a str,
    procedure_name: Option<&'a str>,
    track_procedure_locals: bool,
}

fn module_has_procedure_locals(module: &HModule) -> bool {
    module.declarations.iter().any(|decl| match decl {
        HDeclaration::Procedure { local_vars, .. } => !local_vars.is_empty(),
        _ => false,
    })
}

fn collect_procedure_names(module: &HModule) -> HashSet<String> {
    module
        .declarations
        .iter()
        .filter_map(|decl| match decl {
            HDeclaration::Procedure { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

fn collect_module_constants(module: &HModule) -> HashMap<usize, i64> {
    module
        .declarations
        .iter()
        .filter_map(|decl| match decl {
            HDeclaration::Const { id, value, .. } => Some((*id, *value)),
            _ => None,
        })
        .collect()
}

fn statements_need_state_map(stmts: &[HStatement], procedure_names: &HashSet<String>) -> bool {
    stmts
        .iter()
        .any(|stmt| statement_needs_state_map(stmt, procedure_names))
}

fn statement_needs_state_map(stmt: &HStatement, procedure_names: &HashSet<String>) -> bool {
    match stmt {
        HStatement::Assign { .. } => true,
        HStatement::Call { name, args } => {
            procedure_names.contains(&name.name)
                || args.iter().any(expr_needs_state_map)
        }
        HStatement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_needs_state_map(condition)
                || statements_need_state_map(then_branch, procedure_names)
                || else_branch
                    .as_ref()
                    .is_some_and(|branch| statements_need_state_map(branch, procedure_names))
        }
        HStatement::While { condition, body } => {
            expr_needs_state_map(condition) || statements_need_state_map(body, procedure_names)
        }
    }
}

fn expr_needs_state_map(expr: &HExpr) -> bool {
    match expr {
        HExpr::Integer(_) | HExpr::String(_) => false,
        HExpr::Name(ident) => ident.kind != crate::symbols::SymbolKind::Constant,
        HExpr::Binary { left, right, .. } => expr_needs_state_map(left) || expr_needs_state_map(right),
    }
}

fn format_procedure(
    name: &str,
    params: &[HParam],
    local_vars: &[HResolvedIdent],
    body: &[HStatement],
    constants: &HashMap<usize, i64>,
    procedure_names: &HashSet<String>,
    emit_state: bool,
) -> String {
    let mut out = String::new();
    let mut locals = HashMap::new();

    let mut signature_args = Vec::new();
    signature_args.push("vars: &mut BTreeMap<String, i64>".to_string());

    for param in params {
        let binding = format!("param_{}", param.id);
        locals.insert(param.id, binding.clone());
        signature_args.push(format!("mut {}: i64", binding));
    }

    let ctx = FormatContext {
        locals,
        constants: constants.clone(),
        procedures: procedure_names,
        vars_arg: "vars",
        procedure_name: Some(name),
        track_procedure_locals: emit_state,
    };

    out.push_str(&format!("/// Implements the Oberon0 procedure `{}`.\n", name));
    if !params.is_empty() {
        out.push_str("///\n");
        out.push_str("/// Parameter bindings:\n");
        for param in params {
            out.push_str(&format!(
                "/// - `param_{}` corresponds to the Oberon0 parameter `{}`.\n",
                param.id, param.name
            ));
        }
    }
    out.push_str("#[allow(non_snake_case)]\n");
    out.push_str("#[allow(unused_variables)]\n");
    out.push_str(&format!("fn {}({}) {{\n", name, signature_args.join(", ")));

    if emit_state {
        for param in params {
            out.push_str(&format!(
                "    set_procedure_var(vars, \"{}\", \"{}\", param_{});\n",
                name, param.name, param.id
            ));
        }
    }

    for local in local_vars {
        out.push_str(&format!(
            "    // Local variable backing the Oberon0 `{}` binding.\n",
            local.name
        ));
        out.push_str(&format!("    let mut local_{}: i64 = 0;\n", local.id));
        if emit_state {
            out.push_str(&format!(
                "    set_procedure_var(vars, \"{}\", \"{}\", local_{});\n",
                name, local.name, local.id
            ));
        }
    }

    let mut procedure_ctx = ctx;
    for local in local_vars {
        procedure_ctx
            .locals
            .insert(local.id, format!("local_{}", local.id));
    }

    for stmt in body {
        out.push_str(&format_statement(stmt, "    ", &procedure_ctx));
    }

    out.push_str("}\n");
    out
}

fn format_statement(stmt: &HStatement, indent: &str, ctx: &FormatContext<'_>) -> String {
    match stmt {
        HStatement::Assign { target, value } => {
            if let Some(binding) = ctx.locals.get(&target.id) {
                let mut out = String::new();
                out.push_str(&format!(
                    "{}{} = {};\n",
                    indent,
                    binding,
                    format_top_level_expr(value, ctx)
                ));
                if ctx.track_procedure_locals {
                    if let Some(procedure_name) = ctx.procedure_name {
                        out.push_str(&format!(
                            "{}set_procedure_var(vars, \"{}\", \"{}\", {});\n",
                            indent, procedure_name, target.name, binding
                        ));
                    }
                }
                out
            } else {
                format!(
                    "{}vars.insert(\"{}\".to_string(), {});\n",
                    indent,
                    target.name,
                    format_top_level_expr(value, ctx)
                )
            }
        }
        HStatement::Call { name, args } => {
            if name.name == "WriteInt" {
                match args.first() {
                    Some(first) => format!(
                        "{}print!(\"{{}}\", {});\n",
                        indent,
                        format_top_level_expr(first, ctx)
                    ),
                    None => format!("{}print!(\"\");\n", indent),
                }
            } else if name.name == "WriteLn" {
                format!("{}println!();\n", indent)
            } else if name.name == "WriteString" {
                match args.first() {
                    Some(first) => format!(
                        "{}print!(\"{{}}\", {});\n",
                        indent,
                        format_top_level_expr(first, ctx)
                    ),
                    None => format!("{}print!(\"\");\n", indent),
                }
            } else if ctx.procedures.contains(&name.name) {
                let rendered_args = args
                    .iter()
                    .map(|arg| format_top_level_expr(arg, ctx))
                    .collect::<Vec<_>>();
                let joined_args = if rendered_args.is_empty() {
                    ctx.vars_arg.to_string()
                } else {
                    format!("{}, {}", ctx.vars_arg, rendered_args.join(", "))
                };
                format!("{}{}({});\n", indent, name.name, joined_args)
            } else {
                format!(
                    "{}eprintln!(\"Note: call '{}' is not implemented in the MVP.\");\n",
                    indent, name.name
                )
            }
        }
        HStatement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let mut out = String::new();
            out.push_str(&format!(
                "{}if {} != 0 {{\n",
                indent,
                format_expr(condition, ctx)
            ));
            out.push_str(&format_block(then_branch, &format!("{}    ", indent), ctx));
            out.push_str(&format!("{}}}", indent));

            if let Some(else_branch) = else_branch {
                out.push_str(" else {\n");
                out.push_str(&format_block(else_branch, &format!("{}    ", indent), ctx));
                out.push_str(&format!("{}}}\n", indent));
            } else {
                out.push('\n');
            }

            out
        }
        HStatement::While { condition, body } => {
            let mut out = String::new();
            out.push_str(&format!(
                "{}while {} != 0 {{\n",
                indent,
                format_expr(condition, ctx)
            ));
            out.push_str(&format_block(body, &format!("{}    ", indent), ctx));
            out.push_str(&format!("{}}}\n", indent));
            out
        }
    }
}

fn format_top_level_expr(expr: &HExpr, ctx: &FormatContext<'_>) -> String {
    match expr {
        HExpr::Binary { op, left, right } => {
            let op_s = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
            };
            format!("{} {} {}", format_expr(left, ctx), op_s, format_expr(right, ctx))
        }
        _ => format_expr(expr, ctx),
    }
}

fn format_block(stmts: &[HStatement], indent: &str, ctx: &FormatContext<'_>) -> String {
    let mut out = String::new();
    for stmt in stmts {
        out.push_str(&format_statement(stmt, indent, ctx));
    }
    out
}

fn format_expr(expr: &HExpr, ctx: &FormatContext<'_>) -> String {
    match expr {
        HExpr::Integer(v) => v.to_string(),
        HExpr::String(value) => format!("{:?}", value),
        HExpr::Name(ident) => {
            if let Some(value) = ctx.constants.get(&ident.id) {
                value.to_string()
            } else {
                match ctx.locals.get(&ident.id) {
                    Some(binding) => binding.clone(),
                    None => format!("get_var(&vars, \"{}\")", ident.name),
                }
            }
        }
        HExpr::Binary { op, left, right } => {
            let op_s = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
            };
            format!("({} {} {})", format_expr(left, ctx), op_s, format_expr(right, ctx))
        }
    }
}

#[allow(dead_code)]
fn _validate_import_mapping(module: &HModule, manifest: &ExternalManifest) -> Result<()> {
    for import in &module.imports {
        if manifest.resolve(&import.external_name).is_none() {
            bail!(
                "Import '{}' has no crate binding in the manifest",
                import.external_name
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::ast::BinaryOp;
    use crate::hir::{HDeclaration, HExpr, HImportDecl, HModule, HParam, HResolvedIdent, HStatement};
    use crate::manifest::{CompilerConfig, CrateBinding, ExternalManifest};
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
                },
                HDeclaration::Procedure {
                    id: 2,
                    name: "Walk".to_string(),
                    params: vec![HParam {
                        id: 3,
                        name: "x".to_string(),
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
                },
                HDeclaration::Procedure {
                    id: 2,
                    name: "Show".to_string(),
                    params: vec![HParam {
                        id: 3,
                        name: "x".to_string(),
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

    fn temp_codegen_dir(name: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("oberon0_codegen_{}_{}", name, nanos))
    }
}

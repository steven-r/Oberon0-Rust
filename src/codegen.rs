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
) -> Result<PathBuf> {
    let project_dir = out_root.join(&module.name);
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)
        .with_context(|| format!("Could not create directory: {}", src_dir.display()))?;

    let cargo_toml = generate_cargo_toml(module, manifest)?;
    let main_rs = generate_main_rs(module);

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

fn generate_main_rs(module: &HModule) -> String {
    let mut out = String::new();
    let procedure_names = collect_procedure_names(module);

    out.push_str("use std::collections::HashMap;\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn get_var(vars: &HashMap<String, i64>, name: &str) -> i64 {\n");
    out.push_str("    *vars.get(name).unwrap_or(&0)\n");
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
                &procedure_names,
            ));
            out.push('\n');
        }
    }

    out.push_str("fn main() {\n");
    out.push_str("    let mut vars: HashMap<String, i64> = HashMap::new();\n");

    let main_ctx = FormatContext {
        locals: HashMap::new(),
        procedures: &procedure_names,
        vars_arg: "&mut vars",
    };

    for stmt in &module.statements {
        out.push_str(&format_statement(stmt, "    ", &main_ctx));
    }

    out.push_str("    println!(\"State: {:?}\", vars);\n");
    out.push_str("}\n");

    out
}

struct FormatContext<'a> {
    locals: HashMap<usize, String>,
    procedures: &'a HashSet<String>,
    vars_arg: &'a str,
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

fn format_procedure(
    name: &str,
    params: &[HParam],
    local_vars: &[HResolvedIdent],
    body: &[HStatement],
    procedure_names: &HashSet<String>,
) -> String {
    let mut out = String::new();
    let mut locals = HashMap::new();

    let mut signature_args = Vec::new();
    signature_args.push("vars: &mut HashMap<String, i64>".to_string());

    for param in params {
        let binding = format!("param_{}", param.id);
        locals.insert(param.id, binding.clone());
        signature_args.push(format!("{}: i64", binding));
    }

    let ctx = FormatContext {
        locals,
        procedures: procedure_names,
        vars_arg: "vars",
    };

    out.push_str("#[allow(non_snake_case)]\n");
    out.push_str("#[allow(unused_variables)]\n");
    out.push_str(&format!("fn {}({}) {{\n", name, signature_args.join(", ")));

    for local in local_vars {
        out.push_str(&format!("    let mut local_{}: i64 = 0;\n", local.id));
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
                format!(
                    "{}{} = {};\n",
                    indent,
                    binding,
                    format_top_level_expr(value, ctx)
                )
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
                        "{}println!(\"{{}}\", {});\n",
                        indent,
                        format_top_level_expr(first, ctx)
                    ),
                    None => format!("{}println!();\n", indent),
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
        HExpr::Name(ident) => match ctx.locals.get(&ident.id) {
            Some(binding) => binding.clone(),
            None => format!("get_var(&vars, \"{}\")", ident.name),
        },
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
    use crate::manifest::{CrateBinding, ExternalManifest};
    use crate::symbols::SymbolKind;

    use super::{generate_cargo_toml, generate_main_rs};

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

        let generated = generate_main_rs(&module);

        assert!(generated.contains("fn AddAndPrint(vars: &mut HashMap<String, i64>, param_2: i64)"));
        assert!(generated.contains("let mut local_3: i64 = 0;"));
        assert!(generated.contains("local_3 = param_2;"));
        assert!(generated.contains("println!(\"{}\", local_3);"));
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
        let manifest = ExternalManifest { dependencies };

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

        let generated = generate_main_rs(&module);
        assert!(generated.contains("vars.insert(\"x\".to_string(), 1 + (2 * 3));"));
    }
}

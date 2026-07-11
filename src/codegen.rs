use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::ast::BinaryOp;
use crate::hir::{HExpr, HModule, HStatement};
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

    out.push_str("use std::collections::HashMap;\n\n");
    out.push_str("fn get_var(vars: &HashMap<String, i64>, name: &str) -> i64 {\n");
    out.push_str("    *vars.get(name).unwrap_or(&0)\n");
    out.push_str("}\n\n");

    out.push_str("fn main() {\n");
    out.push_str("    let mut vars: HashMap<String, i64> = HashMap::new();\n");

    for stmt in &module.statements {
        out.push_str(&format_statement(stmt, "    "));
    }

    out.push_str("    println!(\"State: {:?}\", vars);\n");
    out.push_str("}\n");

    out
}

fn format_statement(stmt: &HStatement, indent: &str) -> String {
    match stmt {
        HStatement::Assign { target, value } => {
            format!(
                "{}vars.insert(\"{}\".to_string(), {});\n",
                indent,
                target.name,
                format_top_level_expr(value)
            )
        }
        HStatement::Call { name, args } => {
            if name.name == "WriteInt" {
                match args.first() {
                    Some(first) => {
                        format!("{}println!(\"{{}}\", {});\n", indent, format_top_level_expr(first))
                    }
                    None => format!("{}println!();\n", indent),
                }
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
                format_expr(condition)
            ));
            out.push_str(&format_block(then_branch, &format!("{}    ", indent)));
            out.push_str(&format!("{}}}", indent));

            if let Some(else_branch) = else_branch {
                out.push_str(" else {\n");
                out.push_str(&format_block(else_branch, &format!("{}    ", indent)));
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
                format_expr(condition)
            ));
            out.push_str(&format_block(body, &format!("{}    ", indent)));
            out.push_str(&format!("{}}}\n", indent));
            out
        }
    }
}

fn format_top_level_expr(expr: &HExpr) -> String {
    match expr {
        HExpr::Binary { op, left, right } => {
            let op_s = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
            };
            format!("{} {} {}", format_expr(left), op_s, format_expr(right))
        }
        _ => format_expr(expr),
    }
}

fn format_block(stmts: &[HStatement], indent: &str) -> String {
    let mut out = String::new();
    for stmt in stmts {
        out.push_str(&format_statement(stmt, indent));
    }
    out
}

fn format_expr(expr: &HExpr) -> String {
    match expr {
        HExpr::Integer(v) => v.to_string(),
        HExpr::Name(ident) => format!("get_var(&vars, \"{}\")", ident.name),
        HExpr::Binary { op, left, right } => {
            let op_s = match op {
                BinaryOp::Add => "+",
                BinaryOp::Sub => "-",
                BinaryOp::Mul => "*",
                BinaryOp::Div => "/",
            };
            format!("({} {} {})", format_expr(left), op_s, format_expr(right))
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

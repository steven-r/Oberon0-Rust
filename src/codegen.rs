use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::ast::{BinaryOp, TypeRef, UnaryOp};
use crate::hir::{HDeclaration, HExpr, HModule, HParam, HResolvedIdent, HStatement, HTarget};
use crate::manifest::{CrateBinding, ExternalManifest};

#[derive(Default, Clone, Copy)]
struct IoUsage {
    uses_read_int: bool,
    uses_read_real: bool,
    uses_read_longreal: bool,
    uses_eof: bool,
    uses_write_real: bool,
    uses_write_longreal: bool,
}

impl IoUsage {
    fn any(self) -> bool {
        self.uses_read_int
            || self.uses_read_real
            || self.uses_read_longreal
            || self.uses_eof
            || self.uses_write_real
            || self.uses_write_longreal
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeType {
    Integer,
    Boolean,
    Real,
    LongReal,
    Array,
}

fn runtime_type_from_type_ref(type_ref: Option<&TypeRef>) -> Option<RuntimeType> {
    match type_ref {
        Some(TypeRef::Integer) => Some(RuntimeType::Integer),
        Some(TypeRef::Boolean) => Some(RuntimeType::Boolean),
        Some(TypeRef::Real) => Some(RuntimeType::Real),
        Some(TypeRef::LongReal) => Some(RuntimeType::LongReal),
        Some(TypeRef::Array { .. }) => Some(RuntimeType::Array),
        Some(TypeRef::Named(_)) | Some(TypeRef::Qualified { .. }) => None,
        None => None,
    }
}

fn resolve_named_type<'a>(
    type_ref: &'a TypeRef,
    type_aliases: &'a HashMap<String, TypeRef>,
) -> &'a TypeRef {
    match type_ref {
        TypeRef::Named(name) => match type_aliases.get(name) {
            Some(target) => resolve_named_type(target, type_aliases),
            None => type_ref,
        },
        _ => type_ref,
    }
}

fn collect_type_aliases(module: &HModule) -> HashMap<String, TypeRef> {
    module
        .declarations
        .iter()
        .filter_map(|decl| match decl {
            HDeclaration::Type { name, target, .. } => Some((name.clone(), target.clone())),
            _ => None,
        })
        .collect()
}

fn resolve_runtime_type(
    type_ref: Option<&TypeRef>,
    type_aliases: &HashMap<String, TypeRef>,
) -> Option<RuntimeType> {
    let Some(type_ref) = type_ref else {
        return None;
    };

    match resolve_named_type(type_ref, type_aliases) {
        TypeRef::Array { .. } => Some(RuntimeType::Array),
        resolved => runtime_type_from_type_ref(Some(resolved)),
    }
}

pub fn generate_rust_project(
    module: &HModule,
    manifest: Option<&ExternalManifest>,
    out_root: &std::path::Path,
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
    let procedure_param_modes = collect_procedure_param_modes(module);
    let module_constants = collect_module_constants(module);
    let io_usage = collect_io_usage(module);
    let needs_module_state = statements_need_state_map(&module.statements, &procedure_names);
    let tracks_procedure_state = emit_state && module_has_procedure_locals(module);
    let needs_runtime_state = needs_module_state || tracks_procedure_state;
    let show_state = emit_state && needs_runtime_state;

    out.push_str(&format!(
        "// Generated from Oberon0 module `{}`.\n",
        module.name
    ));
    out.push_str(
        "// Comments preserve the mapping between Oberon0 names and generated Rust bindings.\n\n",
    );
    out.push_str("#![allow(dead_code)]\n");
    out.push_str("#![allow(unused_parens)]\n\n");
    out.push_str("use std::collections::BTreeMap;\n\n");

    if io_usage.any() {
        out.push_str("use std::io::Read;\n");
        out.push_str("use std::sync::{Mutex, OnceLock};\n\n");
    }

    out.push_str("#[allow(dead_code)]\n");
    out.push_str("#[derive(Clone, Debug, PartialEq)]\n");
    out.push_str("enum Value {\n");
    out.push_str("    Integer(i64),\n");
    out.push_str("    Real(f32),\n");
    out.push_str("    LongReal(f64),\n");
    out.push_str("    Array(Vec<Value>),\n");
    out.push_str("}\n\n");

    out.push_str("fn value_integer(value: i64) -> Value {\n");
    out.push_str("    Value::Integer(value)\n");
    out.push_str("}\n\n");
    out.push_str("fn value_real(value: f32) -> Value {\n");
    out.push_str("    Value::Real(value)\n");
    out.push_str("}\n\n");
    out.push_str("fn value_longreal(value: f64) -> Value {\n");
    out.push_str("    Value::LongReal(value)\n");
    out.push_str("}\n\n");
    out.push_str("fn value_array(length: usize) -> Value {\n");
    out.push_str("    Value::Array(vec![Value::Integer(0); length])\n");
    out.push_str("}\n\n");
    out.push_str("fn value_index_from_value(value: &Value) -> usize {\n");
    out.push_str("    match value {\n");
    out.push_str("        Value::Integer(v) => usize::try_from(*v).expect(\"Runtime error: negative array index\"),\n");
    out.push_str("        _ => panic!(\"Runtime error: array index must be INTEGER\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_index(array: &Value, index: &Value) -> Value {\n");
    out.push_str("    let idx = value_index_from_value(index);\n");
    out.push_str("    match array {\n");
    out.push_str("        Value::Array(values) => values.get(idx).cloned().unwrap_or(Value::Integer(0)),\n");
    out.push_str("        _ => panic!(\"Runtime error: indexed access on non-array value\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_set_index(array: &mut Value, index: &Value, new_value: Value) {\n");
    out.push_str("    let idx = value_index_from_value(index);\n");
    out.push_str("    match array {\n");
    out.push_str("        Value::Array(values) => {\n");
    out.push_str("            if idx >= values.len() {\n");
    out.push_str("                values.resize(idx + 1, Value::Integer(0));\n");
    out.push_str("            }\n");
    out.push_str("            values[idx] = new_value;\n");
    out.push_str("        }\n");
    out.push_str("        _ => panic!(\"Runtime error: indexed assignment on non-array value\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_as_real(value: &Value) -> Value {\n");
    out.push_str("    match value {\n");
    out.push_str("        Value::Integer(v) => Value::Real(*v as f32),\n");
    out.push_str("        Value::Real(v) => Value::Real(*v),\n");
    out.push_str("        Value::LongReal(v) => Value::Real(*v as f32),\n");
    out.push_str("        Value::Array(_) => panic!(\"Runtime error: cannot cast ARRAY to REAL\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_as_integer(value: &Value) -> Value {\n");
    out.push_str("    match value {\n");
    out.push_str("        Value::Integer(v) => Value::Integer(*v),\n");
    out.push_str("        Value::Real(v) => Value::Integer(*v as i64),\n");
    out.push_str("        Value::LongReal(v) => Value::Integer(*v as i64),\n");
    out.push_str("        Value::Array(_) => panic!(\"Runtime error: cannot cast ARRAY to INTEGER\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_truthy(value: &Value) -> bool {\n");
    out.push_str("    match value {\n");
    out.push_str("        Value::Integer(v) => *v != 0,\n");
    out.push_str("        Value::Real(v) => *v != 0.0,\n");
    out.push_str("        Value::LongReal(v) => *v != 0.0,\n");
    out.push_str("        Value::Array(v) => !v.is_empty(),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn print_value(value: &Value) {\n");
    out.push_str("    match value {\n");
    out.push_str("        Value::Integer(v) => print!(\"{}\", v),\n");
    out.push_str("        Value::Real(v) => print!(\"{}\", v),\n");
    out.push_str("        Value::LongReal(v) => print!(\"{}\", v),\n");
    out.push_str("        Value::Array(v) => print!(\"[{}]\", v.iter().map(value_to_string).collect::<Vec<_>>().join(\", \")),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn print_value_ln(value: &Value) {\n");
    out.push_str("    match value {\n");
    out.push_str("        Value::Integer(v) => println!(\"{}\", v),\n");
    out.push_str("        Value::Real(v) => println!(\"{}\", v),\n");
    out.push_str("        Value::LongReal(v) => println!(\"{}\", v),\n");
    out.push_str("        Value::Array(v) => println!(\"[{}]\", v.iter().map(value_to_string).collect::<Vec<_>>().join(\", \")),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_add(lhs: &Value, rhs: &Value) -> Value {\n");
    out.push_str("    match (lhs, rhs) {\n");
    out.push_str("        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a + *b),\n");
    out.push_str("        (Value::Real(a), Value::Real(b)) => Value::Real(*a + *b),\n");
    out.push_str("        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a + *b),\n");
    out.push_str("        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 + *b),\n");
    out.push_str("        (Value::Real(a), Value::Integer(b)) => Value::Real(*a + *b as f32),\n");
    out.push_str(
        "        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 + *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a + *b as f64),\n",
    );
    out.push_str(
        "        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 + *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a + *b as f64),\n",
    );
    out.push_str("        _ => panic!(\"Runtime error: unsupported operands for ADD\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_sub(lhs: &Value, rhs: &Value) -> Value {\n");
    out.push_str("    match (lhs, rhs) {\n");
    out.push_str("        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a - *b),\n");
    out.push_str("        (Value::Real(a), Value::Real(b)) => Value::Real(*a - *b),\n");
    out.push_str("        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a - *b),\n");
    out.push_str("        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 - *b),\n");
    out.push_str("        (Value::Real(a), Value::Integer(b)) => Value::Real(*a - *b as f32),\n");
    out.push_str(
        "        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 - *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a - *b as f64),\n",
    );
    out.push_str(
        "        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 - *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a - *b as f64),\n",
    );
    out.push_str("        _ => panic!(\"Runtime error: unsupported operands for SUB\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_mul(lhs: &Value, rhs: &Value) -> Value {\n");
    out.push_str("    match (lhs, rhs) {\n");
    out.push_str("        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a * *b),\n");
    out.push_str("        (Value::Real(a), Value::Real(b)) => Value::Real(*a * *b),\n");
    out.push_str("        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a * *b),\n");
    out.push_str("        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 * *b),\n");
    out.push_str("        (Value::Real(a), Value::Integer(b)) => Value::Real(*a * *b as f32),\n");
    out.push_str(
        "        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 * *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a * *b as f64),\n",
    );
    out.push_str(
        "        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 * *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a * *b as f64),\n",
    );
    out.push_str("        _ => panic!(\"Runtime error: unsupported operands for MUL\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_div(lhs: &Value, rhs: &Value) -> Value {\n");
    out.push_str("    match (lhs, rhs) {\n");
    out.push_str("        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a / *b),\n");
    out.push_str("        (Value::Real(a), Value::Real(b)) => Value::Real(*a / *b),\n");
    out.push_str("        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a / *b),\n");
    out.push_str("        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 / *b),\n");
    out.push_str("        (Value::Real(a), Value::Integer(b)) => Value::Real(*a / *b as f32),\n");
    out.push_str(
        "        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 / *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a / *b as f64),\n",
    );
    out.push_str(
        "        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 / *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a / *b as f64),\n",
    );
    out.push_str("        _ => panic!(\"Runtime error: unsupported operands for DIV\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_neg(value: &Value) -> Value {\n");
    out.push_str("    match value {\n");
    out.push_str("        Value::Integer(v) => Value::Integer(-*v),\n");
    out.push_str("        Value::Real(v) => Value::Real(-*v),\n");
    out.push_str("        Value::LongReal(v) => Value::LongReal(-*v),\n");
    out.push_str("        Value::Array(_) => panic!(\"Runtime error: unary minus on ARRAY\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_not(value: &Value) -> Value {\n");
    out.push_str("    match value {\n");
    out.push_str("        Value::Integer(v) => Value::Integer(if *v != 0 { 0 } else { 1 }),\n");
    out.push_str("        Value::Real(v) => Value::Integer(if *v != 0.0 { 0 } else { 1 }),\n");
    out.push_str("        Value::LongReal(v) => Value::Integer(if *v != 0.0 { 0 } else { 1 }),\n");
    out.push_str("        Value::Array(v) => Value::Integer(if v.is_empty() { 1 } else { 0 }),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("fn value_and(lhs: &Value, rhs: &Value) -> Value {\n");
    out.push_str(
        "    Value::Integer(if value_truthy(lhs) && value_truthy(rhs) { 1 } else { 0 })\n",
    );
    out.push_str("}\n\n");
    out.push_str("fn value_or(lhs: &Value, rhs: &Value) -> Value {\n");
    out.push_str(
        "    Value::Integer(if value_truthy(lhs) || value_truthy(rhs) { 1 } else { 0 })\n",
    );
    out.push_str("}\n\n");
    out.push_str("fn value_mod(lhs: &Value, rhs: &Value) -> Value {\n");
    out.push_str("    match (lhs, rhs) {\n");
    out.push_str("        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a % *b),\n");
    out.push_str("        (Value::Real(a), Value::Real(b)) => Value::Real(*a % *b),\n");
    out.push_str("        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a % *b),\n");
    out.push_str("        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 % *b),\n");
    out.push_str("        (Value::Real(a), Value::Integer(b)) => Value::Real(*a % *b as f32),\n");
    out.push_str(
        "        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 % *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a % *b as f64),\n",
    );
    out.push_str(
        "        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 % *b),\n",
    );
    out.push_str(
        "        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a % *b as f64),\n",
    );
    out.push_str("        _ => panic!(\"Runtime error: unsupported operands for MOD\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str(
        "fn value_bool_from_cmp(lhs: &Value, rhs: &Value, cmp: fn(f64, f64) -> bool) -> Value {\n",
    );
    out.push_str("    match (lhs, rhs) {\n");
    out.push_str("        (Value::Integer(a), Value::Integer(b)) => Value::Integer(if cmp(*a as f64, *b as f64) { 1 } else { 0 }),\n");
    out.push_str("        (Value::Real(a), Value::Real(b)) => Value::Integer(if cmp(*a as f64, *b as f64) { 1 } else { 0 }),\n");
    out.push_str("        (Value::LongReal(a), Value::LongReal(b)) => Value::Integer(if cmp(*a, *b) { 1 } else { 0 }),\n");
    out.push_str("        (Value::Integer(a), Value::Real(b)) => Value::Integer(if cmp(*a as f64, *b as f64) { 1 } else { 0 }),\n");
    out.push_str("        (Value::Real(a), Value::Integer(b)) => Value::Integer(if cmp(*a as f64, *b as f64) { 1 } else { 0 }),\n");
    out.push_str("        (Value::Integer(a), Value::LongReal(b)) => Value::Integer(if cmp(*a as f64, *b) { 1 } else { 0 }),\n");
    out.push_str("        (Value::LongReal(a), Value::Integer(b)) => Value::Integer(if cmp(*a, *b as f64) { 1 } else { 0 }),\n");
    out.push_str("        (Value::Real(a), Value::LongReal(b)) => Value::Integer(if cmp(*a as f64, *b) { 1 } else { 0 }),\n");
    out.push_str("        (Value::LongReal(a), Value::Real(b)) => Value::Integer(if cmp(*a, *b as f64) { 1 } else { 0 }),\n");
    out.push_str("        _ => panic!(\"Runtime error: unsupported operands for comparison\"),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("/// Returns the current value of a module-level Oberon0 variable.\n");
    out.push_str("///\n");
    out.push_str(
        "/// Generated programs keep module state in `vars`, keyed by the original Oberon0 name.\n",
    );
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn get_var(vars: &BTreeMap<String, Value>, name: &str) -> Value {\n");
    out.push_str("    vars.get(name).cloned().unwrap_or(Value::Integer(0))\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn get_var_mut<'a>(vars: &'a mut BTreeMap<String, Value>, name: &str) -> &'a mut Value {\n");
    out.push_str("    vars.entry(name.to_string()).or_insert(Value::Integer(0))\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn set_var_index(vars: &mut BTreeMap<String, Value>, name: &str, index: &Value, value: Value) {\n");
    out.push_str("    let entry = vars.entry(name.to_string()).or_insert_with(|| value_array(0));\n");
    out.push_str("    value_set_index(entry, index, value);\n");
    out.push_str("}\n\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn set_var(vars: &mut BTreeMap<String, Value>, name: &str, value: Value) {\n");
    out.push_str("    vars.insert(name.to_string(), value);\n");
    out.push_str("}\n\n");

    if io_usage.any() {
        out.push_str("#[derive(Default)]\n");
        out.push_str("struct InputState {\n");
        out.push_str("    tokens: Vec<String>,\n");
        out.push_str("    position: usize,\n");
        out.push_str("    initialized: bool,\n");
        out.push_str("}\n\n");

        out.push_str("fn input_state() -> &'static Mutex<InputState> {\n");
        out.push_str("    static STATE: OnceLock<Mutex<InputState>> = OnceLock::new();\n");
        out.push_str("    STATE.get_or_init(|| Mutex::new(InputState::default()))\n");
        out.push_str("}\n\n");

        out.push_str("fn ensure_input_loaded(state: &mut InputState) {\n");
        out.push_str("    if state.initialized {\n");
        out.push_str("        return;\n");
        out.push_str("    }\n");
        out.push_str("\n");
        out.push_str("    let mut input = String::new();\n");
        out.push_str("    std::io::stdin()\n");
        out.push_str("        .read_to_string(&mut input)\n");
        out.push_str("        .expect(\"Runtime IO error: failed to read stdin\");\n");
        out.push_str("\n");
        out.push_str("    state.tokens = input\n");
        out.push_str("        .split_whitespace()\n");
        out.push_str("        .map(|token| token.to_string())\n");
        out.push_str("        .collect();\n");
        out.push_str("    state.position = 0;\n");
        out.push_str("    state.initialized = true;\n");
        out.push_str("}\n\n");

        if io_usage.uses_read_int {
            out.push_str("fn read_int() -> Value {\n");
            out.push_str("    let mut state = input_state()\n");
            out.push_str("        .lock()\n");
            out.push_str("        .expect(\"Runtime IO error: input mutex poisoned\");\n");
            out.push_str("    ensure_input_loaded(&mut state);\n");
            out.push_str("\n");
            out.push_str("    if state.position >= state.tokens.len() {\n");
            out.push_str("        panic!(\"Runtime IO error: ReadInt() reached EOF\");\n");
            out.push_str("    }\n");
            out.push_str("\n");
            out.push_str("    let token = state.tokens[state.position].clone();\n");
            out.push_str("    state.position += 1;\n");
            out.push_str("\n");
            out.push_str("    Value::Integer(token.parse::<i64>().unwrap_or_else(|err| {\n");
            out.push_str("        panic!(\"Runtime IO error: ReadInt() failed to parse integer token '{}' ({})\", token, err)\n");
            out.push_str("    }))\n");
            out.push_str("}\n\n");
        }

        if io_usage.uses_read_real {
            out.push_str("fn read_real() -> Value {\n");
            out.push_str("    let mut state = input_state()\n");
            out.push_str("        .lock()\n");
            out.push_str("        .expect(\"Runtime IO error: input mutex poisoned\");\n");
            out.push_str("    ensure_input_loaded(&mut state);\n");
            out.push_str("\n");
            out.push_str("    if state.position >= state.tokens.len() {\n");
            out.push_str("        panic!(\"Runtime IO error: ReadReal() reached EOF\");\n");
            out.push_str("    }\n");
            out.push_str("\n");
            out.push_str("    let token = state.tokens[state.position].clone();\n");
            out.push_str("    state.position += 1;\n");
            out.push_str("\n");
            out.push_str("    Value::Real(token.parse::<f32>().unwrap_or_else(|err| {\n");
            out.push_str("        panic!(\"Runtime IO error: ReadReal() failed to parse real token '{}' ({})\", token, err)\n");
            out.push_str("    }))\n");
            out.push_str("}\n\n");
        }

        if io_usage.uses_read_longreal {
            out.push_str("fn read_longreal() -> Value {\n");
            out.push_str("    let mut state = input_state()\n");
            out.push_str("        .lock()\n");
            out.push_str("        .expect(\"Runtime IO error: input mutex poisoned\");\n");
            out.push_str("    ensure_input_loaded(&mut state);\n");
            out.push_str("\n");
            out.push_str("    if state.position >= state.tokens.len() {\n");
            out.push_str("        panic!(\"Runtime IO error: ReadLongReal() reached EOF\");\n");
            out.push_str("    }\n");
            out.push_str("\n");
            out.push_str("    let token = state.tokens[state.position].clone();\n");
            out.push_str("    state.position += 1;\n");
            out.push_str("\n");
            out.push_str("    Value::LongReal(token.parse::<f64>().unwrap_or_else(|err| {\n");
            out.push_str("        panic!(\"Runtime IO error: ReadLongReal() failed to parse longreal token '{}' ({})\", token, err)\n");
            out.push_str("    }))\n");
            out.push_str("}\n\n");
        }

        if io_usage.uses_eof {
            out.push_str("fn eof() -> Value {\n");
            out.push_str("    let mut state = input_state()\n");
            out.push_str("        .lock()\n");
            out.push_str("        .expect(\"Runtime IO error: input mutex poisoned\");\n");
            out.push_str("    ensure_input_loaded(&mut state);\n");
            out.push_str("\n");
            out.push_str("    if state.position >= state.tokens.len() {\n");
            out.push_str("        Value::Integer(1)\n");
            out.push_str("    } else {\n");
            out.push_str("        Value::Integer(0)\n");
            out.push_str("    }\n");
            out.push_str("}\n\n");
        }

        if io_usage.uses_write_real {
            out.push_str("fn write_real(value: &Value) {\n");
            out.push_str("    match value {\n");
            out.push_str("        Value::Real(v) => print!(\"{}\", v),\n");
            out.push_str("        Value::Integer(v) => print!(\"{}\", v),\n");
            out.push_str("        Value::LongReal(v) => print!(\"{}\", v),\n");
            out.push_str("        Value::Array(v) => print!(\"[{}]\", v.iter().map(value_to_string).collect::<Vec<_>>().join(\", \")),\n");
            out.push_str("    }\n");
            out.push_str("}\n\n");
        }

        if io_usage.uses_write_longreal {
            out.push_str("fn write_longreal(value: &Value) {\n");
            out.push_str("    match value {\n");
            out.push_str("        Value::Real(v) => print!(\"{}\", v),\n");
            out.push_str("        Value::Integer(v) => print!(\"{}\", v),\n");
            out.push_str("        Value::LongReal(v) => print!(\"{}\", v),\n");
            out.push_str("        Value::Array(v) => print!(\"[{}]\", v.iter().map(value_to_string).collect::<Vec<_>>().join(\", \")),\n");
            out.push_str("    }\n");
            out.push_str("}\n\n");
        }
    }

    out.push_str("fn runtime_state_string(vars: &BTreeMap<String, Value>) -> String {\n");
    out.push_str("    let entries = vars\n");
    out.push_str("        .iter()\n");
    out.push_str(
        "        .map(|(name, value)| format!(\"{:?}: {}\", name, value_to_string(value)))\n",
    );
    out.push_str("        .collect::<Vec<_>>()\n");
    out.push_str("        .join(\", \");\n");
    out.push_str("    format!(\"{{{}}}\", entries)\n");
    out.push_str("}\n\n");
    out.push_str("fn value_to_string(value: &Value) -> String {\n");
    out.push_str("    match value {\n");
    out.push_str("        Value::Integer(v) => v.to_string(),\n");
    out.push_str("        Value::Real(v) => v.to_string(),\n");
    out.push_str("        Value::LongReal(v) => v.to_string(),\n");
    out.push_str("        Value::Array(values) => format!(\"[{}]\", values.iter().map(value_to_string).collect::<Vec<_>>().join(\", \")),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out.push_str("/// Records the current value of a procedure-scoped Oberon0 variable.\n");
    out.push_str("#[allow(dead_code)]\n");
    out.push_str("fn set_procedure_var(vars: &mut BTreeMap<String, Value>, procedure: &str, name: &str, value: &Value) {\n");
    out.push_str("    vars.insert(format!(\"{}.{}\", procedure, name), value.clone());\n");
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
                &procedure_param_modes,
                emit_state,
            ));
            out.push('\n');
        }
    }

    out.push_str(&format!(
        "/// Executes the Oberon0 module `{}`.\n",
        module.name
    ));
    out.push_str("fn main() {\n");
    if needs_runtime_state {
        out.push_str(
            "    // Runtime state keeps module variables and optional procedure-local snapshots.\n",
        );
        out.push_str("    let mut vars: BTreeMap<String, Value> = BTreeMap::new();\n");
    }

    let type_aliases = collect_type_aliases(module);
    let mut module_types = HashMap::new();
    for declaration in &module.declarations {
        match declaration {
            HDeclaration::Var {
                id,
                name,
                declared_type,
            } => {
                if let Some(runtime_type) =
                    resolve_runtime_type(declared_type.as_ref(), &type_aliases)
                {
                    module_types.insert(*id, runtime_type);
                }
                if needs_runtime_state
                    && matches!(
                        resolve_runtime_type(declared_type.as_ref(), &type_aliases),
                        Some(RuntimeType::Array)
                    )
                {
                    out.push_str(&format!(
                        "    vars.insert(\"{}\".to_string(), value_array(0));\n",
                        name
                    ));
                }
            }
            HDeclaration::Procedure { params, .. } => {
                for param in params {
                    if let Some(runtime_type) =
                        resolve_runtime_type(param.declared_type.as_ref(), &type_aliases)
                    {
                        module_types.insert(param.id, runtime_type);
                    }
                }
            }
            _ => {}
        }
    }

    let main_ctx = FormatContext {
        locals: HashMap::new(),
        by_ref_locals: HashSet::new(),
        constants: module_constants,
        procedures: &procedure_names,
        procedure_param_modes: &procedure_param_modes,
        vars_arg: if needs_runtime_state {
            "&mut vars"
        } else {
            "&mut BTreeMap::new()"
        },
        procedure_name: None,
        track_procedure_locals: false,
        types: module_types,
    };

    for stmt in &module.statements {
        out.push_str(&format_statement(stmt, "    ", &main_ctx));
    }

    if show_state {
        out.push_str("    if !vars.is_empty() {\n");
        out.push_str("        println!(\"State: {}\", runtime_state_string(&vars));\n");
        out.push_str("    }\n");
    }
    out.push_str("}\n");

    out
}

struct FormatContext<'a> {
    locals: HashMap<usize, String>,
    by_ref_locals: HashSet<usize>,
    constants: HashMap<usize, &'a HExpr>,
    procedures: &'a HashSet<String>,
    procedure_param_modes: &'a HashMap<String, Vec<bool>>,
    vars_arg: &'a str,
    procedure_name: Option<&'a str>,
    track_procedure_locals: bool,
    types: HashMap<usize, RuntimeType>,
}

fn procedure_ctx_types(ctx: &FormatContext<'_>, id: usize) -> RuntimeType {
    ctx.types.get(&id).copied().unwrap_or(RuntimeType::Integer)
}

fn default_literal(ty: RuntimeType) -> &'static str {
    match ty {
        RuntimeType::Integer | RuntimeType::Boolean => "value_integer(0)",
        RuntimeType::Real => "value_real(0.0)",
        RuntimeType::LongReal => "value_longreal(0.0)",
        RuntimeType::Array => "value_array(0)",
    }
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

fn collect_procedure_param_modes(module: &HModule) -> HashMap<String, Vec<bool>> {
    module
        .declarations
        .iter()
        .filter_map(|decl| match decl {
            HDeclaration::Procedure { name, params, .. } => {
                Some((name.clone(), params.iter().map(|p| p.is_var).collect()))
            }
            _ => None,
        })
        .collect()
}

fn collect_module_constants(module: &HModule) -> HashMap<usize, &HExpr> {
    module
        .declarations
        .iter()
        .filter_map(|decl| match decl {
            HDeclaration::Const { id, value, .. } => Some((*id, value)),
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
        HStatement::Call {
            module: _,
            name,
            args,
        } => procedure_names.contains(&name.name) || args.iter().any(expr_needs_state_map),
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
        HExpr::Integer(_)
        | HExpr::String(_)
        | HExpr::LongReal(_)
        | HExpr::Real(_)
        | HExpr::Boolean(_) => false,
        HExpr::Name(ident) => ident.kind != crate::symbols::SymbolKind::Constant,
        HExpr::Indexed { index, .. } => expr_needs_state_map(index),
        HExpr::Call { args, .. } => args.iter().any(expr_needs_state_map),
        HExpr::Unary { value, .. } => expr_needs_state_map(value),
        HExpr::Binary { left, right, .. } => {
            expr_needs_state_map(left) || expr_needs_state_map(right)
        }
    }
}

fn collect_io_usage(module: &HModule) -> IoUsage {
    let mut usage = IoUsage::default();

    for decl in &module.declarations {
        if let HDeclaration::Procedure { body, .. } = decl {
            usage = merge_io_usage(usage, statements_io_usage(body));
        }
    }

    merge_io_usage(usage, statements_io_usage(&module.statements))
}

fn merge_io_usage(left: IoUsage, right: IoUsage) -> IoUsage {
    IoUsage {
        uses_read_int: left.uses_read_int || right.uses_read_int,
        uses_read_real: left.uses_read_real || right.uses_read_real,
        uses_read_longreal: left.uses_read_longreal || right.uses_read_longreal,
        uses_eof: left.uses_eof || right.uses_eof,
        uses_write_real: left.uses_write_real || right.uses_write_real,
        uses_write_longreal: left.uses_write_longreal || right.uses_write_longreal,
    }
}

fn statements_io_usage(stmts: &[HStatement]) -> IoUsage {
    stmts.iter().fold(IoUsage::default(), |acc, stmt| {
        merge_io_usage(acc, statement_io_usage(stmt))
    })
}

fn statement_io_usage(stmt: &HStatement) -> IoUsage {
    match stmt {
        HStatement::Assign { value, .. } => expr_io_usage(value),
        HStatement::Call { name, args, .. } => {
            let mut usage = IoUsage {
                uses_read_int: name.name == "ReadInt",
                uses_read_real: name.name == "ReadReal",
                uses_read_longreal: name.name == "ReadLongReal",
                uses_eof: name.name == "EOF",
                uses_write_real: name.name == "WriteReal",
                uses_write_longreal: name.name == "WriteLongReal",
            };
            for arg in args {
                usage = merge_io_usage(usage, expr_io_usage(arg));
            }
            usage
        }
        HStatement::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let mut usage = expr_io_usage(condition);
            usage = merge_io_usage(usage, statements_io_usage(then_branch));
            if let Some(branch) = else_branch {
                usage = merge_io_usage(usage, statements_io_usage(branch));
            }
            usage
        }
        HStatement::While { condition, body } => {
            merge_io_usage(expr_io_usage(condition), statements_io_usage(body))
        }
    }
}

fn expr_io_usage(expr: &HExpr) -> IoUsage {
    match expr {
        HExpr::Integer(_)
        | HExpr::String(_)
        | HExpr::Name(_)
        | HExpr::Indexed { .. }
        | HExpr::LongReal(_)
        | HExpr::Real(_)
        | HExpr::Boolean(_) => IoUsage::default(),
        HExpr::Call { name, args } => {
            let mut usage = IoUsage {
                uses_read_int: name.name == "ReadInt",
                uses_read_real: name.name == "ReadReal",
                uses_read_longreal: name.name == "ReadLongReal",
                uses_eof: name.name == "EOF",
                uses_write_real: name.name == "WriteReal",
                uses_write_longreal: name.name == "WriteLongReal",
            };
            for arg in args {
                usage = merge_io_usage(usage, expr_io_usage(arg));
            }
            usage
        }
        HExpr::Unary { value, .. } => expr_io_usage(value),
        HExpr::Binary { left, right, .. } => {
            merge_io_usage(expr_io_usage(left), expr_io_usage(right))
        }
    }
}

fn procedure_assigns_id(body: &[HStatement], ident_id: usize) -> bool {
    body.iter().any(|stmt| statement_assigns_id(stmt, ident_id))
}

fn statement_assigns_id(stmt: &HStatement, ident_id: usize) -> bool {
    match stmt {
        HStatement::Assign { target, .. } => match target {
            HTarget::Name(ident) => ident.id == ident_id,
            HTarget::Indexed { name, .. } => name.id == ident_id,
        },
        HStatement::Call { .. } => false,
        HStatement::If {
            then_branch,
            else_branch,
            ..
        } => {
            then_branch
                .iter()
                .any(|nested| statement_assigns_id(nested, ident_id))
                || else_branch.as_ref().is_some_and(|branch| {
                    branch
                        .iter()
                        .any(|nested| statement_assigns_id(nested, ident_id))
                })
        }
        HStatement::While { body, .. } => body
            .iter()
            .any(|nested| statement_assigns_id(nested, ident_id)),
    }
}

fn format_procedure(
    name: &str,
    params: &[HParam],
    local_vars: &[HResolvedIdent],
    body: &[HStatement],
    constants: &HashMap<usize, &HExpr>,
    procedure_names: &HashSet<String>,
    procedure_param_modes: &HashMap<String, Vec<bool>>,
    emit_state: bool,
) -> String {
    let mut out = String::new();
    let mut locals = HashMap::new();
    let mut types = HashMap::new();

    let mut signature_args = Vec::new();
    signature_args.push("vars: &mut BTreeMap<String, Value>".to_string());

    for param in params {
        let binding = format!("param_{}", param.id);
        locals.insert(param.id, binding.clone());
        let runtime_type = runtime_type_from_type_ref(param.declared_type.as_ref())
            .unwrap_or(RuntimeType::Integer);
        types.insert(param.id, runtime_type);
        if param.is_var {
            signature_args.push(format!("{}: &mut Value", binding));
        } else if procedure_assigns_id(body, param.id) {
            signature_args.push(format!("mut {}: Value", binding));
        } else {
            signature_args.push(format!("{}: Value", binding));
        }
    }

    let ctx = FormatContext {
        locals,
        by_ref_locals: params
            .iter()
            .filter_map(|param| if param.is_var { Some(param.id) } else { None })
            .collect(),
        constants: constants.clone(),
        procedures: procedure_names,
        procedure_param_modes,
        vars_arg: "vars",
        procedure_name: Some(name),
        track_procedure_locals: emit_state,
        types,
    };

    out.push_str(&format!(
        "/// Implements the Oberon0 procedure `{}`.\n",
        name
    ));
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
            if param.is_var {
                out.push_str(&format!(
                    "    set_procedure_var(vars, \"{}\", \"{}\", &*param_{});\n",
                    name, param.name, param.id
                ));
            } else {
                out.push_str(&format!(
                    "    set_procedure_var(vars, \"{}\", \"{}\", &param_{});\n",
                    name, param.name, param.id
                ));
            }
        }
    }

    for local in local_vars {
        out.push_str(&format!(
            "    // Local variable backing the Oberon0 `{}` binding.\n",
            local.name
        ));
        out.push_str("    #[allow(unused_assignments)]\n");
        let local_type = procedure_ctx_types(&ctx, local.id);
        out.push_str(&format!(
            "    let mut local_{}: Value = {};\n",
            local.id,
            default_literal(local_type)
        ));
        if emit_state {
            out.push_str(&format!(
                "    set_procedure_var(vars, \"{}\", \"{}\", &local_{});\n",
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
            let rendered_value = format_top_level_expr(value, ctx);
            match target {
                HTarget::Name(ident) => {
                    if let Some(binding) = ctx.locals.get(&ident.id) {
                        let mut out = String::new();
                        if ctx.by_ref_locals.contains(&ident.id) {
                            out.push_str(&format!("{}*{} = {};\n", indent, binding, rendered_value));
                        } else {
                            out.push_str(&format!("{}{} = {};\n", indent, binding, rendered_value));
                        }
                        if ctx.track_procedure_locals {
                            if let Some(procedure_name) = ctx.procedure_name {
                                if ctx.by_ref_locals.contains(&ident.id) {
                                    out.push_str(&format!(
                                        "{}set_procedure_var(vars, \"{}\", \"{}\", &*{});\n",
                                        indent, procedure_name, ident.name, binding
                                    ));
                                } else {
                                    out.push_str(&format!(
                                        "{}set_procedure_var(vars, \"{}\", \"{}\", &{});\n",
                                        indent, procedure_name, ident.name, binding
                                    ));
                                }
                            }
                        }
                        out
                    } else {
                        format!(
                            "{}vars.insert(\"{}\".to_string(), {});\n",
                            indent, ident.name, rendered_value
                        )
                    }
                }
                HTarget::Indexed { name, index } => {
                    let rendered_index = format_expr(index, ctx);
                    let index_temp = format!("indexed_idx_{}", name.id);
                    let value_temp = format!("indexed_value_{}", name.id);
                    if let Some(binding) = ctx.locals.get(&name.id) {
                        let mut out = String::new();
                        out.push_str(&format!(
                            "{}let {} = ({}).clone();\n",
                            indent, index_temp, rendered_index
                        ));
                        out.push_str(&format!(
                            "{}let {} = ({}).clone();\n",
                            indent, value_temp, rendered_value
                        ));
                        if ctx.by_ref_locals.contains(&name.id) {
                            out.push_str(&format!(
                                "{}value_set_index(&mut *{}, &{}, {});\n",
                                indent, binding, index_temp, value_temp
                            ));
                        } else {
                            out.push_str(&format!(
                                "{}value_set_index(&mut {}, &{}, {});\n",
                                indent, binding, index_temp, value_temp
                            ));
                        }
                        if ctx.track_procedure_locals {
                            if let Some(procedure_name) = ctx.procedure_name {
                                if ctx.by_ref_locals.contains(&name.id) {
                                    out.push_str(&format!(
                                        "{}set_procedure_var(vars, \"{}\", \"{}\", &*{});\n",
                                        indent, procedure_name, name.name, binding
                                    ));
                                } else {
                                    out.push_str(&format!(
                                        "{}set_procedure_var(vars, \"{}\", \"{}\", &{});\n",
                                        indent, procedure_name, name.name, binding
                                    ));
                                }
                            }
                        }
                        out
                    } else {
                        let mut out = String::new();
                        out.push_str(&format!(
                            "{}let {} = ({}).clone();\n",
                            indent, index_temp, rendered_index
                        ));
                        out.push_str(&format!(
                            "{}let {} = ({}).clone();\n",
                            indent, value_temp, rendered_value
                        ));
                        out.push_str(&format!(
                            "{}set_var_index({}, \"{}\", &{}, {});\n",
                            indent, ctx.vars_arg, name.name, index_temp, value_temp
                        ));
                        out
                    }
                }
            }
        }
        HStatement::Call {
            module: _,
            name,
            args,
        } => {
            if name.name == "WriteInt" {
                match args.first() {
                    Some(first) => format!(
                        "{}print_value(&{});\n",
                        indent,
                        format_top_level_expr(first, ctx)
                    ),
                    None => format!("{}print_value(&value_integer(0));\n", indent),
                }
            } else if name.name == "WriteReal" {
                match args.first() {
                    Some(first) => format!(
                        "{}write_real(&{});\n",
                        indent,
                        format_top_level_expr(first, ctx)
                    ),
                    None => format!("{}write_real(0.0);\n", indent),
                }
            } else if name.name == "WriteLongReal" {
                match args.first() {
                    Some(first) => format!(
                        "{}write_longreal(&{});\n",
                        indent,
                        format_top_level_expr(first, ctx)
                    ),
                    None => format!("{}write_longreal(0.0);\n", indent),
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
                let mut out = String::new();
                let mut call_args = Vec::new();
                let mut post_call_updates = Vec::new();
                let param_modes = ctx
                    .procedure_param_modes
                    .get(&name.name)
                    .cloned()
                    .unwrap_or_default();

                for (index, arg) in args.iter().enumerate() {
                    let is_var_param = param_modes.get(index).copied().unwrap_or(false);
                    if is_var_param {
                        match arg {
                            HExpr::Name(ident) => {
                                if let Some(binding) = ctx.locals.get(&ident.id) {
                                    if ctx.by_ref_locals.contains(&ident.id) {
                                        call_args.push(format!("&mut *{}", binding));
                                    } else {
                                        call_args.push(format!("&mut {}", binding));
                                    }
                                } else {
                                    let temp_name = format!("call_arg_{}", index);
                                    out.push_str(&format!(
                                        "{}let mut {} = get_var({}, \"{}\");\n",
                                        indent, temp_name, ctx.vars_arg, ident.name
                                    ));
                                    call_args.push(format!("&mut {}", temp_name));
                                    post_call_updates.push(format!(
                                        "{}set_var({}, \"{}\", {});\n",
                                        indent, ctx.vars_arg, ident.name, temp_name
                                    ));
                                }
                            }
                            _ => {
                                let temp_name = format!("call_arg_{}", index);
                                out.push_str(&format!(
                                    "{}let mut {} = {};\n",
                                    indent,
                                    temp_name,
                                    format_top_level_expr(arg, ctx)
                                ));
                                call_args.push(format!("&mut {}", temp_name));
                            }
                        }
                    } else {
                        let temp_name = format!("call_arg_{}", index);
                        out.push_str(&format!(
                            "{}let {} = {};\n",
                            indent,
                            temp_name,
                            format_top_level_expr(arg, ctx)
                        ));
                        call_args.push(temp_name);
                    }
                }

                let joined_args = if call_args.is_empty() {
                    ctx.vars_arg.to_string()
                } else {
                    format!("{}, {}", ctx.vars_arg, call_args.join(", "))
                };

                out.push_str(&format!("{}{}({});\n", indent, name.name, joined_args));
                for update in post_call_updates {
                    out.push_str(&update);
                }
                out
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
                "{}if value_truthy(&{}) {{\n",
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
                "{}while value_truthy(&{}) {{\n",
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
            format!(
                "{}",
                format_binary_expr(
                    *op,
                    &format_expr(left, ctx),
                    &format_expr(right, ctx),
                    false
                )
            )
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
        HExpr::Integer(v) => format!("value_integer({})", v),
        HExpr::Boolean(v) => format!("value_integer({})", if *v { 1 } else { 0 }),
        HExpr::LongReal(v) => format!("value_longreal({})", v),
        HExpr::Real(v) => format!("value_real({})", v),
        HExpr::String(value) => format!("{:?}", value),
        HExpr::Name(ident) => {
            if let Some(value) = ctx.constants.get(&ident.id) {
                format_expr(value, ctx)
            } else {
                match ctx.locals.get(&ident.id) {
                    Some(binding) => {
                        if ctx.by_ref_locals.contains(&ident.id) {
                            format!("(*{}).clone()", binding)
                        } else {
                            binding.clone()
                        }
                    }
                    None => format!("get_var(&vars, \"{}\")", ident.name),
                }
            }
        }
        HExpr::Indexed { name, index } => {
            let rendered_index = format_expr(index, ctx);
            if let Some(binding) = ctx.locals.get(&name.id) {
                if ctx.by_ref_locals.contains(&name.id) {
                    format!("value_index(&*{}, &{})", binding, rendered_index)
                } else {
                    format!("value_index(&{}, &{})", binding, rendered_index)
                }
            } else {
                format!(
                    "value_index(&get_var(&vars, \"{}\"), &{})",
                    name.name, rendered_index
                )
            }
        }
        HExpr::Call { name, args } => {
            if name.name == "ReadInt" {
                "read_int()".to_string()
            } else if name.name == "ReadReal" {
                "read_real()".to_string()
            } else if name.name == "ReadLongReal" {
                "read_longreal()".to_string()
            } else if name.name == "EOF" {
                "eof()".to_string()
            } else if name.name == "FLT" {
                match args.first() {
                    Some(arg) => format!("value_as_real(&{})", format_expr(arg, ctx)),
                    None => "value_real(0.0)".to_string(),
                }
            } else if name.name == "FLOOR" {
                match args.first() {
                    Some(arg) => format!("value_as_integer(&{})", format_expr(arg, ctx)),
                    None => "value_integer(0)".to_string(),
                }
            } else {
                let rendered_args = args
                    .iter()
                    .map(|arg| format_expr(arg, ctx))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "/* unsupported call expr {}({}) */ 0",
                    name.name, rendered_args
                )
            }
        }
        HExpr::Unary { op, value } => {
            let rendered = format_expr(value, ctx);
            match op {
                UnaryOp::Plus => rendered,
                UnaryOp::Minus => format!("value_neg(&{})", rendered),
                UnaryOp::Not => format!("value_not(&{})", rendered),
            }
        }
        HExpr::Binary { op, left, right } => format_binary_expr(
            *op,
            &format_expr(left, ctx),
            &format_expr(right, ctx),
            false,
        ),
    }
}

fn format_binary_expr(op: BinaryOp, left: &str, right: &str, wrap: bool) -> String {
    let rendered = match op {
        BinaryOp::Add => format!("value_add(&{}, &{})", left, right),
        BinaryOp::Sub => format!("value_sub(&{}, &{})", left, right),
        BinaryOp::Or => format!("value_or(&{}, &{})", left, right),
        BinaryOp::Mul => format!("value_mul(&{}, &{})", left, right),
        BinaryOp::Div => format!("value_div(&{}, &{})", left, right),
        BinaryOp::IntDiv => format!("value_div(&{}, &{})", left, right),
        BinaryOp::Mod => format!("value_mod(&{}, &{})", left, right),
        BinaryOp::And => format!("value_and(&{}, &{})", left, right),
        BinaryOp::Eq => format!("value_bool_from_cmp(&{}, &{}, |a, b| a == b)", left, right),
        BinaryOp::Ne => format!("value_bool_from_cmp(&{}, &{}, |a, b| a != b)", left, right),
        BinaryOp::Lt => format!("value_bool_from_cmp(&{}, &{}, |a, b| a < b)", left, right),
        BinaryOp::Le => format!("value_bool_from_cmp(&{}, &{}, |a, b| a <= b)", left, right),
        BinaryOp::Gt => format!("value_bool_from_cmp(&{}, &{}, |a, b| a > b)", left, right),
        BinaryOp::Ge => format!("value_bool_from_cmp(&{}, &{}, |a, b| a >= b)", left, right),
    };

    if wrap {
        format!("({})", rendered)
    } else {
        rendered
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests;

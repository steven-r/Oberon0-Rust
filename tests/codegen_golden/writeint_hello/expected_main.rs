// Generated from Oberon0 module `Main`.
// Comments preserve the mapping between Oberon0 names and generated Rust bindings.

#![allow(dead_code)]
#![allow(unused_parens)]

use std::collections::BTreeMap;

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Integer(i64),
    Real(f32),
    LongReal(f64),
    Array(Vec<Value>),
    Record(BTreeMap<String, Value>),
}

fn value_integer(value: i64) -> Value {
    Value::Integer(value)
}

fn value_real(value: f32) -> Value {
    Value::Real(value)
}

fn value_longreal(value: f64) -> Value {
    Value::LongReal(value)
}

fn value_array(length: usize) -> Value {
    Value::Array(vec![Value::Integer(0); length])
}

fn value_record() -> Value {
    Value::Record(BTreeMap::new())
}

fn value_index_from_value(value: &Value) -> usize {
    match value {
        Value::Integer(v) => usize::try_from(*v).expect("Runtime error: negative array index"),
        _ => panic!("Runtime error: array index must be INTEGER"),
    }
}

fn value_index(array: &Value, index: &Value) -> Value {
    let idx = value_index_from_value(index);
    match array {
        Value::Array(values) => values.get(idx).cloned().unwrap_or(Value::Integer(0)),
        _ => panic!("Runtime error: indexed access on non-array value"),
    }
}

fn value_set_index(array: &mut Value, index: &Value, new_value: Value) {
    let idx = value_index_from_value(index);
    match array {
        Value::Array(values) => {
            if idx >= values.len() {
                values.resize(idx + 1, Value::Integer(0));
            }
            values[idx] = new_value;
        }
        _ => panic!("Runtime error: indexed assignment on non-array value"),
    }
}

fn value_field(record: &Value, field: &str) -> Value {
    match record {
        Value::Record(fields) => fields.get(field).cloned().unwrap_or(Value::Integer(0)),
        _ => panic!("Runtime error: field access on non-record value"),
    }
}

fn value_set_field(record: &mut Value, field: &str, new_value: Value) {
    match record {
        Value::Record(fields) => {
            fields.insert(field.to_string(), new_value);
        }
        _ => panic!("Runtime error: field assignment on non-record value"),
    }
}

fn value_as_real(value: &Value) -> Value {
    match value {
        Value::Integer(v) => Value::Real(*v as f32),
        Value::Real(v) => Value::Real(*v),
        Value::LongReal(v) => Value::Real(*v as f32),
        Value::Array(_) => panic!("Runtime error: cannot cast ARRAY to REAL"),
        Value::Record(_) => panic!("Runtime error: cannot cast RECORD to REAL"),
    }
}

fn value_as_integer(value: &Value) -> Value {
    match value {
        Value::Integer(v) => Value::Integer(*v),
        Value::Real(v) => Value::Integer(*v as i64),
        Value::LongReal(v) => Value::Integer(*v as i64),
        Value::Array(_) => panic!("Runtime error: cannot cast ARRAY to INTEGER"),
        Value::Record(_) => panic!("Runtime error: cannot cast RECORD to INTEGER"),
    }
}

fn value_truthy(value: &Value) -> bool {
    match value {
        Value::Integer(v) => *v != 0,
        Value::Real(v) => *v != 0.0,
        Value::LongReal(v) => *v != 0.0,
        Value::Array(v) => !v.is_empty(),
        Value::Record(v) => !v.is_empty(),
    }
}

fn print_value(value: &Value) {
    match value {
        Value::Integer(v) => print!("{}", v),
        Value::Real(v) => print!("{}", v),
        Value::LongReal(v) => print!("{}", v),
        Value::Array(v) => print!("[{}]", v.iter().map(value_to_string).collect::<Vec<_>>().join(", ")),
        Value::Record(v) => print!("{{{}}}", v.iter().map(|(k, val)| format!("{}: {}", k, value_to_string(val))).collect::<Vec<_>>().join(", ")),
    }
}

fn print_value_ln(value: &Value) {
    match value {
        Value::Integer(v) => println!("{}", v),
        Value::Real(v) => println!("{}", v),
        Value::LongReal(v) => println!("{}", v),
        Value::Array(v) => println!("[{}]", v.iter().map(value_to_string).collect::<Vec<_>>().join(", ")),
        Value::Record(v) => println!("{{{}}}", v.iter().map(|(k, val)| format!("{}: {}", k, value_to_string(val))).collect::<Vec<_>>().join(", ")),
    }
}

fn value_add(lhs: &Value, rhs: &Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a + *b),
        (Value::Real(a), Value::Real(b)) => Value::Real(*a + *b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a + *b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 + *b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(*a + *b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 + *b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a + *b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 + *b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a + *b as f64),
        _ => panic!("Runtime error: unsupported operands for ADD"),
    }
}

fn value_sub(lhs: &Value, rhs: &Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a - *b),
        (Value::Real(a), Value::Real(b)) => Value::Real(*a - *b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a - *b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 - *b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(*a - *b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 - *b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a - *b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 - *b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a - *b as f64),
        _ => panic!("Runtime error: unsupported operands for SUB"),
    }
}

fn value_mul(lhs: &Value, rhs: &Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a * *b),
        (Value::Real(a), Value::Real(b)) => Value::Real(*a * *b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a * *b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 * *b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(*a * *b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 * *b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a * *b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 * *b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a * *b as f64),
        _ => panic!("Runtime error: unsupported operands for MUL"),
    }
}

fn value_div(lhs: &Value, rhs: &Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a / *b),
        (Value::Real(a), Value::Real(b)) => Value::Real(*a / *b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a / *b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 / *b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(*a / *b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 / *b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a / *b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 / *b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a / *b as f64),
        _ => panic!("Runtime error: unsupported operands for DIV"),
    }
}

fn value_neg(value: &Value) -> Value {
    match value {
        Value::Integer(v) => Value::Integer(-*v),
        Value::Real(v) => Value::Real(-*v),
        Value::LongReal(v) => Value::LongReal(-*v),
        Value::Array(_) => panic!("Runtime error: unary minus on ARRAY"),
        Value::Record(_) => panic!("Runtime error: unary minus on RECORD"),
    }
}

fn value_not(value: &Value) -> Value {
    match value {
        Value::Integer(v) => Value::Integer(if *v != 0 { 0 } else { 1 }),
        Value::Real(v) => Value::Integer(if *v != 0.0 { 0 } else { 1 }),
        Value::LongReal(v) => Value::Integer(if *v != 0.0 { 0 } else { 1 }),
        Value::Array(v) => Value::Integer(if v.is_empty() { 1 } else { 0 }),
        Value::Record(v) => Value::Integer(if v.is_empty() { 1 } else { 0 }),
    }
}

fn value_and(lhs: &Value, rhs: &Value) -> Value {
    Value::Integer(if value_truthy(lhs) && value_truthy(rhs) { 1 } else { 0 })
}

fn value_or(lhs: &Value, rhs: &Value) -> Value {
    Value::Integer(if value_truthy(lhs) || value_truthy(rhs) { 1 } else { 0 })
}

fn value_mod(lhs: &Value, rhs: &Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(*a % *b),
        (Value::Real(a), Value::Real(b)) => Value::Real(*a % *b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(*a % *b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(*a as f32 % *b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(*a % *b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(*a as f64 % *b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(*a % *b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(*a as f64 % *b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(*a % *b as f64),
        _ => panic!("Runtime error: unsupported operands for MOD"),
    }
}

fn value_bool_from_cmp(lhs: &Value, rhs: &Value, cmp: fn(f64, f64) -> bool) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(if cmp(*a as f64, *b as f64) { 1 } else { 0 }),
        (Value::Real(a), Value::Real(b)) => Value::Integer(if cmp(*a as f64, *b as f64) { 1 } else { 0 }),
        (Value::LongReal(a), Value::LongReal(b)) => Value::Integer(if cmp(*a, *b) { 1 } else { 0 }),
        (Value::Integer(a), Value::Real(b)) => Value::Integer(if cmp(*a as f64, *b as f64) { 1 } else { 0 }),
        (Value::Real(a), Value::Integer(b)) => Value::Integer(if cmp(*a as f64, *b as f64) { 1 } else { 0 }),
        (Value::Integer(a), Value::LongReal(b)) => Value::Integer(if cmp(*a as f64, *b) { 1 } else { 0 }),
        (Value::LongReal(a), Value::Integer(b)) => Value::Integer(if cmp(*a, *b as f64) { 1 } else { 0 }),
        (Value::Real(a), Value::LongReal(b)) => Value::Integer(if cmp(*a as f64, *b) { 1 } else { 0 }),
        (Value::LongReal(a), Value::Real(b)) => Value::Integer(if cmp(*a, *b as f64) { 1 } else { 0 }),
        _ => panic!("Runtime error: unsupported operands for comparison"),
    }
}

/// Returns the current value of a module-level Oberon0 variable.
///
/// Generated programs keep module state in `vars`, keyed by the original Oberon0 name.
#[allow(dead_code)]
fn get_var(vars: &BTreeMap<String, Value>, name: &str) -> Value {
    vars.get(name).cloned().unwrap_or(Value::Integer(0))
}

#[allow(dead_code)]
fn get_var_mut<'a>(vars: &'a mut BTreeMap<String, Value>, name: &str) -> &'a mut Value {
    vars.entry(name.to_string()).or_insert(Value::Integer(0))
}

#[allow(dead_code)]
fn set_var_index(vars: &mut BTreeMap<String, Value>, name: &str, index: &Value, value: Value) {
    let entry = vars.entry(name.to_string()).or_insert_with(|| value_array(0));
    value_set_index(entry, index, value);
}

#[allow(dead_code)]
fn set_var_field(vars: &mut BTreeMap<String, Value>, name: &str, field: &str, value: Value) {
    let entry = vars.entry(name.to_string()).or_insert_with(value_record);
    value_set_field(entry, field, value);
}

#[allow(dead_code)]
fn set_var(vars: &mut BTreeMap<String, Value>, name: &str, value: Value) {
    vars.insert(name.to_string(), value);
}

fn runtime_state_string(vars: &BTreeMap<String, Value>) -> String {
    let entries = vars
        .iter()
        .map(|(name, value)| format!("{:?}: {}", name, value_to_string(value)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{}}}", entries)
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Integer(v) => v.to_string(),
        Value::Real(v) => v.to_string(),
        Value::LongReal(v) => v.to_string(),
        Value::Array(values) => format!("[{}]", values.iter().map(value_to_string).collect::<Vec<_>>().join(", ")),
        Value::Record(fields) => format!("{{{}}}", fields.iter().map(|(name, value)| format!("{}: {}", name, value_to_string(value))).collect::<Vec<_>>().join(", ")),
    }
}

/// Records the current value of a procedure-scoped Oberon0 variable.
#[allow(dead_code)]
fn set_procedure_var(vars: &mut BTreeMap<String, Value>, procedure: &str, name: &str, value: &Value) {
    vars.insert(format!("{}.{}", procedure, name), value.clone());
}

/// Executes the Oberon0 module `Main`.
fn main() {
    // Runtime state keeps module variables and optional procedure-local snapshots.
    let mut vars: BTreeMap<String, Value> = BTreeMap::new();
    vars.insert("x".to_string(), value_integer(7));
    print_value(&get_var(&vars, "x"));
    println!();
}

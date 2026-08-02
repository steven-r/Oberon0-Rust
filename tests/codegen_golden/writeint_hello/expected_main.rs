// Generated from Oberon0 module `Main`.
// Comments preserve the mapping between Oberon0 names and generated Rust bindings.

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Integer(i64),
    Real(f32),
    LongReal(f64),
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

fn value_as_real(value: Value) -> Value {
    match value {
        Value::Integer(v) => Value::Real(v as f32),
        Value::Real(v) => Value::Real(v),
        Value::LongReal(v) => Value::Real(v as f32),
    }
}

fn value_as_integer(value: Value) -> Value {
    match value {
        Value::Integer(v) => Value::Integer(v),
        Value::Real(v) => Value::Integer(v as i64),
        Value::LongReal(v) => Value::Integer(v as i64),
    }
}

fn value_truthy(value: &Value) -> bool {
    match value {
        Value::Integer(v) => *v != 0,
        Value::Real(v) => *v != 0.0,
        Value::LongReal(v) => *v != 0.0,
    }
}

fn print_value(value: &Value) {
    match value {
        Value::Integer(v) => print!("{}", v),
        Value::Real(v) => print!("{}", v),
        Value::LongReal(v) => print!("{}", v),
    }
}

fn print_value_ln(value: &Value) {
    match value {
        Value::Integer(v) => println!("{}", v),
        Value::Real(v) => println!("{}", v),
        Value::LongReal(v) => println!("{}", v),
    }
}

fn value_add(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a + b),
        (Value::Real(a), Value::Real(b)) => Value::Real(a + b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(a + b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(a as f32 + b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(a + b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(a as f64 + b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(a + b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(a as f64 + b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(a + b as f64),
    }
}

fn value_sub(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a - b),
        (Value::Real(a), Value::Real(b)) => Value::Real(a - b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(a - b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(a as f32 - b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(a - b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(a as f64 - b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(a - b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(a as f64 - b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(a - b as f64),
    }
}

fn value_mul(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a * b),
        (Value::Real(a), Value::Real(b)) => Value::Real(a * b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(a * b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(a as f32 * b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(a * b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(a as f64 * b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(a * b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(a as f64 * b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(a * b as f64),
    }
}

fn value_div(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a / b),
        (Value::Real(a), Value::Real(b)) => Value::Real(a / b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(a / b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(a as f32 / b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(a / b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(a as f64 / b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(a / b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(a as f64 / b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(a / b as f64),
    }
}

fn value_neg(value: Value) -> Value {
    match value {
        Value::Integer(v) => Value::Integer(-v),
        Value::Real(v) => Value::Real(-v),
        Value::LongReal(v) => Value::LongReal(-v),
    }
}

fn value_not(value: Value) -> Value {
    match value {
        Value::Integer(v) => Value::Integer(if v != 0 { 0 } else { 1 }),
        Value::Real(v) => Value::Integer(if v != 0.0 { 0 } else { 1 }),
        Value::LongReal(v) => Value::Integer(if v != 0.0 { 0 } else { 1 }),
    }
}

fn value_and(lhs: Value, rhs: Value) -> Value {
    Value::Integer(if value_truthy(&lhs) && value_truthy(&rhs) { 1 } else { 0 })
}

fn value_or(lhs: Value, rhs: Value) -> Value {
    Value::Integer(if value_truthy(&lhs) || value_truthy(&rhs) { 1 } else { 0 })
}

fn value_mod(lhs: Value, rhs: Value) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(a % b),
        (Value::Real(a), Value::Real(b)) => Value::Real(a % b),
        (Value::LongReal(a), Value::LongReal(b)) => Value::LongReal(a % b),
        (Value::Integer(a), Value::Real(b)) => Value::Real(a as f32 % b),
        (Value::Real(a), Value::Integer(b)) => Value::Real(a % b as f32),
        (Value::Integer(a), Value::LongReal(b)) => Value::LongReal(a as f64 % b),
        (Value::LongReal(a), Value::Integer(b)) => Value::LongReal(a % b as f64),
        (Value::Real(a), Value::LongReal(b)) => Value::LongReal(a as f64 % b),
        (Value::LongReal(a), Value::Real(b)) => Value::LongReal(a % b as f64),
    }
}

fn value_bool_from_cmp(lhs: Value, rhs: Value, cmp: fn(f64, f64) -> bool) -> Value {
    match (lhs, rhs) {
        (Value::Integer(a), Value::Integer(b)) => Value::Integer(if cmp(a as f64, b as f64) { 1 } else { 0 }),
        (Value::Real(a), Value::Real(b)) => Value::Integer(if cmp(a as f64, b as f64) { 1 } else { 0 }),
        (Value::LongReal(a), Value::LongReal(b)) => Value::Integer(if cmp(a, b) { 1 } else { 0 }),
        (Value::Integer(a), Value::Real(b)) => Value::Integer(if cmp(a as f64, b as f64) { 1 } else { 0 }),
        (Value::Real(a), Value::Integer(b)) => Value::Integer(if cmp(a as f64, b as f64) { 1 } else { 0 }),
        (Value::Integer(a), Value::LongReal(b)) => Value::Integer(if cmp(a as f64, b) { 1 } else { 0 }),
        (Value::LongReal(a), Value::Integer(b)) => Value::Integer(if cmp(a, b as f64) { 1 } else { 0 }),
        (Value::Real(a), Value::LongReal(b)) => Value::Integer(if cmp(a as f64, b) { 1 } else { 0 }),
        (Value::LongReal(a), Value::Real(b)) => Value::Integer(if cmp(a, b as f64) { 1 } else { 0 }),
    }
}

/// Returns the current value of a module-level Oberon0 variable.
///
/// Generated programs keep module state in `vars`, keyed by the original Oberon0 name.
#[allow(dead_code)]
fn get_var(vars: &BTreeMap<String, Value>, name: &str) -> Value {
    vars.get(name).cloned().unwrap_or(Value::Integer(0))
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

// Generated from Oberon0 module `Main`.
// Comments preserve the mapping between Oberon0 names and generated Rust bindings.

use std::collections::BTreeMap;

/// Returns the current value of a module-level Oberon0 variable.
///
/// Generated programs keep module state in `vars`, keyed by the original Oberon0 name.
#[allow(dead_code)]
fn get_var(vars: &BTreeMap<String, i64>, name: &str) -> i64 {
    *vars.get(name).unwrap_or(&0)
}

/// Records the current value of a procedure-scoped Oberon0 variable.
#[allow(dead_code)]
fn set_procedure_var(vars: &mut BTreeMap<String, i64>, procedure: &str, name: &str, value: i64) {
    vars.insert(format!("{}.{}", procedure, name), value);
}

/// Executes the Oberon0 module `Main`.
fn main() {
    // Runtime state keeps module variables and optional procedure-local snapshots.
    let mut vars: BTreeMap<String, i64> = BTreeMap::new();
    vars.insert("x".to_string(), 7);
    print!("{}", get_var(&vars, "x"));
    println!();
}

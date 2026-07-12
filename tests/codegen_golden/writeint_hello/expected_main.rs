// Generated from Oberon0 module `Main`.
// Comments preserve the mapping between Oberon0 names and generated Rust bindings.

use std::collections::BTreeMap;

use std::io::Read;
use std::sync::{Mutex, OnceLock};

/// Returns the current value of a module-level Oberon0 variable.
///
/// Generated programs keep module state in `vars`, keyed by the original Oberon0 name.
#[allow(dead_code)]
fn get_var(vars: &BTreeMap<String, i64>, name: &str) -> i64 {
    *vars.get(name).unwrap_or(&0)
}

#[derive(Default)]
struct InputState {
    tokens: Vec<String>,
    position: usize,
    initialized: bool,
}

fn input_state() -> &'static Mutex<InputState> {
    static STATE: OnceLock<Mutex<InputState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(InputState::default()))
}

fn ensure_input_loaded(state: &mut InputState) {
    if state.initialized {
        return;
    }

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .expect("Runtime IO error: failed to read stdin");

    state.tokens = input
        .split_whitespace()
        .map(|token| token.to_string())
        .collect();
    state.position = 0;
    state.initialized = true;
}

fn read_int() -> i64 {
    let mut state = input_state()
        .lock()
        .expect("Runtime IO error: input mutex poisoned");
    ensure_input_loaded(&mut state);

    if state.position >= state.tokens.len() {
        panic!("Runtime IO error: ReadInt() reached EOF");
    }

    let token = state.tokens[state.position].clone();
    state.position += 1;

    token.parse::<i64>().unwrap_or_else(|err| {
        panic!("Runtime IO error: ReadInt() failed to parse integer token '{}' ({})", token, err)
    })
}

fn eof() -> i64 {
    let mut state = input_state()
        .lock()
        .expect("Runtime IO error: input mutex poisoned");
    ensure_input_loaded(&mut state);

    if state.position >= state.tokens.len() {
        1
    } else {
        0
    }
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

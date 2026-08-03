
use super::ScopedMap;

#[test]
fn nested_scope_allows_shadowing_and_restores_outer_on_exit() {
    let mut map = ScopedMap::new();
    map.declare("x", 1, |_| "duplicate")
        .expect("outer declaration should succeed");
    assert_eq!(map.resolve("x"), Some(&1));

    map.enter_scope();
    map.declare("x", 2, |_| "duplicate")
        .expect("inner shadow declaration should succeed");
    assert_eq!(map.resolve("x"), Some(&2));

    map.exit_scope();
    assert_eq!(map.resolve("x"), Some(&1));
}

#[test]
fn duplicate_symbol_in_same_scope_is_rejected() {
    let mut map = ScopedMap::new();
    map.declare("x", 1, |_| "duplicate")
        .expect("first declaration should succeed");

    let err = map.declare("x", 2, |name| format!("duplicate: {name}"));
    assert_eq!(err.err(), Some("duplicate: x".to_string()));
}

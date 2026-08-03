
use super::{SymbolKind, SymbolTable};
use crate::semantic::SemanticError;

#[test]
fn declares_and_resolves_symbol_in_current_scope() {
    let mut table = SymbolTable::new();
    table
        .declare("x", SymbolKind::Variable)
        .expect("declaration should succeed");

    let resolved = table.resolve("x").expect("symbol should resolve");
    assert_eq!(resolved.kind, SymbolKind::Variable);
    assert_eq!(resolved.scope_depth, 0);
}

#[test]
fn duplicate_symbol_in_same_scope_is_rejected() {
    let mut table = SymbolTable::new();
    table
        .declare("x", SymbolKind::Variable)
        .expect("first declaration should succeed");
    let err = table
        .declare("x", SymbolKind::Variable)
        .expect_err("duplicate declaration should fail");

    match err {
        SemanticError::DuplicateSymbol { name } => assert_eq!(name, "x"),
        other => panic!("expected DuplicateSymbol, got {other:?}"),
    }
}

#[test]
fn nested_scope_allows_shadowing_and_restores_outer_on_exit() {
    let mut table = SymbolTable::new();
    table
        .declare("x", SymbolKind::Variable)
        .expect("outer declaration should succeed");
    let outer_depth = table
        .resolve("x")
        .expect("outer symbol should resolve")
        .scope_depth;

    table.enter_scope();
    table
        .declare("x", SymbolKind::Parameter)
        .expect("inner shadow declaration should succeed");
    let inner = table.resolve("x").expect("inner symbol should resolve");
    assert_eq!(inner.kind, SymbolKind::Parameter);
    assert_eq!(inner.scope_depth, 1);

    table.exit_scope();
    let outer = table
        .resolve("x")
        .expect("outer symbol should resolve again");
    assert_eq!(outer.kind, SymbolKind::Variable);
    assert_eq!(outer.scope_depth, outer_depth);
}

#![allow(dead_code)]

//! Semantic symbol table primitives shared across analysis and lowering.

use crate::scope::ScopedMap;
use crate::semantic::SemanticError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Classification of symbols visible to the Oberon0 front-end.
pub enum SymbolKind {
    Variable,
    Constant,
    Procedure,
    Parameter,
}

#[derive(Debug, Clone)]
/// Symbol table entry with source name, category, and lexical scope depth.
pub struct Symbol {
    /// Source-level identifier text.
    pub name: String,
    /// Resolved kind of this symbol.
    pub kind: SymbolKind,
    /// Scope depth where the symbol was declared.
    pub scope_depth: usize,
}

#[derive(Debug)]
/// Lexically scoped symbol table used during semantic analysis.
pub struct SymbolTable {
    scopes: ScopedMap<Symbol>,
}

impl SymbolTable {
    /// Creates a symbol table with a root scope.
    pub fn new() -> Self {
        Self {
            scopes: ScopedMap::new(),
        }
    }

    /// Returns the current lexical depth.
    pub fn depth(&self) -> usize {
        self.scopes.depth()
    }

    /// Enters a nested lexical scope.
    pub fn enter_scope(&mut self) {
        self.scopes.enter_scope();
    }

    /// Exits the current lexical scope.
    pub fn exit_scope(&mut self) {
        self.scopes.exit_scope();
    }

    /// Declares a symbol in the current scope.
    pub fn declare(&mut self, name: &str, kind: SymbolKind) -> Result<(), SemanticError> {
        let depth = self.depth();
        self.scopes.declare(
            name,
            Symbol {
                name: name.to_string(),
                kind,
                scope_depth: depth,
            },
            |name| SemanticError::DuplicateSymbol {
                name: name.to_string(),
            },
        )
    }

    /// Resolves a name using lexical scoping rules.
    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        self.scopes.resolve(name)
    }
}

#[cfg(test)]
mod tests {
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
        let outer_depth = table.resolve("x").expect("outer symbol should resolve").scope_depth;

        table.enter_scope();
        table
            .declare("x", SymbolKind::Parameter)
            .expect("inner shadow declaration should succeed");
        let inner = table.resolve("x").expect("inner symbol should resolve");
        assert_eq!(inner.kind, SymbolKind::Parameter);
        assert_eq!(inner.scope_depth, 1);

        table.exit_scope();
        let outer = table.resolve("x").expect("outer symbol should resolve again");
        assert_eq!(outer.kind, SymbolKind::Variable);
        assert_eq!(outer.scope_depth, outer_depth);
    }
}

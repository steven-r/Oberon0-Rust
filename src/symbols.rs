#![allow(dead_code)]

//! Semantic symbol table primitives shared across analysis and lowering.

use crate::ast::TypeRef;
use crate::scope::ScopedMap;
use crate::semantic::SemanticError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Classification of symbols visible to the Oberon0 front-end.
pub enum SymbolKind {
    Variable,
    Constant,
    TypeName,
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
    /// Declared type information carried for typed declarations.
    pub declared_type: Option<TypeRef>,
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
        self.declare_with_type(name, kind, None)
    }

    /// Declares a symbol in the current scope with optional declared type data.
    pub fn declare_with_type(
        &mut self,
        name: &str,
        kind: SymbolKind,
        declared_type: Option<TypeRef>,
    ) -> Result<(), SemanticError> {
        let depth = self.depth();
        self.scopes.declare(
            name,
            Symbol {
                name: name.to_string(),
                kind,
                declared_type,
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
mod tests;

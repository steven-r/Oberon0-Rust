#![allow(dead_code)]

use std::collections::HashMap;

use crate::semantic::SemanticError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Variable,
    Constant,
    Procedure,
    Parameter,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub scope_depth: usize,
}

#[derive(Debug, Default)]
struct Scope {
    symbols: HashMap<String, Symbol>,
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    scopes: Vec<Scope>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self::default();
        table.enter_scope();
        table
    }

    pub fn depth(&self) -> usize {
        self.scopes.len().saturating_sub(1)
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::default());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn declare(&mut self, name: &str, kind: SymbolKind) -> Result<(), SemanticError> {
        let depth = self.depth();
        let scope = self
            .scopes
            .last_mut()
            .expect("symbol table must always have an active scope");

        if scope.symbols.contains_key(name) {
            return Err(SemanticError::DuplicateSymbol {
                name: name.to_string(),
            });
        }

        scope.symbols.insert(
            name.to_string(),
            Symbol {
                name: name.to_string(),
                kind,
                scope_depth: depth,
            },
        );

        Ok(())
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.symbols.get(name))
    }
}

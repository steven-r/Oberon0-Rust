#![allow(dead_code)]

//! Lowered, name-resolved representation used by code generation.

use crate::ast::BinaryOp;
use crate::symbols::SymbolKind;

#[derive(Debug, Clone)]
/// Fully lowered module with resolved identifiers and stable symbol ids.
pub struct HModule {
    /// Module name declared after the `MODULE` keyword.
    pub name: String,
    /// Name repeated after the closing `END` keyword.
    pub end_name: String,
    /// Imported aliases after manifest and symbol-table resolution.
    pub imports: Vec<HImportDecl>,
    /// Lowered declarations with stable identifiers.
    pub declarations: Vec<HDeclaration>,
    /// Lowered executable statements for the module body.
    pub statements: Vec<HStatement>,
}

#[derive(Debug, Clone)]
/// Lowered import alias that preserves both local and external names.
pub struct HImportDecl {
    /// Alias used inside the current module.
    pub local_name: String,
    /// External manifest key backing the alias.
    pub external_name: String,
}

#[derive(Debug, Clone)]
/// Identifier annotated with a stable numeric id and resolved symbol kind.
pub struct HResolvedIdent {
    /// Compiler-assigned id that remains stable across later lowering steps.
    pub id: usize,
    /// Original source-level identifier text.
    pub name: String,
    /// Resolved kind used by semantic checks and code generation.
    pub kind: SymbolKind,
}

#[derive(Debug, Clone)]
/// Lowered procedure parameter with a stable id.
pub struct HParam {
    /// Compiler-assigned id for this parameter binding.
    pub id: usize,
    /// Original source-level parameter name.
    pub name: String,
}

#[derive(Debug, Clone)]
/// Lowered declaration nodes.
pub enum HDeclaration {
    /// Constant declaration with its resolved id.
    Const {
        id: usize,
        name: String,
        value: i64,
    },
    /// Variable declaration with its resolved id.
    Var {
        id: usize,
        name: String,
    },
    /// Procedure declaration with resolved parameters and local variables.
    Procedure {
        id: usize,
        name: String,
        params: Vec<HParam>,
        local_vars: Vec<HResolvedIdent>,
        body: Vec<HStatement>,
        end_name: String,
    },
}

#[derive(Debug, Clone)]
/// Lowered statements whose identifiers already resolve to symbols.
pub enum HStatement {
    /// Assignment to a resolved variable or parameter binding.
    Assign {
        target: HResolvedIdent,
        value: HExpr,
    },
    /// Call to a resolved procedure symbol.
    Call {
        name: HResolvedIdent,
        args: Vec<HExpr>,
    },
    /// Lowered conditional branch.
    If {
        condition: HExpr,
        then_branch: Vec<HStatement>,
        else_branch: Option<Vec<HStatement>>,
    },
    /// Lowered while loop.
    While {
        condition: HExpr,
        body: Vec<HStatement>,
    },
}

#[derive(Debug, Clone)]
/// Lowered expressions over resolved identifiers.
pub enum HExpr {
    /// Integer literal.
    Integer(i64),
    /// String literal after parser unescaping.
    String(String),
    /// Reference to a resolved identifier binding.
    Name(HResolvedIdent),
    /// Binary arithmetic expression.
    Binary {
        op: BinaryOp,
        left: Box<HExpr>,
        right: Box<HExpr>,
    },
}

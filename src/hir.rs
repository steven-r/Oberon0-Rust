#![allow(dead_code)]

use crate::ast::BinaryOp;
use crate::symbols::SymbolKind;

#[derive(Debug, Clone)]
pub struct HModule {
    pub name: String,
    pub end_name: String,
    pub imports: Vec<HImportDecl>,
    pub declarations: Vec<HDeclaration>,
    pub statements: Vec<HStatement>,
}

#[derive(Debug, Clone)]
pub struct HImportDecl {
    pub local_name: String,
    pub external_name: String,
}

#[derive(Debug, Clone)]
pub struct HResolvedIdent {
    pub id: usize,
    pub name: String,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone)]
pub struct HParam {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum HDeclaration {
    Const {
        id: usize,
        name: String,
        value: i64,
    },
    Var {
        id: usize,
        name: String,
    },
    Procedure {
        id: usize,
        name: String,
        params: Vec<HParam>,
        body: Vec<HStatement>,
        end_name: String,
    },
}

#[derive(Debug, Clone)]
pub enum HStatement {
    Assign {
        target: HResolvedIdent,
        value: HExpr,
    },
    Call {
        name: HResolvedIdent,
        args: Vec<HExpr>,
    },
    If {
        condition: HExpr,
        then_branch: Vec<HStatement>,
        else_branch: Option<Vec<HStatement>>,
    },
    While {
        condition: HExpr,
        body: Vec<HStatement>,
    },
}

#[derive(Debug, Clone)]
pub enum HExpr {
    Integer(i64),
    Name(HResolvedIdent),
    Binary {
        op: BinaryOp,
        left: Box<HExpr>,
        right: Box<HExpr>,
    },
}

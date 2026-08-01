#![allow(dead_code)]

//! Lowered, name-resolved representation used by code generation.

use crate::ast::{BinaryOp, TypeRef, UnaryOp};
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
    /// Optional declared parameter type preserved from the source.
    pub declared_type: Option<TypeRef>,
    /// Whether the parameter is declared with `VAR` pass-by-reference mode.
    pub is_var: bool,
}

#[derive(Debug, Clone)]
/// Lowered declaration nodes.
pub enum HDeclaration {
    /// Constant declaration with its resolved id.
    Const {
        id: usize,
        name: String,
        value: HExpr,
    },
    /// Type declaration with its resolved id and preserved target type.
    Type {
        id: usize,
        name: String,
        target: TypeRef,
    },
    /// Variable declaration with its resolved id and preserved declared type.
    Var {
        id: usize,
        name: String,
        declared_type: Option<TypeRef>,
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
        module: Option<String>,
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
    Real(f32),
    LongReal(f64),
    Boolean(bool),
    /// String literal after parser unescaping.
    String(String),
    /// Reference to a resolved identifier binding.
    Name(HResolvedIdent),
    /// Function-like call expression with resolved callee and arguments.
    Call {
        name: HResolvedIdent,
        args: Vec<HExpr>,
    },
    /// Unary expression.
    Unary {
        op: UnaryOp,
        value: Box<HExpr>,
    },
    /// Binary expression.
    Binary {
        op: BinaryOp,
        left: Box<HExpr>,
        right: Box<HExpr>,
    },
}

impl HExpr {
    /// Returns `true` if the expression is a literal (integer, real, long real, boolean, or string).
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            HExpr::Integer(_)
                | HExpr::Real(_)
                | HExpr::LongReal(_)
                | HExpr::Boolean(_)
                | HExpr::String(_)
        )
    }

    #[cfg_attr(coverage_nightly, coverage(off))]
    pub(crate) fn to_string(&self) -> String {
        match self {
            HExpr::Integer(v) => v.to_string(),
            HExpr::Real(v) => v.to_string(),
            HExpr::LongReal(v) => v.to_string(),
            HExpr::Boolean(v) => v.to_string(),
            HExpr::String(value) => format!("{:?}", value),
            HExpr::Name(ident) => ident.name.clone(),
            HExpr::Call { name, args } => {
                let args_str = args
                    .iter()
                    .map(|arg| arg.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}({})", name.name, args_str)
            }
            HExpr::Unary { op, value } => {
                let op_str = match op {
                    UnaryOp::Minus => "-",
                    UnaryOp::Not => "NOT ",
                    UnaryOp::Plus => "",
                };
                format!("{}{}", op_str, value.to_string())
            }
            HExpr::Binary { op, left, right } => {
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::IntDiv => "DIV",
                    BinaryOp::Mod => "MOD",
                    BinaryOp::And => "AND",
                    BinaryOp::Or => "OR",
                    BinaryOp::Eq => "=",
                    BinaryOp::Ne => "#",
                    BinaryOp::Lt => "<",
                    BinaryOp::Le => "<=",
                    BinaryOp::Gt => ">",
                    BinaryOp::Ge => ">=",
                };
                format!("({} {} {})", left.to_string(), op_str, right.to_string())
            }
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{HExpr, HResolvedIdent};
    use crate::ast::{BinaryOp, UnaryOp};
    use crate::symbols::SymbolKind;

    #[test]
    fn h_expr_is_literal_detects_literal_and_non_literal_variants() {
        assert!(HExpr::Integer(7).is_literal());
        assert!(HExpr::Real(2.5).is_literal());
        assert!(HExpr::LongReal(3.25).is_literal());
        assert!(HExpr::Boolean(true).is_literal());
        assert!(HExpr::String("hi".to_string()).is_literal());

        assert!(
            !HExpr::Name(HResolvedIdent {
                id: 1,
                name: "value".to_string(),
                kind: SymbolKind::Variable,
            })
            .is_literal()
        );
        assert!(
            !HExpr::Call {
                name: HResolvedIdent {
                    id: 2,
                    name: "f".to_string(),
                    kind: SymbolKind::Procedure,
                },
                args: vec![],
            }
            .is_literal()
        );
        assert!(
            !HExpr::Unary {
                op: UnaryOp::Not,
                value: Box::new(HExpr::Boolean(true)),
            }
            .is_literal()
        );
        assert!(
            !HExpr::Binary {
                op: BinaryOp::Add,
                left: Box::new(HExpr::Integer(1)),
                right: Box::new(HExpr::Integer(2)),
            }
            .is_literal()
        );
    }

    #[test]
    fn h_expr_to_string_formats_names_calls_unary_and_binary_exprs() {
        let ident = HResolvedIdent {
            id: 10,
            name: "value".to_string(),
            kind: SymbolKind::Variable,
        };

        assert_eq!(HExpr::Integer(5).to_string(), "5");
        assert_eq!(HExpr::Boolean(false).to_string(), "false");
        assert_eq!(HExpr::Name(ident.clone()).to_string(), "value");

        let call = HExpr::Call {
            name: HResolvedIdent {
                id: 11,
                name: "f".to_string(),
                kind: SymbolKind::Procedure,
            },
            args: vec![HExpr::Integer(1), HExpr::Integer(2)],
        };
        assert_eq!(call.to_string(), "f(1, 2)");

        let unary = HExpr::Unary {
            op: UnaryOp::Not,
            value: Box::new(HExpr::Boolean(true)),
        };
        assert_eq!(unary.to_string(), "NOT true");

        let binary = HExpr::Binary {
            op: BinaryOp::Add,
            left: Box::new(HExpr::Integer(1)),
            right: Box::new(HExpr::Integer(2)),
        };
        assert_eq!(binary.to_string(), "(1 + 2)");
    }
}

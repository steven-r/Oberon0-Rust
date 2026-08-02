//! Syntax tree nodes produced by parsing before name resolution.

#[derive(Debug, Clone)]
/// Parsed Oberon0 module with declarations and executable statements.
pub struct Module {
    /// Module name declared after the `MODULE` keyword.
    pub name: String,
    /// Name repeated after the closing `END` keyword.
    pub end_name: String,
    /// Imported external procedure namespaces visible in the module.
    pub imports: Vec<ImportDecl>,
    #[allow(dead_code)]
    /// Top-level declarations in source order.
    pub declarations: Vec<Declaration>,
    /// Statements inside the module `BEGIN ... END` block.
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
/// Import alias mapping used by semantic analysis and manifest resolution.
pub struct ImportDecl {
    /// Name used inside the current Oberon0 module.
    pub local_name: String,
    /// External manifest key that the alias resolves to.
    pub external_name: String,
}

#[derive(Debug, Clone)]
/// Procedure parameter declaration, optionally typed and optionally passed by reference.
pub struct ParamDecl {
    /// Source-level parameter name.
    pub name: String,
    /// Optional declared type of the parameter.
    pub declared_type: Option<TypeRef>,
    /// Whether the parameter was declared with `VAR` pass-by-reference mode.
    pub is_var: bool,
}

#[derive(Debug, Clone)]
/// Procedure-local variable declaration with an optional declared type.
pub struct LocalVarDecl {
    /// Source-level local variable name.
    pub name: String,
    /// Optional declared type of the local variable.
    pub declared_type: Option<TypeRef>,
}

#[derive(Debug, Clone, PartialEq)]
/// Type references supported by the current typed-declaration milestone.
pub enum TypeRef {
    /// Built-in INTEGER scalar type.
    Integer,
    /// Built-in BOOLEAN scalar type.
    Boolean,
    /// Built-in REAL scalar type.
    Real,
    /// Built-in LONGREAL scalar type.
    LongReal,
    /// Array type with a constant length expression and nested element type.
    Array {
        length: Expr,
        element_type: Box<TypeRef>,
    },
    /// Named type alias or user-defined type reference.
    Named(String),
    /// Qualified type reference (e.g., B.T for module B's exported type T).
    Qualified { module: String, name: String },
}

#[derive(Debug, Clone)]
/// Executable statements supported by the current Oberon0 subset.
pub enum Statement {
    /// Assigns the evaluated expression to an existing identifier.
    Assign { target: AssignTarget, value: Expr },
    /// Invokes a built-in, imported, or user-defined procedure.
    Call {
        module: Option<String>,
        name: String,
        args: Vec<Expr>,
    },
    /// Conditional branch with an optional `ELSE` block.
    If {
        condition: Expr,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    /// Loop that executes while the condition evaluates to a non-zero value.
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
/// Top-level declarations currently recognized by the compiler.
pub enum Declaration {
    /// Constant declaration with an integer literal value.
    Const { name: String, value: Expr },
    /// Named type alias declaration.
    Type {
        name: String,
        target: TypeRef,
        is_exported: bool,
    },
    /// Mutable variable declaration, optionally with a declared type.
    Var {
        name: String,
        declared_type: Option<TypeRef>,
    },
    /// Procedure declaration with positional parameters and a statement body.
    Procedure {
        name: String,
        params: Vec<ParamDecl>,
        local_vars: Vec<LocalVarDecl>,
        body: Vec<Statement>,
        end_name: String,
        is_exported: bool,
    },
}

#[derive(Debug, Clone)]
/// Expression nodes used in statements and declaration initializers.
pub enum Expr {
    /// Integer literal.
    Integer(i64),
    Real(f32),
    LongReal(f64),
    Boolean(bool),
    /// String literal using Pascal-style doubled quotes for embedded `"` characters.
    String(String),
    /// Reference to an identifier before semantic resolution.
    Variable(String),
    /// Indexed array element reference before semantic resolution.
    Indexed {
        name: String,
        index: Box<Expr>,
    },
    /// Qualified variable reference (e.g., B.T).
    QualifiedVariable {
        module: String,
        name: String,
    },
    /// Function-like call expression.
    Call {
        module: Option<String>,
        name: String,
        args: Vec<Expr>,
    },
    /// Unary expression.
    Unary {
        op: UnaryOp,
        value: Box<Expr>,
    },
    /// Binary arithmetic expression.
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

impl Expr {
    /// Returns `true` if the expression is a literal (integer, real, long real, boolean, or string).
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            Expr::Integer(_)
                | Expr::Real(_)
                | Expr::LongReal(_)
                | Expr::Boolean(_)
                | Expr::String(_)
        )
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Expr::Integer(a), Expr::Integer(b)) => a == b,
            (Expr::Real(a), Expr::Real(b)) => a == b,
            (Expr::LongReal(a), Expr::LongReal(b)) => a == b,
            (Expr::Boolean(a), Expr::Boolean(b)) => a == b,
            (Expr::String(a), Expr::String(b)) => a == b,
            (Expr::Variable(a), Expr::Variable(b)) => a == b,
            (
                Expr::Indexed {
                    name: n1,
                    index: i1,
                },
                Expr::Indexed {
                    name: n2,
                    index: i2,
                },
            ) => n1 == n2 && i1 == i2,
            (
                Expr::QualifiedVariable {
                    module: m1,
                    name: n1,
                },
                Expr::QualifiedVariable {
                    module: m2,
                    name: n2,
                },
            ) => m1 == m2 && n1 == n2,
            (
                Expr::Call {
                    module: m1,
                    name: n1,
                    args: a1,
                },
                Expr::Call {
                    module: m2,
                    name: n2,
                    args: a2,
                },
            ) => m1 == m2 && n1 == n2 && a1 == a2,
            (Expr::Unary { op: op1, value: v1 }, Expr::Unary { op: op2, value: v2 }) => {
                op1 == op2 && v1 == v2
            }
            (
                Expr::Binary {
                    op: op1,
                    left: l1,
                    right: r1,
                },
                Expr::Binary {
                    op: op2,
                    left: l2,
                    right: r2,
                },
            ) => op1 == op2 && l1 == l2 && r1 == r2,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
/// Assignable designators supported by the current subset grammar.
pub enum AssignTarget {
    /// Assign to a named binding.
    Name(String),
    /// Assign to an indexed array element.
    Indexed { name: String, index: Expr },
}

impl PartialEq for AssignTarget {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AssignTarget::Name(a), AssignTarget::Name(b)) => a == b,
            (
                AssignTarget::Indexed {
                    name: n1,
                    index: i1,
                },
                AssignTarget::Indexed {
                    name: n2,
                    index: i2,
                },
            ) => n1 == n2 && i1 == i2,
            _ => false,
        }
    }
}

impl Eq for AssignTarget {}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Supported binary operators in the current subset grammar.
pub enum BinaryOp {
    Add,
    Sub,
    Or,
    Mul,
    Div,
    IntDiv,
    Mod,
    And,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Supported unary operators in the current subset.
pub enum UnaryOp {
    /// Unary plus operator (no effect on the value).
    Plus,
    /// Unary minus operator (negates the value).
    Minus,
    /// Logical negation operator (inverts boolean value).
    Not,
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::{AssignTarget, BinaryOp, Expr, UnaryOp};

    #[test]
    fn expr_is_literal_detects_literal_and_non_literal_nodes() {
        assert!(Expr::Integer(42).is_literal());
        assert!(Expr::Real(3.5).is_literal());
        assert!(Expr::LongReal(4.5).is_literal());
        assert!(Expr::Boolean(true).is_literal());
        assert!(Expr::String("hello".to_string()).is_literal());

        assert!(!Expr::Variable("value".to_string()).is_literal());
        assert!(
            !Expr::Indexed {
                name: "value".to_string(),
                index: Box::new(Expr::Integer(0)),
            }
            .is_literal()
        );
        assert!(
            !Expr::QualifiedVariable {
                module: "M".to_string(),
                name: "value".to_string(),
            }
            .is_literal()
        );
        assert!(
            !Expr::Call {
                module: None,
                name: "f".to_string(),
                args: vec![],
            }
            .is_literal()
        );
        assert!(
            !Expr::Unary {
                op: UnaryOp::Not,
                value: Box::new(Expr::Boolean(true)),
            }
            .is_literal()
        );
        assert!(
            !Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Integer(1)),
                right: Box::new(Expr::Integer(2)),
            }
            .is_literal()
        );
    }

    #[test]
    fn expr_equality_matches_all_supported_expression_variants() {
        assert_eq!(Expr::Integer(1), Expr::Integer(1));
        assert_ne!(Expr::Integer(1), Expr::Integer(2));
        assert_eq!(Expr::Real(1.5), Expr::Real(1.5));
        assert_ne!(Expr::Real(1.5), Expr::Real(2.5));
        assert_eq!(Expr::LongReal(1.25), Expr::LongReal(1.25));
        assert_ne!(Expr::LongReal(1.25), Expr::LongReal(2.25));
        assert_eq!(Expr::Boolean(true), Expr::Boolean(true));
        assert_ne!(Expr::Boolean(true), Expr::Boolean(false));
        assert_eq!(Expr::String("x".to_string()), Expr::String("x".to_string()));
        assert_ne!(Expr::String("x".to_string()), Expr::String("y".to_string()));

        assert_eq!(
            Expr::Variable("name".to_string()),
            Expr::Variable("name".to_string())
        );
        assert_ne!(
            Expr::Variable("name".to_string()),
            Expr::Variable("other".to_string())
        );

        assert_eq!(
            Expr::Indexed {
                name: "values".to_string(),
                index: Box::new(Expr::Integer(0)),
            },
            Expr::Indexed {
                name: "values".to_string(),
                index: Box::new(Expr::Integer(0)),
            }
        );
        assert_ne!(
            Expr::Indexed {
                name: "values".to_string(),
                index: Box::new(Expr::Integer(0)),
            },
            Expr::Indexed {
                name: "values".to_string(),
                index: Box::new(Expr::Integer(1)),
            }
        );

        assert_eq!(
            Expr::QualifiedVariable {
                module: "M".to_string(),
                name: "name".to_string(),
            },
            Expr::QualifiedVariable {
                module: "M".to_string(),
                name: "name".to_string(),
            }
        );
        assert_ne!(
            Expr::QualifiedVariable {
                module: "M".to_string(),
                name: "name".to_string(),
            },
            Expr::QualifiedVariable {
                module: "N".to_string(),
                name: "name".to_string(),
            }
        );

        let call = Expr::Call {
            module: Some("M".to_string()),
            name: "f".to_string(),
            args: vec![Expr::Integer(1)],
        };
        assert_eq!(
            call,
            Expr::Call {
                module: Some("M".to_string()),
                name: "f".to_string(),
                args: vec![Expr::Integer(1)],
            }
        );
        assert_ne!(
            call,
            Expr::Call {
                module: Some("M".to_string()),
                name: "g".to_string(),
                args: vec![Expr::Integer(1)],
            }
        );

        let unary = Expr::Unary {
            op: UnaryOp::Minus,
            value: Box::new(Expr::Integer(3)),
        };
        assert_eq!(
            unary,
            Expr::Unary {
                op: UnaryOp::Minus,
                value: Box::new(Expr::Integer(3)),
            }
        );
        assert_ne!(
            unary,
            Expr::Unary {
                op: UnaryOp::Plus,
                value: Box::new(Expr::Integer(3)),
            }
        );

        let binary = Expr::Binary {
            op: BinaryOp::Add,
            left: Box::new(Expr::Integer(1)),
            right: Box::new(Expr::Integer(2)),
        };
        assert_eq!(
            binary,
            Expr::Binary {
                op: BinaryOp::Add,
                left: Box::new(Expr::Integer(1)),
                right: Box::new(Expr::Integer(2)),
            }
        );
        assert_ne!(
            binary,
            Expr::Binary {
                op: BinaryOp::Sub,
                left: Box::new(Expr::Integer(1)),
                right: Box::new(Expr::Integer(2)),
            }
        );

        assert_eq!(
            AssignTarget::Name("x".to_string()),
            AssignTarget::Name("x".to_string())
        );
        assert_eq!(
            AssignTarget::Indexed {
                name: "values".to_string(),
                index: Expr::Integer(0),
            },
            AssignTarget::Indexed {
                name: "values".to_string(),
                index: Expr::Integer(0),
            }
        );

        assert_ne!(Expr::Integer(1), Expr::Boolean(true));
    }
}

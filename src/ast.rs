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
/// Executable statements supported by the current Oberon0 subset.
pub enum Statement {
    /// Assigns the evaluated expression to an existing identifier.
    Assign { target: String, value: Expr },
    /// Invokes a built-in, imported, or user-defined procedure.
    Call { name: String, args: Vec<Expr> },
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
    Const { name: String, value: i64 },
    /// Mutable variable declaration.
    Var { name: String },
    /// Procedure declaration with positional parameters and a statement body.
    Procedure {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        end_name: String,
    },
}

#[derive(Debug, Clone)]
/// Expression nodes used in statements and declaration initializers.
pub enum Expr {
    /// Integer literal.
    Integer(i64),
    /// Reference to an identifier before semantic resolution.
    Variable(String),
    /// Binary arithmetic expression.
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy)]
/// Supported arithmetic operators in the MVP grammar.
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

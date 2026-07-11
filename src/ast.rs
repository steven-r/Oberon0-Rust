#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub end_name: String,
    pub imports: Vec<ImportDecl>,
    #[allow(dead_code)]
    pub declarations: Vec<Declaration>,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct ImportDecl {
    pub local_name: String,
    pub external_name: String,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Assign { target: String, value: Expr },
    Call { name: String, args: Vec<Expr> },
    If {
        condition: Expr,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
    },
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Declaration {
    Const { name: String, value: i64 },
    Var { name: String },
    Procedure { name: String, params: Vec<String> },
}

#[derive(Debug, Clone)]
pub enum Expr {
    Integer(i64),
    Variable(String),
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

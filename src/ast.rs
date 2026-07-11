#[derive(Debug, Clone)]
pub struct Module {
    pub name: String,
    pub end_name: String,
    pub imports: Vec<ImportDecl>,
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

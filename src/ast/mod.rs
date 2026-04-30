use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Expr {
    Number(f64),
    String(String),
    Identifier(String),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Call { callee: String, args: Vec<Expr> },
}

#[derive(Debug, Clone, Serialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Debug, Clone, Serialize)]
pub enum Stmt {
    Let(String, Expr),
    Assign(String, Expr),
    Return(Expr),
    Expr(Expr),
    Use(UseStmt),
    If { condition: Expr, then_block: Vec<Stmt>, else_block: Option<Vec<Stmt>> },
    While { condition: Expr, body: Vec<Stmt> },
}

#[derive(Debug, Clone, Serialize)]
pub struct UseStmt {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub name: String,
    pub ty: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Program {
    pub uses: Vec<UseStmt>,
    pub functions: Vec<Function>,
}

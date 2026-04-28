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
}

#[derive(Debug, Clone, Serialize)]
pub enum Stmt {
    Let(String, Expr),
    Return(Expr),
    Expr(Expr),
}

#[derive(Debug, Clone, Serialize)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Program {
    pub functions: Vec<Function>,
}

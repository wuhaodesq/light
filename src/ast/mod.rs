use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum Expr {
    Number(f64),
    String(String),
    Identifier(String),
    Array(Vec<Expr>),
    ArrayIndex(Box<Expr>, Box<Expr>),
    StructInit { name: String, fields: Vec<(String, Expr)> },
    FieldAccess(Box<Expr>, String),
    Binary(Box<Expr>, BinaryOp, Box<Expr>),
    Call { callee: String, args: Vec<Expr> },
    Match { expr: Box<Expr>, cases: Vec<MatchCase> },
}

#[derive(Debug, Clone, Serialize)]
pub struct MatchCase {
    pub pattern: MatchPattern,
    pub body: Expr,
}

#[derive(Debug, Clone, Serialize)]
pub enum MatchPattern {
    Wildcard,
    Number(f64),
    String(String),
    Identifier(String),
    EnumVariant { name: String, variant: String, patterns: Vec<MatchPattern> },
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
    Break,
    Continue,
    Expr(Expr),
    Use(UseStmt),
    If { condition: Expr, then_block: Vec<Stmt>, else_block: Option<Vec<Stmt>> },
    While { condition: Expr, body: Vec<Stmt> },
    For { initializer: Option<Box<Stmt>>, condition: Option<Expr>, increment: Option<Box<Expr>>, body: Vec<Stmt> },
    Loop { body: Vec<Stmt> },
    StructDef { name: String, fields: Vec<(String, String)> },
    EnumDef { name: String, variants: Vec<(String, Vec<String>)> },
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
    pub structs: Vec<Stmt>,
    pub enums: Vec<Stmt>,
}

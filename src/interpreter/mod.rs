use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Function, Program, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

#[derive(Debug, Clone)]
enum Value {
    Number(f64),
    String(String),
    Unit,
}

pub fn run(program: &Program) -> Result<(), Diagnostic> {
    let main = program
        .functions
        .iter()
        .find(|f| f.name == "main")
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::RuntimeError,
                "missing entry function `main`",
                Span::new(0, 0),
            )
        })?;

    eval_function(main)?;
    Ok(())
}

fn eval_function(function: &Function) -> Result<Value, Diagnostic> {
    let mut env: HashMap<String, Value> = HashMap::new();

    for stmt in &function.body {
        match stmt {
            Stmt::Let(name, expr) => {
                let value = eval_expr(expr, &mut env)?;
                env.insert(name.clone(), value);
            }
            Stmt::Return(expr) => return eval_expr(expr, &mut env),
            Stmt::Expr(expr) => {
                let _ = eval_expr(expr, &mut env)?;
            }
        }
    }

    Ok(Value::Unit)
}

fn eval_expr(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, Diagnostic> {
    match expr {
        Expr::Number(value) => Ok(Value::Number(*value)),
        Expr::String(value) => Ok(Value::String(value.clone())),
        Expr::Identifier(name) => env.get(name).cloned().ok_or_else(|| {
            Diagnostic::new(
                DiagnosticCode::RuntimeError,
                format!("unknown variable `{name}`"),
                Span::new(0, 0),
            )
        }),
        Expr::Binary(lhs, op, rhs) => {
            let lhs = eval_expr(lhs, env)?;
            let rhs = eval_expr(rhs, env)?;
            eval_binary(lhs, op, rhs)
        }
        Expr::Call { callee, args } => {
            if callee == "print" {
                let mut out = String::new();
                for arg in args {
                    let value = eval_expr(arg, env)?;
                    let rendered = match value {
                        Value::Number(v) => v.to_string(),
                        Value::String(v) => v,
                        Value::Unit => String::from("()"),
                    };
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(&rendered);
                }
                println!("{out}");
                Ok(Value::Unit)
            } else {
                Err(Diagnostic::new(
                    DiagnosticCode::RuntimeError,
                    format!("unknown function `{callee}`"),
                    Span::new(0, 0),
                ))
            }
        }
    }
}

fn eval_binary(lhs: Value, op: &BinaryOp, rhs: Value) -> Result<Value, Diagnostic> {
    match (lhs, rhs) {
        (Value::Number(lhs), Value::Number(rhs)) => {
            let value = match op {
                BinaryOp::Add => lhs + rhs,
                BinaryOp::Sub => lhs - rhs,
                BinaryOp::Mul => lhs * rhs,
                BinaryOp::Div => lhs / rhs,
            };
            Ok(Value::Number(value))
        }
        _ => Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            "binary operators currently support only numbers",
            Span::new(0, 0),
        )),
    }
}

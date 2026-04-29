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
    let mut functions = HashMap::new();
    for function in &program.functions {
        functions.insert(function.name.as_str(), function);
    }

    let main = functions.get("main").copied().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::RuntimeError,
            "missing entry function `main`",
            Span::new(0, 0),
        )
    })?;

    let _ = eval_function(main, Vec::new(), &functions)?;
    Ok(())
}

fn eval_function<'a>(
    function: &'a Function,
    args: Vec<Value>,
    functions: &HashMap<&'a str, &'a Function>,
) -> Result<Value, Diagnostic> {
    if function.params.len() != args.len() {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            format!(
                "function `{}` expects {} arguments but got {}",
                function.name,
                function.params.len(),
                args.len()
            ),
            Span::new(0, 0),
        ));
    }

    let mut env: HashMap<String, Value> = HashMap::new();
    for (param, arg) in function.params.iter().zip(args.into_iter()) {
        env.insert(param.name.clone(), arg);
    }

    for stmt in &function.body {
        match stmt {
            Stmt::Let(name, expr) => {
                let value = eval_expr(expr, &mut env, functions)?;
                env.insert(name.clone(), value);
            }
            Stmt::Return(expr) => return eval_expr(expr, &mut env, functions),
            Stmt::Expr(expr) => {
                let _ = eval_expr(expr, &mut env, functions)?;
            }
        }
    }

    Ok(Value::Unit)
}

fn eval_expr<'a>(
    expr: &Expr,
    env: &mut HashMap<String, Value>,
    functions: &HashMap<&'a str, &'a Function>,
) -> Result<Value, Diagnostic> {
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
            let lhs = eval_expr(lhs, env, functions)?;
            let rhs = eval_expr(rhs, env, functions)?;
            eval_binary(lhs, op, rhs)
        }
        Expr::Call { callee, args } => {
            if callee == "print" {
                let mut out = String::new();
                for arg in args {
                    let value = eval_expr(arg, env, functions)?;
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
                let function = functions.get(callee.as_str()).copied().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        format!("unknown function `{callee}`"),
                        Span::new(0, 0),
                    )
                })?;

                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(eval_expr(arg, env, functions)?);
                }
                eval_function(function, values, functions)
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

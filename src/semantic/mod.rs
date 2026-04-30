use std::collections::{HashMap, HashSet};

use crate::ast::{Expr, Function, Program, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

fn is_builtin_hal_function(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "gpio.pin"
            | "gpio.high"
            | "gpio.low"
            | "gpio.toggle"
            | "gpio.output"
            | "gpio.input"
            | "sleep_ms"
            | "uart.open"
            | "spi.open"
            | "i2c.open"
            | "adc.read"
            | "pwm.start"
            | "pwm.stop"
            | "timer.delay"
    )
}

pub fn analyze(program: &Program) -> Result<(), Diagnostic> {
    let mut signatures: HashMap<String, usize> = HashMap::new();

    for function in &program.functions {
        signatures.insert(function.name.clone(), function.params.len());
    }

    for function in &program.functions {
        analyze_function(function, &signatures)?;
    }

    Ok(())
}

fn analyze_function(
    function: &Function,
    signatures: &HashMap<String, usize>,
) -> Result<(), Diagnostic> {
    let mut bindings = HashSet::new();
    for param in &function.params {
        bindings.insert(param.name.clone());
    }

    for stmt in &function.body {
        match stmt {
            Stmt::Let(name, expr) => {
                check_expr(expr, &bindings, signatures)?;
                bindings.insert(name.clone());
            }
            Stmt::Assign(_name, expr) => {
                check_expr(expr, &bindings, signatures)?;
            }
            Stmt::Return(expr) | Stmt::Expr(expr) => check_expr(expr, &bindings, signatures)?,
            Stmt::Use(_) => {}
            Stmt::If { condition, then_block, else_block } => {
                check_expr(condition, &bindings, signatures)?;
                let then_bindings = bindings.clone();
                for s in then_block {
                    check_stmt(s, &then_bindings, signatures)?;
                }
                if let Some(else_b) = else_block {
                    let else_bindings = bindings.clone();
                    for s in else_b {
                        check_stmt(s, &else_bindings, signatures)?;
                    }
                }
            }
            Stmt::While { condition, body } => {
                check_expr(condition, &bindings, signatures)?;
                let body_bindings = bindings.clone();
                for s in body {
                    check_stmt(s, &body_bindings, signatures)?;
                }
            }
        }
    }

    Ok(())
}

fn check_stmt(
    stmt: &Stmt,
    bindings: &HashSet<String>,
    signatures: &HashMap<String, usize>,
) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Let(_name, expr) => {
            check_expr(expr, bindings, signatures)?;
            Ok(())
        }
        Stmt::Assign(_name, expr) => {
            check_expr(expr, bindings, signatures)?;
            Ok(())
        }
        Stmt::Return(expr) | Stmt::Expr(expr) => check_expr(expr, bindings, signatures),
        Stmt::Use(_) => Ok(()),
        Stmt::If { condition, then_block, else_block } => {
            check_expr(condition, bindings, signatures)?;
            for s in then_block {
                check_stmt(s, bindings, signatures)?;
            }
            if let Some(else_b) = else_block {
                for s in else_b {
                    check_stmt(s, bindings, signatures)?;
                }
            }
            Ok(())
        }
        Stmt::While { condition, body } => {
            check_expr(condition, bindings, signatures)?;
            for s in body {
                check_stmt(s, bindings, signatures)?;
            }
            Ok(())
        }
    }
}

fn check_expr(
    expr: &Expr,
    bindings: &HashSet<String>,
    signatures: &HashMap<String, usize>,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::Identifier(name) => {
            if !bindings.contains(name) {
                return Err(Diagnostic::new(
                    DiagnosticCode::UndefinedVariable,
                    format!("variable `{name}` not found"),
                    Span::new(0, 0),
                ));
            }
            Ok(())
        }
        Expr::Binary(lhs, _, rhs) => {
            check_expr(lhs, bindings, signatures)?;
            check_expr(rhs, bindings, signatures)
        }
        Expr::Call { callee, args } => {
            if !is_builtin_hal_function(callee) {
                let expected = signatures.get(callee).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::ParseError,
                        format!("function `{callee}` not found"),
                        Span::new(0, 0),
                    )
                })?;

                if *expected != args.len() {
                    return Err(Diagnostic::new(
                        DiagnosticCode::ParseError,
                        format!(
                            "function `{callee}` expects {} arguments but got {}",
                            expected,
                            args.len()
                        ),
                        Span::new(0, 0),
                    ));
                }
            }

            for arg in args {
                check_expr(arg, bindings, signatures)?;
            }
            Ok(())
        }
        Expr::Number(_) | Expr::String(_) => Ok(()),
    }
}

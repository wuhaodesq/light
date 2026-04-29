use std::collections::{HashMap, HashSet};

use crate::ast::{Expr, Function, Program, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

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
            Stmt::Return(expr) | Stmt::Expr(expr) => check_expr(expr, &bindings, signatures)?,
        }
    }

    Ok(())
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
            if callee != "print" {
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

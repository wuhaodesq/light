use std::collections::HashSet;

use crate::ast::{Expr, Function, Program, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

pub fn analyze(program: &Program) -> Result<(), Diagnostic> {
    for function in &program.functions {
        analyze_function(function)?;
    }
    Ok(())
}

fn analyze_function(function: &Function) -> Result<(), Diagnostic> {
    let mut bindings = HashSet::new();

    for stmt in &function.body {
        match stmt {
            Stmt::Let(name, expr) => {
                check_expr(expr, &bindings)?;
                bindings.insert(name.clone());
            }
            Stmt::Return(expr) | Stmt::Expr(expr) => check_expr(expr, &bindings)?,
        }
    }

    Ok(())
}

fn check_expr(expr: &Expr, bindings: &HashSet<String>) -> Result<(), Diagnostic> {
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
            check_expr(lhs, bindings)?;
            check_expr(rhs, bindings)
        }
        Expr::Call { args, .. } => {
            for arg in args {
                check_expr(arg, bindings)?;
            }
            Ok(())
        }
        Expr::Number(_) | Expr::String(_) => Ok(()),
    }
}

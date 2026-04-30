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

    let enum_variants: HashMap<String, HashSet<String>> = program
        .enums
        .iter()
        .filter_map(|stmt| {
            if let Stmt::EnumDef { name, variants } = stmt {
                Some((
                    name.clone(),
                    variants.iter().map(|(v, _)| v.clone()).collect(),
                ))
            } else {
                None
            }
        })
        .collect();

    for function in &program.functions {
        analyze_function(function, &signatures, &enum_variants)?;
    }

    Ok(())
}

fn analyze_function(
    function: &Function,
    signatures: &HashMap<String, usize>,
    enum_variants: &HashMap<String, HashSet<String>>,
) -> Result<(), Diagnostic> {
    let mut bindings = HashSet::new();
    for param in &function.params {
        bindings.insert(param.name.clone());
    }

    for stmt in &function.body {
        match stmt {
            Stmt::Let(name, expr) => {
                check_expr(expr, &bindings, signatures, enum_variants)?;
                bindings.insert(name.clone());
            }
            Stmt::Assign(_name, expr) => {
                check_expr(expr, &bindings, signatures, enum_variants)?;
            }
            Stmt::Return(expr) | Stmt::Expr(expr) => check_expr(expr, &bindings, signatures, enum_variants)?,
            Stmt::Use(_) => {}
            Stmt::StructDef { .. } | Stmt::EnumDef { .. } => {}
            Stmt::If { condition, then_block, else_block } => {
                check_expr(condition, &bindings, signatures, enum_variants)?;
                let mut then_bindings = bindings.clone();
                for s in then_block {
                    check_stmt(s, &mut then_bindings, signatures, enum_variants)?;
                }
                if let Some(else_b) = else_block {
                    let mut else_bindings = bindings.clone();
                    for s in else_b {
                        check_stmt(s, &mut else_bindings, signatures, enum_variants)?;
                    }
                }
            }
            Stmt::While { condition, body } => {
                check_expr(condition, &bindings, signatures, enum_variants)?;
                let mut body_bindings = bindings.clone();
                for s in body {
                    check_stmt(s, &mut body_bindings, signatures, enum_variants)?;
                }
            }
            Stmt::For { initializer, condition, increment, body } => {
                let mut for_bindings = bindings.clone();
                if let Some(init) = initializer {
                    check_stmt(init, &mut for_bindings, signatures, enum_variants)?;
                }
                if let Some(cond) = condition {
                    check_expr(cond, &for_bindings, signatures, enum_variants)?;
                }
                if let Some(inc) = increment {
                    check_expr(inc, &for_bindings, signatures, enum_variants)?;
                }
                for s in body {
                    check_stmt(s, &mut for_bindings, signatures, enum_variants)?;
                }
            }
        }
    }

    Ok(())
}

fn check_stmt(
    stmt: &Stmt,
    bindings: &mut HashSet<String>,
    signatures: &HashMap<String, usize>,
    enum_variants: &HashMap<String, HashSet<String>>,
) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Let(name, expr) => {
            check_expr(expr, bindings, signatures, enum_variants)?;
            bindings.insert(name.clone());
            Ok(())
        }
        Stmt::Assign(name, expr) => {
            check_expr(expr, bindings, signatures, enum_variants)?;
            Ok(())
        }
        Stmt::Return(expr) | Stmt::Expr(expr) => check_expr(expr, bindings, signatures, enum_variants),
        Stmt::Use(_) => Ok(()),
        Stmt::StructDef { .. } | Stmt::EnumDef { .. } => Ok(()),
        Stmt::If { condition, then_block, else_block } => {
            check_expr(condition, bindings, signatures, enum_variants)?;
            for s in then_block {
                check_stmt(s, bindings, signatures, enum_variants)?;
            }
            if let Some(else_b) = else_block {
                for s in else_b {
                    check_stmt(s, bindings, signatures, enum_variants)?;
                }
            }
            Ok(())
        }
        Stmt::While { condition, body } => {
            check_expr(condition, bindings, signatures, enum_variants)?;
            for s in body {
                check_stmt(s, bindings, signatures, enum_variants)?;
            }
            Ok(())
        }
        Stmt::For { initializer, condition, increment, body } => {
            if let Some(init) = initializer {
                check_stmt(init, bindings, signatures, enum_variants)?;
            }
            if let Some(cond) = condition {
                check_expr(cond, bindings, signatures, enum_variants)?;
            }
            if let Some(inc) = increment {
                check_expr(inc, bindings, signatures, enum_variants)?;
            }
            for s in body {
                check_stmt(s, bindings, signatures, enum_variants)?;
            }
            Ok(())
        }
    }
}

fn check_expr(
    expr: &Expr,
    bindings: &HashSet<String>,
    signatures: &HashMap<String, usize>,
    enum_variants: &HashMap<String, HashSet<String>>,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::Identifier(name) => {
            if !bindings.contains(name) {
                if name.contains("::") {
                    let parts: Vec<&str> = name.split("::").collect();
                    if parts.len() == 2 {
                        let (enum_name, variant_name) = (parts[0], parts[1]);
                        if let Some(variants) = enum_variants.get(enum_name) {
                            if variants.contains(variant_name) {
                                return Ok(());
                            }
                        }
                    }
                }
                return Err(Diagnostic::new(
                    DiagnosticCode::UndefinedVariable,
                    format!("variable `{name}` not found"),
                    Span::new(0, 0),
                ));
            }
            Ok(())
        }
        Expr::Binary(lhs, _, rhs) => {
            check_expr(lhs, bindings, signatures, enum_variants)?;
            check_expr(rhs, bindings, signatures, enum_variants)
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
                check_expr(arg, bindings, signatures, enum_variants)?;
            }
            Ok(())
        }
        Expr::Number(_) | Expr::String(_) => Ok(()),
        Expr::Array(elements) => {
            for elem in elements {
                check_expr(elem, bindings, signatures, enum_variants)?;
            }
            Ok(())
        }
        Expr::ArrayIndex(arr, index) => {
            check_expr(arr, bindings, signatures, enum_variants)?;
            check_expr(index, bindings, signatures, enum_variants)
        }
        Expr::StructInit { name: _, fields } => {
            for (_, field_expr) in fields {
                check_expr(field_expr, bindings, signatures, enum_variants)?;
            }
            Ok(())
        }
        Expr::FieldAccess(expr, _) => {
            check_expr(expr, bindings, signatures, enum_variants)
        }
        Expr::Match { expr, cases } => {
            check_expr(expr, bindings, signatures, enum_variants)?;
            for case in cases {
                check_expr(&case.body, bindings, signatures, enum_variants)?;
            }
            Ok(())
        }
    }
}

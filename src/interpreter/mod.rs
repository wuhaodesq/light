use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Function, Program, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

#[derive(Debug, Clone)]
enum Value {
    Number(f64),
    String(String),
    Unit,
    GpioPin(u32),
    Array(Vec<Value>),
    Struct(HashMap<String, Value>),
    Enum(String, String),
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

    let _ = eval_function(main, Vec::new(), &functions, &program.uses)?;
    Ok(())
}

fn eval_function<'a>(
    function: &'a Function,
    args: Vec<Value>,
    functions: &HashMap<&'a str, &'a Function>,
    uses: &'a [crate::ast::UseStmt],
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
                let value = eval_expr(expr, &mut env, functions, uses)?;
                env.insert(name.clone(), value);
            }
            Stmt::Assign(name, expr) => {
                let value = eval_expr(expr, &mut env, functions, uses)?;
                env.insert(name.clone(), value);
            }
            Stmt::Return(expr) => return eval_expr(expr, &mut env, functions, uses),
            Stmt::Expr(expr) => {
                let _ = eval_expr(expr, &mut env, functions, uses)?;
            }
            Stmt::Use(_) => {}
            Stmt::StructDef { .. } | Stmt::EnumDef { .. } => {}
            Stmt::If { condition, then_block, else_block } => {
                let cond = eval_expr(condition, &mut env, functions, uses)?;
                let cond_truthy = is_truthy(&cond);
                if cond_truthy {
                    for s in then_block {
                        if let Some(val) = eval_stmt(s, &mut env, functions, uses)? {
                            return Ok(val);
                        }
                    }
                } else if let Some(else_b) = else_block {
                    for s in else_b {
                        if let Some(val) = eval_stmt(s, &mut env, functions, uses)? {
                            return Ok(val);
                        }
                    }
                }
            }
            Stmt::While { condition, body } => {
                loop {
                    let cond = eval_expr(condition, &mut env, functions, uses)?;
                    if !is_truthy(&cond) {
                        break;
                    }
                    for s in body {
                        if let Some(val) = eval_stmt(s, &mut env, functions, uses)? {
                            return Ok(val);
                        }
                    }
                }
            }
            Stmt::For { initializer, condition, increment, body } => {
                if let Some(init) = initializer {
                    eval_stmt(init, &mut env, functions, uses)?;
                }
                loop {
                    if let Some(cond) = condition {
                        let cond_val = eval_expr(cond, &mut env, functions, uses)?;
                        if !is_truthy(&cond_val) {
                            break;
                        }
                    }
                    for s in body {
                        if let Some(val) = eval_stmt(s, &mut env, functions, uses)? {
                            return Ok(val);
                        }
                    }
                    if let Some(inc) = increment {
                        eval_expr(inc, &mut env, functions, uses)?;
                    }
                }
            }
        }
    }

    Ok(Value::Unit)
}

fn eval_expr<'a>(
    expr: &Expr,
    env: &mut HashMap<String, Value>,
    functions: &HashMap<&'a str, &'a Function>,
    uses: &'a [crate::ast::UseStmt],
) -> Result<Value, Diagnostic> {
    match expr {
        Expr::Number(value) => Ok(Value::Number(*value)),
        Expr::String(value) => Ok(Value::String(value.clone())),
        Expr::Identifier(name) => {
            if let Some(value) = env.get(name).cloned() {
                Ok(value)
            } else if name.contains("::") {
                let parts: Vec<&str> = name.split("::").collect();
                if parts.len() == 2 {
                    Ok(Value::Enum(parts[0].to_string(), parts[1].to_string()))
                } else {
                    Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        format!("unknown variable `{name}`"),
                        Span::new(0, 0),
                    ))
                }
            } else {
                Err(Diagnostic::new(
                    DiagnosticCode::RuntimeError,
                    format!("unknown variable `{name}`"),
                    Span::new(0, 0),
                ))
            }
        }
        Expr::Array(elements) => {
            let mut values = Vec::new();
            for elem in elements {
                values.push(eval_expr(elem, env, functions, uses)?);
            }
            Ok(Value::Array(values))
        }
        Expr::ArrayIndex(arr_expr, index_expr) => {
            let arr = eval_expr(arr_expr, env, functions, uses)?;
            let index = eval_expr(index_expr, env, functions, uses)?;
            match arr {
                Value::Array(values) => {
                    match index {
                        Value::Number(idx) => {
                            let i = idx as usize;
                            if i >= values.len() {
                                return Err(Diagnostic::new(
                                    DiagnosticCode::RuntimeError,
                                    format!("array index out of bounds: {} >= {}", i, values.len()),
                                    Span::new(0, 0),
                                ));
                            }
                            Ok(values[i].clone())
                        }
                        _ => Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "array index must be a number",
                            Span::new(0, 0),
                        )),
                    }
                }
                _ => Err(Diagnostic::new(
                    DiagnosticCode::RuntimeError,
                    "cannot index non-array value",
                    Span::new(0, 0),
                )),
            }
        }
        Expr::StructInit { name: _, fields } => {
            let mut struct_values = HashMap::new();
            for (field_name, field_expr) in fields {
                let field_value = eval_expr(field_expr, env, functions, uses)?;
                struct_values.insert(field_name.clone(), field_value);
            }
            Ok(Value::Struct(struct_values))
        }
        Expr::FieldAccess(expr, field) => {
            let struct_val = eval_expr(expr, env, functions, uses)?;
            match struct_val {
                Value::Struct(fields) => {
                    fields.get(field).cloned().ok_or_else(|| {
                        Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            format!("unknown field `{field}`"),
                            Span::new(0, 0),
                        )
                    })
                }
                _ => Err(Diagnostic::new(
                    DiagnosticCode::RuntimeError,
                    "cannot access field on non-struct value",
                    Span::new(0, 0),
                )),
            }
        }
        Expr::Match { expr, cases } => {
            let matched_value = eval_expr(expr, env, functions, uses)?;
            for case in cases {
                let matches = match (&matched_value, &case.pattern) {
                    (Value::Number(n), crate::ast::MatchPattern::Number(p)) => (*n - p).abs() < f64::EPSILON,
                    (Value::String(s), crate::ast::MatchPattern::String(p)) => s == p,
                    (Value::Number(_), crate::ast::MatchPattern::Wildcard) => true,
                    (Value::String(_), crate::ast::MatchPattern::Wildcard) => true,
                    (Value::Enum(name, variant), crate::ast::MatchPattern::EnumVariant { name: pname, variant: pvariant, patterns }) => {
                        name == pname && variant == pvariant && patterns.is_empty()
                    }
                    _ => false,
                };
                if matches {
                    return eval_expr(&case.body, env, functions, uses);
                }
            }
            Err(Diagnostic::new(
                DiagnosticCode::RuntimeError,
                "match expression exhausted without a match",
                Span::new(0, 0),
            ))
        }
        Expr::Binary(lhs, op, rhs) => {
            let lhs = eval_expr(lhs, env, functions, uses)?;
            let rhs = eval_expr(rhs, env, functions, uses)?;
            eval_binary(lhs, op, rhs)
        }
        Expr::Call { callee, args } => {
            let mut arg_values = Vec::new();
            for arg in args {
                arg_values.push(eval_expr(arg, env, functions, uses)?);
            }

            if callee == "print" {
                let mut out = String::new();
                for value in &arg_values {
                    let rendered = match value {
                        Value::Number(v) => v.to_string(),
                        Value::String(v) => v.clone(),
                        Value::Unit => String::from("()"),
                        Value::GpioPin(pin) => format!("GPIO pin {}", pin),
                        Value::Array(arr) => {
                            let items: Vec<String> = arr.iter().map(|v| format!("{:?}", v)).collect();
                            format!("[{}]", items.join(", "))
                        }
                        Value::Struct(fields) => {
                            let items: Vec<String> = fields.iter().map(|(k, v)| format!("{k}: {:?}", v)).collect();
                            format!("{{ {} }}", items.join(", "))
                        }
                        Value::Enum(name, variant) => format!("{}::{}", name, variant),
                    };
                    if !out.is_empty() {
                        out.push(' ');
                    }
                    out.push_str(&rendered);
                }
                println!("{out}");
                Ok(Value::Unit)
            } else if callee == "gpio.pin" {
                if arg_values.is_empty() {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "gpio.pin requires a pin number argument",
                        Span::new(0, 0),
                    ));
                }
                let pin = match &arg_values[0] {
                    Value::Number(n) => *n as u32,
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "gpio.pin requires a number argument",
                            Span::new(0, 0),
                        ))
                    }
                };
                println!("[HAL] gpio.pin({})", pin);
                Ok(Value::GpioPin(pin))
            } else if callee == "sleep_ms" {
                if arg_values.is_empty() {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "sleep_ms requires a duration argument",
                        Span::new(0, 0),
                    ));
                }
                let ms = match &arg_values[0] {
                    Value::Number(n) => *n as u64,
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "sleep_ms requires a number argument",
                            Span::new(0, 0),
                        ))
                    }
                };
                println!("[HAL] sleep_ms({})", ms);
                Ok(Value::Unit)
            } else if callee.ends_with(".high") || callee.ends_with(".low") || callee.ends_with(".toggle") {
                println!("[HAL] {}()", callee);
                Ok(Value::Unit)
            } else {
                let function = functions.get(callee.as_str()).copied().ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        format!("unknown function `{callee}`"),
                        Span::new(0, 0),
                    )
                })?;
                eval_function(function, arg_values, functions, uses)
            }
        }
    }
}

fn eval_stmt<'a>(
    stmt: &'a Stmt,
    env: &mut HashMap<String, Value>,
    functions: &HashMap<&'a str, &'a Function>,
    uses: &'a [crate::ast::UseStmt],
) -> Result<Option<Value>, Diagnostic> {
    match stmt {
        Stmt::Let(name, expr) => {
            let value = eval_expr(expr, env, functions, uses)?;
            env.insert(name.clone(), value);
            Ok(None)
        }
        Stmt::Assign(name, expr) => {
            let value = eval_expr(expr, env, functions, uses)?;
            env.insert(name.clone(), value);
            Ok(None)
        }
        Stmt::Return(expr) => Ok(Some(eval_expr(expr, env, functions, uses)?)),
        Stmt::Expr(expr) => {
            let _ = eval_expr(expr, env, functions, uses)?;
            Ok(None)
        }
        Stmt::Use(_) => Ok(None),
        Stmt::StructDef { .. } | Stmt::EnumDef { .. } => Ok(None),
        Stmt::If { condition, then_block, else_block } => {
            let cond = eval_expr(condition, env, functions, uses)?;
            let cond_truthy = is_truthy(&cond);
            if cond_truthy {
                for s in then_block {
                    if let Some(val) = eval_stmt(s, env, functions, uses)? {
                        return Ok(Some(val));
                    }
                }
            } else if let Some(else_b) = else_block {
                for s in else_b {
                    if let Some(val) = eval_stmt(s, env, functions, uses)? {
                        return Ok(Some(val));
                    }
                }
            }
            Ok(None)
        }
        Stmt::While { condition, body } => {
            loop {
                let cond = eval_expr(condition, env, functions, uses)?;
                if !is_truthy(&cond) {
                    break;
                }
                for s in body {
                    if let Some(val) = eval_stmt(s, env, functions, uses)? {
                        return Ok(Some(val));
                    }
                }
            }
            Ok(None)
        }
        Stmt::For { initializer, condition, increment, body } => {
            if let Some(init) = initializer {
                eval_stmt(init, env, functions, uses)?;
            }
            loop {
                if let Some(cond) = condition {
                    let cond_val = eval_expr(cond, env, functions, uses)?;
                    if !is_truthy(&cond_val) {
                        break;
                    }
                }
                for s in body {
                    if let Some(val) = eval_stmt(s, env, functions, uses)? {
                        return Ok(Some(val));
                    }
                }
                if let Some(inc) = increment {
                    eval_expr(inc, env, functions, uses)?;
                }
            }
            Ok(None)
        }
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Number(n) => *n != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Unit => false,
        Value::GpioPin(_) => true,
        Value::Array(arr) => !arr.is_empty(),
        Value::Struct(fields) => !fields.is_empty(),
        Value::Enum(_, _) => true,
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
                BinaryOp::Equal => if lhs == rhs { 1.0 } else { 0.0 },
                BinaryOp::NotEqual => if lhs != rhs { 1.0 } else { 0.0 },
                BinaryOp::Less => if lhs < rhs { 1.0 } else { 0.0 },
                BinaryOp::LessEqual => if lhs <= rhs { 1.0 } else { 0.0 },
                BinaryOp::Greater => if lhs > rhs { 1.0 } else { 0.0 },
                BinaryOp::GreaterEqual => if lhs >= rhs { 1.0 } else { 0.0 },
            };
            Ok(Value::Number(value))
        }
        (Value::String(lhs), Value::String(rhs)) => {
            match op {
                BinaryOp::Add => Ok(Value::String(lhs + &rhs)),
                BinaryOp::Equal => Ok(Value::Number(if lhs == rhs { 1.0 } else { 0.0 })),
                BinaryOp::NotEqual => Ok(Value::Number(if lhs != rhs { 1.0 } else { 0.0 })),
                _ => Err(Diagnostic::new(
                    DiagnosticCode::RuntimeError,
                    "comparison operators not supported for strings",
                    Span::new(0, 0),
                )),
            }
        }
        (Value::String(lhs), Value::Number(rhs)) => {
            match op {
                BinaryOp::Add => Ok(Value::String(lhs + &rhs.to_string())),
                BinaryOp::Equal => Ok(Value::Number(0.0)),
                BinaryOp::NotEqual => Ok(Value::Number(1.0)),
                _ => Err(Diagnostic::new(
                    DiagnosticCode::RuntimeError,
                    "comparison operators not supported for string/number",
                    Span::new(0, 0),
                )),
            }
        }
        (Value::Number(lhs), Value::String(rhs)) => {
            match op {
                BinaryOp::Add => Ok(Value::String(lhs.to_string() + &rhs)),
                BinaryOp::Equal => Ok(Value::Number(0.0)),
                BinaryOp::NotEqual => Ok(Value::Number(1.0)),
                _ => Err(Diagnostic::new(
                    DiagnosticCode::RuntimeError,
                    "comparison operators not supported for string/number",
                    Span::new(0, 0),
                )),
            }
        }
        (Value::Enum(lhs_name, lhs_variant), Value::Enum(rhs_name, rhs_variant)) => {
            match op {
                BinaryOp::Equal => Ok(Value::Number(if lhs_name == rhs_name && lhs_variant == rhs_variant { 1.0 } else { 0.0 })),
                BinaryOp::NotEqual => Ok(Value::Number(if lhs_name == rhs_name && lhs_variant == rhs_variant { 0.0 } else { 1.0 })),
                _ => Err(Diagnostic::new(
                    DiagnosticCode::RuntimeError,
                    "comparison operators not supported for enums",
                    Span::new(0, 0),
                )),
            }
        }
        _ => Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            "binary operators support numbers and strings only",
            Span::new(0, 0),
        )),
    }
}

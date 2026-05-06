use std::collections::HashMap;

use crate::ast::{BinaryOp, Expr, Function, Program, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

#[derive(Debug, Clone, PartialEq)]
enum Value {
    Number(f64),
    String(String),
    Boolean(bool),
    Unit,
    GpioPin(u32),
    Array(Vec<Value>),
    Struct(HashMap<String, Value>),
    Enum(String, String),
    Break,
    Continue,
}

impl Value {
    fn is_break(&self) -> bool {
        matches!(self, Value::Break)
    }

    fn is_continue(&self) -> bool {
        matches!(self, Value::Continue)
    }

    fn display(&self) -> String {
        match self {
            Value::Number(n) => {
                if *n == n.floor() && n.abs() < 1e10 {
                    n.floor().to_string()
                } else {
                    n.to_string()
                }
            }
            Value::String(s) => s.clone(),
            Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
            Value::Unit => String::from("()"),
            Value::GpioPin(pin) => format!("GPIO pin {}", pin),
            Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.display()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Struct(fields) => {
                let items: Vec<String> = fields.iter().map(|(k, v)| format!("{}: {}", k, v.display())).collect();
                format!("{{ {} }}", items.join(", "))
            }
            Value::Enum(name, variant) => format!("{}::{}", name, variant),
            Value::Break | Value::Continue => String::from("?"),
        }
    }
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
            Stmt::Break => return Ok(Value::Break),
            Stmt::Continue => return Ok(Value::Continue),
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
                'while_loop: loop {
                    let cond = eval_expr(condition, &mut env, functions, uses)?;
                    if !is_truthy(&cond) {
                        break;
                    }
                    'body_loop: for s in body {
                        match eval_stmt(s, &mut env, functions, uses)? {
                            Some(val) if val.is_break() => break 'while_loop,
                            Some(val) if val.is_continue() => break 'body_loop,
                            Some(val) => return Ok(val),
                            None => {}
                        }
                    }
                }
            }
            Stmt::For { initializer, condition, increment, body } => {
                if let Some(init) = initializer {
                    eval_stmt(init, &mut env, functions, uses)?;
                }
                'for_loop: loop {
                    if let Some(cond) = condition {
                        let cond_val = eval_expr(cond, &mut env, functions, uses)?;
                        if !is_truthy(&cond_val) {
                            break;
                        }
                    }
                    'body_loop: for s in body {
                        match eval_stmt(s, &mut env, functions, uses)? {
                            Some(val) if val.is_break() => break 'for_loop,
                            Some(val) if val.is_continue() => break 'body_loop,
                            Some(val) => return Ok(val),
                            None => {}
                        }
                    }
                    if let Some(inc) = increment {
                        eval_expr(inc, &mut env, functions, uses)?;
                    }
                }
            }
            Stmt::Loop { body } => {
                'loop_label: loop {
                    for s in body {
                        match eval_stmt(s, &mut env, functions, uses)? {
                            Some(val) if val.is_break() => break 'loop_label,
                            Some(val) if val.is_continue() => break,
                            Some(val) => return Ok(val),
                            None => {}
                        }
                    }
                }
            }
        }
    }

    Ok(Value::Unit)
}

fn gcd_impl(a: i64, b: i64) -> i64 {
    if b == 0 { a } else { gcd_impl(b, a % b) }
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
                        Value::Boolean(b) => if *b { "true" } else { "false" }.to_string(),
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
                        Value::Break | Value::Continue => continue,
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
            } else if callee == "len" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "len requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                let len = match &arg_values[0] {
                    Value::String(s) => s.len() as f64,
                    Value::Array(arr) => arr.len() as f64,
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "len requires a string or array argument",
                            Span::new(0, 0),
                        ))
                    }
                };
                Ok(Value::Number(len))
            } else if callee == "empty" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "empty requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                let is_empty = match &arg_values[0] {
                    Value::String(s) => s.is_empty(),
                    Value::Array(arr) => arr.is_empty(),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "empty requires a string or array argument",
                            Span::new(0, 0),
                        ))
                    }
                };
                Ok(Value::Number(if is_empty { 1.0 } else { 0.0 }))
            } else if callee == "is_empty" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_empty requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                let is_empty = match &arg_values[0] {
                    Value::String(s) => s.is_empty(),
                    Value::Array(arr) => arr.is_empty(),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "is_empty requires a string or array argument",
                            Span::new(0, 0),
                        ))
                    }
                };
                Ok(Value::Boolean(is_empty))
            } else if callee == "first" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "first requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) if !arr.is_empty() => Ok(arr[0].clone()),
                    Value::Array(_) => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "first on empty array",
                            Span::new(0, 0),
                        ))
                    }
                    Value::String(s) if !s.is_empty() => {
                        Ok(Value::String(s.chars().next().unwrap().to_string()))
                    }
                    Value::String(_) => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "first on empty string",
                            Span::new(0, 0),
                        ))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "first requires an array or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "last" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "last requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) if !arr.is_empty() => Ok(arr[arr.len() - 1].clone()),
                    Value::Array(_) => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "last on empty array",
                            Span::new(0, 0),
                        ))
                    }
                    Value::String(s) if !s.is_empty() => {
                        Ok(Value::String(s.chars().last().unwrap().to_string()))
                    }
                    Value::String(_) => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "last on empty string",
                            Span::new(0, 0),
                        ))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "last requires an array or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "push" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "push requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let mut new_arr = arr.clone();
                        new_arr.push(arg_values[1].clone());
                        Ok(Value::Array(new_arr))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "push requires an array as first argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "pop" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "pop requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) if !arr.is_empty() => {
                        let mut new_arr = arr.clone();
                        new_arr.pop();
                        Ok(Value::Array(new_arr))
                    }
                    Value::Array(_) => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "pop on empty array",
                            Span::new(0, 0),
                        ))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "pop requires an array argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "append" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "append requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr1), Value::Array(arr2)) => {
                        let mut new_arr = arr1.clone();
                        for item in arr2 {
                            new_arr.push(item.clone());
                        }
                        Ok(Value::Array(new_arr))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "append requires two arrays as arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "unshift" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "unshift requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), val) => {
                        let mut new_arr = vec![val.clone()];
                        for item in arr {
                            new_arr.push(item.clone());
                        }
                        Ok(Value::Array(new_arr))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "unshift requires an array and a value",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "product" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "product requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let total: f64 = arr.iter()
                            .filter_map(|v| match v { Value::Number(n) => Some(*n), _ => None })
                            .product();
                        Ok(Value::Number(total))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "product requires an array of numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "sort" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "sort requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let mut nums: Vec<f64> = arr.iter()
                            .filter_map(|v| match v { Value::Number(n) => Some(*n), _ => None })
                            .collect();
                        nums.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let sorted: Vec<Value> = nums.into_iter().map(Value::Number).collect();
                        Ok(Value::Array(sorted))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "sort requires an array of numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "contains" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "contains requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::String(sub)) => {
                        Ok(Value::Boolean(s.contains(sub)))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "contains requires two strings as arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "index_of" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "index_of requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::String(sub)) => {
                        Ok(Value::Number(s.find(sub).map(|i| i as f64).unwrap_or(-1.0)))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "index_of requires two strings as arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "last_index_of" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "last_index_of requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::String(sub)) => {
                        Ok(Value::Number(s.rfind(sub).map(|i| i as f64).unwrap_or(-1.0)))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "last_index_of requires two strings as arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "includes" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "includes requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), val) => {
                        let found = arr.iter().any(|v| v == val);
                        Ok(Value::Boolean(found))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "includes requires an array and a value as arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "find_index" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "find_index requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), val) => {
                        let idx = arr.iter().position(|v| v == val);
                        Ok(Value::Number(idx.map(|i| i as f64).unwrap_or(-1.0)))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "find_index requires an array and a value as arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "reverse" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "reverse requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let mut reversed = arr.clone();
                        reversed.reverse();
                        Ok(Value::Array(reversed))
                    }
                    Value::String(s) => {
                        let mut chars: Vec<char> = s.chars().collect();
                        chars.reverse();
                        Ok(Value::String(chars.into_iter().collect()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "reverse requires an array or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "slice" {
                if arg_values.len() != 3 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "slice requires exactly three arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1], &arg_values[2]) {
                    (Value::Array(arr), Value::Number(start), Value::Number(end)) => {
                        let start = *start as usize;
                        let end = *end as usize;
                        if start < arr.len() && end <= arr.len() && start < end {
                            Ok(Value::Array(arr[start..end].to_vec()))
                        } else {
                            Ok(Value::Array(vec![]))
                        }
                    }
                    (Value::String(s), Value::Number(start), Value::Number(end)) => {
                        let start = *start as usize;
                        let end = *end as usize;
                        let chars: Vec<char> = s.chars().collect();
                        if start < chars.len() && end <= chars.len() && start < end {
                            Ok(Value::String(chars[start..end].iter().collect()))
                        } else {
                            Ok(Value::String(String::new()))
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "slice requires an array/string, start index, and end index",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "char_at" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "char_at requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::Number(idx)) => {
                        let idx = *idx as usize;
                        let chars: Vec<char> = s.chars().collect();
                        if idx < chars.len() {
                            Ok(Value::String(chars[idx].to_string()))
                        } else {
                            Ok(Value::String(String::new()))
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "char_at requires a string and an index",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "take" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "take requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let n = *n as usize;
                        let end = n.min(arr.len());
                        Ok(Value::Array(arr[..end].to_vec()))
                    }
                    (Value::String(s), Value::Number(n)) => {
                        let n = *n as usize;
                        Ok(Value::String(s.chars().take(n).collect()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "take requires an array/string and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "drop" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "drop requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let n = *n as usize;
                        let start = n.min(arr.len());
                        Ok(Value::Array(arr[start..].to_vec()))
                    }
                    (Value::String(s), Value::Number(n)) => {
                        let n = *n as usize;
                        Ok(Value::String(s.chars().skip(n).collect()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "drop requires an array/string and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "range" {
                match arg_values.len() {
                    1 => {
                        match &arg_values[0] {
                            Value::Number(n) => {
                                let n = *n as usize;
                                let arr: Vec<Value> = (0..n).map(|i| Value::Number(i as f64)).collect();
                                Ok(Value::Array(arr))
                            }
                            _ => {
                                return Err(Diagnostic::new(
                                    DiagnosticCode::RuntimeError,
                                    "range requires a number argument",
                                    Span::new(0, 0),
                                ))
                            }
                        }
                    }
                    2 => {
                        match (&arg_values[0], &arg_values[1]) {
                            (Value::Number(start), Value::Number(end)) => {
                                let start = *start as usize;
                                let end = *end as usize;
                                let arr: Vec<Value> = (start..end).map(|i| Value::Number(i as f64)).collect();
                                Ok(Value::Array(arr))
                            }
                            _ => {
                                return Err(Diagnostic::new(
                                    DiagnosticCode::RuntimeError,
                                    "range requires two number arguments",
                                    Span::new(0, 0),
                                ))
                            }
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "range requires one or two arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "lines" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "lines requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => {
                        let parts: Vec<Value> = s.lines()
                            .map(|l| Value::String(l.to_string()))
                            .collect();
                        Ok(Value::Array(parts))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "lines requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "flatten" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "flatten requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let mut result = vec![];
                        fn flatten_recursive(arr: &[Value], result: &mut Vec<Value>) {
                            for item in arr {
                                match item {
                                    Value::Array(inner) => flatten_recursive(inner, result),
                                    _ => result.push(item.clone()),
                                }
                            }
                        }
                        flatten_recursive(arr, &mut result);
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "flatten requires an array argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "unique" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "unique requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let mut result = vec![];
                        for item in arr {
                            if !result.contains(item) {
                                result.push(item.clone());
                            }
                        }
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "unique requires an array argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "any" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "any requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let truthy = arr.iter().any(|v| is_truthy(v));
                        Ok(Value::Boolean(truthy))
                    }
                    Value::String(s) => {
                        Ok(Value::Boolean(!s.is_empty()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "any requires an array or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "all" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "all requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let truthy = arr.iter().all(|v| is_truthy(v));
                        Ok(Value::Boolean(truthy))
                    }
                    Value::String(s) => {
                        Ok(Value::Boolean(!s.is_empty()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "all requires an array or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "zip" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "zip requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr1), Value::Array(arr2)) => {
                        let len = arr1.len().min(arr2.len());
                        let result: Vec<Value> = (0..len).map(|i| {
                            Value::Array(vec![arr1[i].clone(), arr2[i].clone()])
                        }).collect();
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "zip requires two arrays as arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "enumerate" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "enumerate requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let result: Vec<Value> = arr.iter().enumerate().map(|(i, v)| {
                            Value::Array(vec![Value::Number(i as f64), v.clone()])
                        }).collect();
                        Ok(Value::Array(result))
                    }
                    Value::String(s) => {
                        let result: Vec<Value> = s.char_indices().map(|(i, c)| {
                            Value::Array(vec![Value::Number(i as f64), Value::String(c.to_string())])
                        }).collect();
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "enumerate requires an array or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "chunk" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "chunk requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(size)) => {
                        let size = *size as usize;
                        if size == 0 {
                            return Err(Diagnostic::new(
                                DiagnosticCode::RuntimeError,
                                "chunk size must be greater than 0",
                                Span::new(0, 0),
                            ));
                        }
                        let result: Vec<Value> = arr.chunks(size)
                            .map(|chunk| Value::Array(chunk.to_vec()))
                            .collect();
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "chunk requires an array and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "average" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "average requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let nums: Vec<f64> = arr.iter()
                            .filter_map(|v| match v { Value::Number(n) => Some(*n), _ => None })
                            .collect();
                        if nums.is_empty() {
                            return Err(Diagnostic::new(
                                DiagnosticCode::RuntimeError,
                                "average requires a non-empty array of numbers",
                                Span::new(0, 0),
                            ));
                        }
                        let sum: f64 = nums.iter().sum();
                        Ok(Value::Number(sum / nums.len() as f64))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "average requires an array of numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "find" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "find requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(target)) => {
                        let found = arr.iter().find(|v| {
                            if let Value::Number(n) = v {
                                *n == *target
                            } else {
                                false
                            }
                        });
                        Ok(found.cloned().unwrap_or(Value::Unit))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "find requires an array and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "filter" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "filter requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(threshold)) => {
                        let filtered: Vec<Value> = arr.iter()
                            .filter(|v| {
                                if let Value::Number(n) = v {
                                    *n >= *threshold
                                } else {
                                    false
                                }
                            })
                            .cloned()
                            .collect();
                        Ok(Value::Array(filtered))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "filter requires an array and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "reduce" {
                if arg_values.len() != 3 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "reduce requires exactly three arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1], &arg_values[2]) {
                    (Value::Array(arr), Value::Number(init), Value::Number(target)) => {
                        let result = arr.iter().fold(*init, |acc, v| {
                            if let Value::Number(n) = v {
                                if *target == 0.0 {
                                    acc + n
                                } else {
                                    acc * n
                                }
                            } else {
                                acc
                            }
                        });
                        Ok(Value::Number(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "reduce requires an array and two numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "chars" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "chars requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => {
                        let result: Vec<Value> = s.chars()
                            .map(|c| Value::String(c.to_string()))
                            .collect();
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "chars requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "codes" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "codes requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => {
                        let result: Vec<Value> = s.chars()
                            .map(|c| Value::Number(c as u32 as f64))
                            .collect();
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "codes requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "chr" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "chr requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => {
                        let c = char::from_u32(*n as u32).unwrap_or('\u{FFFD}');
                        Ok(Value::String(c.to_string()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "chr requires a number argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "concat" {
                if arg_values.len() < 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "concat requires at least two arguments",
                        Span::new(0, 0),
                    ));
                }
                let mut result = vec![];
                for arg in &arg_values {
                    match arg {
                        Value::Array(arr) => {
                            for item in arr {
                                result.push(item.clone());
                            }
                        }
                        Value::String(s) => {
                            result.push(Value::String(s.clone()));
                        }
                        _ => {
                            return Err(Diagnostic::new(
                                DiagnosticCode::RuntimeError,
                                "concat requires arrays or strings",
                                Span::new(0, 0),
                            ))
                        }
                    }
                }
                Ok(Value::Array(result))
            } else if callee == "every" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "every requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let truthy = arr.iter().all(|v| is_truthy(v));
                        Ok(Value::Boolean(truthy))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "every requires an array argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "some" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "some requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let truthy = arr.iter().any(|v| is_truthy(v));
                        Ok(Value::Boolean(truthy))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "some requires an array argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "count" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "count requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), val) => {
                        let cnt = arr.iter().filter(|v| *v == val).count();
                        Ok(Value::Number(cnt as f64))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "count requires an array and a value",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "has" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "has requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), val) => {
                        let found = arr.iter().any(|v| *v == *val);
                        Ok(Value::Boolean(found))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "has requires an array and a value",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "keys" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "keys requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Struct(fields) => {
                        let keys: Vec<Value> = fields.keys()
                            .map(|k| Value::String(k.clone()))
                            .collect();
                        Ok(Value::Array(keys))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "keys requires a struct argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "values" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "values requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Struct(fields) => {
                        let vals: Vec<Value> = fields.values()
                            .map(|v| v.clone())
                            .collect();
                        Ok(Value::Array(vals))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "values requires a struct argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "fill" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "fill requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Number(size), val) => {
                        let size = *size as usize;
                        let result = vec![val.clone(); size];
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "fill requires a number and a value",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "repeat_n" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "repeat_n requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(n)) => {
                        let n = *n as usize;
                        let mut result = vec![];
                        for _ in 0..n {
                            for item in arr {
                                result.push(item.clone());
                            }
                        }
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "repeat_n requires an array and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "max_by" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "max_by requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let nums: Vec<f64> = arr.iter()
                            .filter_map(|v| match v { Value::Number(n) => Some(*n), _ => None })
                            .collect();
                        if nums.is_empty() {
                            return Err(Diagnostic::new(
                                DiagnosticCode::RuntimeError,
                                "max_by requires a non-empty array",
                                Span::new(0, 0),
                            ));
                        }
                        Ok(Value::Number(nums.into_iter().fold(f64::NAN, |a, b| a.max(b))))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "max_by requires an array",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "min_by" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "min_by requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let nums: Vec<f64> = arr.iter()
                            .filter_map(|v| match v { Value::Number(n) => Some(*n), _ => None })
                            .collect();
                        if nums.is_empty() {
                            return Err(Diagnostic::new(
                                DiagnosticCode::RuntimeError,
                                "min_by requires a non-empty array",
                                Span::new(0, 0),
                            ));
                        }
                        Ok(Value::Number(nums.into_iter().fold(f64::INFINITY, |a, b| a.min(b))))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "min_by requires an array",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "sign" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "sign requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => {
                        Ok(Value::Number(n.signum()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "sign requires a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "trunc" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "trunc requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => {
                        Ok(Value::Number(n.trunc()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "trunc requires a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "fract" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "fract requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => {
                        Ok(Value::Number(n.fract()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "fract requires a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "split_at" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "split_at requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(idx)) => {
                        let idx = *idx as usize;
                        if idx >= arr.len() {
                            return Ok(Value::Array(vec![Value::Array(arr.clone()), Value::Array(vec![])]));
                        }
                        let left = arr[..idx].to_vec();
                        let right = arr[idx..].to_vec();
                        Ok(Value::Array(vec![Value::Array(left), Value::Array(right)]))
                    }
                    (Value::String(s), Value::Number(idx)) => {
                        let idx = *idx as usize;
                        let chars: Vec<char> = s.chars().collect();
                        if idx >= chars.len() {
                            return Ok(Value::Array(vec![Value::String(s.clone()), Value::String(String::new())]));
                        }
                        let left: String = chars[..idx].iter().collect();
                        let right: String = chars[idx..].iter().collect();
                        Ok(Value::Array(vec![Value::String(left), Value::String(right)]))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "split_at requires an array/string and an index",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "partition" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "partition requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(threshold)) => {
                        let mut left = vec![];
                        let mut right = vec![];
                        for item in arr {
                            if let Value::Number(n) = item {
                                if *n < *threshold {
                                    left.push(item.clone());
                                } else {
                                    right.push(item.clone());
                                }
                            }
                        }
                        Ok(Value::Array(vec![Value::Array(left), Value::Array(right)]))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "partition requires an array and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "zip_with" {
                if arg_values.len() != 3 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "zip_with requires exactly three arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1], &arg_values[2]) {
                    (Value::Array(arr1), Value::Array(arr2), Value::Number(op)) => {
                        let len = arr1.len().min(arr2.len());
                        let mut result = vec![];
                        for i in 0..len {
                            let a = &arr1[i];
                            let b = &arr2[i];
                            let res = match (a, b, (*op as u8)) {
                                (Value::Number(a), Value::Number(b), 0) => Value::Number(a + b),
                                (Value::Number(a), Value::Number(b), 1) => Value::Number(a - b),
                                (Value::Number(a), Value::Number(b), 2) => Value::Number(a * b),
                                (Value::Number(a), Value::Number(b), 3) if *b != 0.0 => Value::Number(a / b),
                                _ => Value::Unit,
                            };
                            result.push(res);
                        }
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "zip_with requires two arrays and an operation code",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "intersperse" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "intersperse requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), val) => {
                        if arr.is_empty() {
                            return Ok(Value::Array(vec![]));
                        }
                        let mut result = vec![];
                        for (i, item) in arr.iter().enumerate() {
                            if i > 0 {
                                result.push(val.clone());
                            }
                            result.push(item.clone());
                        }
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "intersperse requires an array and a value",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "windows" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "windows requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(size)) => {
                        let size = *size as usize;
                        if size == 0 || size > arr.len() {
                            return Ok(Value::Array(vec![]));
                        }
                        let result: Vec<Value> = arr.windows(size)
                            .map(|w| Value::Array(w.to_vec()))
                            .collect();
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "windows requires an array and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "transpose" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "transpose requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        if arr.is_empty() {
                            return Ok(Value::Array(vec![]));
                        }
                        let mut result: Vec<Vec<Value>> = vec![];
                        for item in arr {
                            if let Value::Array(inner) = item {
                                result.push(inner.clone());
                            }
                        }
                        if result.is_empty() || result[0].is_empty() {
                            return Ok(Value::Array(vec![]));
                        }
                        let num_cols = result[0].len();
                        let mut transposed: Vec<Vec<Value>> = vec![vec![]; num_cols];
                        for row in result {
                            for (j, val) in row.iter().enumerate() {
                                if j < num_cols {
                                    transposed[j].push(val.clone());
                                }
                            }
                        }
                        Ok(Value::Array(transposed.into_iter().map(Value::Array).collect()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "transpose requires an array of arrays",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "scan" {
                if arg_values.len() != 3 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "scan requires exactly three arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1], &arg_values[2]) {
                    (Value::Array(arr), Value::Number(init), Value::Number(op)) => {
                        let mut acc = *init;
                        let mut result = vec![];
                        for item in arr {
                            if let Value::Number(n) = item {
                                acc = match *op as u8 {
                                    0 => acc + n,
                                    1 => acc - n,
                                    2 => acc * n,
                                    _ => acc,
                                };
                            }
                            result.push(Value::Number(acc));
                        }
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "scan requires an array and two numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "group_by" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "group_by requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(size)) => {
                        let size = *size as usize;
                        if size == 0 {
                            return Err(Diagnostic::new(
                                DiagnosticCode::RuntimeError,
                                "group_by size must be greater than 0",
                                Span::new(0, 0),
                            ));
                        }
                        let mut result = vec![];
                        let mut group = vec![];
                        for item in arr {
                            group.push(item.clone());
                            if group.len() == size {
                                result.push(Value::Array(group.clone()));
                                group.clear();
                            }
                        }
                        if !group.is_empty() {
                            result.push(Value::Array(group));
                        }
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "group_by requires an array and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "to_chars" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "to_chars requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => {
                        let result: Vec<Value> = s.chars()
                            .map(|c| Value::String(c.to_string()))
                            .collect();
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "to_chars requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "to_codes" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "to_codes requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => {
                        let result: Vec<Value> = s.chars()
                            .map(|c| Value::Number(c as u32 as f64))
                            .collect();
                        Ok(Value::Array(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "to_codes requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "capitalize" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "capitalize requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => {
                        let mut chars: Vec<char> = s.chars().collect();
                        if let Some(first) = chars.first() {
                            chars[0] = first.to_uppercase().next().unwrap_or(*first);
                        }
                        Ok(Value::String(chars.into_iter().collect()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "capitalize requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "is_alpha" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_alpha requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => {
                        let result = !s.is_empty() && s.chars().all(|c| c.is_alphabetic());
                        Ok(Value::Boolean(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "is_alpha requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "is_digit" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_digit requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => {
                        let result = !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
                        Ok(Value::Boolean(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "is_digit requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "is_space" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_space requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => {
                        let result = !s.is_empty() && s.chars().all(|c| c.is_whitespace());
                        Ok(Value::Boolean(result))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "is_space requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "abs_diff" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "abs_diff requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Number(a), Value::Number(b)) => {
                        Ok(Value::Number((a - b).abs()))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "abs_diff requires two numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "mod" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "mod requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Number(a), Value::Number(b)) => {
                        if *b != 0.0 {
                            Ok(Value::Number(a % b))
                        } else {
                            return Err(Diagnostic::new(
                                DiagnosticCode::RuntimeError,
                                "mod division by zero",
                                Span::new(0, 0),
                            ))
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "mod requires two numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "gcd" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "gcd requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Number(a), Value::Number(b)) => {
                        let mut a = a.abs() as i64;
                        let mut b = b.abs() as i64;
                        while b != 0 {
                            let temp = b;
                            b = a % b;
                            a = temp;
                        }
                        Ok(Value::Number(a as f64))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "gcd requires two numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "lcm" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "lcm requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Number(a), Value::Number(b)) => {
                        if *a == 0.0 || *b == 0.0 {
                            return Ok(Value::Number(0.0));
                        }
                        let a = a.abs() as i64;
                        let b = b.abs() as i64;
                        let result = (a * b) / gcd_impl(a, b);
                        Ok(Value::Number(result as f64))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "lcm requires two numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "is_negative" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_negative requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Boolean(*n < 0.0)),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "is_negative requires a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "is_positive" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_positive requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Boolean(*n > 0.0)),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "is_positive requires a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "is_zero" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_zero requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Boolean(*n == 0.0)),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "is_zero requires a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "increment" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "increment requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(n + 1.0)),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "increment requires a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "decrement" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "decrement requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(n - 1.0)),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "decrement requires a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "insert" {
                if arg_values.len() != 3 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "insert requires exactly three arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1], &arg_values[2]) {
                    (Value::Array(arr), Value::Number(idx), val) => {
                        let mut new_arr = arr.clone();
                        let idx = *idx as usize;
                        if idx <= new_arr.len() {
                            new_arr.insert(idx, val.clone());
                        }
                        Ok(Value::Array(new_arr))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "insert requires an array, index, and value",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "remove" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "remove requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::Number(idx)) => {
                        let mut new_arr = arr.clone();
                        let idx = *idx as usize;
                        if idx < new_arr.len() {
                            new_arr.remove(idx);
                        }
                        Ok(Value::Array(new_arr))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "remove requires an array and an index",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "split" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "split requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::String(sep)) => {
                        let parts: Vec<Value> = s.split(sep)
                            .map(|part| Value::String(part.to_string()))
                            .collect();
                        Ok(Value::Array(parts))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "split requires two strings as arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "repeat" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "repeat requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::Number(n)) => {
                        let n = *n as usize;
                        Ok(Value::String(s.repeat(n)))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "repeat requires a string and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "replace_all" {
                if arg_values.len() != 3 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "replace_all requires exactly three arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1], &arg_values[2]) {
                    (Value::String(s), Value::String(from), Value::String(to)) => {
                        Ok(Value::String(s.replace(from, to)))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "replace_all requires three string arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "trim_start" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "trim_start requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => Ok(Value::String(s.trim_start().to_string())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "trim_start requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "trim_end" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "trim_end requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => Ok(Value::String(s.trim_end().to_string())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "trim_end requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "pad_start" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "pad_start requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::Number(len)) => {
                        let len = *len as usize;
                        if s.len() < len {
                            let pad = " ".repeat(len - s.len());
                            Ok(Value::String(pad + s))
                        } else {
                            Ok(Value::String(s.clone()))
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "pad_start requires a string and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "pad_end" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "pad_end requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::Number(len)) => {
                        let len = *len as usize;
                        if s.len() < len {
                            let pad = " ".repeat(len - s.len());
                            Ok(Value::String(s.clone() + &pad))
                        } else {
                            Ok(Value::String(s.clone()))
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "pad_end requires a string and a number",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "sum" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "sum requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Array(arr) => {
                        let total: f64 = arr.iter()
                            .filter_map(|v| match v { Value::Number(n) => Some(*n), _ => None })
                            .sum();
                        Ok(Value::Number(total))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "sum requires an array of numbers",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "trim" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "trim requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => Ok(Value::String(s.trim().to_string())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "trim requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "replace" {
                if arg_values.len() != 3 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "replace requires exactly three arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1], &arg_values[2]) {
                    (Value::String(s), Value::String(from), Value::String(to)) => {
                        Ok(Value::String(s.replace(from, to)))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "replace requires three string arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "join" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "join requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Array(arr), Value::String(sep)) => {
                        let parts: Vec<String> = arr.iter().map(|v| v.display()).collect();
                        Ok(Value::String(parts.join(sep)))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "join requires an array and a string as arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "to_uppercase" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "to_uppercase requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => Ok(Value::String(s.to_uppercase())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "to_uppercase requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "to_lowercase" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "to_lowercase requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::String(s) => Ok(Value::String(s.to_lowercase())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "to_lowercase requires a string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "starts_with" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "starts_with requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::String(prefix)) => Ok(Value::Boolean(s.starts_with(prefix))),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "starts_with requires two string arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "ends_with" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "ends_with requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::String(s), Value::String(suffix)) => Ok(Value::Boolean(s.ends_with(suffix))),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "ends_with requires two string arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "abs" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "abs requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(n.abs())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "abs requires a number argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "min" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "min requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.min(*b))),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "min requires two number arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "max" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "max requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.max(*b))),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "max requires two number arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "pow" {
                if arg_values.len() != 2 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "pow requires exactly two arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1]) {
                    (Value::Number(base), Value::Number(exp)) => Ok(Value::Number(base.powf(*exp))),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "pow requires two number arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "floor" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "floor requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(n.floor())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "floor requires a number argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "ceil" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "ceil requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(n.ceil())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "ceil requires a number argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "round" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "round requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(n.round())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "round requires a number argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "sqrt" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "sqrt requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(n.sqrt())),
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "sqrt requires a number argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "to_string" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "to_string requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                Ok(Value::String(arg_values[0].display()))
            } else if callee == "to_i32" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "to_i32 requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(n.trunc())),
                    Value::String(s) => {
                        match s.parse::<f64>() {
                            Ok(n) => Ok(Value::Number(n.trunc())),
                            Err(_) => {
                                return Err(Diagnostic::new(
                                    DiagnosticCode::RuntimeError,
                                    format!("cannot convert string \"{}\" to i32", s),
                                    Span::new(0, 0),
                                ))
                            }
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "to_i32 requires a number or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "to_f64" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "to_f64 requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(*n)),
                    Value::String(s) => {
                        match s.parse::<f64>() {
                            Ok(n) => Ok(Value::Number(n)),
                            Err(_) => {
                                return Err(Diagnostic::new(
                                    DiagnosticCode::RuntimeError,
                                    format!("cannot convert string \"{}\" to f64", s),
                                    Span::new(0, 0),
                                ))
                            }
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "to_f64 requires a number or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "parse_int" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "parse_int requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(n.trunc())),
                    Value::String(s) => {
                        match s.parse::<f64>() {
                            Ok(n) => Ok(Value::Number(n.trunc())),
                            Err(_) => {
                                return Err(Diagnostic::new(
                                    DiagnosticCode::RuntimeError,
                                    format!("cannot convert string \"{}\" to i32", s),
                                    Span::new(0, 0),
                                ))
                            }
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "parse_int requires a number or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "parse_float" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "parse_float requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                match &arg_values[0] {
                    Value::Number(n) => Ok(Value::Number(*n)),
                    Value::String(s) => {
                        match s.parse::<f64>() {
                            Ok(n) => Ok(Value::Number(n)),
                            Err(_) => {
                                return Err(Diagnostic::new(
                                    DiagnosticCode::RuntimeError,
                                    format!("cannot convert string \"{}\" to f64", s),
                                    Span::new(0, 0),
                                ))
                            }
                        }
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "parse_float requires a number or string argument",
                            Span::new(0, 0),
                        ))
                    }
                }
            } else if callee == "is_number" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_number requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                Ok(Value::Boolean(matches!(&arg_values[0], Value::Number(_))))
            } else if callee == "is_string" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_string requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                Ok(Value::Boolean(matches!(&arg_values[0], Value::String(_))))
            } else if callee == "is_array" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_array requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                Ok(Value::Boolean(matches!(&arg_values[0], Value::Array(_))))
            } else if callee == "is_boolean" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "is_boolean requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                Ok(Value::Boolean(matches!(&arg_values[0], Value::Boolean(_))))
            } else if callee == "typeof" {
                if arg_values.len() != 1 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "typeof requires exactly one argument",
                        Span::new(0, 0),
                    ));
                }
                let type_name = match &arg_values[0] {
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Boolean(_) => "boolean",
                    Value::Unit => "unit",
                    Value::GpioPin(_) => "gpio",
                    Value::Array(_) => "array",
                    Value::Struct(_) => "struct",
                    Value::Enum(_, _) => "enum",
                    Value::Break | Value::Continue => "control",
                };
                Ok(Value::String(type_name.to_string()))
            } else if callee == "clamp" {
                if arg_values.len() != 3 {
                    return Err(Diagnostic::new(
                        DiagnosticCode::RuntimeError,
                        "clamp requires exactly three arguments",
                        Span::new(0, 0),
                    ));
                }
                match (&arg_values[0], &arg_values[1], &arg_values[2]) {
                    (Value::Number(val), Value::Number(min), Value::Number(max)) => {
                        Ok(Value::Number(val.max(*min).min(*max)))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::RuntimeError,
                            "clamp requires three number arguments",
                            Span::new(0, 0),
                        ))
                    }
                }
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
        Stmt::Loop { body } => {
            loop {
                for s in body {
                    match eval_stmt(s, env, functions, uses)? {
                        Some(val) if val.is_break() => break,
                        Some(val) if val.is_continue() => break,
                        Some(val) => return Ok(Some(val)),
                        None => {}
                    }
                }
            }
        }
        Stmt::Break => Ok(Some(Value::Break)),
        Stmt::Continue => Ok(Some(Value::Continue)),
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Number(n) => *n != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Boolean(b) => *b,
        Value::Unit => false,
        Value::GpioPin(_) => true,
        Value::Array(arr) => !arr.is_empty(),
        Value::Struct(fields) => !fields.is_empty(),
        Value::Enum(_, _) => true,
        Value::Break | Value::Continue => false,
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

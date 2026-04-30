use crate::diagnostics::Diagnostic;

pub fn format_source(source: &str) -> Result<String, Diagnostic> {
    let tokens = crate::lexer::lex(source)?;
    let program = crate::parser::parse(tokens)?;
    let mut output = String::new();

    for use_stmt in &program.uses {
        output.push_str(&format!("use {}\n", use_stmt.path));
    }

    if !program.uses.is_empty() && !program.functions.is_empty() {
        output.push('\n');
    }

    for function in program.functions {
        let params = function
            .params
            .iter()
            .map(|p| match &p.ty {
                Some(ty) => format!("{}: {}", p.name, ty),
                None => p.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");

        output.push_str(&format!("fn {}({})", function.name, params));
        if let Some(ret) = function.return_type {
            output.push_str(&format!(" -> {ret}"));
        }
        output.push_str(" {\n");

        for stmt in function.body {
            output.push_str("    ");
            output.push_str(&format_stmt(&stmt));
            output.push('\n');
        }
        output.push_str("}\n");
    }

    Ok(output)
}

fn format_stmt(stmt: &crate::ast::Stmt) -> String {
    match stmt {
        crate::ast::Stmt::Let(name, expr) => format!("let {name} = {}", format_expr(expr)),
        crate::ast::Stmt::Assign(name, expr) => format!("{name} = {}", format_expr(expr)),
        crate::ast::Stmt::Return(expr) => format!("return {}", format_expr(expr)),
        crate::ast::Stmt::Expr(expr) => format_expr(expr),
        crate::ast::Stmt::Use(use_stmt) => format!("use {}", use_stmt.path),
        crate::ast::Stmt::StructDef { name, fields } => {
            let fields_str = fields.iter().map(|(n, t)| format!("{n}: {t}")).collect::<Vec<_>>().join(", ");
            format!("struct {name} {{ {fields_str} }}")
        }
        crate::ast::Stmt::EnumDef { name, variants } => {
            let variants_str = variants.iter().map(|(n, ts)| {
                if ts.is_empty() {
                    n.clone()
                } else {
                    format!("{}({})", n, ts.join(", "))
                }
            }).collect::<Vec<_>>().join(", ");
            format!("enum {name} {{ {variants_str} }}")
        }
        crate::ast::Stmt::If { condition, then_block, else_block } => {
            let mut out = format!("if {} {{\n", format_expr(condition));
            for s in then_block {
                out.push_str("    ");
                out.push_str(&format_stmt(s));
                out.push('\n');
            }
            out.push_str("}");
            if let Some(else_b) = else_block {
                if else_b.len() == 1 && matches!(&else_b[0], crate::ast::Stmt::If { .. }) {
                    out.push_str(" else ");
                    out.push_str(&format_stmt(&else_b[0]));
                } else {
                    out.push_str(" else {\n");
                    for s in else_b {
                        out.push_str("    ");
                        out.push_str(&format_stmt(s));
                        out.push('\n');
                    }
                    out.push('}');
                }
            }
            out
        }
        crate::ast::Stmt::While { condition, body } => {
            let mut out = format!("while {} {{\n", format_expr(condition));
            for s in body {
                out.push_str("    ");
                out.push_str(&format_stmt(s));
                out.push('\n');
            }
            out.push('}');
            out
        }
        crate::ast::Stmt::For { initializer, condition, increment, body } => {
            let init_str = initializer.as_ref().map(|s| format_stmt(s)).unwrap_or_default();
            let cond_str = condition.as_ref().map(|e| format_expr(e)).unwrap_or_default();
            let incr_str = increment.as_ref().map(|e| format_expr(e)).unwrap_or_default();
            let mut out = format!("for ({}; {}; {}) {{\n", init_str, cond_str, incr_str);
            for s in body {
                out.push_str("    ");
                out.push_str(&format_stmt(s));
                out.push('\n');
            }
            out.push('}');
            out
        }
    }
}

fn format_expr(expr: &crate::ast::Expr) -> String {
    match expr {
        crate::ast::Expr::Number(v) => v.to_string(),
        crate::ast::Expr::String(v) => format!("\"{v}\""),
        crate::ast::Expr::Identifier(v) => v.clone(),
        crate::ast::Expr::Array(elements) => {
            let items = elements.iter().map(format_expr).collect::<Vec<_>>().join(", ");
            format!("[{items}]")
        }
        crate::ast::Expr::ArrayIndex(arr, index) => {
            format!("{}[{}]", format_expr(arr), format_expr(index))
        }
        crate::ast::Expr::StructInit { name, fields } => {
            let fields_str = fields.iter().map(|(n, e)| format!("{n}: {}", format_expr(e))).collect::<Vec<_>>().join(", ");
            format!("{name} {{ {fields_str} }}")
        }
        crate::ast::Expr::FieldAccess(expr, field) => {
            format!("{}.{}", format_expr(expr), field)
        }
        crate::ast::Expr::Binary(lhs, op, rhs) => format!(
            "{} {} {}",
            format_expr(lhs),
            match op {
                crate::ast::BinaryOp::Add => "+",
                crate::ast::BinaryOp::Sub => "-",
                crate::ast::BinaryOp::Mul => "*",
                crate::ast::BinaryOp::Div => "/",
                crate::ast::BinaryOp::Equal => "==",
                crate::ast::BinaryOp::NotEqual => "!=",
                crate::ast::BinaryOp::Less => "<",
                crate::ast::BinaryOp::LessEqual => "<=",
                crate::ast::BinaryOp::Greater => ">",
                crate::ast::BinaryOp::GreaterEqual => ">=",
            },
            format_expr(rhs)
        ),
        crate::ast::Expr::Call { callee, args } => {
            let args = args.iter().map(format_expr).collect::<Vec<_>>().join(", ");
            format!("{callee}({args})")
        }
        crate::ast::Expr::Match { expr, cases } => {
            let expr_str = format_expr(expr);
            let cases_str = cases.iter().map(|c| {
                let pattern_str = match &c.pattern {
                    crate::ast::MatchPattern::Wildcard => "_".to_string(),
                    crate::ast::MatchPattern::Number(n) => n.to_string(),
                    crate::ast::MatchPattern::String(s) => format!("\"{s}\""),
                    crate::ast::MatchPattern::Identifier(i) => i.clone(),
                    crate::ast::MatchPattern::EnumVariant { name, patterns, .. } => {
                        if patterns.is_empty() {
                            name.clone()
                        } else {
                            format!("{}({})", name, patterns.iter().map(|_| "_".to_string()).collect::<Vec<_>>().join(", "))
                        }
                    }
                };
                format!("{pattern_str} -> {}", format_expr(&c.body))
            }).collect::<Vec<_>>().join(", ");
            format!("match {expr_str} {{ {cases_str} }}")
        }
    }
}

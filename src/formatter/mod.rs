use crate::diagnostics::Diagnostic;

pub fn format_source(source: &str) -> Result<String, Diagnostic> {
    let tokens = crate::lexer::lex(source)?;
    let program = crate::parser::parse(tokens)?;
    let mut output = String::new();

    for function in program.functions {
        output.push_str(&format!("fn {}() {{\n", function.name));
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
        crate::ast::Stmt::Return(expr) => format!("return {}", format_expr(expr)),
        crate::ast::Stmt::Expr(expr) => format_expr(expr),
    }
}

fn format_expr(expr: &crate::ast::Expr) -> String {
    match expr {
        crate::ast::Expr::Number(v) => v.to_string(),
        crate::ast::Expr::String(v) => format!("\"{v}\""),
        crate::ast::Expr::Identifier(v) => v.clone(),
        crate::ast::Expr::Binary(lhs, op, rhs) => format!(
            "{} {} {}",
            format_expr(lhs),
            match op {
                crate::ast::BinaryOp::Add => "+",
                crate::ast::BinaryOp::Sub => "-",
                crate::ast::BinaryOp::Mul => "*",
                crate::ast::BinaryOp::Div => "/",
            },
            format_expr(rhs)
        ),
        crate::ast::Expr::Call { callee, args } => {
            let args = args.iter().map(format_expr).collect::<Vec<_>>().join(", ");
            format!("{callee}({args})")
        }
    }
}

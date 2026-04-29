use crate::ast::{Expr, Function, Program, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

pub fn build_program(program: &Program, target: String, no_std: bool) -> Result<(), Diagnostic> {
    if no_std && target.ends_with("-linux") {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            "--no-std is only valid for bare-metal targets",
            Span::new(0, 0),
        ));
    }

    let ir = lower_to_ir(program);
    println!("build ok, target={target}, no_std={no_std}");
    println!("; light-ir");
    println!("{ir}");
    Ok(())
}

pub fn supported_targets() -> &'static [&'static str] {
    &[
        "x86_64-linux",
        "aarch64-linux",
        "armv7-linux",
        "riscv64-linux",
        "thumbv7em-none-eabihf",
        "riscv32imac-none-elf",
        "esp32-none-elf",
    ]
}

fn lower_to_ir(program: &Program) -> String {
    let mut out = String::new();
    for function in &program.functions {
        out.push_str(&lower_function(function));
        out.push('\n');
    }
    out
}

fn lower_function(function: &Function) -> String {
    let mut out = format!("func {}({})", function.name, function.params.len());
    if let Some(ty) = &function.return_type {
        out.push_str(&format!(" -> {ty}"));
    }
    out.push_str(" {\n");

    for stmt in &function.body {
        out.push_str("  ");
        out.push_str(&lower_stmt(stmt));
        out.push('\n');
    }

    out.push('}');
    out
}

fn lower_stmt(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Let(name, expr) => format!("let {name} = {}", lower_expr(expr)),
        Stmt::Return(expr) => format!("ret {}", lower_expr(expr)),
        Stmt::Expr(expr) => lower_expr(expr),
    }
}

fn lower_expr(expr: &Expr) -> String {
    match expr {
        Expr::Number(v) => v.to_string(),
        Expr::String(v) => format!("\"{v}\""),
        Expr::Identifier(v) => v.clone(),
        Expr::Binary(lhs, op, rhs) => {
            let op = match op {
                crate::ast::BinaryOp::Add => "add",
                crate::ast::BinaryOp::Sub => "sub",
                crate::ast::BinaryOp::Mul => "mul",
                crate::ast::BinaryOp::Div => "div",
            };
            format!("{op}({}, {})", lower_expr(lhs), lower_expr(rhs))
        }
        Expr::Call { callee, args } => {
            let args = args.iter().map(lower_expr).collect::<Vec<_>>().join(", ");
            format!("call {callee}({args})")
        }
    }
}

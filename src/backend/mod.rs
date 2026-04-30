use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::{Expr, Function, Program, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

pub fn validate_no_std_source(source: &str) -> Result<(), Diagnostic> {
    let banned = ["std.fs", "std.net", "alloc(", "vec!", "Box<"];
    for pattern in banned {
        if source.contains(pattern) {
            return Err(Diagnostic::new(
                DiagnosticCode::RuntimeError,
                format!("`{pattern}` is not allowed in --no-std mode"),
                Span::new(0, 0),
            ));
        }
    }
    Ok(())
}

pub struct TargetInfo {
    pub triple: &'static str,
    pub cpu: &'static str,
    pub features: &'static str,
    pub linker_script: Option<&'static str>,
    pub is_bare_metal: bool,
}

pub fn get_target_info(target: &str) -> Option<TargetInfo> {
    match target {
        "x86_64-linux" => Some(TargetInfo {
            triple: "x86_64-unknown-linux-gnu",
            cpu: "x86_64",
            features: "",
            linker_script: None,
            is_bare_metal: false,
        }),
        "aarch64-linux" => Some(TargetInfo {
            triple: "aarch64-unknown-linux-gnu",
            cpu: "cortex-a72",
            features: "",
            linker_script: None,
            is_bare_metal: false,
        }),
        "armv7-linux" => Some(TargetInfo {
            triple: "armv7-unknown-linux-gnueabihf",
            cpu: "cortex-a9",
            features: "+neon",
            linker_script: None,
            is_bare_metal: false,
        }),
        "riscv64-linux" => Some(TargetInfo {
            triple: "riscv64gc-unknown-linux-gnu",
            cpu: "rocket",
            features: "",
            linker_script: None,
            is_bare_metal: false,
        }),
        "thumbv7em-none-eabihf" => Some(TargetInfo {
            triple: "thumbv7em-none-eabihf",
            cpu: "cortex-m4",
            features: "+fp",
            linker_script: Some("stm32f407.ld"),
            is_bare_metal: true,
        }),
        "riscv32imac-none-elf" => Some(TargetInfo {
            triple: "riscv32imac-unknown-elf",
            cpu: "generic-rv32imac",
            features: "+m,+a,+c",
            linker_script: Some("riscv32.ld"),
            is_bare_metal: true,
        }),
        "esp32-none-elf" => Some(TargetInfo {
            triple: "riscv32-unknown-elf",
            cpu: "esp32",
            features: "",
            linker_script: Some("esp32.ld"),
            is_bare_metal: true,
        }),
        "stm32f407" => Some(TargetInfo {
            triple: "thumbv7em-none-eabihf",
            cpu: "cortex-m4",
            features: "+fp",
            linker_script: Some("stm32f407.ld"),
            is_bare_metal: true,
        }),
        "stm32f103" => Some(TargetInfo {
            triple: "thumbv7em-none-eabihf",
            cpu: "cortex-m3",
            features: "",
            linker_script: Some("stm32f103.ld"),
            is_bare_metal: true,
        }),
        _ => None,
    }
}

pub fn build_program(
    program: &Program,
    target: String,
    no_std: bool,
    linker_script: Option<&Path>,
) -> Result<(), Diagnostic> {
    validate_target(&target, no_std)?;

    let target_info = get_target_info(&target).ok_or_else(|| {
        Diagnostic::new(
            DiagnosticCode::RuntimeError,
            format!("unsupported target `{target}`"),
            Span::new(0, 0),
        )
    })?;

    let ir = lower_to_ir(program);
    let out_dir = PathBuf::from("build").join(&target);
    fs::create_dir_all(&out_dir).map_err(|e| {
        Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
    })?;

    let stem = program_entry(program).unwrap_or("main");
    let elf_path = out_dir.join(format!("{stem}.elf"));
    let bin_path = out_dir.join(format!("{stem}.bin"));
    let hex_path = out_dir.join(format!("{stem}.hex"));
    let disasm_path = out_dir.join(format!("{stem}.dis"));

    let effective_linker_script = linker_script
        .map(|p| p.to_path_buf())
        .or_else(|| target_info.linker_script.map(|s| PathBuf::from("linker").join(s)));

    let mut ir_out = String::new();
    ir_out.push_str(&format!("; Target: {}\n", target_info.triple));
    ir_out.push_str(&format!("; CPU: {}\n", target_info.cpu));
    if !target_info.features.is_empty() {
        ir_out.push_str(&format!("; Features: {}\n", target_info.features));
    }
    if let Some(ref script) = effective_linker_script {
        ir_out.push_str(&format!("; Linker: {}\n", script.display()));
    }
    ir_out.push_str(&format!("; no_std: {}\n", no_std));
    ir_out.push_str(&format!("\n{ir}"));

    if target_info.is_bare_metal {
        let ll_file = out_dir.join(format!("{stem}.ll"));
        fs::write(&ll_file, &ir_out).map_err(|e| {
            Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
        })?;
    }

    let elf_content = format!("; target={target}\n; no_std={no_std}\n{ir}");
    fs::write(&elf_path, elf_content).map_err(|e| {
        Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
    })?;
    fs::write(&bin_path, b"LIGHT-BIN\n").map_err(|e| {
        Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
    })?;
    fs::write(&hex_path, b":4C494748540A\n").map_err(|e| {
        Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
    })?;
    fs::write(&disasm_path, &ir_out).map_err(|e| {
        Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
    })?;

    println!("build ok, target={target}, no_std={no_std}");
    if let Some(ref script) = effective_linker_script {
        println!("linker script: {}", script.display());
    }
    println!("{}", elf_path.display());
    println!("{}", bin_path.display());
    println!("{}", hex_path.display());
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
        "stm32f407",
        "stm32f103",
    ]
}

pub fn is_bare_metal_target(target: &str) -> bool {
    matches!(
        target,
        "thumbv7em-none-eabihf"
            | "riscv32imac-none-elf"
            | "esp32-none-elf"
            | "stm32f407"
            | "stm32f103"
    )
}

fn validate_target(target: &str, no_std: bool) -> Result<(), Diagnostic> {
    if !supported_targets().contains(&target) {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            format!("unsupported target `{target}`"),
            Span::new(0, 0),
        ));
    }

    if no_std && !is_bare_metal_target(target) {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            "--no-std is only valid for bare-metal targets",
            Span::new(0, 0),
        ));
    }

    Ok(())
}

fn program_entry(program: &Program) -> Option<&str> {
    program
        .functions
        .iter()
        .find(|f| f.name == "main")
        .map(|f| f.name.as_str())
        .or_else(|| program.functions.first().map(|f| f.name.as_str()))
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
        Stmt::Assign(name, expr) => format!("{name} = {}", lower_expr(expr)),
        Stmt::Return(expr) => format!("ret {}", lower_expr(expr)),
        Stmt::Expr(expr) => lower_expr(expr),
        Stmt::Use(use_stmt) => format!("use {}", use_stmt.path),
        Stmt::If { condition, then_block, else_block } => {
            let mut out = format!("if {} {{\n", lower_expr(condition));
            for s in then_block {
                out.push_str("  ");
                out.push_str(&lower_stmt(s));
                out.push('\n');
            }
            out.push_str("}");
            if let Some(else_b) = else_block {
                out.push_str(" else {\n");
                for s in else_b {
                    out.push_str("  ");
                    out.push_str(&lower_stmt(s));
                    out.push('\n');
                }
                out.push('}');
            }
            out
        }
        Stmt::While { condition, body } => {
            let mut out = format!("while {} {{\n", lower_expr(condition));
            for s in body {
                out.push_str("  ");
                out.push_str(&lower_stmt(s));
                out.push('\n');
            }
            out.push('}');
            out
        }
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
                crate::ast::BinaryOp::Equal => "eq",
                crate::ast::BinaryOp::NotEqual => "neq",
                crate::ast::BinaryOp::Less => "lt",
                crate::ast::BinaryOp::LessEqual => "lte",
                crate::ast::BinaryOp::Greater => "gt",
                crate::ast::BinaryOp::GreaterEqual => "gte",
            };
            format!("{op}({}, {})", lower_expr(lhs), lower_expr(rhs))
        }
        Expr::Call { callee, args } => {
            let args = args.iter().map(lower_expr).collect::<Vec<_>>().join(", ");
            format!("call {callee}({args})")
        }
    }
}

use std::path::Path;

use crate::ast::Program;
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

pub fn build_firmware(program: &Program, target: &str) -> Result<(), Diagnostic> {
    if !crate::backend::supported_targets().contains(&target) {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            format!("unsupported target `{target}`"),
            Span::new(0, 0),
        ));
    }

    crate::backend::build_program(program, target.to_string(), true)?;
    println!("build/firmware/main.elf");
    println!("build/firmware/main.bin");
    println!("build/firmware/main.hex");
    Ok(())
}

pub fn flash_image(
    image: &Path,
    target: &str,
    interface: Option<&str>,
    port: Option<&str>,
) -> Result<(), Diagnostic> {
    let transport = interface.or(port).unwrap_or("default");
    println!(
        "flashing {} to target={} via {}",
        image.display(),
        target,
        transport
    );
    if !image.exists() {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            format!("image not found: {}", image.display()),
            Span::new(0, 0),
        ));
    }
    Ok(())
}

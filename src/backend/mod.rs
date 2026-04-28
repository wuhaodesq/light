use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

pub fn build(target: String, no_std: bool) -> Result<(), Diagnostic> {
    if no_std && target.ends_with("-linux") {
        return Err(Diagnostic::new(
            DiagnosticCode::RuntimeError,
            "--no-std is only valid for bare-metal targets",
            Span::new(0, 0),
        ));
    }

    println!("build ok (mvp interpreter backend), target={target}, no_std={no_std}");
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

use std::fs;
use std::path::Path;

use crate::ast::Program;
use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

#[derive(Debug)]
pub struct LoadedProgram {
    pub source: String,
    pub program: Program,
}

pub fn load_program(path: &Path) -> Result<LoadedProgram, Diagnostic> {
    let source = fs::read_to_string(path).map_err(|e| {
        Diagnostic::new(DiagnosticCode::RuntimeError, e.to_string(), Span::new(0, 0))
    })?;
    let tokens = crate::lexer::lex(&source)?;
    let program = crate::parser::parse(tokens)?;
    Ok(LoadedProgram { source, program })
}

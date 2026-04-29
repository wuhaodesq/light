use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum DiagnosticCode {
    UnexpectedToken,
    ParseError,
    UndefinedVariable,
    RuntimeError,
}

#[derive(Debug, Error, Clone, Serialize)]
#[error("{message}")]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn new(code: DiagnosticCode, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
        }
    }

    pub fn as_json(&self) -> String {
        serde_json::json!({
            "error": format!("{:?}", self.code).to_uppercase(),
            "message": self.message,
            "span": { "start": self.span.start, "end": self.span.end }
        })
        .to_string()
    }
}

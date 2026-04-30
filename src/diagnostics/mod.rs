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

    #[allow(dead_code)]
    pub fn to_line_col(&self, source: &str) -> (usize, usize, usize, usize) {
        let mut line = 1;
        let mut start_line = 1;
        let mut start_col = 1;
        let mut end_line = 1;
        let mut end_col = 1;
        let mut col = 1;

        for (i, ch) in source.char_indices() {
            if i == self.start {
                start_line = line;
                start_col = col;
            }
            if i == self.end {
                end_line = line;
                end_col = col;
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        if self.end >= source.len() && self.start < source.len() {
            end_line = line;
            end_col = col;
        }

        (start_line, start_col, end_line, end_col)
    }

    #[allow(dead_code)]
    pub fn snippet(&self, source: &str) -> Option<String> {
        let (start_line, start_col, end_line, end_col) = self.to_line_col(source);

        let lines: Vec<&str> = source.lines().collect();
        if start_line == 0 || start_line > lines.len() {
            return None;
        }

        let mut result = String::new();

        if start_line == end_line {
            let line = lines.get(start_line - 1)?;
            result.push_str(line);
            result.push('\n');
            result.push_str(&" ".repeat(start_col.saturating_sub(1)));
            result.push_str(&"^".repeat(end_col.saturating_sub(start_col).max(1)));
        } else {
            if let Some(line) = lines.get(start_line - 1) {
                result.push_str(line);
                result.push('\n');
                result.push_str(&" ".repeat(start_col.saturating_sub(1)));
                result.push_str(&"^".repeat(line.len().saturating_sub(start_col - 1)));
            }
            for line_num in (start_line + 1)..end_line {
                if let Some(line) = lines.get(line_num - 1) {
                    result.push('\n');
                    result.push_str(line);
                }
            }
            if let Some(line) = lines.get(end_line - 1) {
                result.push('\n');
                result.push_str(&"^".repeat(end_col.min(line.len())));
            }
        }

        Some(result)
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
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.span.start == 0 && self.span.end == 0 {
            write!(f, "[{:?}] {}", self.code, self.message)
        } else {
            write!(f, "[{:?}] {}:{}: {}", self.code, self.span.start, self.span.end, self.message)
        }
    }
}

impl Diagnostic {
    pub fn new(code: DiagnosticCode, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
        }
    }

    #[allow(dead_code)]
    pub fn with_source(&self, source: &str) -> String {
        let (start_line, start_col, end_line, end_col) = self.span.to_line_col(source);
        let code_name = format!("{:?}", self.code).to_uppercase();

        if start_line == end_line && start_col == end_col {
            format!(
                "[{}] {}:{}: {}\n  {}",
                code_name, start_line, start_col, self.message,
                self.span.snippet(source).unwrap_or_default()
            )
        } else {
            format!(
                "[{}] {}:{} - {}:{}: {}\n  {}",
                code_name, start_line, start_col, end_line, end_col, self.message,
                self.span.snippet(source).unwrap_or_default()
            )
        }
    }

    #[allow(dead_code)]
    pub fn as_json(&self) -> String {
        serde_json::json!({
            "error": format!("{:?}", self.code).to_uppercase(),
            "message": self.message,
            "span": { "start": self.span.start, "end": self.span.end }
        })
        .to_string()
    }
}

use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Fn,
    Let,
    Return,
    If,
    Else,
    Identifier(String),
    Number(f64),
    String(String),
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Arrow,
    Newline,
    Eof,
}

#[derive(Debug, Clone)]
pub struct TokenWithSpan {
    pub token: Token,
    pub span: Span,
}

pub fn lex(source: &str) -> Result<Vec<TokenWithSpan>, Diagnostic> {
    let mut chars = source.char_indices().peekable();
    let mut tokens = Vec::new();

    while let Some((idx, ch)) = chars.next() {
        let token = match ch {
            ' ' | '\t' | '\r' => continue,
            '\n' => Token::Newline,
            '+' => Token::Plus,
            '-' => {
                if let Some((_, '>')) = chars.peek() {
                    chars.next();
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }
            '*' => Token::Star,
            '/' => Token::Slash,
            '=' => Token::Equal,
            '(' => Token::LParen,
            ')' => Token::RParen,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            ',' => Token::Comma,
            '"' => {
                let mut content = String::new();
                let mut end = idx;
                loop {
                    match chars.next() {
                        Some((string_idx, '"')) => {
                            end = string_idx;
                            break;
                        }
                        Some((_, c)) => content.push(c),
                        None => {
                            return Err(Diagnostic::new(
                                DiagnosticCode::UnexpectedToken,
                                "unterminated string literal",
                                Span::new(idx, idx + 1),
                            ));
                        }
                    }
                }
                tokens.push(TokenWithSpan {
                    token: Token::String(content),
                    span: Span::new(idx, end + 1),
                });
                continue;
            }
            c if c.is_ascii_digit() => {
                let mut num = String::from(c);
                let mut end = idx;
                while let Some((next_idx, next)) = chars.peek() {
                    if next.is_ascii_digit() || *next == '.' {
                        num.push(*next);
                        end = *next_idx;
                        chars.next();
                    } else {
                        break;
                    }
                }
                let value = num.parse::<f64>().map_err(|_| {
                    Diagnostic::new(
                        DiagnosticCode::UnexpectedToken,
                        "invalid number literal",
                        Span::new(idx, end + 1),
                    )
                })?;
                tokens.push(TokenWithSpan {
                    token: Token::Number(value),
                    span: Span::new(idx, end + 1),
                });
                continue;
            }
            c if is_ident_start(c) => {
                let mut ident = String::from(c);
                let mut end = idx;
                while let Some((next_idx, next)) = chars.peek() {
                    if is_ident_continue(*next) {
                        ident.push(*next);
                        end = *next_idx;
                        chars.next();
                    } else {
                        break;
                    }
                }
                let token = match ident.as_str() {
                    "fn" => Token::Fn,
                    "let" => Token::Let,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "else" => Token::Else,
                    _ => Token::Identifier(ident),
                };
                tokens.push(TokenWithSpan {
                    token,
                    span: Span::new(idx, end + 1),
                });
                continue;
            }
            _ => {
                return Err(Diagnostic::new(
                    DiagnosticCode::UnexpectedToken,
                    format!("unexpected character: {ch}"),
                    Span::new(idx, idx + 1),
                ));
            }
        };

        tokens.push(TokenWithSpan {
            token,
            span: Span::new(idx, idx + 1),
        });
    }

    tokens.push(TokenWithSpan {
        token: Token::Eof,
        span: Span::new(source.len(), source.len()),
    });

    Ok(tokens)
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

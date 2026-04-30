use crate::diagnostics::{Diagnostic, DiagnosticCode, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Fn,
    Let,
    Return,
    If,
    Else,
    While,
    For,
    Use,
    Struct,
    Enum,
    Match,
    Underscore,
    Identifier(String),
    Number(f64),
    String(String),
    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    ColonColon,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    Arrow,
    Dot,
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
            '/' => {
                if let Some((_, '/')) = chars.peek() {
                    while let Some((_, ch)) = chars.next() {
                        if ch == '\n' {
                            break;
                        }
                    }
                    continue;
                } else if let Some((_, '*')) = chars.peek() {
                    chars.next();
                    let start = idx;
                    let mut depth = 1;
                    while let Some((_, ch)) = chars.next() {
                        if ch == '*' {
                            if let Some((_, '/')) = chars.peek() {
                                chars.next();
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                        } else if ch == '/' {
                            if let Some((_, '*')) = chars.peek() {
                                chars.next();
                                depth += 1;
                            }
                        }
                    }
                    if depth != 0 {
                        return Err(Diagnostic::new(
                            DiagnosticCode::UnexpectedToken,
                            "unterminated multi-line comment",
                            Span::new(start, start + 2),
                        ));
                    }
                    continue;
                } else {
                    Token::Slash
                }
            }
            '=' => {
                if let Some((_, '=')) = chars.peek() {
                    chars.next();
                    Token::EqualEqual
                } else {
                    Token::Equal
                }
            }
            '!' => {
                if let Some((_, '=')) = chars.peek() {
                    chars.next();
                    Token::NotEqual
                } else {
                    return Err(Diagnostic::new(
                        DiagnosticCode::UnexpectedToken,
                        "unexpected character: ! (did you mean !=?)",
                        Span::new(idx, idx + 1),
                    ));
                }
            }
            '<' => {
                if let Some((_, '=')) = chars.peek() {
                    chars.next();
                    Token::LessEqual
                } else {
                    Token::Less
                }
            }
            '>' => {
                if let Some((_, '=')) = chars.peek() {
                    chars.next();
                    Token::GreaterEqual
                } else {
                    Token::Greater
                }
            }
            '(' => Token::LParen,
            ')' => Token::RParen,
            '[' => Token::LBracket,
            ']' => Token::RBracket,
            '{' => Token::LBrace,
            '}' => Token::RBrace,
            ',' => Token::Comma,
            ';' => Token::Semicolon,
            ':' => {
                if let Some((_, ':')) = chars.peek() {
                    chars.next();
                    Token::ColonColon
                } else {
                    Token::Colon
                }
            }
            '.' => Token::Dot,
            '"' => {
                let start = idx;
                let mut content = String::new();
                #[allow(unused_assignments)]
                let mut end = start;
                loop {
                    match chars.next() {
                        Some((next_idx, '"')) => {
                            end = next_idx + 1;
                            break;
                        }
                        Some((esc_idx, '\\')) => {
                            if let Some((_, esc)) = chars.next() {
                                match esc {
                                    'n' => content.push('\n'),
                                    't' => content.push('\t'),
                                    '"' => content.push('"'),
                                    '\\' => content.push('\\'),
                                    _ => {
                                        return Err(Diagnostic::new(
                                            DiagnosticCode::UnexpectedToken,
                                            format!("unknown escape sequence: \\{esc}"),
                                            Span::new(esc_idx, esc_idx + 2),
                                        ));
                                    }
                                }
                            }
                        }
                        Some((_, c)) => {
                            content.push(c);
                        }
                        None => {
                            return Err(Diagnostic::new(
                                DiagnosticCode::UnexpectedToken,
                                "unterminated string literal",
                                Span::new(start, start + 1),
                            ));
                        }
                    }
                }
                tokens.push(TokenWithSpan {
                    token: Token::String(content),
                    span: Span::new(start, end),
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
                    "while" => Token::While,
                    "for" => Token::For,
                    "use" => Token::Use,
                    "struct" => Token::Struct,
                    "enum" => Token::Enum,
                    "match" => Token::Match,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let tokens = lex("fn let return if else while use").unwrap();
        let expected = vec![
            Token::Fn,
            Token::Let,
            Token::Return,
            Token::If,
            Token::Else,
            Token::While,
            Token::Use,
            Token::Eof,
        ];
        assert_eq!(tokens.len(), expected.len());
        for (t, e) in tokens.iter().zip(expected.iter()) {
            assert_eq!(&t.token, e);
        }
    }

    #[test]
    fn test_identifiers() {
        let tokens = lex("foo bar123 _under").unwrap();
        assert!(matches!(tokens[0].token, Token::Identifier(_) if matches!(&tokens[0].token, Token::Identifier(s) if s == "foo")));
        assert!(matches!(tokens[1].token, Token::Identifier(_) if matches!(&tokens[1].token, Token::Identifier(s) if s == "bar123")));
        assert!(matches!(tokens[2].token, Token::Identifier(_) if matches!(&tokens[2].token, Token::Identifier(s) if s == "_under")));
    }

    #[test]
    fn test_numbers() {
        let tokens = lex("42 3.14 0.5").unwrap();
        assert!(matches!(tokens[0].token, Token::Number(n) if (n - 42.0).abs() < f64::EPSILON));
        assert!(matches!(tokens[1].token, Token::Number(n) if (n - 3.14).abs() < 0.001));
        assert!(matches!(tokens[2].token, Token::Number(n) if (n - 0.5).abs() < 0.001));
    }

    #[test]
    fn test_string_simple() {
        let tokens = lex("\"hello\"").unwrap();
        assert!(matches!(&tokens[0].token, Token::String(s) if s == "hello"));
        assert_eq!(tokens[0].span.start, 0);
        assert_eq!(tokens[0].span.end, 7);
    }

    #[test]
    fn test_string_with_escapes() {
        let tokens = lex("\"hello\\nworld\"").unwrap();
        assert!(matches!(&tokens[0].token, Token::String(s) if s == "hello\nworld"));
    }

    #[test]
    fn test_string_escaped_quote() {
        let tokens = lex("\"hello\\\"world\"").unwrap();
        assert!(matches!(&tokens[0].token, Token::String(s) if s == "hello\"world"));
    }

    #[test]
    fn test_comments() {
        let source = "// this is a comment\nlet x = 10";
        let tokens = lex(source).unwrap();
        assert!(tokens.len() > 4);
        let non_newline: Vec<&TokenWithSpan> = tokens.iter().filter(|t| !matches!(t.token, Token::Newline)).collect();
        assert_eq!(non_newline[0].token, Token::Let);
        assert_eq!(non_newline[1].token, Token::Identifier("x".to_string()));
        assert_eq!(non_newline[2].token, Token::Equal);
    }

    #[test]
    fn test_operators() {
        let tokens = lex("+ - * / == != < <= > >= = ->").unwrap();
        assert_eq!(tokens[0].token, Token::Plus);
        assert_eq!(tokens[1].token, Token::Minus);
        assert_eq!(tokens[2].token, Token::Star);
        assert_eq!(tokens[3].token, Token::Slash);
        assert_eq!(tokens[4].token, Token::EqualEqual);
        assert_eq!(tokens[5].token, Token::NotEqual);
        assert_eq!(tokens[6].token, Token::Less);
        assert_eq!(tokens[7].token, Token::LessEqual);
        assert_eq!(tokens[8].token, Token::Greater);
        assert_eq!(tokens[9].token, Token::GreaterEqual);
        assert_eq!(tokens[10].token, Token::Equal);
        assert_eq!(tokens[11].token, Token::Arrow);
    }

    #[test]
    fn test_punctuation() {
        let tokens = lex("( ) { } , : .").unwrap();
        assert_eq!(tokens[0].token, Token::LParen);
        assert_eq!(tokens[1].token, Token::RParen);
        assert_eq!(tokens[2].token, Token::LBrace);
        assert_eq!(tokens[3].token, Token::RBrace);
        assert_eq!(tokens[4].token, Token::Comma);
        assert_eq!(tokens[5].token, Token::Colon);
        assert_eq!(tokens[6].token, Token::Dot);
    }

    #[test]
    fn test_unterminated_string() {
        let result = lex("\"hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_escape() {
        let result = lex("\"\\q\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiline_comments() {
        let source = "/* comment */let x = 10";
        let tokens = lex(source).unwrap();
        assert!(tokens.len() >= 3);
        assert_eq!(tokens[0].token, Token::Let);
        assert_eq!(tokens[1].token, Token::Identifier("x".to_string()));
    }

    #[test]
    fn test_multiline_comments_nested() {
        let source = "/* outer /* nested */ still outer */let x = 10";
        let tokens = lex(source).unwrap();
        assert!(tokens.len() >= 3);
        assert_eq!(tokens[0].token, Token::Let);
    }

    #[test]
    fn test_unterminated_multiline_comment() {
        let result = lex("/* unclosed comment");
        assert!(result.is_err());
    }
}

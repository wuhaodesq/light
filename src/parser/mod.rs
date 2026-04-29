use crate::ast::{BinaryOp, Expr, Function, Param, Program, Stmt};
use crate::diagnostics::{Diagnostic, DiagnosticCode};
use crate::lexer::{Token, TokenWithSpan};

pub fn parse(tokens: Vec<TokenWithSpan>) -> Result<Program, Diagnostic> {
    let mut parser = Parser { tokens, pos: 0 };
    parser.parse_program()
}

struct Parser {
    tokens: Vec<TokenWithSpan>,
    pos: usize,
}

impl Parser {
    fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut functions = Vec::new();

        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() {
                break;
            }
            functions.push(self.parse_function()?);
            self.skip_newlines();
        }

        Ok(Program { functions })
    }

    fn parse_function(&mut self) -> Result<Function, Diagnostic> {
        self.expect(Token::Fn, "expected `fn`")?;
        let name = self.expect_identifier("expected function name")?;
        self.expect(Token::LParen, "expected `(`")?;
        let params = self.parse_params()?;
        self.expect(Token::RParen, "expected `)`")?;

        let return_type = if self.check(&Token::Arrow) {
            self.advance();
            Some(self.expect_identifier("expected return type after `->`")?)
        } else {
            None
        };

        self.expect(Token::LBrace, "expected `{`")?;
        self.skip_newlines();

        let mut body = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }

        self.expect(Token::RBrace, "expected `}`")?;
        Ok(Function {
            name,
            params,
            return_type,
            body,
        })
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, Diagnostic> {
        let mut params = Vec::new();
        if self.check(&Token::RParen) {
            return Ok(params);
        }

        loop {
            let name = self.expect_identifier("expected parameter name")?;
            let ty = if self.check(&Token::Colon) {
                self.advance();
                Some(self.expect_identifier("expected parameter type")?)
            } else {
                None
            };
            params.push(Param { name, ty });

            if self.check(&Token::Comma) {
                self.advance();
                continue;
            }
            break;
        }

        Ok(params)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(&Token::Let) {
            self.advance();
            let name = self.expect_identifier("expected variable name after let")?;
            self.expect(Token::Equal, "expected `=` after variable name")?;
            let expr = self.parse_expr()?;
            return Ok(Stmt::Let(name, expr));
        }

        if self.check(&Token::Return) {
            self.advance();
            let expr = self.parse_expr()?;
            return Ok(Stmt::Return(expr));
        }

        Ok(Stmt::Expr(self.parse_expr()?))
    }

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_term()
    }

    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_factor()?;

        while self.check(&Token::Plus) || self.check(&Token::Minus) {
            let op = if self.check(&Token::Plus) {
                self.advance();
                BinaryOp::Add
            } else {
                self.advance();
                BinaryOp::Sub
            };
            let rhs = self.parse_factor()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_primary()?;

        while self.check(&Token::Star) || self.check(&Token::Slash) {
            let op = if self.check(&Token::Star) {
                self.advance();
                BinaryOp::Mul
            } else {
                self.advance();
                BinaryOp::Div
            };
            let rhs = self.parse_primary()?;
            expr = Expr::Binary(Box::new(expr), op, Box::new(rhs));
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let current = self.peek();
        match &current.token {
            Token::Number(v) => {
                let v = *v;
                self.advance();
                Ok(Expr::Number(v))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::String(s))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                if self.check(&Token::LParen) {
                    self.advance();
                    let mut args = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if self.check(&Token::Comma) {
                                self.advance();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RParen, "expected `)`")?;
                    Ok(Expr::Call { callee: name, args })
                } else {
                    Ok(Expr::Identifier(name))
                }
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::RParen, "expected `)`")?;
                Ok(expr)
            }
            _ => Err(Diagnostic::new(
                DiagnosticCode::ParseError,
                "expected expression",
                current.span,
            )),
        }
    }

    fn expect(&mut self, expected: Token, message: &str) -> Result<(), Diagnostic> {
        if self.check(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(Diagnostic::new(
                DiagnosticCode::ParseError,
                message,
                self.peek().span,
            ))
        }
    }

    fn expect_identifier(&mut self, message: &str) -> Result<String, Diagnostic> {
        match &self.peek().token {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            _ => Err(Diagnostic::new(
                DiagnosticCode::ParseError,
                message,
                self.peek().span,
            )),
        }
    }

    fn check(&self, token: &Token) -> bool {
        std::mem::discriminant(&self.peek().token) == std::mem::discriminant(token)
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&Token::Newline) {
            self.advance();
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token, Token::Eof)
    }

    fn peek(&self) -> &TokenWithSpan {
        self.tokens
            .get(self.pos)
            .unwrap_or_else(|| self.tokens.last().expect("tokens are never empty"))
    }
}

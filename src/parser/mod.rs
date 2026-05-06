use crate::ast::{BinaryOp, Expr, Function, MatchCase, MatchPattern, Param, Program, Stmt, UseStmt};
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
        let mut uses = Vec::new();
        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();

        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() {
                break;
            }

            if self.check(&Token::Use) {
                uses.push(self.parse_use_stmt()?);
            } else if self.check(&Token::Struct) {
                structs.push(self.parse_struct_def()?);
            } else if self.check(&Token::Enum) {
                enums.push(self.parse_enum_def()?);
            } else {
                functions.push(self.parse_function()?);
            }
            self.skip_newlines();
        }

        Ok(Program { uses, functions, structs, enums })
    }

    fn parse_struct_def(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect(Token::Struct, "expected `struct`")?;
        let name = self.expect_identifier("expected struct name")?;
        self.expect(Token::LBrace, "expected `{`")?;
        self.skip_newlines();

        let mut fields = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let field_name = self.expect_identifier("expected field name")?;
            self.expect(Token::Colon, "expected `:`")?;
            let field_type = self.expect_identifier("expected field type")?;
            fields.push((field_name, field_type));
            self.skip_newlines();
            if self.check(&Token::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(Token::RBrace, "expected `}`")?;
        Ok(Stmt::StructDef { name, fields })
    }

    fn parse_enum_def(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect(Token::Enum, "expected `enum`")?;
        let name = self.expect_identifier("expected enum name")?;
        self.expect(Token::LBrace, "expected `{`")?;
        self.skip_newlines();

        let mut variants = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let variant_name = self.expect_identifier("expected variant name")?;
            let mut variant_types = Vec::new();
            if self.check(&Token::LParen) {
                self.advance();
                while !self.check(&Token::RParen) && !self.is_at_end() {
                    let ty = self.expect_identifier("expected type")?;
                    variant_types.push(ty);
                    if self.check(&Token::Comma) {
                        self.advance();
                    }
                }
                self.expect(Token::RParen, "expected `)`")?;
            }
            variants.push((variant_name, variant_types));
            self.skip_newlines();
            if self.check(&Token::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(Token::RBrace, "expected `}`")?;
        Ok(Stmt::EnumDef { name, variants })
    }

    fn parse_use_stmt(&mut self) -> Result<UseStmt, Diagnostic> {
        self.expect(Token::Use, "expected `use`")?;
        let path = self.parse_path()?;
        Ok(UseStmt { path })
    }

    fn parse_path(&mut self) -> Result<String, Diagnostic> {
        let first = self.expect_identifier("expected identifier in path")?;
        let mut path = first;

        while self.check(&Token::Dot) {
            self.advance();
            let next = self.expect_identifier("expected identifier after `.`")?;
            path.push('.');
            path.push_str(&next);
        }

        Ok(path)
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
        if self.check(&Token::Semicolon) {
            self.advance();
            return self.parse_stmt();
        }

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

        if self.check(&Token::Use) {
            return Ok(Stmt::Use(self.parse_use_stmt()?));
        }

        if self.check(&Token::If) {
            return self.parse_if_stmt();
        }

        if self.check(&Token::While) {
            return self.parse_while_stmt();
        }

        if self.check(&Token::For) {
            return self.parse_for_stmt();
        }

        if self.check(&Token::Loop) {
            return self.parse_loop_stmt();
        }

        if self.check(&Token::Break) {
            self.advance();
            return Ok(Stmt::Break);
        }

        if self.check(&Token::Continue) {
            self.advance();
            return Ok(Stmt::Continue);
        }

        if self.check(&Token::Match) {
            return self.parse_match_stmt();
        }

        // Check for assignment: identifier = expr (lookahead)
        let is_assignment = matches!(&self.peek().token, Token::Identifier(_))
            && {
                let saved_pos = self.pos;
                self.advance(); // skip identifier
                let is_eq = self.check(&Token::Equal);
                self.pos = saved_pos; // restore position
                is_eq
            };

        if is_assignment {
            let name = self.expect_identifier("expected variable name")?;
            self.advance(); // skip '='
            let expr = self.parse_expr()?;
            return Ok(Stmt::Assign(name, expr));
        }

        Ok(Stmt::Expr(self.parse_expr()?))
    }

    fn parse_if_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect(Token::If, "expected `if`")?;
        let condition = self.parse_expr()?;
        self.expect(Token::LBrace, "expected `{`")?;
        self.skip_newlines();

        let mut then_block = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            then_block.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.expect(Token::RBrace, "expected `}`")?;
        self.skip_newlines();

        let else_block = if self.check(&Token::Else) {
            self.advance();
            self.skip_newlines();
            if self.check(&Token::If) {
                Some(vec![self.parse_if_stmt()?])
            } else {
                self.expect(Token::LBrace, "expected `{`")?;
                self.skip_newlines();
                let mut else_body = Vec::new();
                while !self.check(&Token::RBrace) && !self.is_at_end() {
                    else_body.push(self.parse_stmt()?);
                    self.skip_newlines();
                }
                self.expect(Token::RBrace, "expected `}`")?;
                Some(else_body)
            }
        } else {
            None
        };

        Ok(Stmt::If { condition, then_block, else_block })
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect(Token::While, "expected `while`")?;
        let condition = self.parse_expr()?;
        self.expect(Token::LBrace, "expected `{`")?;
        self.skip_newlines();

        let mut body = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.expect(Token::RBrace, "expected `}`")?;

        Ok(Stmt::While { condition, body })
    }

    fn parse_for_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect(Token::For, "expected `for`")?;
        self.expect(Token::LParen, "expected `(`")?;
        self.skip_newlines();

        let initializer = if !self.check(&Token::Semicolon) {
            Some(Box::new(self.parse_for_init()?))
        } else {
            None
        };

        self.expect(Token::Semicolon, "expected `;`")?;
        self.skip_newlines();

        let condition = if !self.check(&Token::Semicolon) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect(Token::Semicolon, "expected `;`")?;
        self.skip_newlines();

        let increment = if !self.check(&Token::RParen) {
            Some(Box::new(self.parse_expr()?))
        } else {
            None
        };

        self.expect(Token::RParen, "expected `)`")?;
        self.skip_newlines();

        self.expect(Token::LBrace, "expected `{`")?;
        self.skip_newlines();

        let mut body = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.expect(Token::RBrace, "expected `}`")?;

        Ok(Stmt::For { initializer, condition, increment, body })
    }

    fn parse_loop_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect(Token::Loop, "expected `loop`")?;
        self.expect(Token::LBrace, "expected `{`")?;
        self.skip_newlines();

        let mut body = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            body.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.expect(Token::RBrace, "expected `}`")?;

        Ok(Stmt::Loop { body })
    }

    fn parse_for_init(&mut self) -> Result<Stmt, Diagnostic> {
        if self.check(&Token::Let) {
            self.advance();
            let name = self.expect_identifier("expected variable name after let")?;
            self.expect(Token::Equal, "expected `=` after variable name")?;
            let expr = self.parse_expr()?;
            Ok(Stmt::Let(name, expr))
        } else if matches!(&self.peek().token, Token::Identifier(_))
            && {
                let saved_pos = self.pos;
                self.advance();
                let is_eq = self.check(&Token::Equal);
                self.pos = saved_pos;
                is_eq
            }
        {
            let name = self.expect_identifier("expected variable name")?;
            self.advance();
            let expr = self.parse_expr()?;
            Ok(Stmt::Assign(name, expr))
        } else {
            Ok(Stmt::Expr(self.parse_expr()?))
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, Diagnostic> {
        let mut result: Result<Expr, Diagnostic> = self.parse_term();

        while self.check(&Token::EqualEqual)
            || self.check(&Token::NotEqual)
            || self.check(&Token::Less)
            || self.check(&Token::LessEqual)
            || self.check(&Token::Greater)
            || self.check(&Token::GreaterEqual)
        {
            let op = if self.check(&Token::EqualEqual) {
                self.advance();
                BinaryOp::Equal
            } else if self.check(&Token::NotEqual) {
                self.advance();
                BinaryOp::NotEqual
            } else if self.check(&Token::Less) {
                self.advance();
                BinaryOp::Less
            } else if self.check(&Token::LessEqual) {
                self.advance();
                BinaryOp::LessEqual
            } else if self.check(&Token::Greater) {
                self.advance();
                BinaryOp::Greater
            } else {
                self.advance();
                BinaryOp::GreaterEqual
            };
            let rhs = self.parse_term()?;
            result = result.map(|expr| Expr::Binary(Box::new(expr), op, Box::new(rhs)));
        }

        result
    }

    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        let mut result: Result<Expr, Diagnostic> = self.parse_factor();

        while self.check(&Token::Plus) || self.check(&Token::Minus) {
            let op = if self.check(&Token::Plus) {
                self.advance();
                BinaryOp::Add
            } else {
                self.advance();
                BinaryOp::Sub
            };
            let rhs = self.parse_factor()?;
            result = result.map(|expr| Expr::Binary(Box::new(expr), op, Box::new(rhs)));
        }

        result
    }

    fn parse_factor(&mut self) -> Result<Expr, Diagnostic> {
        let mut result: Result<Expr, Diagnostic> = self.parse_primary();

        while self.check(&Token::Star) || self.check(&Token::Slash) {
            let op = if self.check(&Token::Star) {
                self.advance();
                BinaryOp::Mul
            } else {
                self.advance();
                BinaryOp::Div
            };
            let rhs = self.parse_primary()?;
            result = result.map(|expr| Expr::Binary(Box::new(expr), op, Box::new(rhs)));
        }

        result
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        let current = self.peek();
        match &current.token {
            Token::Minus => {
                self.advance();
                let next = self.peek();
                match &next.token {
                    Token::Number(n) => {
                        let v = -*n;
                        self.advance();
                        Ok(Expr::Number(v))
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            DiagnosticCode::ParseError,
                            "expected expression",
                            next.span,
                        ));
                    }
                }
            }
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

                let mut base_expr = Expr::Identifier(name.clone());

                while self.check(&Token::Dot) {
                    self.advance();
                    let field = self.expect_identifier("expected identifier after `.`")?;
                    base_expr = Expr::FieldAccess(Box::new(base_expr), field);
                }

                if self.check(&Token::ColonColon) && !self.check(&Token::LBrace) {
                    self.advance();
                    let next = self.expect_identifier("expected identifier after `::`")?;
                    if self.check(&Token::LBrace) {
                        self.advance();
                        self.skip_newlines();
                        let mut fields = Vec::new();
                        if !self.check(&Token::RBrace) {
                            loop {
                                let field_name = self.expect_identifier("expected field name")?;
                                self.expect(Token::Colon, "expected `:`")?;
                                let field_value = self.parse_expr()?;
                                fields.push((field_name, field_value));
                                self.skip_newlines();
                                if self.check(&Token::Comma) {
                                    self.advance();
                                    self.skip_newlines();
                                } else {
                                    break;
                                }
                            }
                        }
                        self.expect(Token::RBrace, "expected `}`")?;
                        Ok(Expr::StructInit { name: format!("{}::{}", name, next), fields })
                    } else {
                        base_expr = Expr::Identifier(format!("{}::{}", name, next));
                        while self.check(&Token::Dot) {
                            self.advance();
                            let field = self.expect_identifier("expected identifier after `.`")?;
                            base_expr = Expr::FieldAccess(Box::new(base_expr), field);
                        }
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
                            match base_expr {
                                Expr::Identifier(id) => Ok(Expr::Call { callee: id, args }),
                                _ => Ok(base_expr),
                            }
                        } else {
                            Ok(base_expr)
                        }
                    }
                } else if self.check(&Token::LParen) {
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
                    match base_expr {
                        Expr::Identifier(id) => Ok(Expr::Call { callee: id, args }),
                        Expr::FieldAccess(expr, field) => Ok(Expr::Call {
                            callee: format!("{:?}.{:?}", expr, field),
                            args,
                        }),
                        _ => Ok(Expr::Identifier(name)),
                    }
                } else if self.check(&Token::LBracket) {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(Token::RBracket, "expected `]`")?;
                    Ok(Expr::ArrayIndex(Box::new(base_expr), Box::new(index)))
                } else if self.check(&Token::LBrace) && name.starts_with(char::is_uppercase) {
                    self.advance();
                    self.skip_newlines();
                    let mut fields = Vec::new();
                    if !self.check(&Token::RBrace) {
                        loop {
                            let field_name = self.expect_identifier("expected field name")?;
                            self.expect(Token::Colon, "expected `:`")?;
                            let field_value = self.parse_expr()?;
                            fields.push((field_name, field_value));
                            self.skip_newlines();
                            if self.check(&Token::Comma) {
                                self.advance();
                                self.skip_newlines();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Token::RBrace, "expected `}`")?;
                    Ok(Expr::StructInit { name, fields })
                } else {
                    Ok(base_expr)
                }
            }
            Token::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&Token::RBracket) {
                    loop {
                        elements.push(self.parse_expr()?);
                        if self.check(&Token::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(Token::RBracket, "expected `]`")?;
                Ok(Expr::Array(elements))
            }
            Token::Match => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(Token::LBrace, "expected `{`")?;
                self.skip_newlines();

                let mut cases = Vec::new();
                while !self.check(&Token::RBrace) && !self.is_at_end() {
                    let pattern = self.parse_match_pattern()?;
                    self.skip_newlines();
                    self.expect(Token::Arrow, "expected `->`")?;
                    self.skip_newlines();
                    let body = self.parse_expr()?;
                    cases.push(MatchCase { pattern, body });
                    self.skip_newlines();
                    if self.check(&Token::Comma) {
                        self.advance();
                        self.skip_newlines();
                    }
                }

                self.expect(Token::RBrace, "expected `}`")?;
                Ok(Expr::Match { expr: Box::new(expr), cases })
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

    fn parse_match_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        self.expect(Token::Match, "expected `match`")?;
        let expr = self.parse_expr()?;
        self.expect(Token::LBrace, "expected `{`")?;
        self.skip_newlines();

        let mut cases = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let pattern = self.parse_match_pattern()?;
            self.skip_newlines();
            self.expect(Token::Arrow, "expected `->`")?;
            self.skip_newlines();
            let body = self.parse_expr()?;
            cases.push(MatchCase { pattern, body });
            self.skip_newlines();
            if self.check(&Token::Comma) {
                self.advance();
                self.skip_newlines();
            }
        }

        self.expect(Token::RBrace, "expected `}`")?;
        Ok(Stmt::Expr(Expr::Match { expr: Box::new(expr), cases }))
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, Diagnostic> {
        let current = self.peek();
        match &current.token {
            Token::Number(n) => {
                let n = *n;
                self.advance();
                Ok(MatchPattern::Number(n))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(MatchPattern::String(s))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                if self.check(&Token::ColonColon) {
                    self.advance();
                    let variant = self.expect_identifier("expected variant name")?;
                    if self.check(&Token::LBrace) {
                        self.advance();
                        self.skip_newlines();
                        let mut patterns = Vec::new();
                        while !self.check(&Token::RBrace) && !self.is_at_end() {
                            patterns.push(self.parse_match_pattern()?);
                            self.skip_newlines();
                            if self.check(&Token::Comma) {
                                self.advance();
                                self.skip_newlines();
                            }
                        }
                        self.expect(Token::RBrace, "expected `}`")?;
                        Ok(MatchPattern::EnumVariant { name: name.clone(), variant, patterns })
                    } else {
                        Ok(MatchPattern::EnumVariant { name: name.clone(), variant, patterns: vec![] })
                    }
                } else if self.check(&Token::LBrace) {
                    self.advance();
                    self.skip_newlines();
                    let mut patterns = Vec::new();
                    while !self.check(&Token::RBrace) && !self.is_at_end() {
                        patterns.push(self.parse_match_pattern()?);
                        self.skip_newlines();
                        if self.check(&Token::Comma) {
                            self.advance();
                            self.skip_newlines();
                        }
                    }
                    self.expect(Token::RBrace, "expected `}`")?;
                    Ok(MatchPattern::EnumVariant { name: name.clone(), variant: name, patterns })
                } else if name == "_" {
                    Ok(MatchPattern::Wildcard)
                } else {
                    Ok(MatchPattern::Identifier(name))
                }
            }
            Token::Underscore => {
                self.advance();
                Ok(MatchPattern::Wildcard)
            }
            _ => Err(Diagnostic::new(
                DiagnosticCode::ParseError,
                "expected pattern",
                current.span,
            )),
        }
    }
}

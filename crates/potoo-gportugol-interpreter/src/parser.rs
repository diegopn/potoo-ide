use crate::{
    ExecutionOptions,
    ast::{
        BinaryOp, Block, Expr, ExprKind, FunctionDecl, Identifier, LValue, Literal, Program,
        Statement, UnaryOp, VariableDecl,
    },
    lexer::{Token, TokenKind, lex},
    source::{DataType, Diagnostic, DiagnosticKind, SourceSpan},
};

type ParseResult<T> = Result<T, Diagnostic>;

pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    parse_with_options(source, ExecutionOptions::default())
}

pub fn parse_with_options(
    source: &str,
    options: ExecutionOptions,
) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lex(source)?;
    parse_tokens_with_options(tokens, options)
}

pub(crate) fn parse_tokens_with_options(
    tokens: Vec<Token>,
    options: ExecutionOptions,
) -> Result<Program, Vec<Diagnostic>> {
    Parser::new(tokens, options)
        .program()
        .map_err(|diagnostic| vec![diagnostic])
}

struct Parser {
    tokens: Vec<Token>,
    current: usize,
    options: ExecutionOptions,
    nesting: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>, options: ExecutionOptions) -> Self {
        Self {
            tokens,
            current: 0,
            options,
            nesting: 0,
        }
    }

    fn program(mut self) -> ParseResult<Program> {
        let start = self.current().span.start;
        self.expect(TokenKind::Algorithm, "expected 'algoritmo'")?;
        let name = self.expect(TokenKind::Identifier, "expected algorithm name")?;
        self.expect(TokenKind::Semicolon, "expected ';' after algorithm name")?;
        let globals = if self.matches(TokenKind::Variables) {
            self.variable_block()?
        } else {
            Vec::new()
        };
        let main = self.block()?;
        let mut functions = Vec::new();
        while self.matches(TokenKind::Function) {
            functions.push(self.function()?);
        }
        let end = self
            .expect(TokenKind::EndOfFile, "expected end of file")?
            .span
            .end;
        Ok(Program {
            name: name.lexeme,
            name_span: name.span,
            globals,
            main,
            functions,
            span: SourceSpan { start, end },
        })
    }

    fn variable_block(&mut self) -> ParseResult<Vec<VariableDecl>> {
        let mut declarations = Vec::new();
        while !self.check(TokenKind::EndVariables) && !self.check(TokenKind::EndOfFile) {
            declarations.push(self.variable_declaration(true)?);
        }
        if declarations.is_empty() {
            return Err(self.error("expected a variable after 'variáveis'"));
        }
        self.expect(TokenKind::EndVariables, "expected 'fim-variáveis'")?;
        Ok(declarations)
    }

    fn variable_declaration(&mut self, semicolon: bool) -> ParseResult<VariableDecl> {
        let start = self.current().span.start;
        let mut names = vec![self.identifier()?];
        while self.matches(TokenKind::Comma) {
            names.push(self.identifier()?);
        }
        self.expect(TokenKind::Colon, "expected ':' after variable name")?;
        let (data_type, dimensions) = self.type_specifier()?;
        let end = if semicolon {
            self.expect(TokenKind::Semicolon, "expected ';' after declaration")?
                .span
                .end
        } else {
            self.previous().span.end
        };
        Ok(VariableDecl {
            names,
            data_type,
            dimensions,
            span: SourceSpan { start, end },
        })
    }

    fn type_specifier(&mut self) -> ParseResult<(DataType, Vec<usize>)> {
        if self.matches(TokenKind::Matrix) {
            let mut dimensions = Vec::new();
            while self.matches(TokenKind::LeftBracket) {
                let size = self.expect(TokenKind::IntegerLiteral, "expected matrix dimension")?;
                dimensions.push(parse_integer(&size)? as usize);
                self.expect(
                    TokenKind::RightBracket,
                    "expected ']' after matrix dimension",
                )?;
            }
            if dimensions.is_empty() {
                return Err(self.error("expected at least one matrix dimension"));
            }
            self.expect(TokenKind::From, "expected 'de' before matrix element type")?;
            let data_type = self.plural_type()?;
            Ok((data_type, dimensions))
        } else {
            Ok((self.primitive_type()?, Vec::new()))
        }
    }

    fn primitive_type(&mut self) -> ParseResult<DataType> {
        let token = self.advance();
        match token.kind {
            TokenKind::Integer => Ok(DataType::Integer),
            TokenKind::Real => Ok(DataType::Real),
            TokenKind::Character => Ok(DataType::Character),
            TokenKind::Literal => Ok(DataType::Literal),
            TokenKind::Logical => Ok(DataType::Logical),
            _ => Err(Diagnostic::new(
                DiagnosticKind::Syntax,
                "expected a primitive type",
                token.span,
            )),
        }
    }

    fn plural_type(&mut self) -> ParseResult<DataType> {
        let token = self.advance();
        match token.kind {
            TokenKind::Integers => Ok(DataType::Integer),
            TokenKind::Reals => Ok(DataType::Real),
            TokenKind::Characters => Ok(DataType::Character),
            TokenKind::Literals => Ok(DataType::Literal),
            TokenKind::Logicals => Ok(DataType::Logical),
            _ => Err(Diagnostic::new(
                DiagnosticKind::Syntax,
                "expected a plural primitive type for matrix elements",
                token.span,
            )),
        }
    }

    fn block(&mut self) -> ParseResult<Block> {
        let start = self
            .expect(TokenKind::Start, "expected 'início'")?
            .span
            .start;
        let mut statements = Vec::new();
        while !self.check(TokenKind::End) && !self.check(TokenKind::EndOfFile) {
            statements.push(self.statement()?);
        }
        let end = self
            .expect(TokenKind::End, "expected 'fim' to close block")?
            .span
            .end;
        Ok(Block {
            statements,
            span: SourceSpan { start, end },
        })
    }

    fn statement(&mut self) -> ParseResult<Statement> {
        match self.current().kind {
            TokenKind::If => self.if_statement(),
            TokenKind::While => self.while_statement(),
            TokenKind::Repeat => self.repeat_statement(),
            TokenKind::For => self.for_statement(),
            TokenKind::Return => self.return_statement(),
            TokenKind::Identifier if self.peek(1).kind == TokenKind::LeftParen => {
                let call = self.call_expression()?;
                let end = self
                    .expect(TokenKind::Semicolon, "expected ';' after function call")?
                    .span
                    .end;
                let span = SourceSpan {
                    start: call.span.start,
                    end,
                };
                Ok(Statement::Call { call, span })
            }
            TokenKind::Identifier => self.assignment_statement(),
            _ => Err(self.error("expected a statement")),
        }
    }

    fn assignment_statement(&mut self) -> ParseResult<Statement> {
        let target = self.lvalue()?;
        self.expect(TokenKind::Assign, "expected ':=' in assignment")?;
        let value = self.expression(0)?;
        let end = self
            .expect(TokenKind::Semicolon, "expected ';' after assignment")?
            .span
            .end;
        Ok(Statement::Assign {
            span: SourceSpan {
                start: target.span.start,
                end,
            },
            target,
            value,
        })
    }

    fn return_statement(&mut self) -> ParseResult<Statement> {
        let start = self.advance().span.start;
        let value = if self.check(TokenKind::Semicolon) {
            None
        } else {
            Some(self.expression(0)?)
        };
        let end = self
            .expect(TokenKind::Semicolon, "expected ';' after return")?
            .span
            .end;
        Ok(Statement::Return {
            value,
            span: SourceSpan { start, end },
        })
    }

    fn if_statement(&mut self) -> ParseResult<Statement> {
        let start = self.advance().span.start;
        let condition = self.expression(0)?;
        self.expect(TokenKind::Then, "expected 'então' after condition")?;
        let then_branch = Box::new(self.statement_sequence(&[TokenKind::Else, TokenKind::EndIf])?);
        let else_branch = if self.matches(TokenKind::Else) {
            Some(Box::new(self.statement_sequence(&[TokenKind::EndIf])?))
        } else {
            None
        };
        let end = self.expect(TokenKind::EndIf, "expected 'fim-se'")?.span.end;
        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
            span: SourceSpan { start, end },
        })
    }

    fn while_statement(&mut self) -> ParseResult<Statement> {
        let start = self.advance().span.start;
        let condition = self.expression(0)?;
        self.expect(TokenKind::Do, "expected 'faça' after condition")?;
        let body = Box::new(self.statement_sequence(&[TokenKind::EndWhile])?);
        let end = self
            .expect(TokenKind::EndWhile, "expected 'fim-enquanto'")?
            .span
            .end;
        Ok(Statement::While {
            condition,
            body,
            span: SourceSpan { start, end },
        })
    }

    fn repeat_statement(&mut self) -> ParseResult<Statement> {
        let start = self.advance().span.start;
        let body = Box::new(self.statement_sequence(&[TokenKind::To])?);
        self.expect(TokenKind::To, "expected 'até' after repeat body")?;
        let condition = self.expression(0)?;
        let end = self
            .expect(TokenKind::Semicolon, "expected ';' after repeat condition")?
            .span
            .end;
        Ok(Statement::Repeat {
            body,
            condition,
            span: SourceSpan { start, end },
        })
    }

    fn for_statement(&mut self) -> ParseResult<Statement> {
        let start = self.advance().span.start;
        let target = self.lvalue()?;
        self.expect(TokenKind::From, "expected 'de' in for loop")?;
        let from = self.expression(0)?;
        self.expect(TokenKind::To, "expected 'até' in for loop")?;
        let to = self.expression(0)?;
        let step = if self.matches(TokenKind::Step) {
            let sign = if self.matches(TokenKind::Minus) {
                -1
            } else {
                self.matches(TokenKind::Plus);
                1
            };
            let value = self.expect(TokenKind::IntegerLiteral, "expected integer step")?;
            sign * parse_integer(&value)?
        } else {
            1
        };
        self.expect(TokenKind::Do, "expected 'faça' in for loop")?;
        let body = Box::new(self.statement_sequence(&[TokenKind::EndFor])?);
        let end = self
            .expect(TokenKind::EndFor, "expected 'fim-para'")?
            .span
            .end;
        Ok(Statement::For {
            target,
            start: from,
            end: to,
            step,
            body,
            span: SourceSpan { start, end },
        })
    }

    fn statement_sequence(&mut self, terminators: &[TokenKind]) -> ParseResult<Statement> {
        let start = self.current().span.start;
        let mut statements = Vec::new();
        while !terminators.contains(&self.current().kind) && !self.check(TokenKind::EndOfFile) {
            statements.push(self.with_nesting(Self::statement)?);
        }
        let end = statements
            .last()
            .map_or(start, |statement| statement.span().end);
        Ok(Statement::Sequence {
            statements,
            span: SourceSpan { start, end },
        })
    }

    fn with_nesting<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> ParseResult<T>,
    ) -> ParseResult<T> {
        self.nesting += 1;
        if self.nesting > self.options.max_parse_depth {
            self.nesting -= 1;
            return Err(self.error("parser nesting limit exceeded"));
        }
        let result = parse(self);
        self.nesting -= 1;
        result
    }

    fn function(&mut self) -> ParseResult<FunctionDecl> {
        let start = self.previous().span.start;
        let name = self.identifier()?;
        self.expect(TokenKind::LeftParen, "expected '(' after function name")?;
        let mut parameters = Vec::new();
        if !self.check(TokenKind::RightParen) {
            loop {
                let parameter = self.identifier()?;
                let start = parameter.span.start;
                self.expect(TokenKind::Colon, "expected ':' after parameter name")?;
                let (data_type, dimensions) = self.type_specifier()?;
                parameters.push(VariableDecl {
                    names: vec![parameter],
                    data_type,
                    dimensions,
                    span: SourceSpan {
                        start,
                        end: self.previous().span.end,
                    },
                });
                if !self.matches(TokenKind::Comma) {
                    break;
                }
            }
        }
        self.expect(TokenKind::RightParen, "expected ')' after parameters")?;
        let return_type = if self.matches(TokenKind::Colon) {
            Some(self.primitive_type()?)
        } else {
            None
        };
        let mut locals = Vec::new();
        while self.check(TokenKind::Identifier) {
            locals.push(self.variable_declaration(true)?);
        }
        let body = self.block()?;
        Ok(FunctionDecl {
            name: name.name,
            name_span: name.span,
            parameters,
            locals,
            return_type,
            span: SourceSpan {
                start,
                end: body.span.end,
            },
            body,
        })
    }

    fn lvalue(&mut self) -> ParseResult<LValue> {
        let name = self.identifier()?;
        let mut indexes = Vec::new();
        while self.matches(TokenKind::LeftBracket) {
            indexes.push(self.with_nesting(|parser| parser.expression(0))?);
            self.expect(TokenKind::RightBracket, "expected ']' after index")?;
        }
        let end = self.previous().span.end;
        Ok(LValue {
            name: name.name,
            name_span: name.span,
            indexes,
            span: SourceSpan {
                start: name.span.start,
                end,
            },
        })
    }

    fn call_expression(&mut self) -> ParseResult<Expr> {
        self.with_nesting(Self::call_expression_inner)
    }

    fn call_expression_inner(&mut self) -> ParseResult<Expr> {
        let name = self.identifier()?;
        self.expect(TokenKind::LeftParen, "expected '(' after function name")?;
        let mut args = Vec::new();
        if !self.check(TokenKind::RightParen) {
            loop {
                args.push(self.expression(0)?);
                if !self.matches(TokenKind::Comma) {
                    break;
                }
            }
        }
        let end = self
            .expect(TokenKind::RightParen, "expected ')' after arguments")?
            .span
            .end;
        Ok(Expr {
            kind: ExprKind::Call {
                name: name.name,
                name_span: name.span,
                args,
            },
            span: SourceSpan {
                start: name.span.start,
                end,
            },
        })
    }

    fn expression(&mut self, min_precedence: u8) -> ParseResult<Expr> {
        let mut left = self.unary()?;
        while let Some((precedence, operator)) = binary_operator(self.current().kind) {
            if precedence < min_precedence {
                break;
            }
            self.advance();
            let right = self.expression(precedence + 1)?;
            left = Expr {
                span: SourceSpan {
                    start: left.span.start,
                    end: right.span.end,
                },
                kind: ExprKind::Binary {
                    left: Box::new(left),
                    op: operator,
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> ParseResult<Expr> {
        let operator = match self.current().kind {
            TokenKind::Plus => Some(UnaryOp::Positive),
            TokenKind::Minus => Some(UnaryOp::Negative),
            TokenKind::Not => Some(UnaryOp::Not),
            TokenKind::Tilde => Some(UnaryOp::BitwiseNot),
            _ => None,
        };
        if let Some(op) = operator {
            let start = self.advance().span.start;
            let value = self.primary()?;
            return Ok(Expr {
                span: SourceSpan {
                    start,
                    end: value.span.end,
                },
                kind: ExprKind::Unary {
                    op,
                    value: Box::new(value),
                },
            });
        }
        self.primary()
    }

    fn primary(&mut self) -> ParseResult<Expr> {
        if self.matches(TokenKind::LeftParen) {
            let start = self.previous().span.start;
            let mut expression = self.with_nesting(|parser| parser.expression(0))?;
            let end = self
                .expect(TokenKind::RightParen, "expected ')' after expression")?
                .span
                .end;
            expression.span = SourceSpan { start, end };
            return Ok(expression);
        }
        if self.check(TokenKind::Identifier) {
            if self.peek(1).kind == TokenKind::LeftParen {
                return self.call_expression();
            }
            let value = self.lvalue()?;
            let span = value.span;
            return Ok(Expr {
                kind: ExprKind::Variable(value),
                span,
            });
        }
        let token = self.advance();
        let literal = match token.kind {
            TokenKind::IntegerLiteral => Literal::Integer(parse_integer(&token)?),
            TokenKind::RealLiteral => Literal::Real(token.lexeme.parse().map_err(|_| {
                Diagnostic::new(DiagnosticKind::Syntax, "invalid real literal", token.span)
            })?),
            TokenKind::CharacterLiteral => Literal::Character(parse_character(&token)?),
            TokenKind::StringLiteral => Literal::String(decode_quoted(&token.lexeme, '"')),
            TokenKind::True => Literal::Logical(true),
            TokenKind::False => Literal::Logical(false),
            _ => {
                return Err(Diagnostic::new(
                    DiagnosticKind::Syntax,
                    "expected an expression",
                    token.span,
                ));
            }
        };
        Ok(Expr {
            kind: ExprKind::Literal(literal),
            span: token.span,
        })
    }

    fn identifier(&mut self) -> ParseResult<Identifier> {
        let token = self.expect(TokenKind::Identifier, "expected identifier")?;
        Ok(Identifier {
            name: token.lexeme,
            span: token.span,
        })
    }

    fn current(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn peek(&self, distance: usize) -> &Token {
        self.tokens
            .get(self.current + distance)
            .unwrap_or_else(|| self.tokens.last().expect("lexer always emits EOF"))
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if token.kind != TokenKind::EndOfFile {
            self.current += 1;
        }
        token
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
    }

    fn matches(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind, message: &str) -> ParseResult<Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(message))
        }
    }

    fn error(&self, message: &str) -> Diagnostic {
        let location = self.current().span.start;
        Diagnostic::new(
            DiagnosticKind::Syntax,
            message,
            SourceSpan {
                start: location,
                end: self.current().span.end,
            },
        )
    }
}

fn binary_operator(kind: TokenKind) -> Option<(u8, BinaryOp)> {
    Some(match kind {
        TokenKind::Or | TokenKind::LogicalOr => (1, BinaryOp::Or),
        TokenKind::And | TokenKind::LogicalAnd => (2, BinaryOp::And),
        TokenKind::Pipe => (3, BinaryOp::BitwiseOr),
        TokenKind::Caret => (4, BinaryOp::BitwiseXor),
        TokenKind::Ampersand => (5, BinaryOp::BitwiseAnd),
        TokenKind::Equal => (6, BinaryOp::Equal),
        TokenKind::NotEqual => (6, BinaryOp::NotEqual),
        TokenKind::Less => (7, BinaryOp::Less),
        TokenKind::LessEqual => (7, BinaryOp::LessEqual),
        TokenKind::Greater => (7, BinaryOp::Greater),
        TokenKind::GreaterEqual => (7, BinaryOp::GreaterEqual),
        TokenKind::Plus => (8, BinaryOp::Add),
        TokenKind::Minus => (8, BinaryOp::Subtract),
        TokenKind::Star => (9, BinaryOp::Multiply),
        TokenKind::Slash => (9, BinaryOp::Divide),
        TokenKind::Percent => (9, BinaryOp::Remainder),
        _ => return None,
    })
}

fn parse_integer(token: &Token) -> ParseResult<i64> {
    let text = token.lexeme.as_str();
    let value = if text.len() > 2 && text.starts_with('0') {
        match text.as_bytes()[1].to_ascii_lowercase() {
            b'b' => i64::from_str_radix(&text[2..], 2).map(|value| i64::from(value as i32)),
            b'c' => i64::from_str_radix(&text[2..], 8).map(|value| i64::from(value as i32)),
            b'x' => i64::from_str_radix(&text[2..], 16).map(|value| i64::from(value as i32)),
            _ => text.parse(),
        }
    } else {
        text.parse()
    };
    value.map_err(|_| {
        Diagnostic::new(
            DiagnosticKind::Syntax,
            "integer literal is out of range",
            token.span,
        )
    })
}

fn parse_character(token: &Token) -> ParseResult<char> {
    let decoded = decode_quoted(&token.lexeme, '\'');
    let mut chars = decoded.chars();
    Ok(chars.next().unwrap_or('\0'))
}

fn decode_quoted(text: &str, quote: char) -> String {
    let inner = text
        .strip_prefix(quote)
        .and_then(|s| s.strip_suffix(quote))
        .unwrap_or("");
    let mut result = String::new();
    let mut chars = inner.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            result.push(character);
            continue;
        }
        let Some(escaped) = chars.next() else { break };
        result.push(match escaped {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '0' => '0',
            '\\' => '\\',
            '"' => '"',
            '\'' => '\'',
            other => other,
        });
    }
    result
}

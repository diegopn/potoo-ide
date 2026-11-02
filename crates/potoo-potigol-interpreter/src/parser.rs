use crate::ast::{
    Block, Expr, ExprKind, ForGenerator, ForSource, FunctionBody, FunctionDecl, MatchCase, NodeId,
    Parameter, Pattern, PatternKind, Program, Stmt, StmtKind, StringPart, TypeDecl, TypeExpr,
    TypeExprKind, TypeField,
};
use crate::{
    Diagnostic, DiagnosticKind, ExecutionOptions, SourceLocation, SourceSpan, Token, TokenKind, lex,
};

/// Parses a complete Potigol source file using the default nesting limit.
pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    parse_with_options(source, ExecutionOptions::default())
}

/// Parses a Potigol source file using host-provided resource limits.
pub fn parse_with_options(
    source: &str,
    options: ExecutionOptions,
) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lex(source)?;
    Parser::new(tokens, options.max_parse_depth).parse_program()
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    next_id: u32,
    depth: usize,
    max_depth: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>, max_depth: usize) -> Self {
        Self::with_next_id(tokens, max_depth, 0)
    }

    fn with_next_id(tokens: Vec<Token>, max_depth: usize, next_id: u32) -> Self {
        Self {
            tokens,
            index: 0,
            next_id,
            depth: 0,
            max_depth: max_depth.max(1),
        }
    }

    fn parse_program(mut self) -> Result<Program, Vec<Diagnostic>> {
        let start = self.current().span.start;
        let statements = self.parse_block_until(&[])?;
        let end = self.current().span.end;
        Ok(Program {
            id: self.id(),
            statements,
            span: SourceSpan { start, end },
        })
    }

    fn parse_block_until(&mut self, endings: &[&str]) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        self.parse_block_until_mode(endings, false)
    }

    fn parse_block_until_mode(
        &mut self,
        endings: &[&str],
        allow_inline_statements: bool,
    ) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| {
            parser.parse_block_until_inner(endings, allow_inline_statements)
        })
    }

    fn parse_block_until_inner(
        &mut self,
        endings: &[&str],
        allow_inline_statements: bool,
    ) -> Result<Vec<Stmt>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        loop {
            self.skip_separators();
            if self.is_eof() || endings.iter().any(|word| self.at_keyword(word)) {
                return Ok(statements);
            }
            let before = self.index;
            let statement = self.parse_statement()?;
            if self.index == before {
                return Err(vec![
                    self.error_here("não foi possível avançar ao analisar o bloco"),
                ]);
            }
            let statement_end = statement.span.end;
            statements.push(statement);
            if !self.is_eof()
                && !self.at_symbol(';')
                && !self.at_newline()
                && !endings.iter().any(|word| self.at_keyword(word))
            {
                if allow_inline_statements
                    && self.current().span.start.line == statement_end.line
                    && self.current().span.start.column > statement_end.column
                {
                    continue;
                }
                return Err(vec![
                    self.error_here("esperava uma quebra de linha entre instruções"),
                ]);
            }
        }
    }

    fn parse_statement(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        if self.at_keyword("var") {
            return self.parse_mutable_binding();
        }
        if self.at_keyword("tipo") {
            return self.parse_type_declaration();
        }
        if self.consume_keyword("def").is_some() {
            return self.parse_function_declaration();
        }
        if self.at_keyword("escreva") || self.at_keyword("imprima") {
            let token = self.advance().clone();
            let newline = matches!(&token.kind, TokenKind::Identifier(word) if word == "escreva");
            let value = self.parse_expression_list()?;
            return Ok(self.stmt(
                token.span.start,
                value.span.end,
                StmtKind::Output { newline, value },
            ));
        }
        if self.at_keyword("retorne") {
            let start = self.advance().span.start;
            let value = if self.at_statement_end() {
                None
            } else {
                Some(self.parse_expression(0)?)
            };
            let end = value
                .as_ref()
                .map_or(self.previous().span.end, |value| value.span.end);
            return Ok(self.stmt(start, end, StmtKind::Return(value)));
        }
        if self.at_keyword("enquanto") {
            return self.parse_while_statement();
        }
        if self.at_keyword("para") {
            return self.parse_for_statement();
        }
        if self.looks_like_function_declaration() {
            return self.parse_function_declaration();
        }
        if self.looks_like_immutable_binding() {
            return self.parse_immutable_binding();
        }

        let expression = self.parse_expression_list()?;
        if self.consume_operator(":=").is_some() {
            let value = self.parse_expression_list()?;
            let span_start = expression.span.start;
            let span_end = value.span.end;
            return Ok(self.stmt(
                span_start,
                span_end,
                StmtKind::Assignment {
                    target: expression,
                    value,
                },
            ));
        }
        let span = expression.span;
        Ok(self.stmt(span.start, span.end, StmtKind::Expression(expression)))
    }

    fn parse_mutable_binding(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let start = self.advance().span.start;
        let pattern = self.parse_pattern()?;
        let annotation = if self.consume_symbol(':').is_some() {
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        if self.consume_operator(":=").is_none() && self.consume_operator("=").is_none() {
            return Err(vec![
                self.error_here("esperava ':=' ou '=' após a variável"),
            ]);
        }
        let value = self.parse_expression_list()?;
        let end = value.span.end;
        Ok(self.stmt(
            start,
            end,
            StmtKind::Binding {
                pattern,
                annotation,
                mutable: true,
                value,
            },
        ))
    }

    fn parse_immutable_binding(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let start = self.current().span.start;
        let pattern = self.parse_pattern()?;
        let annotation = if self.consume_symbol(':').is_some() {
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        if self.consume_operator("=").is_none() {
            return Err(vec![self.error_here("esperava '=' na declaração de valor")]);
        }
        let value = self.parse_expression_list()?;
        let end = value.span.end;
        Ok(self.stmt(
            start,
            end,
            StmtKind::Binding {
                pattern,
                annotation,
                mutable: false,
                value,
            },
        ))
    }

    fn parse_function_declaration(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let declaration = self.parse_function_signature()?;
        let start = declaration.span.start;
        let body = if self.consume_operator("=").is_some() {
            FunctionBody::Expression(self.parse_expression(0)?)
        } else {
            self.skip_newlines();
            let body_start = self.current().span.start;
            let statements = self.parse_block_until(&["fim"])?;
            let end = self.current().span.start;
            self.expect_keyword("fim")?;
            FunctionBody::Block(Block {
                id: self.id(),
                statements,
                span: SourceSpan {
                    start: body_start,
                    end,
                },
            })
        };
        let end = match &body {
            FunctionBody::Expression(expression) => expression.span.end,
            FunctionBody::Block(block) => block.span.end,
        };
        let function = FunctionDecl {
            body,
            span: SourceSpan { start, end },
            ..declaration
        };
        Ok(self.stmt(start, end, StmtKind::Function(function)))
    }

    fn parse_function_signature(&mut self) -> Result<FunctionDecl, Vec<Diagnostic>> {
        let name_token = self.expect_identifier()?;
        let name = self.identifier_value(&name_token)?;
        self.expect_symbol('(')?;
        let parameters = self.parse_parameters(')')?;
        self.expect_symbol(')')?;
        self.skip_newlines();
        let return_type = if self.consume_symbol(':').is_some() {
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        let end = return_type
            .as_ref()
            .map_or(self.previous().span.end, |ty| ty.span.end);
        Ok(FunctionDecl {
            name,
            parameters,
            return_type,
            body: FunctionBody::Expression(Expr {
                id: self.id(),
                span: SourceSpan { start: end, end },
                kind: ExprKind::Tuple(Vec::new()),
            }),
            id: self.id(),
            span: SourceSpan {
                start: name_token.span.start,
                end,
            },
        })
    }

    fn parse_type_declaration(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let start = self.advance().span.start;
        let abstract_type = self.consume_keyword("abstrato").is_some();
        let name_token = self.expect_identifier()?;
        let name = self.identifier_value(&name_token)?;
        let parent = if self.consume_symbol(':').is_some() {
            let parent = self.expect_identifier()?;
            Some(self.identifier_value(&parent)?)
        } else {
            None
        };
        self.skip_separators();
        let mut fields = Vec::new();
        let mut methods = Vec::new();
        while !self.is_eof() && !self.at_keyword("fim") {
            self.skip_separators();
            if self.at_keyword("fim") || self.is_eof() {
                break;
            }
            let before = self.index;
            if self.consume_keyword("def").is_some() || self.looks_like_function_declaration() {
                let mut method = self.parse_function_signature()?;
                let body = if self.consume_operator("=").is_some() {
                    FunctionBody::Expression(self.parse_expression(0)?)
                } else {
                    self.skip_newlines();
                    let body_start = self.current().span.start;
                    let statements = self.parse_block_until(&["fim"])?;
                    let body_end = self.current().span.start;
                    self.expect_keyword("fim")?;
                    FunctionBody::Block(Block {
                        id: self.id(),
                        statements,
                        span: SourceSpan {
                            start: body_start,
                            end: body_end,
                        },
                    })
                };
                method.span.end = match &body {
                    FunctionBody::Expression(expression) => expression.span.end,
                    FunctionBody::Block(block) => block.span.end,
                };
                method.body = body;
                methods.push(method);
            } else {
                fields.extend(self.parse_type_field()?);
            }
            if self.index == before {
                return Err(vec![self.error_here("declaração de tipo sem progresso")]);
            }
            if !self.at_newline() && !self.at_symbol(';') && !self.at_keyword("fim") {
                return Err(vec![
                    self.error_here("esperava uma quebra de linha no tipo"),
                ]);
            }
            self.skip_separators();
        }
        let end = self.current().span.start;
        self.expect_keyword("fim")?;
        let declaration = TypeDecl {
            id: self.id(),
            name,
            parent,
            abstract_type,
            fields,
            methods,
            span: SourceSpan { start, end },
        };
        Ok(self.stmt(start, self.previous().span.end, StmtKind::Type(declaration)))
    }

    fn parse_type_field(&mut self) -> Result<Vec<TypeField>, Vec<Diagnostic>> {
        let mutable = self.consume_keyword("var").is_some();
        let mut names = vec![self.expect_identifier()?];
        while self.consume_symbol(',').is_some() {
            names.push(self.expect_identifier()?);
        }
        let annotation = if self.consume_symbol(':').is_some() {
            Some(self.parse_type_expression()?)
        } else {
            None
        };
        let default_value =
            if self.consume_operator("=").is_some() || self.consume_operator(":=").is_some() {
                Some(self.parse_expression_list()?)
            } else {
                None
            };
        if annotation.is_none() && default_value.is_none() {
            return Err(vec![
                self.error_here("atributo precisa de um tipo ou valor inicial"),
            ]);
        }
        let values = match default_value {
            Some(Expr {
                kind: ExprKind::Tuple(values),
                ..
            }) if names.len() == values.len() => values.into_iter().map(Some).collect(),
            value if names.len() == 1 => vec![value],
            Some(value) => {
                return Err(vec![self.error_at(
                    "quantidade de valores iniciais diferente da quantidade de atributos",
                    value.span,
                )]);
            }
            None => vec![None; names.len()],
        };
        let end = values.iter().flatten().last().map_or_else(
            || {
                annotation
                    .as_ref()
                    .map_or(names[0].span.end, |ty| ty.span.end)
            },
            |value| value.span.end,
        );
        let mut fields = Vec::with_capacity(names.len());
        for (name_token, default_value) in names.into_iter().zip(values) {
            fields.push(TypeField {
                id: self.id(),
                name: self.identifier_value(&name_token)?,
                annotation: annotation.clone(),
                mutable,
                constructor_parameter: default_value.is_none(),
                default_value,
                span: SourceSpan {
                    start: name_token.span.start,
                    end,
                },
            });
        }
        Ok(fields)
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let start = self.advance().span.start;
        let condition = self.parse_expression(0)?;
        if self.consume_keyword("faça").is_none() && self.consume_keyword("faca").is_none() {
            return Err(vec![self.error_here("esperava 'faça' após a condição")]);
        }
        self.skip_newlines();
        let body_start = self.current().span.start;
        let statements = self.parse_block_until(&["fim"])?;
        let end = self.current().span.start;
        self.expect_keyword("fim")?;
        let body = Block {
            id: self.id(),
            statements,
            span: SourceSpan {
                start: body_start,
                end,
            },
        };
        Ok(self.stmt(
            start,
            self.previous().span.end,
            StmtKind::While { condition, body },
        ))
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let start = self.advance().span.start;
        let generators = self.parse_generators_until(&["faça", "faca"])?;
        if self.consume_keyword("faça").is_none() && self.consume_keyword("faca").is_none() {
            return Err(vec![
                self.error_here("esperava 'faça' após os limites do laço"),
            ]);
        }
        self.skip_newlines();
        let body_start = self.current().span.start;
        let statements = self.parse_block_until(&["fim"])?;
        let end = self.current().span.start;
        self.expect_keyword("fim")?;
        let statement_end = self.previous().span.end;
        let body = Block {
            id: self.id(),
            statements,
            span: SourceSpan {
                start: body_start,
                end,
            },
        };
        Ok(self.stmt(start, statement_end, StmtKind::For { generators, body }))
    }

    fn parse_generators_until(
        &mut self,
        endings: &[&str],
    ) -> Result<Vec<ForGenerator>, Vec<Diagnostic>> {
        let mut generators = Vec::new();
        loop {
            let start = self.current().span.start;
            let pattern = self.parse_pattern()?;
            let source = if self.consume_keyword("de").is_some() {
                let range_start = self.parse_expression(0)?;
                if self.consume_keyword("até").is_none() && self.consume_keyword("ate").is_none() {
                    return Err(vec![self.error_here("esperava 'até' no gerador do laço")]);
                }
                let end = self.parse_expression(0)?;
                let step = if self.consume_keyword("passo").is_some() {
                    Some(self.parse_expression(0)?)
                } else {
                    None
                };
                ForSource::Range {
                    start: range_start,
                    end,
                    step,
                }
            } else if self.consume_keyword("em").is_some() {
                ForSource::Iterable(self.parse_expression(0)?)
            } else {
                return Err(vec![
                    self.error_here("esperava 'de' ou 'em' no gerador do laço"),
                ]);
            };
            let condition = if self.consume_keyword("se").is_some() {
                Some(self.parse_expression(0)?)
            } else {
                None
            };
            let end = match &source {
                ForSource::Range { step, end, .. } => {
                    step.as_ref().map_or(end.span.end, |step| step.span.end)
                }
                ForSource::Iterable(iterable) => iterable.span.end,
            };
            let end = condition
                .as_ref()
                .map_or(end, |condition| condition.span.end);
            generators.push(ForGenerator {
                id: self.id(),
                pattern,
                source,
                condition,
                span: SourceSpan { start, end },
            });
            if self.consume_symbol(',').is_none() {
                break;
            }
            self.skip_newlines();
            if endings.iter().any(|word| self.at_keyword(word)) {
                return Err(vec![self.error_here("gerador incompleto após ','")]);
            }
        }
        Ok(generators)
    }

    fn parse_expression(&mut self, minimum_precedence: u8) -> Result<Expr, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| parser.parse_expression_inner(minimum_precedence))
    }

    fn parse_expression_list(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        let first = self.parse_expression(0)?;
        if self.consume_symbol(',').is_none() {
            return Ok(first);
        }
        let start = first.span.start;
        let mut elements = vec![first];
        loop {
            self.skip_newlines();
            elements.push(self.parse_expression(0)?);
            if self.consume_symbol(',').is_none() {
                break;
            }
        }
        let end = elements.last().map_or(start, |element| element.span.end);
        Ok(self.expr(SourceSpan { start, end }, ExprKind::Tuple(elements)))
    }

    fn parse_expression_inner(&mut self, minimum_precedence: u8) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.parse_prefix_expression()?;
        loop {
            self.skip_newlines_before_operator();
            if let Some(postfix) = self.parse_postfix(left.clone())? {
                left = postfix;
                continue;
            }
            let Some((operator, precedence, right_associative)) = self.current_binary_operator()
            else {
                break;
            };
            if precedence < minimum_precedence {
                break;
            }
            self.advance();
            let next_minimum = if right_associative {
                precedence
            } else {
                precedence + 1
            };
            let right = self.parse_expression(next_minimum)?;
            let span = SourceSpan {
                start: left.span.start,
                end: right.span.end,
            };
            left = self.expr(
                span,
                ExprKind::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                },
            );
        }
        if minimum_precedence == 0 && self.consume_operator("=>").is_some() {
            let span_start = left.span.start;
            let parameters = match left.kind {
                ExprKind::Identifier(name) => vec![Parameter {
                    id: self.id(),
                    name,
                    annotation: None,
                    mutable: false,
                    span: left.span,
                }],
                ExprKind::Tuple(elements) => elements
                    .into_iter()
                    .map(|element| match element.kind {
                        ExprKind::Identifier(name) => Ok(Parameter {
                            id: self.id(),
                            name,
                            annotation: None,
                            mutable: false,
                            span: element.span,
                        }),
                        _ => Err(vec![self.error_at(
                            "parâmetros de lambda devem ser identificadores",
                            element.span,
                        )]),
                    })
                    .collect::<Result<Vec<_>, _>>()?,
                _ => {
                    return Err(vec![
                        self.error_at("parâmetros inválidos para lambda", left.span),
                    ]);
                }
            };
            let body = self.parse_expression(0)?;
            let span = SourceSpan {
                start: span_start,
                end: body.span.end,
            };
            left = self.expr(
                span,
                ExprKind::Lambda {
                    parameters,
                    body: Box::new(body),
                },
            );
        }
        Ok(left)
    }

    fn parse_prefix_expression(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        self.skip_newlines();
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Integer(value) => Ok(self.expr(token.span, ExprKind::Integer(value))),
            TokenKind::Real(value) => Ok(self.expr(token.span, ExprKind::Real(value))),
            TokenKind::Character(value) => Ok(self.expr(token.span, ExprKind::Character(value))),
            TokenKind::String(value) => {
                let parts = self.parse_string_parts(&value, token.span)?;
                Ok(self.expr(token.span, ExprKind::Text(parts)))
            }
            TokenKind::Identifier(word) if word == "verdadeiro" => {
                Ok(self.expr(token.span, ExprKind::Logical(true)))
            }
            TokenKind::Identifier(word) if word == "falso" => {
                Ok(self.expr(token.span, ExprKind::Logical(false)))
            }
            TokenKind::Identifier(word) if word == "_" => {
                Ok(self.expr(token.span, ExprKind::Placeholder))
            }
            TokenKind::Identifier(word) if word == "se" => {
                self.parse_conditional_expression(token.span.start)
            }
            TokenKind::Identifier(word) if word == "escolha" => {
                self.parse_match_expression(token.span.start)
            }
            TokenKind::Identifier(word) if word == "para" => {
                self.parse_comprehension(token.span.start)
            }
            TokenKind::Identifier(word) if matches!(word.as_str(), "não" | "nao") => {
                let operand = self.parse_expression(3)?;
                let span = SourceSpan {
                    start: token.span.start,
                    end: operand.span.end,
                };
                Ok(self.expr(
                    span,
                    ExprKind::Unary {
                        operator: word,
                        operand: Box::new(operand),
                    },
                ))
            }
            TokenKind::Operator(operator) if matches!(operator.as_str(), "-" | "+") => {
                let operand = self.parse_expression(7)?;
                let span = SourceSpan {
                    start: token.span.start,
                    end: operand.span.end,
                };
                Ok(self.expr(
                    span,
                    ExprKind::Unary {
                        operator,
                        operand: Box::new(operand),
                    },
                ))
            }
            TokenKind::Identifier(name) => Ok(self.expr(token.span, ExprKind::Identifier(name))),
            TokenKind::Symbol('(') => self.parse_parenthesized(token.span.start),
            TokenKind::Symbol('[') => self.parse_list(token.span.start),
            _ => Err(vec![self.error_at("esperava uma expressão", token.span)]),
        }
    }

    fn parse_parenthesized(&mut self, start: SourceLocation) -> Result<Expr, Vec<Diagnostic>> {
        if self.at_symbol(')') {
            let end = self.advance().span.end;
            return Ok(self.expr(SourceSpan { start, end }, ExprKind::Tuple(Vec::new())));
        }
        if self.looks_like_lambda_parameters() {
            let parameters = self.parse_parameters(')')?;
            self.expect_symbol(')')?;
            self.expect_operator("=>")?;
            let body = self.parse_expression(0)?;
            let span = SourceSpan {
                start,
                end: body.span.end,
            };
            return Ok(self.expr(
                span,
                ExprKind::Lambda {
                    parameters,
                    body: Box::new(body),
                },
            ));
        }
        let first = self.parse_expression(0)?;
        if self.consume_symbol(',').is_none() {
            self.expect_symbol(')')?;
            return Ok(first);
        }
        let mut elements = vec![first];
        loop {
            self.skip_newlines();
            if self.at_symbol(')') {
                break;
            }
            elements.push(self.parse_expression(0)?);
            if self.consume_symbol(',').is_none() {
                break;
            }
        }
        let end = self.expect_symbol(')')?.span.end;
        Ok(self.expr(SourceSpan { start, end }, ExprKind::Tuple(elements)))
    }

    fn parse_list(&mut self, start: SourceLocation) -> Result<Expr, Vec<Diagnostic>> {
        let mut elements = Vec::new();
        self.skip_newlines();
        if !self.at_symbol(']') {
            loop {
                elements.push(self.parse_expression(0)?);
                self.skip_newlines();
                if self.consume_symbol(',').is_none() {
                    break;
                }
                self.skip_newlines();
            }
        }
        let end = self.expect_symbol(']')?.span.end;
        Ok(self.expr(SourceSpan { start, end }, ExprKind::List(elements)))
    }

    fn parse_postfix(&mut self, expression: Expr) -> Result<Option<Expr>, Vec<Diagnostic>> {
        if self.consume_symbol('(').is_some() {
            let mut arguments = Vec::new();
            self.skip_newlines();
            if !self.at_symbol(')') {
                loop {
                    arguments.push(self.parse_expression(0)?);
                    self.skip_newlines();
                    if self.consume_symbol(',').is_none() {
                        break;
                    }
                    self.skip_newlines();
                }
            }
            let end = self.expect_symbol(')')?.span.end;
            return Ok(Some(self.expr(
                SourceSpan {
                    start: expression.span.start,
                    end,
                },
                ExprKind::Call {
                    callee: Box::new(expression),
                    arguments,
                },
            )));
        }
        if self.consume_symbol('[').is_some() {
            let mut indices = Vec::new();
            loop {
                indices.push(self.parse_expression(0)?);
                self.skip_newlines();
                if self.consume_symbol(',').is_none() {
                    break;
                }
                self.skip_newlines();
            }
            let end = self.expect_symbol(']')?.span.end;
            return Ok(Some(self.expr(
                SourceSpan {
                    start: expression.span.start,
                    end,
                },
                ExprKind::Index {
                    receiver: Box::new(expression),
                    indices,
                },
            )));
        }
        if self.consume_symbol('.').is_some() {
            let member = self.expect_identifier()?;
            let name = self.identifier_value(&member)?;
            return Ok(Some(self.expr(
                SourceSpan {
                    start: expression.span.start,
                    end: member.span.end,
                },
                ExprKind::Member {
                    receiver: Box::new(expression),
                    name,
                },
            )));
        }
        Ok(None)
    }

    fn parse_conditional_expression(
        &mut self,
        start: SourceLocation,
    ) -> Result<Expr, Vec<Diagnostic>> {
        let condition = self.parse_expression(0)?;
        self.parse_conditional_branch(start, condition)
    }

    fn parse_conditional_branch(
        &mut self,
        start: SourceLocation,
        condition: Expr,
    ) -> Result<Expr, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| parser.parse_conditional_branch_inner(start, condition))
    }

    fn parse_conditional_branch_inner(
        &mut self,
        start: SourceLocation,
        condition: Expr,
    ) -> Result<Expr, Vec<Diagnostic>> {
        if self.consume_keyword("então").is_none() && self.consume_keyword("entao").is_none() {
            return Err(vec![self.error_here("esperava 'então' após a condição")]);
        }
        self.skip_newlines();
        let then_start = self.current().span.start;
        let then_statements =
            self.parse_block_until_mode(&["senãose", "senaose", "senão", "senao", "fim"], true)?;
        let then_end = self.current().span.start;
        let (else_branch, end) = if self.consume_keyword("senãose").is_some()
            || self.consume_keyword("senaose").is_some()
        {
            let nested_start = self.previous().span.start;
            let nested_condition = self.parse_expression(0)?;
            let nested = self.parse_conditional_branch(nested_start, nested_condition)?;
            let end = nested.span.end;
            let statement = self.stmt(
                nested.span.start,
                nested.span.end,
                StmtKind::Expression(nested),
            );
            (
                Some(Block {
                    id: self.id(),
                    statements: vec![statement],
                    span: SourceSpan {
                        start: nested_start,
                        end,
                    },
                }),
                end,
            )
        } else if self.consume_keyword("senão").is_some() || self.consume_keyword("senao").is_some()
        {
            self.skip_newlines();
            let else_start = self.current().span.start;
            let statements = self.parse_block_until_mode(&["fim"], true)?;
            let end = self.expect_keyword("fim")?.span.end;
            (
                Some(Block {
                    id: self.id(),
                    statements,
                    span: SourceSpan {
                        start: else_start,
                        end: self.previous().span.start,
                    },
                }),
                end,
            )
        } else {
            let end = self.expect_keyword("fim")?.span.end;
            (None, end)
        };
        let then_branch = Block {
            id: self.id(),
            statements: then_statements,
            span: SourceSpan {
                start: then_start,
                end: then_end,
            },
        };
        Ok(self.expr(
            SourceSpan { start, end },
            ExprKind::Conditional {
                condition: Box::new(condition),
                then_branch,
                else_branch,
            },
        ))
    }

    fn parse_match_expression(&mut self, start: SourceLocation) -> Result<Expr, Vec<Diagnostic>> {
        let value = self.parse_expression(0)?;
        self.skip_newlines();
        let mut cases = Vec::new();
        while self.consume_keyword("caso").is_some() {
            let case_start = self.previous().span.start;
            let pattern = self.parse_pattern()?;
            let guard = if self.consume_keyword("se").is_some() {
                Some(self.parse_expression(0)?)
            } else {
                None
            };
            self.expect_operator("=>")?;
            self.skip_newlines();
            let body_start = self.current().span.start;
            let statements = self.parse_block_until(&["caso", "fim"])?;
            let body_end = self.current().span.start;
            cases.push(MatchCase {
                id: self.id(),
                pattern,
                guard,
                value: Block {
                    id: self.id(),
                    statements,
                    span: SourceSpan {
                        start: body_start,
                        end: body_end,
                    },
                },
                span: SourceSpan {
                    start: case_start,
                    end: body_end,
                },
            });
            self.skip_newlines();
        }
        let end = self.expect_keyword("fim")?.span.end;
        if cases.is_empty() {
            return Err(vec![self.error_at(
                "'escolha' precisa de ao menos um caso",
                SourceSpan { start, end },
            )]);
        }
        Ok(self.expr(
            SourceSpan { start, end },
            ExprKind::Match {
                value: Box::new(value),
                cases,
            },
        ))
    }

    fn parse_comprehension(&mut self, start: SourceLocation) -> Result<Expr, Vec<Diagnostic>> {
        let generators = self.parse_generators_until(&["gere"])?;
        self.expect_keyword("gere")?;
        let value = self.parse_expression(0)?;
        let end = self.expect_keyword("fim")?.span.end;
        Ok(self.expr(
            SourceSpan { start, end },
            ExprKind::Comprehension {
                generators,
                value: Box::new(value),
            },
        ))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| parser.parse_pattern_inner())
    }

    fn parse_pattern_inner(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        let mut pattern = self.parse_cons_pattern()?;
        if self.consume_symbol(',').is_some() {
            let mut elements = vec![pattern];
            loop {
                elements.push(self.parse_cons_pattern()?);
                if self.consume_symbol(',').is_none() {
                    break;
                }
            }
            let span = SourceSpan {
                start: elements
                    .first()
                    .map_or(self.current().span.start, |item| item.span.start),
                end: elements
                    .last()
                    .map_or(self.current().span.end, |item| item.span.end),
            };
            pattern = Pattern {
                id: self.id(),
                span,
                kind: PatternKind::Tuple(elements),
            };
        }
        Ok(pattern)
    }

    fn parse_cons_pattern(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| parser.parse_cons_pattern_inner())
    }

    fn parse_cons_pattern_inner(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        let mut pattern = self.parse_atomic_pattern()?;
        if self.consume_operator("::").is_some() {
            let tail = self.parse_cons_pattern()?;
            let span = SourceSpan {
                start: pattern.span.start,
                end: tail.span.end,
            };
            pattern = Pattern {
                id: self.id(),
                span,
                kind: PatternKind::Cons(Box::new(pattern), Box::new(tail)),
            };
        }
        Ok(pattern)
    }

    fn parse_atomic_pattern(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        let token = self.advance().clone();
        let kind = match token.kind {
            TokenKind::Integer(value) => PatternKind::Integer(value),
            TokenKind::Real(value) => PatternKind::Real(value),
            TokenKind::Character(value) => PatternKind::Character(value),
            TokenKind::String(value) => PatternKind::Text(value),
            TokenKind::Identifier(name) if name == "_" => PatternKind::Wildcard,
            TokenKind::Identifier(name) if name == "verdadeiro" => PatternKind::Logical(true),
            TokenKind::Identifier(name) if name == "falso" => PatternKind::Logical(false),
            TokenKind::Identifier(name) => {
                if self.consume_symbol('(').is_some() {
                    let mut fields = Vec::new();
                    if !self.at_symbol(')') {
                        loop {
                            fields.push(self.parse_cons_pattern()?);
                            if self.consume_symbol(',').is_none() {
                                break;
                            }
                        }
                    }
                    let end = self.expect_symbol(')')?.span.end;
                    return Ok(Pattern {
                        id: self.id(),
                        span: SourceSpan {
                            start: token.span.start,
                            end,
                        },
                        kind: PatternKind::Constructor { name, fields },
                    });
                }
                PatternKind::Binding(name)
            }
            TokenKind::Symbol('[') => {
                let mut elements = Vec::new();
                if !self.at_symbol(']') {
                    loop {
                        elements.push(self.parse_cons_pattern()?);
                        if self.consume_symbol(',').is_none() {
                            break;
                        }
                    }
                }
                let end = self.expect_symbol(']')?.span.end;
                return Ok(Pattern {
                    id: self.id(),
                    span: SourceSpan {
                        start: token.span.start,
                        end,
                    },
                    kind: PatternKind::List(elements),
                });
            }
            TokenKind::Symbol('(') => {
                let mut elements = vec![self.parse_cons_pattern()?];
                if self.consume_symbol(',').is_some() {
                    loop {
                        elements.push(self.parse_cons_pattern()?);
                        if self.consume_symbol(',').is_none() {
                            break;
                        }
                    }
                    let end = self.expect_symbol(')')?.span.end;
                    return Ok(Pattern {
                        id: self.id(),
                        span: SourceSpan {
                            start: token.span.start,
                            end,
                        },
                        kind: PatternKind::Tuple(elements),
                    });
                }
                self.expect_symbol(')')?;
                return Ok(elements.remove(0));
            }
            _ => return Err(vec![self.error_at("esperava um padrão", token.span)]),
        };
        Ok(Pattern {
            id: self.id(),
            span: token.span,
            kind,
        })
    }

    fn parse_type_expression(&mut self) -> Result<TypeExpr, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| parser.parse_type_expression_inner())
    }

    fn parse_type_expression_inner(&mut self) -> Result<TypeExpr, Vec<Diagnostic>> {
        let mut left = self.parse_atomic_type()?;
        if self.consume_operator("=>").is_some() || self.consume_operator("->").is_some() {
            let result = self.parse_type_expression()?;
            let parameters = match left.kind {
                TypeExprKind::Tuple(elements) => elements,
                other => vec![TypeExpr {
                    id: left.id,
                    span: left.span,
                    kind: other,
                }],
            };
            let span = SourceSpan {
                start: left.span.start,
                end: result.span.end,
            };
            left = TypeExpr {
                id: self.id(),
                span,
                kind: TypeExprKind::Function(parameters, Box::new(result)),
            };
        }
        Ok(left)
    }

    fn parse_atomic_type(&mut self) -> Result<TypeExpr, Vec<Diagnostic>> {
        if self.consume_symbol('(').is_some() {
            let start = self.previous().span.start;
            let mut elements = Vec::new();
            if !self.at_symbol(')') {
                loop {
                    elements.push(self.parse_type_expression()?);
                    if self.consume_symbol(',').is_none() {
                        break;
                    }
                }
            }
            let end = self.expect_symbol(')')?.span.end;
            return Ok(TypeExpr {
                id: self.id(),
                span: SourceSpan { start, end },
                kind: TypeExprKind::Tuple(elements),
            });
        }
        let name_token = self.expect_identifier()?;
        let name = self.identifier_value(&name_token)?;
        let mut arguments = Vec::new();
        if self.consume_symbol('[').is_some() {
            loop {
                arguments.push(self.parse_type_expression()?);
                if self.consume_symbol(',').is_none() {
                    break;
                }
            }
            let end = self.expect_symbol(']')?.span.end;
            return Ok(TypeExpr {
                id: self.id(),
                span: SourceSpan {
                    start: name_token.span.start,
                    end,
                },
                kind: TypeExprKind::Named(name, arguments),
            });
        }
        Ok(TypeExpr {
            id: self.id(),
            span: name_token.span,
            kind: TypeExprKind::Named(name, arguments),
        })
    }

    fn parse_parameters(&mut self, closing: char) -> Result<Vec<Parameter>, Vec<Diagnostic>> {
        let mut parameters = Vec::new();
        let mut pending = Vec::<(String, SourceSpan, bool)>::new();
        if self.at_symbol(closing) {
            return Ok(parameters);
        }
        loop {
            self.skip_newlines();
            let mutable = self.consume_keyword("var").is_some();
            let token = self.expect_identifier()?;
            let name = self.identifier_value(&token)?;
            pending.push((name, token.span, mutable));
            if self.consume_symbol(':').is_some() {
                let annotation = self.parse_type_expression()?;
                parameters.extend(pending.drain(..).map(|(name, span, mutable)| Parameter {
                    id: self.id(),
                    name,
                    annotation: Some(annotation.clone()),
                    mutable,
                    span: SourceSpan {
                        start: span.start,
                        end: annotation.span.end,
                    },
                }));
            }
            if self.consume_symbol(',').is_none() {
                break;
            }
            self.skip_newlines();
        }
        parameters.extend(pending.into_iter().map(|(name, span, mutable)| Parameter {
            id: self.id(),
            name,
            annotation: None,
            mutable,
            span,
        }));
        Ok(parameters)
    }

    fn parse_string_parts(
        &mut self,
        raw: &str,
        token_span: SourceSpan,
    ) -> Result<Vec<StringPart>, Vec<Diagnostic>> {
        let chars: Vec<char> = raw.chars().collect();
        let mut parts = Vec::new();
        let mut text = String::new();
        let mut index = 0usize;
        while index < chars.len() {
            if chars[index] == '\\' {
                text.push(chars[index]);
                index += 1;
                if index < chars.len() {
                    text.push(chars[index]);
                    index += 1;
                }
                continue;
            }
            if chars[index] != '{' {
                text.push(chars[index]);
                index += 1;
                continue;
            }
            if !text.is_empty() {
                parts.push(StringPart::Text(std::mem::take(&mut text)));
            }
            let expression_start = index + 1;
            index += 1;
            let mut depth = 1usize;
            let mut quote = None;
            while index < chars.len() && depth > 0 {
                let character = chars[index];
                if let Some(delimiter) = quote {
                    if character == '\\' {
                        index = (index + 2).min(chars.len());
                        continue;
                    }
                    if character == delimiter {
                        quote = None;
                    }
                } else {
                    match character {
                        '"' | '\'' => quote = Some(character),
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {}
                    }
                }
                index += 1;
            }
            if depth != 0 {
                return Err(vec![
                    self.error_at("interpolação não terminada", token_span),
                ]);
            }
            let expression_end = index - 1;
            let embedded: String = chars[expression_start..expression_end].iter().collect();
            let string_bytes = token_span
                .end
                .byte_offset
                .saturating_sub(token_span.start.byte_offset);
            let quote_bytes = string_bytes.saturating_sub(raw.len());
            let quote_count = if quote_bytes >= 6 { 3 } else { 1 };
            let base =
                string_content_location(token_span.start, quote_count, &chars[..expression_start]);
            let expression = self.parse_embedded_expression(&embedded, base)?;
            parts.push(StringPart::Expression(expression));
        }
        if !text.is_empty() || parts.is_empty() {
            parts.push(StringPart::Text(text));
        }
        Ok(parts)
    }

    fn parse_embedded_expression(
        &mut self,
        source: &str,
        base: SourceLocation,
    ) -> Result<Expr, Vec<Diagnostic>> {
        let mut tokens = lex(source).map_err(|mut errors| {
            for error in &mut errors {
                error.span = shift_span(error.span, base);
            }
            errors
        })?;
        for token in &mut tokens {
            token.span = shift_span(token.span, base);
        }
        let mut parser = Parser::with_next_id(tokens, self.max_depth, self.next_id);
        let result = (|| {
            let expression = parser.parse_expression(0)?;
            parser.skip_newlines();
            if !parser.is_eof() {
                return Err(vec![
                    parser.error_here("conteúdo extra após expressão interpolada"),
                ]);
            }
            Ok(expression)
        })();
        self.next_id = parser.next_id;
        result
    }

    fn current_binary_operator(&self) -> Option<(String, u8, bool)> {
        let (operator, precedence, right_associative) = match &self.current().kind {
            TokenKind::Operator(operator) => match operator.as_str() {
                "==" | "<>" | "<" | "<=" | ">" | ">=" => (operator.as_str(), 3, false),
                "::" => (operator.as_str(), 6, true),
                "+" | "-" => (operator.as_str(), 5, false),
                "*" | "/" => (operator.as_str(), 6, false),
                "^" => (operator.as_str(), 7, true),
                _ => return None,
            },
            TokenKind::Identifier(word) => match word.as_str() {
                "ou" => (word.as_str(), 1, false),
                "e" => (word.as_str(), 2, false),
                "mod" | "div" => (word.as_str(), 6, false),
                "formato" => (word.as_str(), 7, false),
                _ => return None,
            },
            _ => return None,
        };
        Some((operator.to_owned(), precedence, right_associative))
    }

    fn looks_like_function_declaration(&self) -> bool {
        if !matches!(self.current().kind, TokenKind::Identifier(_))
            || !self.token_at(self.index + 1, |kind| {
                matches!(kind, TokenKind::Symbol('('))
            })
        {
            return false;
        }
        let mut index = self.index + 2;
        let mut nesting = 1usize;
        while index < self.tokens.len() && nesting > 0 {
            match self.tokens[index].kind {
                TokenKind::Symbol('(') => nesting += 1,
                TokenKind::Symbol(')') => nesting -= 1,
                TokenKind::Eof => return false,
                _ => {}
            }
            index += 1;
        }
        if nesting != 0 {
            return false;
        }
        let starts_new_line = self.token_at(index, |kind| matches!(kind, TokenKind::Newline));
        while matches!(
            self.tokens.get(index).map(|token| &token.kind),
            Some(TokenKind::Newline)
        ) {
            index += 1;
        }
        if self.token_at(index, |kind| matches!(kind, TokenKind::Symbol(':'))) {
            return true;
        }
        if self.token_at(
            index,
            |kind| matches!(kind, TokenKind::Operator(operator) if operator == "="),
        ) {
            return true;
        }
        let has_block_terminator = self.tokens[index..]
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::Identifier(word) if word == "fim"));
        let body_is_indented = self.tokens.get(index).is_some_and(|body| {
            starts_new_line
                && body.span.start.line > self.current().span.start.line
                && body.span.start.column > self.current().span.start.column
        });
        has_block_terminator && body_is_indented
    }

    fn looks_like_immutable_binding(&self) -> bool {
        if let TokenKind::Identifier(word) = &self.current().kind {
            if matches!(
                word.as_str(),
                "se" | "senão"
                    | "senao"
                    | "escolha"
                    | "caso"
                    | "enquanto"
                    | "para"
                    | "tipo"
                    | "var"
                    | "escreva"
                    | "imprima"
                    | "retorne"
                    | "fim"
                    | "verdadeiro"
                    | "falso"
                    | "não"
                    | "nao"
                    | "ou"
                    | "e"
            ) {
                return false;
            }
        }
        let mut index = self.index;
        let mut nesting = 0usize;
        loop {
            let Some(token) = self.tokens.get(index) else {
                return false;
            };
            match &token.kind {
                TokenKind::Symbol('(' | '[') => nesting += 1,
                TokenKind::Symbol(')' | ']') => {
                    if nesting == 0 {
                        return false;
                    }
                    nesting -= 1;
                }
                TokenKind::Operator(operator) if nesting == 0 && operator == "=" => {
                    return true;
                }
                TokenKind::Newline | TokenKind::Eof if nesting == 0 => return false,
                TokenKind::Eof => return false,
                _ => {}
            }
            index += 1;
        }
    }

    fn looks_like_lambda_parameters(&self) -> bool {
        let mut index = self.index;
        let mut nesting = 1usize;
        while let Some(token) = self.tokens.get(index) {
            match token.kind {
                TokenKind::Symbol('(') => nesting += 1,
                TokenKind::Symbol(')') => {
                    nesting = nesting.saturating_sub(1);
                    if nesting == 0 {
                        return self.token_at(index + 1, |kind| {
                            matches!(kind, TokenKind::Operator(operator) if operator == "=>")
                        });
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            index += 1;
        }
        false
    }

    fn skip_newlines_before_operator(&mut self) {
        let mut next = self.index;
        while matches!(
            self.tokens.get(next).map(|token| &token.kind),
            Some(TokenKind::Newline)
        ) {
            next += 1;
        }
        if self.token_at(next, |kind| {
            matches!(kind, TokenKind::Operator(operator) if matches!(operator.as_str(), "+" | "-" | "*" | "/" | "^" | "==" | "<>" | "<" | "<=" | ">" | ">=" | "::"))
                || matches!(kind, TokenKind::Identifier(word) if matches!(word.as_str(), "e" | "ou" | "div" | "mod"))
        }) {
            self.skip_newlines();
        }
    }

    fn at_statement_end(&self) -> bool {
        self.at_newline() || self.at_symbol(';') || self.is_eof() || self.at_keyword("fim")
    }

    fn skip_separators(&mut self) {
        while self.at_newline() || self.at_symbol(';') {
            self.advance();
        }
    }

    fn skip_newlines(&mut self) {
        while self.at_newline() {
            self.advance();
        }
    }

    fn at_newline(&self) -> bool {
        matches!(self.current().kind, TokenKind::Newline)
    }

    fn at_keyword(&self, keyword: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(word) if word == keyword)
    }

    fn consume_keyword(&mut self, keyword: &str) -> Option<Token> {
        if self.at_keyword(keyword) {
            Some(self.advance().clone())
        } else {
            None
        }
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<Token, Vec<Diagnostic>> {
        self.consume_keyword(keyword)
            .ok_or_else(|| vec![self.error_here(format!("esperava '{keyword}'"))])
    }

    fn at_symbol(&self, symbol: char) -> bool {
        matches!(self.current().kind, TokenKind::Symbol(found) if found == symbol)
    }

    fn consume_symbol(&mut self, symbol: char) -> Option<Token> {
        if self.at_symbol(symbol) {
            Some(self.advance().clone())
        } else {
            None
        }
    }

    fn expect_symbol(&mut self, symbol: char) -> Result<Token, Vec<Diagnostic>> {
        self.consume_symbol(symbol)
            .ok_or_else(|| vec![self.error_here(format!("esperava '{symbol}'"))])
    }

    fn consume_operator(&mut self, operator: &str) -> Option<Token> {
        if matches!(&self.current().kind, TokenKind::Operator(found) if found == operator) {
            Some(self.advance().clone())
        } else {
            None
        }
    }

    fn expect_operator(&mut self, operator: &str) -> Result<Token, Vec<Diagnostic>> {
        self.consume_operator(operator)
            .ok_or_else(|| vec![self.error_here(format!("esperava '{operator}'"))])
    }

    fn expect_identifier(&mut self) -> Result<Token, Vec<Diagnostic>> {
        if matches!(self.current().kind, TokenKind::Identifier(_)) {
            Ok(self.advance().clone())
        } else {
            Err(vec![self.error_here("esperava um identificador")])
        }
    }

    fn identifier_value(&self, token: &Token) -> Result<String, Vec<Diagnostic>> {
        match &token.kind {
            TokenKind::Identifier(value) => Ok(value.clone()),
            _ => Err(vec![self.error_at("esperava um identificador", token.span)]),
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index.min(self.tokens.len() - 1)]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.index.saturating_sub(1).min(self.tokens.len() - 1)]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_eof() {
            self.index += 1;
        }
        self.previous()
    }

    fn is_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn token_at(&self, index: usize, predicate: impl FnOnce(&TokenKind) -> bool) -> bool {
        self.tokens
            .get(index)
            .is_some_and(|token| predicate(&token.kind))
    }

    fn id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    fn expr(&mut self, span: SourceSpan, kind: ExprKind) -> Expr {
        Expr {
            id: self.id(),
            span,
            kind,
        }
    }

    fn stmt(&mut self, start: SourceLocation, end: SourceLocation, kind: StmtKind) -> Stmt {
        Stmt {
            id: self.id(),
            span: SourceSpan { start, end },
            kind,
        }
    }

    fn error_here(&self, message: impl Into<String>) -> Diagnostic {
        self.error_at(message, self.current().span)
    }

    fn error_at(&self, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
        Diagnostic::new(DiagnosticKind::Syntax, message, span)
    }

    fn with_parse_depth<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, Vec<Diagnostic>>,
    ) -> Result<T, Vec<Diagnostic>> {
        self.depth += 1;
        if self.depth > self.max_depth {
            self.depth -= 1;
            return Err(vec![
                self.error_here("aninhamento excedeu o limite do analisador"),
            ]);
        }
        let result = parse(self);
        self.depth -= 1;
        result
    }
}

fn string_content_location(
    opening_quote: SourceLocation,
    quote_count: usize,
    prefix: &[char],
) -> SourceLocation {
    let mut location = SourceLocation {
        line: opening_quote.line,
        column: opening_quote.column + quote_count,
        byte_offset: opening_quote.byte_offset + quote_count,
    };
    for character in prefix {
        location.byte_offset += character.len_utf8();
        if *character == '\n' {
            location.line += 1;
            location.column = 1;
        } else {
            location.column += 1;
        }
    }
    location
}

fn shift_span(span: SourceSpan, base: SourceLocation) -> SourceSpan {
    SourceSpan {
        start: shift_location(span.start, base),
        end: shift_location(span.end, base),
    }
}

fn shift_location(location: SourceLocation, base: SourceLocation) -> SourceLocation {
    SourceLocation {
        line: base.line + location.line.saturating_sub(1),
        column: if location.line == 1 {
            base.column + location.column.saturating_sub(1)
        } else {
            location.column
        },
        byte_offset: base.byte_offset + location.byte_offset,
    }
}

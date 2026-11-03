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
        _allow_inline_statements: bool,
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
            statements.push(statement);
        }
    }

    fn parse_statement(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        if self.at_keyword("var") {
            return self.parse_mutable_binding();
        }
        if self.at_keyword("tipo") {
            return self.parse_type_declaration();
        }
        if self.at_keyword("val") {
            return self.parse_immutable_binding();
        }
        if self.at_keyword("use") {
            let start = self.advance().span.start;
            self.skip_newlines();
            let token = self.advance().clone();
            let TokenKind::String(path) = token.kind else {
                return Err(vec![
                    self.error_at("esperava o caminho entre aspas", token.span),
                ]);
            };
            return Ok(self.stmt(
                start,
                token.span.end,
                StmtKind::Import(decode_string_escapes(&path)),
            ));
        }
        if self.at_keyword("faça") || self.at_keyword("faca") {
            return self.parse_standalone_block();
        }
        if self.consume_keyword("def").is_some() {
            return self.parse_function_declaration();
        }
        if self.at_keyword("escreva") || self.at_keyword("imprima") {
            let token = self.advance().clone();
            let newline = matches!(&token.kind, TokenKind::Identifier(word) if word == "escreva");
            let value = self.parse_expression(0)?;
            return Ok(self.stmt(
                token.span.start,
                value.span.end,
                StmtKind::Output { newline, value },
            ));
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

        let (expression, target_count) = self.parse_expression_sequence()?;
        if self.consume_operator(":=").is_some() {
            let targets = match (&expression.kind, target_count) {
                (ExprKind::Tuple(targets), count) if count > 1 => targets.as_slice(),
                (_, 1) => std::slice::from_ref(&expression),
                _ => &[],
            };
            if targets.is_empty()
                || !targets
                    .iter()
                    .all(|target| is_assignment_target(&target.kind))
            {
                return Err(vec![
                    self.error_at("alvo de atribuição inválido", expression.span),
                ]);
            }
            let value = if target_count > 1 {
                self.parse_expression_sequence()?.0
            } else {
                self.parse_expression(0)?
            };
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
        if target_count > 1 {
            return Err(vec![self.error_at(
                "expressão não pode conter uma lista de valores sem parênteses",
                expression.span,
            )]);
        }
        let span = expression.span;
        Ok(self.stmt(span.start, span.end, StmtKind::Expression(expression)))
    }

    fn parse_mutable_binding(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let start = self.advance().span.start;
        let (mut pattern, names) = self.parse_binding_names()?;
        let annotation = self.parse_binding_annotation(&mut pattern)?;
        if self.consume_operator(":=").is_none() && self.consume_operator("=").is_none() {
            return Err(vec![
                self.error_here("esperava ':=' ou '=' após a variável"),
            ]);
        }
        let value = if names > 1 {
            self.parse_expression_sequence()?.0
        } else {
            self.parse_expression(0)?
        };
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
        let start = self
            .consume_keyword("val")
            .map_or(self.current().span.start, |token| token.span.start);
        let (mut pattern, names) = self.parse_binding_names()?;
        let annotation = self.parse_binding_annotation(&mut pattern)?;
        if self.consume_operator("=").is_none() {
            return Err(vec![self.error_here("esperava '=' na declaração de valor")]);
        }
        let value = if names > 1 {
            self.parse_expression_sequence()?.0
        } else {
            self.parse_expression(0)?
        };
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

    fn parse_binding_names(&mut self) -> Result<(Pattern, usize), Vec<Diagnostic>> {
        let mut patterns = Vec::new();
        loop {
            let token = self.expect_identifier()?;
            let name = self.identifier_value(&token)?;
            if name == "_" {
                return Err(vec![
                    self.error_at("esperava um nome para a declaração", token.span),
                ]);
            }
            patterns.push(Pattern {
                id: self.id(),
                span: token.span,
                annotation: None,
                kind: PatternKind::Binding(name),
            });
            if self.consume_symbol(',').is_none() {
                break;
            }
        }
        let count = patterns.len();
        if count == 1 {
            return Ok((patterns.remove(0), count));
        }
        let span = SourceSpan {
            start: patterns[0].span.start,
            end: patterns[count - 1].span.end,
        };
        Ok((
            Pattern {
                id: self.id(),
                span,
                annotation: None,
                kind: PatternKind::Tuple(patterns),
            },
            count,
        ))
    }

    fn parse_binding_annotation(
        &mut self,
        pattern: &mut Pattern,
    ) -> Result<Option<TypeExpr>, Vec<Diagnostic>> {
        self.skip_newlines();
        let Some(annotation) = pattern.annotation.take() else {
            return if self.consume_symbol(':').is_some() {
                Ok(Some(self.parse_type_expression()?))
            } else {
                Ok(None)
            };
        };
        if self.consume_operator("=>").is_none() {
            return Ok(Some(annotation));
        }
        let result = self.parse_type_expression()?;
        let parameters = match annotation.kind {
            TypeExprKind::Tuple(parameters) => parameters,
            kind => vec![TypeExpr {
                id: annotation.id,
                span: annotation.span,
                kind,
            }],
        };
        Ok(Some(TypeExpr {
            id: self.id(),
            span: SourceSpan {
                start: annotation.span.start,
                end: result.span.end,
            },
            kind: TypeExprKind::Function(parameters, Box::new(result)),
        }))
    }

    fn parse_function_declaration(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let declaration = self.parse_function_signature()?;
        let start = declaration.span.start;
        let body = if self.consume_operator("=").is_some() {
            FunctionBody::Expression(Box::new(self.parse_expression(0)?))
        } else {
            let block = self.parse_function_body_block()?;
            self.expect_keyword("fim")?;
            FunctionBody::Block(block)
        };
        let end = match &body {
            FunctionBody::Expression(expression) => expression.span.end,
            FunctionBody::Block(block) => block.span.end,
            FunctionBody::Abstract => declaration.span.end,
        };
        let function = FunctionDecl {
            body,
            span: SourceSpan { start, end },
            ..declaration
        };
        Ok(self.stmt(start, end, StmtKind::Function(function)))
    }

    fn parse_function_body_block(&mut self) -> Result<Block, Vec<Diagnostic>> {
        self.skip_newlines();
        let start = self.current().span.start;
        let mut statements = Vec::new();
        loop {
            self.skip_separators();
            if self.is_eof() || self.at_keyword("fim") {
                break;
            }
            if self.at_keyword("retorne") {
                let return_start = self.advance().span.start;
                self.skip_newlines();
                if self.is_eof() || self.at_keyword("fim") || self.at_symbol(';') {
                    return Err(vec![self.error_here("'retorne' exige uma expressão")]);
                }
                let value = self.parse_expression(0)?;
                let end = value.span.end;
                statements.push(self.stmt(return_start, end, StmtKind::Return(Some(value))));
                self.skip_separators();
                if !self.at_keyword("fim") {
                    return Err(vec![
                        self.error_here("'retorne' deve ser a última expressão da função"),
                    ]);
                }
                break;
            }
            statements.push(self.parse_statement()?);
        }
        let end = self.current().span.start;
        Ok(Block {
            id: self.id(),
            statements,
            span: SourceSpan { start, end },
        })
    }

    fn parse_function_signature(&mut self) -> Result<FunctionDecl, Vec<Diagnostic>> {
        let name_token = self.expect_identifier()?;
        let name = self.identifier_value(&name_token)?;
        self.expect_symbol('(')?;
        let parameters = self.parse_parameters(')', true)?;
        self.skip_newlines();
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
            body: FunctionBody::Expression(Box::new(Expr {
                id: self.id(),
                span: SourceSpan { start: end, end },
                kind: ExprKind::Tuple(Vec::new()),
            })),
            id: self.id(),
            span: SourceSpan {
                start: name_token.span.start,
                end,
            },
        })
    }

    fn parse_type_declaration(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let start = self.advance().span.start;
        self.skip_newlines();
        let abstract_type = self.consume_keyword("abstrato").is_some();
        let name_token = self.expect_identifier()?;
        let name = self.identifier_value(&name_token)?;
        if self.consume_operator("=").is_some() {
            if abstract_type {
                return Err(vec![self.error_here("alias de tipo não pode ser abstrato")]);
            }
            let target = self.parse_type_expression()?;
            let end = target.span.end;
            return Ok(self.stmt(start, end, StmtKind::TypeAlias { name, target }));
        }
        let mut parents = Vec::new();
        if self.consume_symbol(':').is_some() {
            loop {
                let parent = self.expect_identifier()?;
                parents.push(self.identifier_value(&parent)?);
                if self.consume_symbol(',').is_none() {
                    break;
                }
                self.skip_newlines();
            }
        }
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
                self.skip_newlines();
                let body = if self.consume_operator("=").is_some() {
                    FunctionBody::Expression(Box::new(self.parse_expression(0)?))
                } else if abstract_type && self.starts_type_member() {
                    FunctionBody::Abstract
                } else {
                    let block = self.parse_function_body_block()?;
                    self.expect_keyword("fim")?;
                    FunctionBody::Block(block)
                };
                method.span.end = match &body {
                    FunctionBody::Expression(expression) => expression.span.end,
                    FunctionBody::Block(block) => block.span.end,
                    FunctionBody::Abstract => method.span.end,
                };
                method.body = body;
                methods.push(method);
            } else {
                fields.extend(self.parse_type_field()?);
            }
            if self.index == before {
                return Err(vec![self.error_here("declaração de tipo sem progresso")]);
            }
            self.skip_separators();
        }
        let end = self.current().span.start;
        self.expect_keyword("fim")?;
        let declaration = TypeDecl {
            id: self.id(),
            name,
            parents,
            abstract_type,
            fields,
            methods,
            span: SourceSpan { start, end },
        };
        Ok(self.stmt(start, self.previous().span.end, StmtKind::Type(declaration)))
    }

    fn parse_type_field(&mut self) -> Result<Vec<TypeField>, Vec<Diagnostic>> {
        let mutable = self.consume_keyword("var").is_some();
        if !mutable {
            self.consume_keyword("val");
        }
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
                annotation: annotation
                    .as_ref()
                    .map(|annotation| self.type_expr_with_fresh_ids(annotation)),
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
        let expression = self.parse_while_construct(start)?;
        let Expr {
            span: expression_span,
            kind,
            ..
        } = expression;
        let ExprKind::While { condition, body } = kind else {
            unreachable!()
        };
        Ok(self.stmt(
            start,
            expression_span.end,
            StmtKind::While {
                condition: *condition,
                body,
            },
        ))
    }

    fn parse_while_construct(&mut self, start: SourceLocation) -> Result<Expr, Vec<Diagnostic>> {
        let condition = self.parse_expression(0)?;
        self.skip_newlines();
        if self.consume_keyword("faça").is_none() && self.consume_keyword("faca").is_none() {
            return Err(vec![self.error_here("esperava 'faça' após a condição")]);
        }
        self.skip_newlines();
        let body_start = self.current().span.start;
        let statements = self.parse_block_until(&["fim"])?;
        let end = self.expect_keyword("fim")?.span.end;
        let body = Block {
            id: self.id(),
            statements,
            span: SourceSpan {
                start: body_start,
                end: self.previous().span.start,
            },
        };
        Ok(self.expr(
            SourceSpan { start, end },
            ExprKind::While {
                condition: Box::new(condition),
                body,
            },
        ))
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let start = self.advance().span.start;
        let expression = self.parse_for_construct(start)?;
        let Expr { id, span, kind } = expression;
        match kind {
            ExprKind::For { generators, body } => {
                Ok(self.stmt(start, span.end, StmtKind::For { generators, body }))
            }
            kind => Ok(self.stmt(
                span.start,
                span.end,
                StmtKind::Expression(Expr { id, span, kind }),
            )),
        }
    }

    fn parse_for_construct(&mut self, start: SourceLocation) -> Result<Expr, Vec<Diagnostic>> {
        let generators = self.parse_generators_until(&["gere", "faça", "faca"])?;
        if self.consume_keyword("gere").is_some() {
            self.skip_newlines();
            let body_start = self.current().span.start;
            let statements = self.parse_block_until(&["fim"])?;
            let body_end = self.current().span.start;
            let end = self.expect_keyword("fim")?.span.end;
            let body = Block {
                id: self.id(),
                statements,
                span: SourceSpan {
                    start: body_start,
                    end: body_end,
                },
            };
            return Ok(self.expr(
                SourceSpan { start, end },
                ExprKind::Comprehension { generators, body },
            ));
        }
        if self.consume_keyword("faça").is_none() && self.consume_keyword("faca").is_none() {
            return Err(vec![
                self.error_here("esperava 'faça' após os limites do laço"),
            ]);
        }
        self.skip_newlines();
        let body_start = self.current().span.start;
        let statements = self.parse_block_until(&["fim"])?;
        let body_end = self.current().span.start;
        let end = self.expect_keyword("fim")?.span.end;
        let body = Block {
            id: self.id(),
            statements,
            span: SourceSpan {
                start: body_start,
                end: body_end,
            },
        };
        Ok(self.expr(
            SourceSpan { start, end },
            ExprKind::For { generators, body },
        ))
    }

    fn parse_standalone_block(&mut self) -> Result<Stmt, Vec<Diagnostic>> {
        let start = self.advance().span.start;
        self.skip_newlines();
        let body_start = self.current().span.start;
        let statements = self.parse_block_until(&["fim"])?;
        let body_end = self.current().span.start;
        let end = self.expect_keyword("fim")?.span.end;
        let block = Block {
            id: self.id(),
            statements,
            span: SourceSpan {
                start: body_start,
                end: body_end,
            },
        };
        Ok(self.stmt(start, end, StmtKind::Block(block)))
    }

    fn parse_generators_until(
        &mut self,
        endings: &[&str],
    ) -> Result<Vec<ForGenerator>, Vec<Diagnostic>> {
        let mut generators = Vec::new();
        loop {
            self.skip_newlines();
            let start = self.current().span.start;
            let pattern = self.parse_pattern()?;
            self.skip_newlines();
            let source = if self.consume_keyword("de").is_some() {
                let range_start = self.parse_expression(0)?;
                self.skip_newlines();
                if self.consume_keyword("até").is_none() && self.consume_keyword("ate").is_none() {
                    return Err(vec![self.error_here("esperava 'até' no gerador do laço")]);
                }
                let end = self.parse_expression(0)?;
                self.skip_newlines();
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
            let end = match &source {
                ForSource::Range { step, end, .. } => {
                    step.as_ref().map_or(end.span.end, |step| step.span.end)
                }
                ForSource::Iterable(iterable) => iterable.span.end,
            };
            generators.push(ForGenerator {
                id: self.id(),
                pattern,
                source,
                condition: None,
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
        self.skip_newlines();
        if self.consume_keyword("se").is_some() {
            let condition = self.parse_expression(0)?;
            let Some(generator) = generators.last_mut() else {
                return Err(vec![self.error_here("filtro sem gerador")]);
            };
            generator.span.end = condition.span.end;
            generator.condition = Some(condition);
        }
        Ok(generators)
    }

    fn parse_expression(&mut self, minimum_precedence: u8) -> Result<Expr, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| parser.parse_expression_inner(minimum_precedence))
    }

    fn parse_expression_list(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        Ok(self.parse_expression_sequence()?.0)
    }

    fn parse_expression_sequence(&mut self) -> Result<(Expr, usize), Vec<Diagnostic>> {
        let first = self.parse_expression(0)?;
        let start = first.span.start;
        let mut elements = vec![first];
        loop {
            self.skip_newlines();
            if self.consume_symbol(',').is_none() {
                break;
            }
            self.skip_newlines();
            elements.push(self.parse_expression(0)?);
        }
        let count = elements.len();
        let end = elements.last().map_or(start, |element| element.span.end);
        let expression = if count == 1 {
            elements.remove(0)
        } else {
            self.expr(SourceSpan { start, end }, ExprKind::Tuple(elements))
        };
        Ok((expression, count))
    }

    fn parse_expression_inner(&mut self, minimum_precedence: u8) -> Result<Expr, Vec<Diagnostic>> {
        let mut left = self.parse_prefix_expression()?;
        let mut operator_depth = 0usize;
        loop {
            self.skip_newlines_before_continuation();
            if let Some(postfix) = self.parse_postfix(&left)? {
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
            operator_depth += 1;
            if self.depth.saturating_add(operator_depth) > self.max_depth {
                return Err(vec![
                    self.error_here("aninhamento excedeu o limite do analisador"),
                ]);
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
            let body = self.parse_lambda_body(span_start)?;
            let span = SourceSpan {
                start: span_start,
                end: match &body {
                    FunctionBody::Expression(expression) => expression.span.end,
                    FunctionBody::Block(block) => block.span.end,
                    FunctionBody::Abstract => span_start,
                },
            };
            left = self.expr(span, ExprKind::Lambda { parameters, body });
        }
        Ok(left)
    }

    fn parse_prefix_expression(&mut self) -> Result<Expr, Vec<Diagnostic>> {
        self.skip_newlines();
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Integer(value) => i32::try_from(value)
                .map(|value| self.expr(token.span, ExprKind::Integer(value)))
                .map_err(|_| vec![self.error_at("inteiro fora do limite", token.span)]),
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
            TokenKind::Identifier(word) if word == "isto" => {
                Ok(self.expr(token.span, ExprKind::This))
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
            TokenKind::Identifier(word) if word == "enquanto" => {
                self.parse_while_construct(token.span.start)
            }
            TokenKind::Identifier(word) if matches!(word.as_str(), "faça" | "faca") => {
                self.parse_block_expression(token.span.start)
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
            TokenKind::Identifier(word) if is_reserved_keyword(&word) => {
                Err(vec![self.error_at("esperava uma expressão", token.span)])
            }
            TokenKind::Operator(operator) if operator == "-" => {
                if matches!(
                    self.current().kind,
                    TokenKind::Integer(value) if value == i64::from(i32::MAX) + 1
                ) {
                    let literal = self.advance().clone();
                    return Ok(self.expr(
                        SourceSpan {
                            start: token.span.start,
                            end: literal.span.end,
                        },
                        ExprKind::Integer(i32::MIN),
                    ));
                }
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
            TokenKind::Operator(operator) if operator == "+" => {
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
            return Err(vec![
                self.error_here("tupla precisa de ao menos dois elementos"),
            ]);
        }
        if self.looks_like_lambda_parameters() {
            let parameters = self.parse_parameters(')', false)?;
            self.expect_symbol(')')?;
            self.skip_newlines();
            self.expect_operator("=>")?;
            let body = self.parse_lambda_body(start)?;
            let end = match &body {
                FunctionBody::Expression(expression) => expression.span.end,
                FunctionBody::Block(block) => block.span.end,
                FunctionBody::Abstract => start,
            };
            let span = SourceSpan { start, end };
            return Ok(self.expr(span, ExprKind::Lambda { parameters, body }));
        }
        let first = self.parse_expression(0)?;
        if self.consume_symbol(',').is_none() {
            self.skip_newlines();
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
        if elements.len() < 2 {
            return Err(vec![
                self.error_here("tupla precisa de ao menos dois elementos"),
            ]);
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

    fn parse_postfix(&mut self, expression: &Expr) -> Result<Option<Expr>, Vec<Diagnostic>> {
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
                    callee: Box::new(expression.clone()),
                    arguments,
                },
            )));
        }
        if self.consume_symbol('[').is_some() {
            let index = self.parse_expression(0)?;
            let end = self.expect_symbol(']')?.span.end;
            return Ok(Some(self.expr(
                SourceSpan {
                    start: expression.span.start,
                    end,
                },
                ExprKind::Index {
                    receiver: Box::new(expression.clone()),
                    indices: vec![index],
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
                    receiver: Box::new(expression.clone()),
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
        self.consume_keyword("então")
            .or_else(|| self.consume_keyword("entao"));
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
            self.skip_newlines();
            let guard = if self.consume_keyword("se").is_some() {
                Some(self.parse_expression(1)?)
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
        self.parse_for_construct(start)
    }

    fn parse_block_expression(&mut self, start: SourceLocation) -> Result<Expr, Vec<Diagnostic>> {
        self.skip_newlines();
        let body_start = self.current().span.start;
        let statements = self.parse_block_until(&["fim"])?;
        let body_end = self.current().span.start;
        let end = self.expect_keyword("fim")?.span.end;
        let block = Block {
            id: self.id(),
            statements,
            span: SourceSpan {
                start: body_start,
                end: body_end,
            },
        };
        Ok(self.expr(SourceSpan { start, end }, ExprKind::Block(block)))
    }

    fn parse_lambda_body(
        &mut self,
        start: SourceLocation,
    ) -> Result<FunctionBody, Vec<Diagnostic>> {
        let mut body_index = self.index;
        while matches!(
            self.tokens.get(body_index).map(|token| &token.kind),
            Some(TokenKind::Newline)
        ) {
            body_index += 1;
        }
        let line_column = self.tokens[..self.index]
            .iter()
            .rev()
            .take_while(|token| token.span.start.line == start.line)
            .last()
            .map_or(start.column, |token| token.span.start.column);
        if let Some(body) = self.tokens.get(body_index)
            && body.span.start.line > start.line
            && body.span.start.column > line_column
        {
            self.skip_newlines();
            let body_start = self.current().span.start;
            let mut statements = Vec::new();
            loop {
                let before = self.index;
                statements.push(self.parse_statement()?);
                if self.index == before {
                    return Err(vec![self.error_here("corpo de lambda sem progresso")]);
                }
                if !self.at_newline() {
                    continue;
                }
                self.skip_newlines();
                if self.is_eof() || self.current().span.start.column <= line_column {
                    break;
                }
            }
            let end = statements.last().map_or(body_start, |stmt| stmt.span.end);
            return Ok(FunctionBody::Block(Block {
                id: self.id(),
                statements,
                span: SourceSpan {
                    start: body_start,
                    end,
                },
            }));
        }
        self.skip_newlines();
        Ok(FunctionBody::Expression(Box::new(
            self.parse_expression(0)?,
        )))
    }

    fn parse_pattern(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| parser.parse_pattern_inner())
    }

    fn parse_pattern_inner(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        self.skip_newlines();
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
                annotation: None,
                kind: PatternKind::Tuple(elements),
            };
        }
        Ok(pattern)
    }

    fn parse_cons_pattern(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| parser.parse_cons_pattern_inner())
    }

    fn parse_cons_pattern_inner(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        self.skip_newlines();
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
                annotation: None,
                kind: PatternKind::Cons(Box::new(pattern), Box::new(tail)),
            };
        }
        if self.consume_symbol(':').is_some() {
            let annotation = self.parse_atomic_type()?;
            pattern.span.end = annotation.span.end;
            pattern.annotation = Some(annotation);
        }
        Ok(pattern)
    }

    fn parse_atomic_pattern(&mut self) -> Result<Pattern, Vec<Diagnostic>> {
        let token = self.advance().clone();
        let kind = match token.kind {
            TokenKind::Integer(value) => match i32::try_from(value) {
                Ok(value) => PatternKind::Integer(value),
                Err(_) => return Err(vec![self.error_at("inteiro fora do limite", token.span)]),
            },
            TokenKind::Real(value) => PatternKind::Real(value),
            TokenKind::Character(value) => PatternKind::Character(value),
            TokenKind::String(value) => PatternKind::Text(decode_string_escapes(&value)),
            TokenKind::Identifier(name) if name == "_" => PatternKind::Wildcard,
            TokenKind::Identifier(name) if name == "verdadeiro" => PatternKind::Logical(true),
            TokenKind::Identifier(name) if name == "falso" => PatternKind::Logical(false),
            TokenKind::Identifier(name) if is_reserved_keyword(&name) => {
                return Err(vec![self.error_at("esperava um padrão", token.span)]);
            }
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
                        annotation: None,
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
                    annotation: None,
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
                        annotation: None,
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
            annotation: None,
            kind,
        })
    }

    fn parse_type_expression(&mut self) -> Result<TypeExpr, Vec<Diagnostic>> {
        self.with_parse_depth(|parser| parser.parse_type_expression_inner())
    }

    fn parse_type_expression_inner(&mut self) -> Result<TypeExpr, Vec<Diagnostic>> {
        let mut left = self.parse_atomic_type()?;
        if self.consume_operator("=>").is_some() {
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
            let mut elements = vec![self.parse_type_expression()?];
            if self.consume_symbol(',').is_none() {
                return Err(vec![
                    self.error_here("tupla de tipos precisa de ao menos dois elementos"),
                ]);
            }
            loop {
                elements.push(self.parse_type_expression()?);
                if self.consume_symbol(',').is_none() {
                    break;
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
            arguments.push(self.parse_type_expression()?);
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

    fn parse_parameters(
        &mut self,
        closing: char,
        require_annotations: bool,
    ) -> Result<Vec<Parameter>, Vec<Diagnostic>> {
        let mut parameters = Vec::new();
        let mut pending = Vec::<(String, SourceSpan, bool)>::new();
        self.skip_newlines();
        if self.at_symbol(closing) {
            return Ok(parameters);
        }
        loop {
            self.skip_newlines();
            let mutable = self.consume_keyword("var").is_some();
            if !mutable {
                self.consume_keyword("val");
            }
            let token = self.expect_identifier()?;
            let name = self.identifier_value(&token)?;
            pending.push((name, token.span, mutable));
            self.skip_newlines();
            if self.consume_symbol(':').is_some() {
                let annotation = self.parse_type_expression()?;
                for (name, span, mutable) in pending.drain(..) {
                    let annotation = self.type_expr_with_fresh_ids(&annotation);
                    parameters.push(Parameter {
                        id: self.id(),
                        name,
                        annotation: Some(annotation.clone()),
                        mutable,
                        span: SourceSpan {
                            start: span.start,
                            end: annotation.span.end,
                        },
                    });
                }
            }
            if self.consume_symbol(',').is_none() {
                break;
            }
            self.skip_newlines();
        }
        if require_annotations && !pending.is_empty() {
            return Err(vec![self.error_here("parâmetro de função precisa de tipo")]);
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
                text.push(chars[index + 1]);
                index += 2;
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
                        index += 2;
                        continue;
                    } else if character == delimiter {
                        quote = None;
                    }
                } else {
                    match character {
                        '"' | '\'' => quote = Some(character),
                        '#' => {
                            while index < chars.len() && chars[index] != '\n' {
                                index += 1;
                            }
                            continue;
                        }
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
            let base = string_content_location(token_span.start, 1, &chars[..expression_start]);
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
        parser.depth = self.depth;
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
                "::" => (operator.as_str(), 7, true),
                "+" | "-" => (operator.as_str(), 5, false),
                "*" | "/" => (operator.as_str(), 6, false),
                "^" => (operator.as_str(), 8, true),
                _ => return None,
            },
            TokenKind::Identifier(word) => match word.as_str() {
                "ou" => (word.as_str(), 1, false),
                "e" => (word.as_str(), 2, false),
                "mod" | "div" => (word.as_str(), 6, false),
                "formato" => (word.as_str(), 4, false),
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
        if self.token_at(index, |kind| {
            matches!(kind, TokenKind::Identifier(word) if matches!(word.as_str(), "fim" | "caso" | "senão" | "senao"))
        }) {
            return false;
        }
        if self.token_at(index, |kind| matches!(kind, TokenKind::Identifier(_)))
            || self.token_at(index, |kind| {
                matches!(
                    kind,
                    TokenKind::Integer(_)
                        | TokenKind::Real(_)
                        | TokenKind::String(_)
                        | TokenKind::Character(_)
                        | TokenKind::Symbol('(' | '[')
                )
            })
        {
            return self.tokens[index..]
                .iter()
                .any(|token| matches!(&token.kind, TokenKind::Identifier(word) if word == "fim"));
        }
        false
    }

    fn looks_like_immutable_binding(&self) -> bool {
        if let TokenKind::Identifier(word) = &self.current().kind
            && is_reserved_keyword(word)
        {
            return false;
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
                        let mut arrow = index + 1;
                        while self.token_at(arrow, |kind| matches!(kind, TokenKind::Newline)) {
                            arrow += 1;
                        }
                        return self.token_at(arrow, |kind| {
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

    fn starts_type_member(&self) -> bool {
        let mut index = self.index;
        while matches!(
            self.tokens.get(index).map(|token| &token.kind),
            Some(TokenKind::Newline)
        ) {
            index += 1;
        }
        if self.token_at(
            index,
            |kind| matches!(kind, TokenKind::Identifier(word) if word == "fim"),
        ) {
            return true;
        }
        if self.token_at(index, |kind| {
            matches!(kind, TokenKind::Identifier(word) if matches!(word.as_str(), "def" | "val" | "var"))
        }) {
            index += 1;
        }
        if !self.token_at(index, |kind| matches!(kind, TokenKind::Identifier(_))) {
            return false;
        }
        index += 1;
        loop {
            while matches!(
                self.tokens.get(index).map(|token| &token.kind),
                Some(TokenKind::Newline)
            ) {
                index += 1;
            }
            if !self.token_at(index, |kind| matches!(kind, TokenKind::Symbol(','))) {
                break;
            }
            index += 1;
            while matches!(
                self.tokens.get(index).map(|token| &token.kind),
                Some(TokenKind::Newline)
            ) {
                index += 1;
            }
            if !self.token_at(index, |kind| matches!(kind, TokenKind::Identifier(_))) {
                return false;
            }
            index += 1;
        }
        if self.token_at(index, |kind| matches!(kind, TokenKind::Symbol(':'))) {
            return true;
        }
        if !self.token_at(index, |kind| matches!(kind, TokenKind::Symbol('('))) {
            return false;
        }
        let mut depth = 0usize;
        while index < self.tokens.len() {
            match self.tokens[index].kind {
                TokenKind::Symbol('(') => depth += 1,
                TokenKind::Symbol(')') => {
                    depth -= 1;
                    if depth == 0 {
                        index += 1;
                        break;
                    }
                }
                TokenKind::Eof => return false,
                _ => {}
            }
            index += 1;
        }
        while matches!(
            self.tokens.get(index).map(|token| &token.kind),
            Some(TokenKind::Newline)
        ) {
            index += 1;
        }
        self.token_at(index, |kind| {
            matches!(kind, TokenKind::Symbol(':'))
                || matches!(kind, TokenKind::Operator(operator) if operator == "=")
        })
    }

    fn skip_newlines_before_continuation(&mut self) {
        let mut next = self.index;
        while matches!(
            self.tokens.get(next).map(|token| &token.kind),
            Some(TokenKind::Newline)
        ) {
            next += 1;
        }
        if self.token_at(next, |kind| {
            matches!(kind, TokenKind::Operator(operator) if matches!(operator.as_str(), "+" | "-" | "*" | "/" | "^" | "==" | "<>" | "<" | "<=" | ">" | ">=" | "::"))
                || matches!(kind, TokenKind::Symbol('.' | '(' | '['))
                || matches!(kind, TokenKind::Identifier(word) if matches!(word.as_str(), "e" | "ou" | "div" | "mod" | "formato"))
        }) {
            self.skip_newlines();
        }
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
        self.skip_newlines();
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
        self.skip_newlines();
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
        self.skip_newlines();
        if let TokenKind::Identifier(word) = &self.current().kind
            && !is_reserved_keyword(word)
        {
            return Ok(self.advance().clone());
        }
        Err(vec![self.error_here("esperava um identificador")])
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

    fn type_expr_with_fresh_ids(&mut self, expression: &TypeExpr) -> TypeExpr {
        let kind = match &expression.kind {
            TypeExprKind::Named(name, arguments) => TypeExprKind::Named(
                name.clone(),
                arguments
                    .iter()
                    .map(|argument| self.type_expr_with_fresh_ids(argument))
                    .collect(),
            ),
            TypeExprKind::Tuple(elements) => TypeExprKind::Tuple(
                elements
                    .iter()
                    .map(|element| self.type_expr_with_fresh_ids(element))
                    .collect(),
            ),
            TypeExprKind::Function(parameters, result) => TypeExprKind::Function(
                parameters
                    .iter()
                    .map(|parameter| self.type_expr_with_fresh_ids(parameter))
                    .collect(),
                Box::new(self.type_expr_with_fresh_ids(result)),
            ),
        };
        TypeExpr {
            id: self.id(),
            span: expression.span,
            kind,
        }
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

fn is_assignment_target(kind: &ExprKind) -> bool {
    match kind {
        ExprKind::Identifier(_) | ExprKind::Member { .. } => is_qualified_identifier(kind),
        ExprKind::Index { receiver, indices } => {
            indices.len() == 1
                && (is_qualified_identifier(&receiver.kind) || is_assignment_target(&receiver.kind))
        }
        _ => false,
    }
}

fn is_qualified_identifier(kind: &ExprKind) -> bool {
    match kind {
        ExprKind::Identifier(_) => true,
        ExprKind::Member { receiver, .. } => is_qualified_identifier(&receiver.kind),
        _ => false,
    }
}

fn decode_string_escapes(raw: &str) -> String {
    let mut result = String::with_capacity(raw.len());
    let mut characters = raw.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            if let Some(escaped) = characters.next() {
                result.push(escaped);
            }
        } else {
            result.push(character);
        }
    }
    result
}

fn is_reserved_keyword(word: &str) -> bool {
    matches!(
        word,
        "abstrato"
            | "até"
            | "ate"
            | "caso"
            | "de"
            | "def"
            | "div"
            | "e"
            | "em"
            | "enquanto"
            | "então"
            | "entao"
            | "escolha"
            | "escreva"
            | "faca"
            | "faça"
            | "falso"
            | "fim"
            | "formato"
            | "gere"
            | "imprima"
            | "mod"
            | "nao"
            | "não"
            | "ou"
            | "para"
            | "passo"
            | "retorne"
            | "se"
            | "senao"
            | "senaose"
            | "senão"
            | "senãose"
            | "tipo"
            | "isto"
            | "use"
            | "val"
            | "var"
            | "verdadeiro"
    )
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

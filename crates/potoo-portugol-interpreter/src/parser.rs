use std::mem::discriminant;

use crate::{Diagnostic, DiagnosticKind, ast::*, lexer::Token, lexer::TokenKind};

const MAX_STATEMENT_NESTING: usize = 128;
const MAX_EXPRESSION_NESTING: usize = 128;

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    statement_depth: usize,
    expression_depth: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            statement_depth: 0,
            expression_depth: 0,
        }
    }

    pub fn parse(mut self) -> Result<Program, Vec<Diagnostic>> {
        self.skip_terminators();
        self.consume_word("programa", "o programa deve começar com 'programa'")?;
        self.skip_newlines();
        self.consume(TokenKind::LeftBrace, "esperado '{' após 'programa'")?;

        let mut libraries = Vec::new();
        let mut functions = Vec::new();
        let mut statements = Vec::new();
        let mut program_content_started = false;
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            if self.check_word("inclua") {
                if program_content_started {
                    return Err(
                        self.error("as bibliotecas devem ser incluídas antes das declarações")
                    );
                }
                libraries.push(self.include()?);
            } else if self.check_word("funcao") {
                program_content_started = true;
                functions.push(self.function()?);
            } else if self.check_word("const") || self.is_type() {
                program_content_started = true;
                let is_constant = self.matches_word("const");
                statements.push(self.variable_declaration(is_constant)?);
            } else {
                return Err(self.error("esperada uma função ou declaração no programa"));
            }
        }
        self.consume(TokenKind::RightBrace, "esperado '}' para fechar o programa")?;
        self.skip_newlines();
        if !self.is_at_end() {
            return Err(self.error("há código depois do fim do programa"));
        }
        Ok(Program {
            libraries,
            functions,
            statements,
        })
    }

    fn function(&mut self) -> Result<Function, Vec<Diagnostic>> {
        let start = self.advance().clone();
        let return_type = if self.matches_word("vazio") {
            None
        } else if self.is_type() {
            Some(self.parse_type()?)
        } else {
            None
        };
        let name = self.consume_identifier("esperado o nome da função")?;
        self.consume(TokenKind::LeftParen, "esperado '(' após o nome da função")?;
        self.skip_newlines();
        let mut parameters = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                let ty = self.parse_type()?;
                let by_reference = self.matches(&TokenKind::Ampersand);
                let name = self.consume_identifier("esperado o nome do parâmetro")?;
                let mut dimensions = 0;
                while self.matches(&TokenKind::LeftBracket) {
                    self.consume(
                        TokenKind::RightBracket,
                        "parâmetros vetoriais exigem dimensões vazias",
                    )?;
                    dimensions += 1;
                    if dimensions > 2 {
                        return Err(self.error("vetores podem ter no máximo duas dimensões"));
                    }
                }
                parameters.push(Parameter {
                    ty,
                    name,
                    by_reference,
                    dimensions,
                });
                self.skip_newlines();
                if !self.matches(&TokenKind::Comma) {
                    break;
                }
                self.skip_newlines();
            }
        }
        self.consume(TokenKind::RightParen, "esperado ')' após os parâmetros")?;
        let body_token = self.peek().clone();
        if !self.matches(&TokenKind::LeftBrace) {
            return Err(self.error("o corpo da função deve ser delimitado por chaves"));
        }
        let body = self.block_statement(body_token)?;
        Ok(Function {
            name,
            return_type,
            parameters,
            body,
            line: start.line,
            column: start.column,
        })
    }

    fn include(&mut self) -> Result<Library, Vec<Diagnostic>> {
        self.advance();
        self.consume_word("biblioteca", "esperado 'biblioteca' após 'inclua'")?;
        let name = self.consume_identifier("esperado o nome da biblioteca")?;
        let alias = if self.matches(&TokenKind::Arrow) {
            Some(self.consume_identifier("esperado um apelido para a biblioteca")?)
        } else {
            None
        };
        Ok(Library { name, alias })
    }

    fn statement(&mut self) -> Result<Statement, Vec<Diagnostic>> {
        if self.statement_depth >= MAX_STATEMENT_NESTING {
            return Err(self.error("programa excede o limite de blocos aninhados"));
        }
        self.statement_depth += 1;
        let result = self.statement_inner();
        self.statement_depth -= 1;
        result
    }

    fn statement_inner(&mut self) -> Result<Statement, Vec<Diagnostic>> {
        let token = self.peek().clone();
        if self.check_word("se") {
            self.advance();
            return self.if_statement(token);
        }
        if self.matches_word("enquanto") {
            let condition = self.condition()?;
            let body = self.block_or_statement()?;
            return Ok(Statement {
                kind: StatementKind::While {
                    condition,
                    body: Box::new(body),
                },
                line: token.line,
                column: token.column,
            });
        }
        if self.matches_word("faca") {
            let body = self.block_or_statement()?;
            self.skip_newlines();
            self.consume_word("enquanto", "esperado 'enquanto' após o bloco 'faca'")?;
            let condition = self.condition()?;
            return Ok(Statement {
                kind: StatementKind::DoWhile {
                    body: Box::new(body),
                    condition,
                },
                line: token.line,
                column: token.column,
            });
        }
        if self.matches_word("para") {
            return self.for_statement(token);
        }
        if self.matches_word("escolha") {
            return self.switch_statement(token);
        }
        if self.matches_word("retorne") {
            let value = if self.at_statement_end() {
                None
            } else {
                Some(self.expression(0)?)
            };
            return Ok(Statement {
                kind: StatementKind::Return(value),
                line: token.line,
                column: token.column,
            });
        }
        if self.matches_word("pare") {
            return Ok(Statement {
                kind: StatementKind::Break,
                line: token.line,
                column: token.column,
            });
        }
        if self.matches_word("const") {
            return self.variable_declaration(true);
        }
        if self.is_type() {
            return self.variable_declaration(false);
        }
        let expression = self.assignment_or_expression()?;
        Ok(Statement {
            kind: StatementKind::Expression(expression),
            line: token.line,
            column: token.column,
        })
    }

    fn block_statement(&mut self, token: Token) -> Result<Statement, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            statements.push(self.statement()?);
        }
        self.consume(TokenKind::RightBrace, "esperado '}' para fechar o bloco")?;
        Ok(Statement {
            kind: StatementKind::Block(statements),
            line: token.line,
            column: token.column,
        })
    }

    fn if_statement(&mut self, token: Token) -> Result<Statement, Vec<Diagnostic>> {
        let condition = self.condition()?;
        let then_branch = self.block_or_statement()?;
        let separator_position = self.current;
        self.skip_newlines();
        let else_branch = if self.matches_word("senao") {
            self.skip_newlines();
            if self.matches_word("se") {
                self.current -= 1;
                Some(Box::new(self.statement()?))
            } else {
                Some(Box::new(self.block_or_statement()?))
            }
        } else {
            self.current = separator_position;
            None
        };
        Ok(Statement {
            kind: StatementKind::If {
                condition,
                then_branch: Box::new(then_branch),
                else_branch,
            },
            line: token.line,
            column: token.column,
        })
    }

    fn for_statement(&mut self, token: Token) -> Result<Statement, Vec<Diagnostic>> {
        self.consume(TokenKind::LeftParen, "esperado '(' após 'para'")?;
        self.skip_newlines();
        let initializer = if self.matches(&TokenKind::Semicolon) {
            None
        } else {
            let initializer = if self.matches_word("const") {
                self.variable_declaration(true)?
            } else if self.is_type() {
                self.variable_declaration(false)?
            } else {
                let expression = self.assignment_or_expression()?;
                if !matches!(
                    expression.kind,
                    ExpressionKind::Variable(_)
                        | ExpressionKind::Assign {
                            operator: AssignmentOperator::Assign,
                            ..
                        }
                ) {
                    return Err(self.error("inicialização inválida no cabeçalho do 'para'"));
                }
                Statement {
                    kind: StatementKind::Expression(expression),
                    line: token.line,
                    column: token.column,
                }
            };
            self.consume(TokenKind::Semicolon, "esperado ';' após o início do 'para'")?;
            Some(Box::new(initializer))
        };

        self.skip_newlines();
        let condition = Some(self.expression(0)?);
        self.consume(
            TokenKind::Semicolon,
            "esperado ';' após a condição do 'para'",
        )?;

        self.skip_newlines();
        let increment = Some(self.assignment_or_expression()?);
        self.consume(
            TokenKind::RightParen,
            "esperado ')' após o cabeçalho do 'para'",
        )?;
        let body = self.block_or_statement()?;
        Ok(Statement {
            kind: StatementKind::For {
                initializer,
                condition,
                increment,
                body: Box::new(body),
            },
            line: token.line,
            column: token.column,
        })
    }

    fn switch_statement(&mut self, token: Token) -> Result<Statement, Vec<Diagnostic>> {
        let expression = self.condition()?;
        self.skip_newlines();
        self.consume(
            TokenKind::LeftBrace,
            "esperado '{' após a expressão de 'escolha'",
        )?;
        let mut cases = Vec::new();
        self.skip_terminators();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            self.consume_word("caso", "esperado 'caso' dentro de 'escolha'")?;
            self.skip_newlines();
            if self.matches_word("contrario") {
                self.consume_case_separator()?;
                cases.push(SwitchCase {
                    value: None,
                    body: self.case_body()?,
                });
            } else {
                let value = self.expression(0)?;
                self.consume_case_separator()?;
                let body = self.case_body()?;
                cases.push(SwitchCase {
                    value: Some(value),
                    body,
                });
            }
            self.skip_terminators();
        }
        self.consume(TokenKind::RightBrace, "esperado '}' para fechar 'escolha'")?;
        Ok(Statement {
            kind: StatementKind::Switch { expression, cases },
            line: token.line,
            column: token.column,
        })
    }

    fn case_body(&mut self) -> Result<Vec<Statement>, Vec<Diagnostic>> {
        if self.matches(&TokenKind::LeftBrace) {
            let mut statements = Vec::new();
            self.skip_terminators();
            while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                statements.push(self.statement()?);
                self.consume_statement_end()?;
                self.skip_terminators();
            }
            self.consume(TokenKind::RightBrace, "esperado '}' para fechar o caso")?;
            self.skip_newlines();
            if self.matches_word("pare") {
                let token = self.previous();
                statements.push(Statement {
                    kind: StatementKind::Break,
                    line: token.line,
                    column: token.column,
                });
            }
            return Ok(statements);
        }

        let mut statements = Vec::new();
        self.skip_terminators();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() && !self.check_word("caso") {
            statements.push(self.statement()?);
            self.consume_statement_end()?;
            self.skip_terminators();
        }
        Ok(statements)
    }

    fn consume_case_separator(&mut self) -> Result<(), Vec<Diagnostic>> {
        self.consume(TokenKind::Colon, "esperado ':' após o rótulo do caso")
    }

    fn variable_declaration(&mut self, is_constant: bool) -> Result<Statement, Vec<Diagnostic>> {
        let token = self.peek().clone();
        let ty = self.parse_type()?;
        let mut variables = Vec::new();
        loop {
            let name = self.consume_identifier("esperado o nome da variável")?;
            let mut dimensions = Vec::new();
            while self.matches(&TokenKind::LeftBracket) {
                if dimensions.len() == 2 {
                    return Err(self.error("vetores podem ter no máximo duas dimensões"));
                }
                self.skip_newlines();
                if self.matches(&TokenKind::RightBracket) {
                    dimensions.push(None);
                    continue;
                }
                dimensions.push(Some(self.expression(0)?));
                self.consume(
                    TokenKind::RightBracket,
                    "esperado ']' após o tamanho do vetor",
                )?;
            }
            let initializer = if self.matches(&TokenKind::Equal) {
                Some(match dimensions.len() {
                    0 => self.expression(0)?,
                    1 => self.array_initializer()?,
                    2 => self.matrix_initializer()?,
                    _ => unreachable!("array dimensions checked above"),
                })
            } else {
                None
            };
            variables.push(Variable {
                ty,
                name,
                is_constant,
                dimensions,
                initializer,
            });
            if !self.matches(&TokenKind::Comma) {
                break;
            }
        }
        Ok(Statement {
            kind: StatementKind::Variables(variables),
            line: token.line,
            column: token.column,
        })
    }

    fn array_initializer(&mut self) -> Result<Expression, Vec<Diagnostic>> {
        let token = self.peek().clone();
        self.consume(
            TokenKind::LeftBrace,
            "esperado '{' para inicializar um vetor",
        )?;
        let mut values = Vec::new();
        if !self.check(&TokenKind::RightBrace) {
            loop {
                values.push(self.assignment_or_expression()?);
                if !self.matches(&TokenKind::Comma) {
                    break;
                }
            }
        }
        self.consume(
            TokenKind::RightBrace,
            "esperado '}' após os valores do vetor",
        )?;
        Ok(Expression {
            kind: ExpressionKind::ArrayLiteral(values),
            line: token.line,
            column: token.column,
        })
    }

    fn matrix_initializer(&mut self) -> Result<Expression, Vec<Diagnostic>> {
        let token = self.peek().clone();
        self.consume(
            TokenKind::LeftBrace,
            "esperado '{' para inicializar uma matriz",
        )?;
        let mut rows = Vec::new();
        loop {
            rows.push(self.array_initializer()?);
            if !self.matches(&TokenKind::Comma) {
                break;
            }
        }
        self.consume(
            TokenKind::RightBrace,
            "esperado '}' após as linhas da matriz",
        )?;
        Ok(Expression {
            kind: ExpressionKind::ArrayLiteral(rows),
            line: token.line,
            column: token.column,
        })
    }

    fn condition(&mut self) -> Result<Expression, Vec<Diagnostic>> {
        self.skip_newlines();
        self.consume(TokenKind::LeftParen, "esperado '(' antes da condição")?;
        self.skip_newlines();
        let expression = self.expression(0)?;
        self.skip_newlines();
        self.consume(TokenKind::RightParen, "esperado ')' após a condição")?;
        Ok(expression)
    }

    fn block_or_statement(&mut self) -> Result<Statement, Vec<Diagnostic>> {
        self.skip_newlines();
        let token = self.peek().clone();
        if self.matches(&TokenKind::LeftBrace) {
            self.block_statement(token)
        } else {
            self.statement()
        }
    }

    fn expression(&mut self, minimum_precedence: u8) -> Result<Expression, Vec<Diagnostic>> {
        if self.expression_depth >= MAX_EXPRESSION_NESTING {
            return Err(self.error("expressão excede o limite de aninhamento"));
        }
        self.expression_depth += 1;
        let result = self.expression_inner(minimum_precedence);
        self.expression_depth -= 1;
        result
    }

    fn assignment_or_expression(&mut self) -> Result<Expression, Vec<Diagnostic>> {
        let target = self.expression(0)?;
        let operator = match &self.peek().kind {
            TokenKind::Equal => Some(AssignmentOperator::Assign),
            TokenKind::PlusEqual => Some(AssignmentOperator::Add),
            TokenKind::MinusEqual => Some(AssignmentOperator::Subtract),
            TokenKind::StarEqual => Some(AssignmentOperator::Multiply),
            TokenKind::SlashEqual => Some(AssignmentOperator::Divide),
            _ => None,
        };
        let Some(operator) = operator else {
            return Ok(target);
        };
        self.advance();
        let value = self.expression(0)?;
        Ok(Expression {
            line: target.line,
            column: target.column,
            kind: ExpressionKind::Assign {
                target: Box::new(target),
                operator,
                value: Box::new(value),
            },
        })
    }

    fn expression_inner(&mut self, minimum_precedence: u8) -> Result<Expression, Vec<Diagnostic>> {
        self.skip_newlines();
        let (mut left, mut left_was_parenthesized) = self.prefix()?;
        let mut left_height = expression_height(&left);
        loop {
            if self.check(&TokenKind::LeftParen) && 25 >= minimum_precedence {
                if !is_callable_name(&left) {
                    return Err(self.error("a chamada deve usar um nome de função ou biblioteca"));
                }
                self.advance();
                let mut arguments = Vec::new();
                let mut argument_height = 0;
                self.skip_newlines();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        let argument = self.assignment_or_expression()?;
                        argument_height = argument_height.max(expression_height(&argument));
                        arguments.push(argument);
                        self.skip_newlines();
                        if !self.matches(&TokenKind::Comma) {
                            break;
                        }
                        self.skip_newlines();
                    }
                }
                self.consume(TokenKind::RightParen, "esperado ')' após os argumentos")?;
                left_height = left_height.max(argument_height) + 1;
                self.check_expression_height(left_height)?;
                left = Expression {
                    line: left.line,
                    column: left.column,
                    kind: ExpressionKind::Call {
                        callee: Box::new(left),
                        arguments,
                    },
                };
                left_was_parenthesized = false;
                continue;
            }
            if self.check(&TokenKind::LeftBracket) && 25 >= minimum_precedence {
                if !is_array_reference(&left) || array_reference_depth(&left) == 2 {
                    return Err(self.error("referência de vetor inválida"));
                }
                self.advance();
                let index = self.expression(0)?;
                left_height = left_height.max(expression_height(&index)) + 1;
                self.check_expression_height(left_height)?;
                self.consume(TokenKind::RightBracket, "esperado ']' após o índice")?;
                left = Expression {
                    line: left.line,
                    column: left.column,
                    kind: ExpressionKind::Index {
                        target: Box::new(left),
                        index: Box::new(index),
                    },
                };
                left_was_parenthesized = false;
                continue;
            }
            if self.check(&TokenKind::Dot) && 25 >= minimum_precedence {
                if !matches!(&left.kind, ExpressionKind::Variable(_)) {
                    return Err(self.error("o escopo da biblioteca deve ser um identificador"));
                }
                self.advance();
                let name = self.consume_identifier("esperado um nome após '.'")?;
                left_height += 1;
                self.check_expression_height(left_height)?;
                left = Expression {
                    line: left.line,
                    column: left.column,
                    kind: ExpressionKind::Member {
                        target: Box::new(left),
                        name,
                    },
                };
                left_was_parenthesized = false;
                continue;
            }
            if self.check(&TokenKind::PlusPlus) || self.check(&TokenKind::MinusMinus) {
                if 25 < minimum_precedence {
                    break;
                }
                if left_was_parenthesized || !is_increment_target(&left) {
                    return Err(self.error("incremento exige uma variável ou elemento de vetor"));
                }
                let operator = if self.matches(&TokenKind::PlusPlus) {
                    PostfixOperator::Increment
                } else {
                    self.advance();
                    PostfixOperator::Decrement
                };
                left_height += 1;
                self.check_expression_height(left_height)?;
                left = Expression {
                    line: left.line,
                    column: left.column,
                    kind: ExpressionKind::Postfix {
                        target: Box::new(left),
                        operator,
                    },
                };
                continue;
            }

            let Some((precedence, operator)) = self.binary_operator() else {
                break;
            };
            if precedence < minimum_precedence {
                break;
            }
            self.advance();
            let right = self.expression(precedence + 1)?;
            left_height = left_height.max(expression_height(&right)) + 1;
            self.check_expression_height(left_height)?;
            let line = left.line;
            let column = left.column;
            left = Expression {
                kind: ExpressionKind::Binary {
                    left: Box::new(left),
                    operator: operator.binary(),
                    right: Box::new(right),
                },
                line,
                column,
            };
            left_was_parenthesized = false;
        }
        Ok(left)
    }

    fn check_expression_height(&self, height: usize) -> Result<(), Vec<Diagnostic>> {
        if height > MAX_EXPRESSION_NESTING {
            Err(self.error("expressão excede o limite de aninhamento"))
        } else {
            Ok(())
        }
    }

    fn prefix(&mut self) -> Result<(Expression, bool), Vec<Diagnostic>> {
        let token = self.advance().clone();
        let expression = match token.kind {
            TokenKind::Integer(value) => ExpressionKind::Integer(value),
            TokenKind::Real(value) => ExpressionKind::Real(value),
            TokenKind::String(value) => ExpressionKind::String(value),
            TokenKind::Character(value) => ExpressionKind::Character(value),
            TokenKind::Identifier(value) if value == "verdadeiro" => ExpressionKind::Logical(true),
            TokenKind::Identifier(value) if value == "falso" => ExpressionKind::Logical(false),
            TokenKind::Identifier(value) if value == "nao" => ExpressionKind::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(self.expression(25)?),
            },
            TokenKind::Identifier(value) if !is_reserved_word(&value) => {
                ExpressionKind::Variable(value)
            }
            TokenKind::PlusPlus => {
                if self.check(&TokenKind::LeftParen) {
                    return Err(self.error("incremento exige uma variável ou elemento de vetor"));
                }
                let target = self.expression(25)?;
                if !is_increment_target(&target) {
                    return Err(self.error("incremento exige uma variável ou elemento de vetor"));
                }
                return Ok((
                    Expression {
                        kind: ExpressionKind::Prefix {
                            target: Box::new(target),
                            operator: PostfixOperator::Increment,
                        },
                        line: token.line,
                        column: token.column,
                    },
                    false,
                ));
            }
            TokenKind::MinusMinus => {
                if self.check(&TokenKind::LeftParen) {
                    return Err(self.error("decremento exige uma variável ou elemento de vetor"));
                }
                let target = self.expression(25)?;
                if !is_increment_target(&target) {
                    return Err(self.error("decremento exige uma variável ou elemento de vetor"));
                }
                return Ok((
                    Expression {
                        kind: ExpressionKind::Prefix {
                            target: Box::new(target),
                            operator: PostfixOperator::Decrement,
                        },
                        line: token.line,
                        column: token.column,
                    },
                    false,
                ));
            }
            TokenKind::Plus => ExpressionKind::Unary {
                operator: UnaryOperator::Positive,
                operand: Box::new(self.expression(25)?),
            },
            TokenKind::Minus => ExpressionKind::Unary {
                operator: UnaryOperator::Negate,
                operand: Box::new(self.expression(25)?),
            },
            TokenKind::BitNot => ExpressionKind::Unary {
                operator: UnaryOperator::BitNot,
                operand: Box::new(self.expression(25)?),
            },
            TokenKind::LeftParen => {
                self.skip_newlines();
                let inner = self.expression(0)?;
                self.skip_newlines();
                self.consume(TokenKind::RightParen, "esperado ')' após a expressão")?;
                return Ok((inner, true));
            }
            _ => return Err(self.error_at(&token, "esperada uma expressão")),
        };
        Ok((
            Expression {
                kind: expression,
                line: token.line,
                column: token.column,
            },
            false,
        ))
    }

    fn binary_operator(&self) -> Option<(u8, Operator)> {
        let result = match &self.peek().kind {
            TokenKind::Identifier(word) if word == "ou" => (12, Operator::Or),
            TokenKind::Identifier(word) if word == "e" => (13, Operator::And),
            TokenKind::Pipe => (7, Operator::BitOr),
            TokenKind::Caret => (11, Operator::BitXor),
            TokenKind::Ampersand => (8, Operator::BitAnd),
            TokenKind::EqualEqual => (19, Operator::Equal),
            TokenKind::BangEqual => (18, Operator::NotEqual),
            TokenKind::Less => (16, Operator::Less),
            TokenKind::LessEqual => (15, Operator::LessEqual),
            TokenKind::Greater => (17, Operator::Greater),
            TokenKind::GreaterEqual => (14, Operator::GreaterEqual),
            TokenKind::ShiftLeft => (10, Operator::ShiftLeft),
            TokenKind::ShiftRight => (9, Operator::ShiftRight),
            TokenKind::Plus => (21, Operator::Add),
            TokenKind::Minus => (20, Operator::Subtract),
            TokenKind::Star => (24, Operator::Multiply),
            TokenKind::Slash => (23, Operator::Divide),
            TokenKind::Percent => (22, Operator::Remainder),
            _ => return None,
        };
        Some(result)
    }

    fn parse_type(&mut self) -> Result<Type, Vec<Diagnostic>> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Identifier(ref name) => match name.as_str() {
                "inteiro" => Ok(Type::Integer),
                "real" => Ok(Type::Real),
                "logico" => Ok(Type::Logical),
                "caracter" => Ok(Type::Character),
                "cadeia" => Ok(Type::String),
                "vazio" => Ok(Type::Void),
                _ => Err(self.error_at(&token, "tipo de variável desconhecido")),
            },
            _ => Err(self.error_at(&token, "esperado o tipo da variável")),
        }
    }

    fn is_type(&self) -> bool {
        matches!(&self.peek().kind, TokenKind::Identifier(name) if is_type_name(name))
    }

    fn consume_identifier(&mut self, message: &str) -> Result<String, Vec<Diagnostic>> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Identifier(name) if !is_reserved_word(&name) => Ok(name),
            _ => Err(self.error_at(&token, message)),
        }
    }

    fn consume_word(&mut self, word: &str, message: &str) -> Result<(), Vec<Diagnostic>> {
        if self.matches_word(word) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn consume(&mut self, kind: TokenKind, message: &str) -> Result<(), Vec<Diagnostic>> {
        if self.matches(&kind) {
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn consume_statement_end(&mut self) -> Result<(), Vec<Diagnostic>> {
        Ok(())
    }

    fn at_statement_end(&self) -> bool {
        !self.is_expression_start()
    }

    fn is_expression_start(&self) -> bool {
        match &self.peek().kind {
            TokenKind::Integer(_)
            | TokenKind::Real(_)
            | TokenKind::String(_)
            | TokenKind::Character(_)
            | TokenKind::LeftParen
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::PlusPlus
            | TokenKind::MinusMinus
            | TokenKind::BitNot => true,
            TokenKind::Identifier(word) => {
                !is_reserved_word(word) || matches!(word.as_str(), "verdadeiro" | "falso" | "nao")
            }
            _ => false,
        }
    }

    fn skip_terminators(&mut self) {
        self.skip_newlines();
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    fn check_word(&self, word: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Identifier(value) if value == word)
    }

    fn matches_word(&mut self, word: &str) -> bool {
        if self.check_word(word) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        discriminant(&self.peek().kind) == discriminant(kind)
    }

    fn matches(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn is_at_end(&self) -> bool {
        self.check(&TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn error(&self, message: &str) -> Vec<Diagnostic> {
        let token = self.peek();
        self.error_at(token, message)
    }

    fn error_at(&self, token: &Token, message: &str) -> Vec<Diagnostic> {
        vec![Diagnostic::new(
            DiagnosticKind::Syntax,
            message,
            token.line,
            token.column,
        )]
    }
}

#[derive(Clone, Copy)]
enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    BitAnd,
    BitOr,
    BitXor,
    ShiftLeft,
    ShiftRight,
    And,
    Or,
}

impl Operator {
    fn binary(self) -> BinaryOperator {
        match self {
            Self::Add => BinaryOperator::Add,
            Self::Subtract => BinaryOperator::Subtract,
            Self::Multiply => BinaryOperator::Multiply,
            Self::Divide => BinaryOperator::Divide,
            Self::Remainder => BinaryOperator::Remainder,
            Self::Equal => BinaryOperator::Equal,
            Self::NotEqual => BinaryOperator::NotEqual,
            Self::Less => BinaryOperator::Less,
            Self::LessEqual => BinaryOperator::LessEqual,
            Self::Greater => BinaryOperator::Greater,
            Self::GreaterEqual => BinaryOperator::GreaterEqual,
            Self::BitAnd => BinaryOperator::BitAnd,
            Self::BitOr => BinaryOperator::BitOr,
            Self::BitXor => BinaryOperator::BitXor,
            Self::ShiftLeft => BinaryOperator::ShiftLeft,
            Self::ShiftRight => BinaryOperator::ShiftRight,
            Self::And => BinaryOperator::And,
            Self::Or => BinaryOperator::Or,
        }
    }
}

fn is_type_name(name: &str) -> bool {
    matches!(
        name,
        "inteiro" | "real" | "logico" | "caracter" | "cadeia" | "vazio"
    )
}

fn is_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "real"
            | "inteiro"
            | "vazio"
            | "logico"
            | "cadeia"
            | "caracter"
            | "faca"
            | "enquanto"
            | "para"
            | "se"
            | "senao"
            | "const"
            | "funcao"
            | "programa"
            | "escolha"
            | "caso"
            | "contrario"
            | "pare"
            | "retorne"
            | "inclua"
            | "biblioteca"
            | "nao"
            | "e"
            | "ou"
            | "verdadeiro"
            | "falso"
    )
}

fn is_callable_name(expression: &Expression) -> bool {
    match &expression.kind {
        ExpressionKind::Variable(_) => true,
        ExpressionKind::Member { target, .. } => {
            matches!(&target.kind, ExpressionKind::Variable(_))
        }
        _ => false,
    }
}

fn is_array_reference(expression: &Expression) -> bool {
    match &expression.kind {
        ExpressionKind::Variable(_) => true,
        ExpressionKind::Member { target, .. } => {
            matches!(&target.kind, ExpressionKind::Variable(_))
        }
        ExpressionKind::Index { target, .. } => is_array_reference(target),
        _ => false,
    }
}

fn array_reference_depth(expression: &Expression) -> usize {
    match &expression.kind {
        ExpressionKind::Index { target, .. } => array_reference_depth(target) + 1,
        _ => 0,
    }
}

fn is_increment_target(expression: &Expression) -> bool {
    match &expression.kind {
        ExpressionKind::Variable(_) => true,
        ExpressionKind::Index { target, .. } => is_increment_target(target),
        _ => false,
    }
}

fn expression_height(expression: &Expression) -> usize {
    let child_height = match &expression.kind {
        ExpressionKind::ArrayLiteral(values) => {
            values.iter().map(expression_height).max().unwrap_or(0)
        }
        ExpressionKind::Unary { operand, .. } => expression_height(operand),
        ExpressionKind::Binary { left, right, .. }
        | ExpressionKind::Assign {
            target: left,
            value: right,
            ..
        }
        | ExpressionKind::Index {
            target: left,
            index: right,
        } => expression_height(left).max(expression_height(right)),
        ExpressionKind::Call { callee, arguments } => arguments
            .iter()
            .map(expression_height)
            .fold(expression_height(callee), usize::max),
        ExpressionKind::Member { target, .. }
        | ExpressionKind::Postfix { target, .. }
        | ExpressionKind::Prefix { target, .. } => expression_height(target),
        ExpressionKind::Integer(_)
        | ExpressionKind::Real(_)
        | ExpressionKind::Logical(_)
        | ExpressionKind::Character(_)
        | ExpressionKind::String(_)
        | ExpressionKind::Variable(_) => 0,
    };
    child_height
        + usize::from(matches!(
            &expression.kind,
            ExpressionKind::Unary { .. }
                | ExpressionKind::Binary { .. }
                | ExpressionKind::Assign { .. }
                | ExpressionKind::Call { .. }
                | ExpressionKind::Index { .. }
                | ExpressionKind::Member { .. }
                | ExpressionKind::Postfix { .. }
                | ExpressionKind::Prefix { .. }
        ))
}

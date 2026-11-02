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
        self.skip_terminators();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            if self.check_word("funcao") {
                functions.push(self.function()?);
            } else if self.check_word("inclua") {
                libraries.push(self.include()?);
            } else {
                statements.push(self.statement()?);
            }
            self.consume_statement_end()?;
            self.skip_terminators();
        }
        self.consume(TokenKind::RightBrace, "esperado '}' para fechar o programa")?;
        self.skip_terminators();
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
                    self.skip_newlines();
                    self.consume(
                        TokenKind::RightBracket,
                        "esperado ']' após a dimensão do parâmetro",
                    )?;
                    dimensions += 1;
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
        let body = self.block_or_statement()?;
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
        if self.matches(&TokenKind::LeftBrace) {
            let mut statements = Vec::new();
            self.skip_terminators();
            while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
                statements.push(self.statement()?);
                self.consume_statement_end()?;
                self.skip_terminators();
            }
            self.consume(TokenKind::RightBrace, "esperado '}' para fechar o bloco")?;
            return Ok(Statement {
                kind: StatementKind::Block(statements),
                line: token.line,
                column: token.column,
            });
        }
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
        if self.matches_word("constante") || self.matches_word("const") {
            return self.variable_declaration(true);
        }
        if self.is_type() {
            return self.variable_declaration(false);
        }
        let expression = self.expression(0)?;
        Ok(Statement {
            kind: StatementKind::Expression(expression),
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
            let initializer = if self.matches_word("constante") || self.matches_word("const") {
                self.variable_declaration(true)?
            } else if self.is_type() {
                self.variable_declaration(false)?
            } else {
                let expression = self.expression(0)?;
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
        let condition = if self.check(&TokenKind::Semicolon) {
            None
        } else {
            Some(self.expression(0)?)
        };
        self.consume(
            TokenKind::Semicolon,
            "esperado ';' após a condição do 'para'",
        )?;

        self.skip_newlines();
        let increment = if self.check(&TokenKind::RightParen) {
            None
        } else {
            Some(self.expression(0)?)
        };
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
        let mut default = Vec::new();
        self.skip_terminators();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            self.consume_word("caso", "esperado 'caso' dentro de 'escolha'")?;
            self.skip_newlines();
            if self.matches_word("contrario") {
                self.consume_case_separator()?;
                default = self.case_body()?;
            } else {
                let value = self.expression(0)?;
                self.consume_case_separator()?;
                let body = self.case_body()?;
                cases.push(SwitchCase { value, body });
            }
            self.skip_terminators();
        }
        self.consume(TokenKind::RightBrace, "esperado '}' para fechar 'escolha'")?;
        Ok(Statement {
            kind: StatementKind::Switch {
                expression,
                cases,
                default,
            },
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
            // A `pare` after a braced case is accepted but ignored by the original dialect.
            self.skip_newlines();
            self.matches_word("pare");
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
        if self.matches(&TokenKind::Colon) || self.check(&TokenKind::LeftBrace) {
            Ok(())
        } else if self.check(&TokenKind::Newline) {
            self.skip_newlines();
            Ok(())
        } else {
            Err(self.error("esperado ':' ou bloco após o rótulo do caso"))
        }
    }

    fn variable_declaration(&mut self, is_constant: bool) -> Result<Statement, Vec<Diagnostic>> {
        let token = self.peek().clone();
        let ty = self.parse_type()?;
        let mut variables = Vec::new();
        loop {
            let name = self.consume_identifier("esperado o nome da variável")?;
            let mut dimensions = Vec::new();
            while self.matches(&TokenKind::LeftBracket) {
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
                Some(self.expression(0)?)
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

    fn condition(&mut self) -> Result<Expression, Vec<Diagnostic>> {
        self.skip_newlines();
        if self.matches(&TokenKind::LeftParen) {
            self.skip_newlines();
            let expression = self.expression(0)?;
            self.skip_newlines();
            self.consume(TokenKind::RightParen, "esperado ')' após a condição")?;
            Ok(expression)
        } else {
            self.expression(0)
        }
    }

    fn block_or_statement(&mut self) -> Result<Statement, Vec<Diagnostic>> {
        self.skip_newlines();
        self.statement()
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

    fn expression_inner(&mut self, minimum_precedence: u8) -> Result<Expression, Vec<Diagnostic>> {
        self.skip_newlines();
        let mut left = self.prefix()?;
        loop {
            if self.check(&TokenKind::LeftParen) && 13 >= minimum_precedence {
                self.advance();
                let mut arguments = Vec::new();
                self.skip_newlines();
                if !self.check(&TokenKind::RightParen) {
                    loop {
                        arguments.push(self.expression(0)?);
                        self.skip_newlines();
                        if !self.matches(&TokenKind::Comma) {
                            break;
                        }
                        self.skip_newlines();
                    }
                }
                self.consume(TokenKind::RightParen, "esperado ')' após os argumentos")?;
                left = Expression {
                    line: left.line,
                    column: left.column,
                    kind: ExpressionKind::Call {
                        callee: Box::new(left),
                        arguments,
                    },
                };
                continue;
            }
            if self.check(&TokenKind::LeftBracket) && 13 >= minimum_precedence {
                self.advance();
                let index = self.expression(0)?;
                self.consume(TokenKind::RightBracket, "esperado ']' após o índice")?;
                left = Expression {
                    line: left.line,
                    column: left.column,
                    kind: ExpressionKind::Index {
                        target: Box::new(left),
                        index: Box::new(index),
                    },
                };
                continue;
            }
            if self.check(&TokenKind::Dot) && 13 >= minimum_precedence {
                self.advance();
                let name = self.consume_identifier("esperado um nome após '.'")?;
                left = Expression {
                    line: left.line,
                    column: left.column,
                    kind: ExpressionKind::Member {
                        target: Box::new(left),
                        name,
                    },
                };
                continue;
            }
            if self.check(&TokenKind::PlusPlus) || self.check(&TokenKind::MinusMinus) {
                if 13 < minimum_precedence {
                    break;
                }
                let operator = if self.matches(&TokenKind::PlusPlus) {
                    PostfixOperator::Increment
                } else {
                    self.advance();
                    PostfixOperator::Decrement
                };
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
            let right = if precedence == 1 {
                self.expression(precedence)?
            } else {
                self.expression(precedence + 1)?
            };
            let line = left.line;
            let column = left.column;
            if let Some(operator) = operator.assignment() {
                left = Expression {
                    kind: ExpressionKind::Assign {
                        target: Box::new(left),
                        operator,
                        value: Box::new(right),
                    },
                    line,
                    column,
                };
            } else {
                left = Expression {
                    kind: ExpressionKind::Binary {
                        left: Box::new(left),
                        operator: operator.binary().expect("non-assignment operator"),
                        right: Box::new(right),
                    },
                    line,
                    column,
                };
            }
        }
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Expression, Vec<Diagnostic>> {
        let token = self.advance().clone();
        let expression = match token.kind {
            TokenKind::Integer(value) => ExpressionKind::Integer(value),
            TokenKind::Real(value) => ExpressionKind::Real(value),
            TokenKind::String(value) => ExpressionKind::String(value),
            TokenKind::Character(value) => ExpressionKind::Character(value),
            TokenKind::Identifier(value) if value.eq_ignore_ascii_case("verdadeiro") => {
                ExpressionKind::Logical(true)
            }
            TokenKind::Identifier(value) if value.eq_ignore_ascii_case("falso") => {
                ExpressionKind::Logical(false)
            }
            TokenKind::Identifier(value) if value.eq_ignore_ascii_case("nao") => {
                ExpressionKind::Unary {
                    operator: UnaryOperator::Not,
                    operand: Box::new(self.expression(12)?),
                }
            }
            TokenKind::Identifier(value) => ExpressionKind::Variable(value),
            TokenKind::PlusPlus => {
                return Ok(Expression {
                    kind: ExpressionKind::Prefix {
                        target: Box::new(self.expression(12)?),
                        operator: PostfixOperator::Increment,
                    },
                    line: token.line,
                    column: token.column,
                });
            }
            TokenKind::MinusMinus => {
                return Ok(Expression {
                    kind: ExpressionKind::Prefix {
                        target: Box::new(self.expression(12)?),
                        operator: PostfixOperator::Decrement,
                    },
                    line: token.line,
                    column: token.column,
                });
            }
            TokenKind::Plus => ExpressionKind::Unary {
                operator: UnaryOperator::Positive,
                operand: Box::new(self.expression(12)?),
            },
            TokenKind::Minus => ExpressionKind::Unary {
                operator: UnaryOperator::Negate,
                operand: Box::new(self.expression(12)?),
            },
            TokenKind::Not => ExpressionKind::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(self.expression(12)?),
            },
            TokenKind::BitNot => ExpressionKind::Unary {
                operator: UnaryOperator::BitNot,
                operand: Box::new(self.expression(12)?),
            },
            TokenKind::LeftParen => {
                self.skip_newlines();
                let inner = self.expression(0)?;
                self.skip_newlines();
                self.consume(TokenKind::RightParen, "esperado ')' após a expressão")?;
                return Ok(inner);
            }
            TokenKind::LeftBrace => {
                let mut values = Vec::new();
                self.skip_terminators();
                if !self.check(&TokenKind::RightBrace) {
                    loop {
                        values.push(self.expression(0)?);
                        self.skip_newlines();
                        if !self.matches(&TokenKind::Comma) {
                            break;
                        }
                        self.skip_terminators();
                    }
                }
                self.consume(TokenKind::RightBrace, "esperado '}' após os valores")?;
                ExpressionKind::ArrayLiteral(values)
            }
            _ => return Err(self.error_at(&token, "esperada uma expressão")),
        };
        Ok(Expression {
            kind: expression,
            line: token.line,
            column: token.column,
        })
    }

    fn binary_operator(&self) -> Option<(u8, Operator)> {
        let result = match &self.peek().kind {
            TokenKind::Equal => (1, Operator::Assign),
            TokenKind::PlusEqual => (1, Operator::AddAssign),
            TokenKind::MinusEqual => (1, Operator::SubtractAssign),
            TokenKind::StarEqual => (1, Operator::MultiplyAssign),
            TokenKind::SlashEqual => (1, Operator::DivideAssign),
            TokenKind::Identifier(word) if word.eq_ignore_ascii_case("ou") => (2, Operator::Or),
            TokenKind::Identifier(word) if word.eq_ignore_ascii_case("e") => (3, Operator::And),
            TokenKind::Pipe => (4, Operator::BitOr),
            TokenKind::Caret => (5, Operator::BitXor),
            TokenKind::Ampersand => (6, Operator::BitAnd),
            TokenKind::EqualEqual => (7, Operator::Equal),
            TokenKind::BangEqual => (7, Operator::NotEqual),
            TokenKind::Less => (8, Operator::Less),
            TokenKind::LessEqual => (8, Operator::LessEqual),
            TokenKind::Greater => (8, Operator::Greater),
            TokenKind::GreaterEqual => (8, Operator::GreaterEqual),
            TokenKind::ShiftLeft => (9, Operator::ShiftLeft),
            TokenKind::ShiftRight => (9, Operator::ShiftRight),
            TokenKind::Plus => (10, Operator::Add),
            TokenKind::Minus => (10, Operator::Subtract),
            TokenKind::Star => (11, Operator::Multiply),
            TokenKind::Slash => (11, Operator::Divide),
            TokenKind::IntegerSlash => (11, Operator::IntegerDivide),
            TokenKind::Percent => (11, Operator::Remainder),
            _ => return None,
        };
        Some(result)
    }

    fn parse_type(&mut self) -> Result<Type, Vec<Diagnostic>> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Identifier(ref name) => match name.to_ascii_lowercase().as_str() {
                "inteiro" => Ok(Type::Integer),
                "real" => Ok(Type::Real),
                "logico" => Ok(Type::Logical),
                "caracter" => Ok(Type::Character),
                "cadeia" => Ok(Type::String),
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
            TokenKind::Identifier(name) => Ok(name),
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
        if self.check(&TokenKind::Newline) || self.check(&TokenKind::Semicolon) {
            self.skip_terminators();
            Ok(())
        } else if self.check(&TokenKind::RightBrace) || self.is_at_end() {
            Ok(())
        } else {
            Err(self.error("esperada uma nova linha ou ';' entre instruções"))
        }
    }

    fn at_statement_end(&self) -> bool {
        self.check(&TokenKind::Newline)
            || self.check(&TokenKind::Semicolon)
            || self.check(&TokenKind::RightBrace)
            || self.is_at_end()
    }

    fn skip_terminators(&mut self) {
        while self.check(&TokenKind::Newline) || self.check(&TokenKind::Semicolon) {
            self.advance();
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }

    fn check_word(&self, word: &str) -> bool {
        matches!(&self.peek().kind, TokenKind::Identifier(value) if value.eq_ignore_ascii_case(word))
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
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    Add,
    Subtract,
    Multiply,
    Divide,
    IntegerDivide,
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
    fn assignment(self) -> Option<AssignmentOperator> {
        match self {
            Self::Assign => Some(AssignmentOperator::Assign),
            Self::AddAssign => Some(AssignmentOperator::Add),
            Self::SubtractAssign => Some(AssignmentOperator::Subtract),
            Self::MultiplyAssign => Some(AssignmentOperator::Multiply),
            Self::DivideAssign => Some(AssignmentOperator::Divide),
            _ => None,
        }
    }

    fn binary(self) -> Option<BinaryOperator> {
        match self {
            Self::Add => Some(BinaryOperator::Add),
            Self::Subtract => Some(BinaryOperator::Subtract),
            Self::Multiply => Some(BinaryOperator::Multiply),
            Self::Divide => Some(BinaryOperator::Divide),
            Self::IntegerDivide => Some(BinaryOperator::IntegerDivide),
            Self::Remainder => Some(BinaryOperator::Remainder),
            Self::Equal => Some(BinaryOperator::Equal),
            Self::NotEqual => Some(BinaryOperator::NotEqual),
            Self::Less => Some(BinaryOperator::Less),
            Self::LessEqual => Some(BinaryOperator::LessEqual),
            Self::Greater => Some(BinaryOperator::Greater),
            Self::GreaterEqual => Some(BinaryOperator::GreaterEqual),
            Self::BitAnd => Some(BinaryOperator::BitAnd),
            Self::BitOr => Some(BinaryOperator::BitOr),
            Self::BitXor => Some(BinaryOperator::BitXor),
            Self::ShiftLeft => Some(BinaryOperator::ShiftLeft),
            Self::ShiftRight => Some(BinaryOperator::ShiftRight),
            Self::And => Some(BinaryOperator::And),
            Self::Or => Some(BinaryOperator::Or),
            _ => None,
        }
    }
}

fn is_type_name(name: &str) -> bool {
    matches!(
        name.to_ascii_lowercase().as_str(),
        "inteiro" | "real" | "logico" | "caracter" | "cadeia"
    )
}

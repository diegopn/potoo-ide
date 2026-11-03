use crate::ast::{
    Expression, ExpressionKind, Module, ModuleDeclaration, Program, Statement, StatementKind, Type,
    VariableDeclaration, VectorBounds,
};
use crate::lexer::{self, Token, TokenKind};
use crate::{Diagnostic, DiagnosticKind};

pub(crate) fn parse(
    source: &str,
    max_depth: usize,
    max_string_bytes: usize,
) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lexer::lex(source, max_string_bytes).map_err(|diagnostic| vec![diagnostic])?;
    Parser {
        tokens,
        index: 0,
        max_depth,
    }
    .program()
    .map_err(|diagnostic| vec![diagnostic])
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    max_depth: usize,
}

impl Parser {
    fn program(mut self) -> Result<Program, Diagnostic> {
        self.expect_word("variaveis")?;
        let mut variables = Vec::new();
        let mut module_declarations = Vec::new();
        while !self.at_word("inicio") {
            if self.at_end() {
                return Err(self.error("esperado 'inicio' após as declarações"));
            }
            self.declaration(&mut variables, &mut module_declarations)?;
        }

        self.expect_word("inicio")?;
        let statements = self.statements_until(&["fim"], 0)?;
        self.expect_word("fim")?;
        self.consume_symbol(".");

        let mut modules = Vec::new();
        while !self.at_end() {
            let start = self.current().clone();
            self.expect_word("modulo")?;
            let name = self.identifier()?;
            self.consume_symbol(";");
            let body = self.statements_until(&["fim"], 0)?;
            self.expect_word("fim")?;
            self.expect_word("modulo")?;
            self.expect_symbol(";")?;
            modules.push(Module {
                name,
                statements: body,
                line: start.line,
                column: start.column,
            });
        }

        Ok(Program {
            variables,
            module_declarations,
            statements,
            modules,
        })
    }

    fn declaration(
        &mut self,
        variables: &mut Vec<VariableDeclaration>,
        modules: &mut Vec<ModuleDeclaration>,
    ) -> Result<(), Diagnostic> {
        let start = self.current().clone();
        let mut names = vec![self.identifier()?];
        while self.consume_symbol(",") {
            names.push(self.identifier()?);
        }
        self.expect_symbol(":")?;

        if self.consume_word("modulo") {
            self.expect_symbol(";")?;
            modules.extend(names.into_iter().map(|name| ModuleDeclaration {
                name,
                line: start.line,
                column: start.column,
            }));
            return Ok(());
        }

        let vector = if self.consume_word("vetor") {
            self.expect_symbol("[")?;
            let lower = self.integer_constant("limite inferior do vetor")?;
            self.expect_symbol("..")?;
            let upper = self.integer_constant("limite superior do vetor")?;
            self.expect_symbol("]")?;
            self.expect_word("de")?;
            if lower > upper {
                return Err(self.error("o limite inicial do vetor não pode superar o final"));
            }
            Some(VectorBounds { lower, upper })
        } else {
            None
        };
        let data_type = self.data_type()?;
        self.expect_symbol(";")?;
        variables.extend(names.into_iter().map(|name| VariableDeclaration {
            name,
            data_type,
            vector,
            line: start.line,
            column: start.column,
        }));
        Ok(())
    }

    fn data_type(&mut self) -> Result<Type, Diagnostic> {
        let token = self.current().clone();
        let TokenKind::Word(name) = token.kind else {
            return Err(self.error("esperado um tipo de dado"));
        };
        let data_type = match name.as_str() {
            "inteiro" => Type::Integer,
            "real" => Type::Real,
            "logico" => Type::Logical,
            "cadeia" => Type::Chain,
            "caractere" => Type::Character,
            _ => return Err(self.error("tipo de dado Mapler desconhecido")),
        };
        self.advance();
        Ok(data_type)
    }

    fn statements_until(
        &mut self,
        terminators: &[&str],
        depth: usize,
    ) -> Result<Vec<Statement>, Diagnostic> {
        if depth > self.max_depth {
            return Err(self.error("blocos aninhados excedem o limite de análise"));
        }
        let mut statements = Vec::new();
        loop {
            if terminators.iter().any(|word| self.at_word(word)) {
                return Ok(statements);
            }
            if self.at_end() {
                return Err(self.error("bloco não finalizado"));
            }
            statements.push(self.statement(depth)?);
        }
    }

    fn statement(&mut self, depth: usize) -> Result<Statement, Diagnostic> {
        let start = self.current().clone();
        let kind = if self.consume_word("escrever") {
            let mut values = vec![self.expression(0, depth + 1)?];
            while self.consume_symbol(",") {
                values.push(self.expression(0, depth + 1)?);
            }
            self.expect_symbol(";")?;
            StatementKind::Write(values)
        } else if self.consume_word("ler") {
            let target = self.target(depth + 1)?;
            self.expect_symbol(";")?;
            StatementKind::Read(target)
        } else if self.consume_word("se") {
            let condition = self.expression(0, depth + 1)?;
            self.expect_word("entao")?;
            let then_branch = self.statements_until(&["senao", "fim"], depth + 1)?;
            let else_branch = if self.consume_word("senao") {
                self.statements_until(&["fim"], depth + 1)?
            } else {
                Vec::new()
            };
            self.end_block("se")?;
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            }
        } else if self.consume_word("enquanto") {
            let condition = self.expression(0, depth + 1)?;
            self.expect_word("faca")?;
            let body = self.statements_until(&["fim"], depth + 1)?;
            self.end_block("enquanto")?;
            StatementKind::While { condition, body }
        } else if self.consume_word("repita") {
            let body = self.statements_until(&["ate"], depth + 1)?;
            self.expect_word("ate")?;
            let condition = self.expression(0, depth + 1)?;
            self.expect_symbol(";")?;
            StatementKind::Repeat { condition, body }
        } else if self.consume_word("para") {
            let variable = self.identifier()?;
            self.expect_word("de")?;
            let start_value = self.expression(0, depth + 1)?;
            self.expect_word("ate")?;
            let end_value = self.expression(0, depth + 1)?;
            self.expect_word("passo")?;
            let step = self.expression(0, depth + 1)?;
            self.expect_word("faca")?;
            let body = self.statements_until(&["fim"], depth + 1)?;
            self.end_block("para")?;
            StatementKind::For {
                variable,
                start: start_value,
                end: end_value,
                step,
                body,
            }
        } else {
            self.assignment_or_module_call(depth + 1)?
        };
        Ok(Statement {
            kind,
            line: start.line,
            column: start.column,
        })
    }

    fn assignment_or_module_call(&mut self, depth: usize) -> Result<StatementKind, Diagnostic> {
        let (name, token) = self.identifier_with_token()?;
        let target = if self.consume_symbol("[") {
            let index = self.expression(0, depth + 1)?;
            self.expect_symbol("]")?;
            Expression {
                kind: ExpressionKind::Index {
                    name: name.clone(),
                    index: Box::new(index),
                },
                line: token.line,
                column: token.column,
            }
        } else {
            Expression {
                kind: ExpressionKind::Identifier(name.clone()),
                line: token.line,
                column: token.column,
            }
        };

        if self.consume_symbol("<-") {
            let value = self.expression(0, depth + 1)?;
            self.expect_symbol(";")?;
            Ok(StatementKind::Assign { target, value })
        } else if matches!(target.kind, ExpressionKind::Identifier(_)) && self.consume_symbol(";") {
            Ok(StatementKind::CallModule(name))
        } else {
            Err(self.error("esperado '<-' ou uma chamada de módulo"))
        }
    }

    fn target(&mut self, depth: usize) -> Result<Expression, Diagnostic> {
        let (name, token) = self.identifier_with_token()?;
        if self.consume_symbol("[") {
            let index = self.expression(0, depth + 1)?;
            self.expect_symbol("]")?;
            Ok(Expression {
                kind: ExpressionKind::Index {
                    name,
                    index: Box::new(index),
                },
                line: token.line,
                column: token.column,
            })
        } else {
            Ok(Expression {
                kind: ExpressionKind::Identifier(name),
                line: token.line,
                column: token.column,
            })
        }
    }

    fn end_block(&mut self, name: &str) -> Result<(), Diagnostic> {
        self.expect_word("fim")?;
        self.expect_word(name)?;
        self.expect_symbol(";")
    }

    fn expression(&mut self, minimum: u8, depth: usize) -> Result<Expression, Diagnostic> {
        if depth > self.max_depth {
            return Err(self.error("expressão aninhada excede o limite de análise"));
        }
        let token = self.current().clone();
        let mut left = match token.kind.clone() {
            TokenKind::Integer(value) => {
                self.advance();
                Expression {
                    kind: ExpressionKind::Integer(value),
                    line: token.line,
                    column: token.column,
                }
            }
            TokenKind::Real(value) => {
                self.advance();
                Expression {
                    kind: ExpressionKind::Real(value),
                    line: token.line,
                    column: token.column,
                }
            }
            TokenKind::String(value) => {
                self.advance();
                Expression {
                    kind: ExpressionKind::String(value),
                    line: token.line,
                    column: token.column,
                }
            }
            TokenKind::Word(word) if word == "verdadeiro" || word == "falso" => {
                self.advance();
                Expression {
                    kind: ExpressionKind::Logical(word == "verdadeiro"),
                    line: token.line,
                    column: token.column,
                }
            }
            TokenKind::Word(word) if word == "nao" => {
                self.advance();
                let operand = self.expression(6, depth + 1)?;
                Expression {
                    kind: ExpressionKind::Unary {
                        operator: word,
                        operand: Box::new(operand),
                    },
                    line: token.line,
                    column: token.column,
                }
            }
            TokenKind::Symbol("-") => {
                self.advance();
                let operand = self.expression(6, depth + 1)?;
                Expression {
                    kind: ExpressionKind::Unary {
                        operator: "-".to_owned(),
                        operand: Box::new(operand),
                    },
                    line: token.line,
                    column: token.column,
                }
            }
            TokenKind::Symbol("(") => {
                self.advance();
                let expression = self.expression(0, depth + 1)?;
                self.expect_symbol(")")?;
                expression
            }
            TokenKind::Word(name) => {
                self.advance();
                if self.consume_symbol("[") {
                    let index = self.expression(0, depth + 1)?;
                    self.expect_symbol("]")?;
                    Expression {
                        kind: ExpressionKind::Index {
                            name,
                            index: Box::new(index),
                        },
                        line: token.line,
                        column: token.column,
                    }
                } else {
                    Expression {
                        kind: ExpressionKind::Identifier(name),
                        line: token.line,
                        column: token.column,
                    }
                }
            }
            _ => return Err(self.error("esperada uma expressão")),
        };

        loop {
            let Some((operator, precedence)) = self.binary_operator() else {
                break;
            };
            if precedence < minimum {
                break;
            }
            self.advance();
            let right = self.expression(precedence + 1, depth + 1)?;
            left = Expression {
                line: left.line,
                column: left.column,
                kind: ExpressionKind::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn binary_operator(&self) -> Option<(String, u8)> {
        let (operator, precedence) = match &self.current().kind {
            TokenKind::Word(word) if word == "ou" => (word.as_str(), 1),
            TokenKind::Word(word) if word == "e" => (word.as_str(), 2),
            TokenKind::Symbol("=") => ("=", 3),
            TokenKind::Symbol("<") => ("<", 3),
            TokenKind::Symbol("<=") => ("<=", 3),
            TokenKind::Symbol(">") => (">", 3),
            TokenKind::Symbol(">=") => (">=", 3),
            TokenKind::Symbol("+") => ("+", 4),
            TokenKind::Symbol("-") => ("-", 4),
            TokenKind::Symbol("*") => ("*", 5),
            TokenKind::Symbol("/") => ("/", 5),
            _ => return None,
        };
        Some((operator.to_owned(), precedence))
    }

    fn integer_constant(&mut self, description: &str) -> Result<i64, Diagnostic> {
        let token = self.current().clone();
        if let TokenKind::Integer(value) = token.kind {
            self.advance();
            Ok(value)
        } else {
            Err(self.error(&format!("esperado {description} inteiro")))
        }
    }

    fn identifier(&mut self) -> Result<String, Diagnostic> {
        self.identifier_with_token().map(|(name, _)| name)
    }

    fn identifier_with_token(&mut self) -> Result<(String, Token), Diagnostic> {
        let token = self.current().clone();
        if let TokenKind::Word(name) = token.kind.clone() {
            self.advance();
            Ok((name, token))
        } else {
            Err(self.error("esperado um identificador"))
        }
    }

    fn expect_word(&mut self, word: &str) -> Result<(), Diagnostic> {
        if self.consume_word(word) {
            Ok(())
        } else {
            Err(self.error(&format!("esperado '{word}'")))
        }
    }

    fn expect_symbol(&mut self, symbol: &'static str) -> Result<(), Diagnostic> {
        if self.consume_symbol(symbol) {
            Ok(())
        } else {
            Err(self.error(&format!("esperado '{symbol}'")))
        }
    }

    fn consume_word(&mut self, word: &str) -> bool {
        if self.at_word(word) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_symbol(&mut self, symbol: &'static str) -> bool {
        if matches!(self.current().kind, TokenKind::Symbol(current) if current == symbol) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at_word(&self, word: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Word(current) if current == word)
    }

    fn at_end(&self) -> bool {
        matches!(self.current().kind, TokenKind::End)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if !self.at_end() {
            self.index += 1;
        }
        token
    }

    fn error(&self, message: &str) -> Diagnostic {
        let token = self.current();
        Diagnostic::new(DiagnosticKind::Syntax, message, token.line, token.column)
    }
}

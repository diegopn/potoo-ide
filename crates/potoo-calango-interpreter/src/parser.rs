use crate::ast::{
    Expression, ExpressionKind, Parameter, Program, Routine, Statement, StatementKind, Type,
    VariableDeclaration,
};
use crate::lexer::{self, Token, TokenKind};
use crate::{Diagnostic, DiagnosticKind, ExecutionOptions};

pub fn parse(source: &str) -> Result<Program, Vec<Diagnostic>> {
    parse_with_options(source, ExecutionOptions::default())
}

pub fn parse_with_options(
    source: &str,
    options: ExecutionOptions,
) -> Result<Program, Vec<Diagnostic>> {
    let tokens = lexer::lex(source)?;
    Parser::new(tokens, options.max_parse_depth).program()
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    max_depth: usize,
    depth: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>, max_depth: usize) -> Self {
        Self {
            tokens,
            index: 0,
            max_depth: max_depth.max(1),
            depth: 0,
        }
    }

    fn program(mut self) -> Result<Program, Vec<Diagnostic>> {
        self.expect_word("algoritmo")?;
        let (name, _, _) = self.expect_identifier()?;
        self.expect_symbol(";")?;

        let mut main = None;
        let mut routines = Vec::new();
        while !self.is_end() {
            if self.match_word("principal") {
                if main.is_some() {
                    return Err(vec![
                        self.error("o algoritmo contém mais de um bloco principal"),
                    ]);
                }
                main = Some(self.body(&["fimPrincipal"])?);
                self.expect_word("fimPrincipal")?;
            } else if self.check_word("funcao") || self.check_word("procedimento") {
                let routine = self.routine()?;
                if routines
                    .iter()
                    .any(|known: &Routine| known.name == routine.name)
                {
                    return Err(vec![
                        self.error(format!("a rotina '{}' já foi declarada", routine.name)),
                    ]);
                }
                routines.push(routine);
            } else if self.match_symbol(";") {
                continue;
            } else {
                return Err(vec![self.error(
                    "esperado 'principal', 'funcao' ou 'procedimento' após o cabeçalho",
                )]);
            }
        }
        let Some(main) = main else {
            return Err(vec![
                self.error("o algoritmo precisa de um bloco 'principal'"),
            ]);
        };
        Ok(Program {
            name,
            main,
            routines,
        })
    }

    fn routine(&mut self) -> Result<Routine, Vec<Diagnostic>> {
        let is_function = self.match_word("funcao");
        if !is_function {
            self.expect_word("procedimento")?;
        }
        let (return_type, name) = if is_function {
            let return_type = self.parse_type()?;
            let (name, _, _) = self.expect_identifier()?;
            (Some(return_type), name)
        } else {
            let (name, _, _) = self.expect_identifier()?;
            (None, name)
        };
        let location = self.previous().clone();
        let parameters = self.parameters()?;
        let terminator = if is_function {
            "fimFuncao"
        } else {
            "fimProcedimento"
        };
        let body = self.body(&[terminator])?;
        self.expect_word(terminator)?;
        Ok(Routine {
            name,
            return_type,
            parameters,
            body,
            line: location.line,
            column: location.column,
        })
    }

    fn parameters(&mut self) -> Result<Vec<Parameter>, Vec<Diagnostic>> {
        self.expect_symbol("(")?;
        let mut parameters = Vec::new();
        if self.match_symbol(")") {
            return Ok(parameters);
        }
        loop {
            let by_reference = self.match_word("ref");
            let base = self.parse_type()?;
            let by_reference = by_reference || self.match_word("ref");
            let (name, line, column) = self.expect_identifier()?;
            if parameters
                .iter()
                .any(|parameter: &Parameter| parameter.name == name)
            {
                return Err(vec![Diagnostic::new(
                    DiagnosticKind::Syntax,
                    format!("o parâmetro '{name}' já foi declarado"),
                    line,
                    column,
                )]);
            }
            let dimensions = self.array_dimensions()?;
            let ty = Self::with_dimensions(base, dimensions);
            parameters.push(Parameter {
                name,
                ty,
                by_reference,
                line,
                column,
            });
            if self.match_symbol(")") {
                break;
            }
            self.expect_symbol(",")?;
        }
        Ok(parameters)
    }

    fn body(&mut self, terminators: &[&str]) -> Result<Vec<Statement>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        while !self.is_end() && !terminators.iter().any(|word| self.check_word(word)) {
            if self.match_symbol(";") {
                continue;
            }
            statements.push(self.statement()?);
        }
        if self.is_end() {
            return Err(vec![self.error(format!(
                "esperado {} antes do fim do arquivo",
                terminators.join(" ou ")
            ))]);
        }
        Ok(statements)
    }

    fn statement(&mut self) -> Result<Statement, Vec<Diagnostic>> {
        self.depth += 1;
        if self.depth > self.max_depth {
            self.depth -= 1;
            return Err(vec![
                self.error("blocos aninhados excedem o limite de profundidade"),
            ]);
        }
        let result = self.statement_inner();
        self.depth -= 1;
        result
    }

    fn statement_inner(&mut self) -> Result<Statement, Vec<Diagnostic>> {
        let start = self.current().clone();
        let kind = if self.is_type_word() {
            StatementKind::Declare(self.declaration()?)
        } else if self.match_word("escreva") {
            StatementKind::Write {
                newline: false,
                values: self.expression_arguments()?,
            }
        } else if self.match_word("escreval") {
            StatementKind::Write {
                newline: true,
                values: self.expression_arguments()?,
            }
        } else if self.match_word("leia") {
            StatementKind::Read {
                targets: self.expression_arguments()?,
                character_only: false,
            }
        } else if self.match_word("leiaCaracter") {
            let targets = self.expression_arguments()?;
            if targets.len() != 1 {
                return Err(vec![Diagnostic::new(
                    DiagnosticKind::Syntax,
                    "'leiaCaracter' exige exatamente um destino",
                    start.line,
                    start.column,
                )]);
            }
            StatementKind::Read {
                targets,
                character_only: true,
            }
        } else if self.match_word("limpaTela") {
            self.empty_call_arguments()?;
            self.expect_symbol(";")?;
            StatementKind::ClearScreen
        } else if self.match_word("se") {
            self.if_statement()?
        } else if self.match_word("enquanto") {
            self.while_statement()?
        } else if self.match_word("faca") {
            self.do_while_statement()?
        } else if self.match_word("para") {
            self.for_statement()?
        } else if self.match_word("escolha") {
            self.choose_statement()?
        } else if self.match_word("interrompa") {
            self.expect_symbol(";")?;
            StatementKind::Break
        } else if self.match_word("retorna") {
            let value = if self.check_symbol(";") {
                None
            } else {
                Some(self.expression()?)
            };
            self.expect_symbol(";")?;
            StatementKind::Return(value)
        } else {
            self.assignment_or_call()?
        };
        Ok(Statement {
            kind,
            line: start.line,
            column: start.column,
        })
    }

    fn declaration(&mut self) -> Result<Vec<VariableDeclaration>, Vec<Diagnostic>> {
        let base = self.parse_type()?;
        let mut declarations = Vec::new();
        loop {
            let (name, line, column) = self.expect_identifier()?;
            let ty = Self::with_dimensions(base.clone(), self.array_dimensions()?);
            declarations.push(VariableDeclaration {
                name,
                ty,
                line,
                column,
            });
            if !self.match_symbol(",") {
                break;
            }
        }
        self.expect_symbol(";")?;
        Ok(declarations)
    }

    fn assignment_or_call(&mut self) -> Result<StatementKind, Vec<Diagnostic>> {
        let (name, line, column) = self.expect_identifier()?;
        let target = Expression {
            kind: ExpressionKind::Variable(name.clone()),
            line,
            column,
        };
        if self.check_symbol("(") {
            self.expect_symbol("(")?;
            let arguments = self.call_arguments()?;
            self.expect_symbol(";")?;
            return Ok(StatementKind::Call { name, arguments });
        }
        let mut target = target;
        while self.match_symbol("[") {
            let index = self.expression()?;
            self.expect_symbol("]")?;
            target = Expression {
                kind: ExpressionKind::Index(Box::new(target), Box::new(index)),
                line,
                column,
            };
        }
        self.expect_symbol("=")?;
        let value = self.expression()?;
        self.expect_symbol(";")?;
        Ok(StatementKind::Assign { target, value })
    }

    fn if_statement(&mut self) -> Result<StatementKind, Vec<Diagnostic>> {
        let condition = self.condition()?;
        self.expect_word("entao")?;
        let mut branches = vec![(condition, self.body(&["senaoSe", "senao", "fimSe"])?)];
        while self.match_word("senaoSe") {
            let condition = self.condition()?;
            self.expect_word("entao")?;
            branches.push((condition, self.body(&["senaoSe", "senao", "fimSe"])?));
        }
        let otherwise = if self.match_word("senao") {
            self.body(&["fimSe"])?
        } else {
            Vec::new()
        };
        self.expect_word("fimSe")?;
        Ok(StatementKind::If {
            branches,
            otherwise,
        })
    }

    fn while_statement(&mut self) -> Result<StatementKind, Vec<Diagnostic>> {
        let condition = self.condition()?;
        self.expect_word("faca")?;
        let body = self.body(&["fimEnquanto"])?;
        self.expect_word("fimEnquanto")?;
        Ok(StatementKind::While { condition, body })
    }

    fn do_while_statement(&mut self) -> Result<StatementKind, Vec<Diagnostic>> {
        let body = self.body(&["enquanto"])?;
        self.expect_word("enquanto")?;
        let condition = self.condition()?;
        self.expect_symbol(";")?;
        Ok(StatementKind::DoWhile { body, condition })
    }

    fn for_statement(&mut self) -> Result<StatementKind, Vec<Diagnostic>> {
        let parenthesized = self.match_symbol("(");
        let (variable, _, _) = self.expect_identifier()?;
        self.expect_word("de")?;
        let start = self.expression()?;
        self.expect_word("ate")?;
        let end = self.expression()?;
        self.expect_word("passo")?;
        let step = self.expression()?;
        if parenthesized {
            self.expect_symbol(")")?;
        }
        self.expect_word("faca")?;
        let body = self.body(&["fimPara"])?;
        self.expect_word("fimPara")?;
        Ok(StatementKind::For {
            variable,
            start,
            end,
            step,
            body,
        })
    }

    fn choose_statement(&mut self) -> Result<StatementKind, Vec<Diagnostic>> {
        let value = self.condition()?;
        let mut cases = Vec::new();
        while self.match_word("caso") {
            let candidate = self.expression()?;
            let body = self.body(&["caso", "outroCaso", "fimEscolha"])?;
            cases.push((candidate, body));
        }
        let otherwise = if self.match_word("outroCaso") {
            self.body(&["fimEscolha"])?
        } else {
            Vec::new()
        };
        self.expect_word("fimEscolha")?;
        if cases.is_empty() && otherwise.is_empty() {
            return Err(vec![
                self.error("'escolha' precisa conter pelo menos um 'caso'"),
            ]);
        }
        Ok(StatementKind::Choose {
            value,
            cases,
            otherwise,
        })
    }

    fn condition(&mut self) -> Result<Expression, Vec<Diagnostic>> {
        if self.match_symbol("(") {
            let expression = self.expression()?;
            self.expect_symbol(")")?;
            Ok(expression)
        } else {
            self.expression()
        }
    }

    fn expression_arguments(&mut self) -> Result<Vec<Expression>, Vec<Diagnostic>> {
        self.expect_symbol("(")?;
        let mut arguments = Vec::new();
        if self.match_symbol(")") {
            self.expect_symbol(";")?;
            return Ok(arguments);
        }
        loop {
            arguments.push(self.expression()?);
            if self.match_symbol(")") {
                break;
            }
            self.expect_symbol(",")?;
        }
        self.expect_symbol(";")?;
        Ok(arguments)
    }

    fn empty_call_arguments(&mut self) -> Result<(), Vec<Diagnostic>> {
        self.expect_symbol("(")?;
        self.expect_symbol(")")
    }

    fn expression(&mut self) -> Result<Expression, Vec<Diagnostic>> {
        self.expression_with_precedence(1)
    }

    fn expression_with_precedence(&mut self, minimum: u8) -> Result<Expression, Vec<Diagnostic>> {
        self.depth += 1;
        if self.depth > self.max_depth {
            self.depth -= 1;
            return Err(vec![
                self.error("expressão excede o limite de profundidade"),
            ]);
        }
        let mut left = self.prefix()?;
        loop {
            if self.check_symbol(":") && minimum <= 7 {
                self.advance();
                let width = self.dimension_size()?;
                let decimals = if self.match_symbol(":") {
                    Some(self.dimension_size()?)
                } else {
                    None
                };
                left = Expression {
                    line: left.line,
                    column: left.column,
                    kind: ExpressionKind::Formatted {
                        value: Box::new(left),
                        width,
                        decimals,
                    },
                };
                continue;
            }
            let Some((operator, precedence)) = self.binary_operator() else {
                break;
            };
            if precedence < minimum {
                break;
            }
            self.advance();
            let right = self.expression_with_precedence(precedence + 1)?;
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
        self.depth -= 1;
        Ok(left)
    }

    fn prefix(&mut self) -> Result<Expression, Vec<Diagnostic>> {
        let token = self.advance().clone();
        let mut expression = match token.kind {
            TokenKind::Integer(value) => Expression {
                kind: ExpressionKind::Integer(value.parse().map_err(|_| {
                    vec![Diagnostic::new(
                        DiagnosticKind::Syntax,
                        "literal inteiro fora do limite",
                        token.line,
                        token.column,
                    )]
                })?),
                line: token.line,
                column: token.column,
            },
            TokenKind::Real(value) => {
                let value = value.parse::<f64>().map_err(|_| {
                    vec![Diagnostic::new(
                        DiagnosticKind::Syntax,
                        "literal real inválido",
                        token.line,
                        token.column,
                    )]
                })?;
                if !value.is_finite() {
                    return Err(vec![Diagnostic::new(
                        DiagnosticKind::Syntax,
                        "literal real fora do intervalo",
                        token.line,
                        token.column,
                    )]);
                }
                Expression {
                    kind: ExpressionKind::Real(value),
                    line: token.line,
                    column: token.column,
                }
            }
            TokenKind::Text(value) => Expression {
                kind: ExpressionKind::Text(value),
                line: token.line,
                column: token.column,
            },
            TokenKind::Character(value) => Expression {
                kind: ExpressionKind::Character(value),
                line: token.line,
                column: token.column,
            },
            TokenKind::Word(word) if word == "verdadeiro" => Expression {
                kind: ExpressionKind::Logical(true),
                line: token.line,
                column: token.column,
            },
            TokenKind::Word(word) if word == "falso" => Expression {
                kind: ExpressionKind::Logical(false),
                line: token.line,
                column: token.column,
            },
            TokenKind::Word(operator) if matches!(operator.as_str(), "nao") => {
                let operand = self.expression_with_precedence(7)?;
                Expression {
                    kind: ExpressionKind::Unary {
                        operator,
                        operand: Box::new(operand),
                    },
                    line: token.line,
                    column: token.column,
                }
            }
            TokenKind::Symbol(operator) if operator == "-" => {
                let operand = self.expression_with_precedence(7)?;
                Expression {
                    kind: ExpressionKind::Unary {
                        operator,
                        operand: Box::new(operand),
                    },
                    line: token.line,
                    column: token.column,
                }
            }
            TokenKind::Symbol(symbol) if symbol == "(" => {
                let expression = self.expression()?;
                self.expect_symbol(")")?;
                expression
            }
            TokenKind::Word(name) => {
                if self.check_symbol("(") {
                    self.advance();
                    let arguments = self.call_arguments()?;
                    Expression {
                        kind: ExpressionKind::Call { name, arguments },
                        line: token.line,
                        column: token.column,
                    }
                } else {
                    Expression {
                        kind: ExpressionKind::Variable(name),
                        line: token.line,
                        column: token.column,
                    }
                }
            }
            _ => {
                return Err(vec![Diagnostic::new(
                    DiagnosticKind::Syntax,
                    "esperada uma expressão",
                    token.line,
                    token.column,
                )]);
            }
        };
        while self.match_symbol("[") {
            let index = self.expression()?;
            self.expect_symbol("]")?;
            expression = Expression {
                kind: ExpressionKind::Index(Box::new(expression.clone()), Box::new(index)),
                line: expression.line,
                column: expression.column,
            };
        }
        Ok(expression)
    }

    fn call_arguments(&mut self) -> Result<Vec<Expression>, Vec<Diagnostic>> {
        let mut arguments = Vec::new();
        if self.match_symbol(")") {
            return Ok(arguments);
        }
        loop {
            arguments.push(self.expression()?);
            if self.match_symbol(")") {
                break;
            }
            self.expect_symbol(",")?;
        }
        Ok(arguments)
    }

    fn binary_operator(&self) -> Option<(String, u8)> {
        match &self.current().kind {
            TokenKind::Word(word) => match word.as_str() {
                "ou" => Some((word.clone(), 1)),
                "e" => Some((word.clone(), 2)),
                "mod" => Some((word.clone(), 6)),
                _ => None,
            },
            TokenKind::Symbol(symbol) => match symbol.as_str() {
                "=" | "==" | "!=" => Some((symbol.clone(), 3)),
                "<" | "<=" | ">" | ">=" => Some((symbol.clone(), 4)),
                "+" | "-" => Some((symbol.clone(), 5)),
                "*" | "/" | "\\" | "%" => Some((symbol.clone(), 6)),
                _ => None,
            },
            _ => None,
        }
    }

    fn parse_type(&mut self) -> Result<Type, Vec<Diagnostic>> {
        let token = self.advance().clone();
        match token.kind {
            TokenKind::Word(word) => match word.as_str() {
                "inteiro" => Ok(Type::Integer),
                "real" => Ok(Type::Real),
                "logico" => Ok(Type::Logical),
                "caracter" => Ok(Type::Character),
                "texto" => Ok(Type::Text),
                _ => Err(vec![Diagnostic::new(
                    DiagnosticKind::Syntax,
                    format!("tipo Calango desconhecido '{word}'"),
                    token.line,
                    token.column,
                )]),
            },
            _ => Err(vec![Diagnostic::new(
                DiagnosticKind::Syntax,
                "esperado um tipo Calango",
                token.line,
                token.column,
            )]),
        }
    }

    fn array_dimensions(&mut self) -> Result<Vec<Option<usize>>, Vec<Diagnostic>> {
        let mut dimensions = Vec::new();
        while self.match_symbol("[") {
            let size = if self.check_symbol("]") {
                None
            } else {
                Some(self.dimension_size()?)
            };
            self.expect_symbol("]")?;
            dimensions.push(size);
            if dimensions.len() > self.max_depth {
                return Err(vec![
                    self.error("dimensões excedem o limite de profundidade"),
                ]);
            }
        }
        Ok(dimensions)
    }

    fn dimension_size(&mut self) -> Result<usize, Vec<Diagnostic>> {
        let token = self.advance().clone();
        let TokenKind::Integer(size) = token.kind else {
            return Err(vec![Diagnostic::new(
                DiagnosticKind::Syntax,
                "esperado um número inteiro positivo",
                token.line,
                token.column,
            )]);
        };
        size.parse().map_err(|_| {
            vec![Diagnostic::new(
                DiagnosticKind::Syntax,
                "número inteiro fora do limite",
                token.line,
                token.column,
            )]
        })
    }

    fn with_dimensions(base: Type, dimensions: Vec<Option<usize>>) -> Type {
        if dimensions.is_empty() {
            base
        } else {
            Type::Array(Box::new(base), dimensions)
        }
    }

    fn is_type_word(&self) -> bool {
        ["inteiro", "real", "logico", "caracter", "texto"]
            .iter()
            .any(|word| self.check_word(word))
    }

    fn expect_identifier(&mut self) -> Result<(String, usize, usize), Vec<Diagnostic>> {
        let token = self.advance().clone();
        if let TokenKind::Word(name) = token.kind {
            Ok((name, token.line, token.column))
        } else {
            Err(vec![Diagnostic::new(
                DiagnosticKind::Syntax,
                "esperado um identificador",
                token.line,
                token.column,
            )])
        }
    }

    fn expect_word(&mut self, word: &str) -> Result<(), Vec<Diagnostic>> {
        if self.match_word(word) {
            Ok(())
        } else {
            Err(vec![self.error(format!("esperado '{word}'"))])
        }
    }

    fn expect_symbol(&mut self, symbol: &str) -> Result<(), Vec<Diagnostic>> {
        if self.match_symbol(symbol) {
            Ok(())
        } else {
            Err(vec![self.error(format!("esperado '{symbol}'"))])
        }
    }

    fn match_word(&mut self, word: &str) -> bool {
        if self.check_word(word) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_symbol(&mut self, symbol: &str) -> bool {
        if self.check_symbol(symbol) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check_word(&self, word: &str) -> bool {
        self.current().is_word(word)
    }

    fn check_symbol(&self, symbol: &str) -> bool {
        self.current().is_symbol(symbol)
    }

    fn current(&self) -> &Token {
        self.tokens
            .get(self.index)
            .unwrap_or_else(|| self.tokens.last().expect("lexer always emits EOF"))
    }

    fn previous(&self) -> &Token {
        self.tokens
            .get(self.index.saturating_sub(1))
            .unwrap_or_else(|| self.current())
    }

    fn advance(&mut self) -> &Token {
        if !self.is_end() {
            self.index += 1;
        }
        self.previous()
    }

    fn is_end(&self) -> bool {
        matches!(self.current().kind, TokenKind::End)
    }

    fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(
            DiagnosticKind::Syntax,
            message,
            self.current().line,
            self.current().column,
        )
    }
}

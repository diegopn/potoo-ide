use std::collections::HashSet;

use crate::ast::{
    ArrayDimension, ChoiceCase, DataFileDeclaration, Expression, ExpressionKind,
    FormattedExpression, NumericConstant, Parameter, Program, Routine, RoutineKind, Statement,
    StatementKind, Type, Variable,
};
use crate::lexer::{Token, TokenKind, scan};
use crate::{Diagnostic, DiagnosticKind};

pub(crate) struct Parser {
    tokens: Vec<Token>,
    index: usize,
    max_depth: usize,
}

impl Parser {
    pub(crate) fn with_max_depth(tokens: Vec<Token>, max_depth: usize) -> Self {
        Self {
            tokens,
            index: 0,
            max_depth,
        }
    }

    pub(crate) fn parse(mut self) -> Result<Program, Vec<Diagnostic>> {
        self.parse_program()
    }

    fn parse_program(&mut self) -> Result<Program, Vec<Diagnostic>> {
        self.skip_newlines();
        self.expect_keyword("algoritmo")?;
        match self.advance().kind {
            TokenKind::String(_) => {}
            _ => return self.error_current("esperado o nome do algoritmo entre aspas"),
        }
        self.finish_line()?;

        let mut globals = Vec::new();
        let mut global_names = HashSet::new();
        let mut routines = Vec::new();
        let mut routine_names = HashSet::new();
        let mut arquivo = None;

        loop {
            self.skip_newlines();
            if self.at_keyword("inicio") {
                break;
            }
            if self.at_eof() {
                return self.error_current("esperado 'inicio' antes do fim do arquivo");
            }
            if self.match_keyword("var") {
                globals.extend(self.parse_declarations(&mut global_names, true)?);
            } else if self.match_keyword("arquivo") {
                if arquivo.is_some() {
                    return self.error_previous("a declaração 'arquivo' só pode aparecer uma vez");
                }
                let token = self.advance();
                let path = match token.kind {
                    TokenKind::String(path) => path,
                    _ => return self.error_current("esperado o nome do arquivo entre aspas"),
                };
                arquivo = Some(DataFileDeclaration {
                    path,
                    line: token.line,
                    column: token.column,
                });
                self.finish_line()?;
            } else if self.at_keyword("procedimento") || self.at_keyword("funcao") {
                let routine = self.parse_routine()?;
                if !routine_names.insert(routine.name.clone()) {
                    return Err(vec![Diagnostic::new(
                        DiagnosticKind::Syntax,
                        format!("rotina duplicada: '{}'", routine.name),
                        routine.line,
                        routine.column,
                    )]);
                }
                routines.push(routine);
            } else {
                return self.error_current("esperado 'var', 'arquivo', uma rotina ou 'inicio'");
            }
        }

        self.expect_keyword("inicio")?;
        self.finish_line()?;
        let statements = self.parse_statements_until(&["fimalgoritmo"], 0)?;
        self.expect_keyword("fimalgoritmo")?;

        Ok(Program {
            globals,
            arquivo,
            routines,
            statements,
        })
    }

    fn parse_declarations(
        &mut self,
        names_in_scope: &mut HashSet<String>,
        top_level: bool,
    ) -> Result<Vec<Variable>, Vec<Diagnostic>> {
        let mut declarations = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_eof()
                || self.at_keyword("inicio")
                || (top_level
                    && (self.at_keyword("arquivo")
                        || self.at_keyword("procedimento")
                        || self.at_keyword("funcao")))
            {
                return Ok(declarations);
            }
            declarations.extend(self.parse_declaration_line(names_in_scope)?);
            self.finish_line()?;
        }
    }

    fn parse_declaration_line(
        &mut self,
        names_in_scope: &mut HashSet<String>,
    ) -> Result<Vec<Variable>, Vec<Diagnostic>> {
        let names = self.parse_name_list()?;
        self.expect_symbol(':')?;
        let (data_type, dimensions) = self.parse_type_specification()?;
        let mut variables = Vec::with_capacity(names.len());
        for (name, token) in names {
            if !names_in_scope.insert(name.clone()) {
                return Err(vec![Diagnostic::new(
                    DiagnosticKind::Syntax,
                    format!("nome duplicado no mesmo escopo: '{name}'"),
                    token.line,
                    token.column,
                )]);
            }
            variables.push(Variable {
                name,
                data_type,
                dimensions: dimensions.clone(),
                line: token.line,
                column: token.column,
            });
        }
        self.match_symbol(';');
        Ok(variables)
    }

    fn parse_type_specification(&mut self) -> Result<(Type, Vec<ArrayDimension>), Vec<Diagnostic>> {
        let dimensions = if self.match_keyword("vetor") {
            self.expect_symbol('[')?;
            let mut dimensions = Vec::new();
            loop {
                let (lower, lower_token) = self.parse_signed_integer()?;
                self.expect_operator("..")?;
                let (upper, _) = self.parse_signed_integer()?;
                if lower >= upper {
                    return Err(vec![Diagnostic::new(
                        DiagnosticKind::Syntax,
                        "o limite final do vetor deve ser maior que o limite inicial",
                        lower_token.line,
                        lower_token.column,
                    )]);
                }
                dimensions.push(ArrayDimension { lower, upper });
                if !self.match_symbol(',') {
                    break;
                }
                if dimensions.len() >= 2 {
                    return self.error_current("vetores podem ter no máximo duas dimensões");
                }
            }
            self.expect_symbol(']')?;
            self.expect_keyword("de")?;
            dimensions
        } else {
            Vec::new()
        };
        let data_type = self.parse_scalar_type()?;
        Ok((data_type, dimensions))
    }

    fn parse_scalar_type(&mut self) -> Result<Type, Vec<Diagnostic>> {
        let token = self.current().clone();
        let TokenKind::Identifier(name) = &token.kind else {
            return self.error_current("esperado um tipo: inteiro, real, caractere ou logico");
        };
        let data_type = match name.as_str() {
            "inteiro" => Type::Integer,
            "real" => Type::Real,
            "caractere" => Type::Character,
            "logico" => Type::Logical,
            _ => return self.error_current("tipo VisuAlg desconhecido"),
        };
        self.advance();
        Ok(data_type)
    }

    fn parse_signed_integer(&mut self) -> Result<(i64, Token), Vec<Diagnostic>> {
        let negative = if self.match_operator("-") {
            true
        } else {
            self.match_operator("+");
            false
        };
        let token = self.current().clone();
        let TokenKind::Integer(value) = token.kind else {
            return self.error_current("esperado um limite inteiro constante para o vetor");
        };
        self.advance();
        let value = if negative {
            value.checked_neg().ok_or_else(|| {
                vec![Diagnostic::new(
                    DiagnosticKind::Syntax,
                    "limite inteiro fora do intervalo permitido",
                    token.line,
                    token.column,
                )]
            })?
        } else {
            value
        };
        Ok((value, token))
    }

    fn parse_name_list(&mut self) -> Result<Vec<(String, Token)>, Vec<Diagnostic>> {
        let mut names = vec![self.expect_identifier()?];
        while self.match_symbol(',') {
            names.push(self.expect_identifier()?);
        }
        Ok(names)
    }

    fn parse_routine(&mut self) -> Result<Routine, Vec<Diagnostic>> {
        let token = self.advance();
        let (kind, end_keyword) = match token.kind {
            TokenKind::Identifier(name) if name == "procedimento" => {
                (RoutineKind::Procedure, "fimprocedimento")
            }
            TokenKind::Identifier(name) if name == "funcao" => (RoutineKind::Function, "fimfuncao"),
            _ => unreachable!("only routine declarations enter this function"),
        };
        let (name, name_token) = self.expect_identifier()?;
        let parameters = self.parse_parameters()?;
        let return_type = if kind == RoutineKind::Function && self.match_symbol(':') {
            Some(self.parse_scalar_type()?)
        } else {
            None
        };
        if kind == RoutineKind::Function && return_type.is_none() {
            return self.error_current("a função precisa declarar o tipo do resultado");
        }
        if kind == RoutineKind::Procedure && return_type.is_some() {
            return self.error_current("um procedimento não declara tipo de retorno");
        }
        self.finish_line()?;

        let mut local_names = parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .collect::<HashSet<_>>();
        let mut locals = Vec::new();
        while self.match_keyword("var") {
            locals.extend(self.parse_declarations(&mut local_names, false)?);
        }
        self.expect_keyword("inicio")?;
        self.finish_line()?;
        let body = self.parse_statements_until(&[end_keyword], 0)?;
        self.expect_keyword(end_keyword)?;
        self.finish_line()?;

        Ok(Routine {
            name,
            kind,
            parameters,
            return_type,
            locals,
            body,
            line: token.line,
            column: name_token.column,
        })
    }

    fn parse_parameters(&mut self) -> Result<Vec<Parameter>, Vec<Diagnostic>> {
        if !self.match_symbol('(') {
            return Ok(Vec::new());
        }
        let mut parameters = Vec::new();
        if self.match_symbol(')') {
            return Ok(parameters);
        }
        loop {
            let by_reference = self.match_keyword("var");
            let names = self.parse_name_list()?;
            self.expect_symbol(':')?;
            let (data_type, dimensions) = self.parse_type_specification()?;
            for (name, token) in names {
                if parameters
                    .iter()
                    .any(|parameter: &Parameter| parameter.name == name)
                {
                    return Err(vec![Diagnostic::new(
                        DiagnosticKind::Syntax,
                        format!("parâmetro duplicado: '{name}'"),
                        token.line,
                        token.column,
                    )]);
                }
                parameters.push(Parameter {
                    name,
                    data_type,
                    by_reference,
                    dimensions: dimensions.len(),
                    bounds: dimensions.clone(),
                });
            }
            if self.match_symbol(')') {
                break;
            }
            self.expect_symbol(';')?;
        }
        Ok(parameters)
    }

    fn parse_statements_until(
        &mut self,
        stop_keywords: &[&str],
        depth: usize,
    ) -> Result<Vec<Statement>, Vec<Diagnostic>> {
        let mut statements = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_eof() || stop_keywords.iter().any(|keyword| self.at_keyword(keyword)) {
                return Ok(statements);
            }
            statements.push(self.parse_statement(depth)?);
            self.finish_line()?;
        }
    }

    fn parse_statement(&mut self, depth: usize) -> Result<Statement, Vec<Diagnostic>> {
        let token = self.current().clone();
        if self.match_keyword("escreva") {
            let values = self.parse_formatted_list()?;
            return Ok(Statement {
                kind: StatementKind::Write {
                    newline: false,
                    values,
                },
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("escreval") {
            let values = self.parse_formatted_list()?;
            return Ok(Statement {
                kind: StatementKind::Write {
                    newline: true,
                    values,
                },
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("leia") {
            let expressions = self.parse_argument_list('(')?;
            return Ok(Statement {
                kind: StatementKind::Read(expressions),
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("se") {
            return self.parse_if(token, depth);
        }
        if self.match_keyword("escolha") {
            return self.parse_choice(token, depth);
        }
        if self.match_keyword("enquanto") {
            return self.parse_while(token, depth);
        }
        if self.match_keyword("repita") {
            return self.parse_repeat(token, depth);
        }
        if self.match_keyword("para") {
            return self.parse_for(token, depth);
        }
        if self.match_keyword("interrompa") {
            return Ok(Statement {
                kind: StatementKind::Break,
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("aleatorio") {
            return self.parse_random_command(token);
        }
        if self.match_keyword("timer") {
            let millis = if self.match_keyword("off") {
                None
            } else if self.match_keyword("on") {
                Some(500)
            } else if self.at_newline() || self.at_eof() {
                Some(500)
            } else {
                let (value, _) = self.parse_signed_integer()?;
                Some(value.clamp(0, 10_000) as u64)
            };
            return Ok(Statement {
                kind: StatementKind::Timer(millis),
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("pausa") {
            return Ok(Statement {
                kind: StatementKind::Pause,
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("debug") {
            let condition = self.parse_expression(0, depth + 1)?;
            return Ok(Statement {
                kind: StatementKind::Debug(condition),
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("eco") {
            let enabled = self.parse_on_off(&token)?;
            return Ok(Statement {
                kind: StatementKind::Echo(enabled),
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("cronometro") {
            let enabled = self.parse_on_off(&token)?;
            return Ok(Statement {
                kind: StatementKind::Chronometer(enabled),
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("limpatela") {
            return Ok(Statement {
                kind: StatementKind::ClearScreen,
                line: token.line,
                column: token.column,
            });
        }
        if self.match_keyword("retorne") {
            let value = if self.at_newline() || self.at_eof() {
                None
            } else {
                Some(self.parse_expression(0, depth + 1)?)
            };
            return Ok(Statement {
                kind: StatementKind::Return(value),
                line: token.line,
                column: token.column,
            });
        }

        let target_or_expression = self.parse_expression(0, depth + 1)?;
        let expression = if self.match_operator("<-") {
            if !matches!(
                &target_or_expression.kind,
                ExpressionKind::Identifier(_) | ExpressionKind::Index { .. }
            ) {
                return Err(vec![Diagnostic::new(
                    DiagnosticKind::Syntax,
                    "o lado esquerdo de '<-' precisa ser uma variável",
                    target_or_expression.line,
                    target_or_expression.column,
                )]);
            }
            let value = self.parse_expression(0, depth + 1)?;
            Expression {
                line: target_or_expression.line,
                column: target_or_expression.column,
                kind: ExpressionKind::Assign {
                    target: Box::new(target_or_expression),
                    value: Box::new(value),
                },
            }
        } else {
            target_or_expression
        };
        Ok(Statement {
            kind: StatementKind::Expression(expression),
            line: token.line,
            column: token.column,
        })
    }

    fn parse_if(&mut self, token: Token, depth: usize) -> Result<Statement, Vec<Diagnostic>> {
        if depth >= self.max_depth {
            return Err(vec![Diagnostic::new(
                DiagnosticKind::Syntax,
                format!(
                    "profundidade máxima de blocos ({}) excedida",
                    self.max_depth
                ),
                token.line,
                token.column,
            )]);
        }
        let condition = self.parse_expression(0, depth + 1)?;
        self.expect_keyword("entao")?;
        self.finish_line()?;
        let then_branch = self.parse_statements_until(&["senao", "fimse"], depth + 1)?;
        let else_branch = if self.match_keyword("senao") {
            self.finish_line()?;
            self.parse_statements_until(&["fimse"], depth + 1)?
        } else {
            Vec::new()
        };
        self.expect_keyword("fimse")?;
        Ok(Statement {
            kind: StatementKind::If {
                condition,
                then_branch,
                else_branch,
            },
            line: token.line,
            column: token.column,
        })
    }

    fn parse_choice(&mut self, token: Token, depth: usize) -> Result<Statement, Vec<Diagnostic>> {
        self.check_block_depth(&token, depth)?;
        let expression = self.parse_expression(0, depth + 1)?;
        self.finish_line()?;
        let mut cases = Vec::new();
        let mut default = Vec::new();
        loop {
            self.skip_newlines();
            if self.at_keyword("fimescolha") || self.at_eof() {
                break;
            }
            if self.match_keyword("caso") {
                let mut values = vec![self.parse_expression(0, depth + 1)?];
                while self.match_symbol(',') {
                    values.push(self.parse_expression(0, depth + 1)?);
                }
                self.finish_line()?;
                let body =
                    self.parse_statements_until(&["caso", "outrocaso", "fimescolha"], depth + 1)?;
                cases.push(ChoiceCase { values, body });
            } else if self.match_keyword("outrocaso") {
                self.finish_line()?;
                default = self.parse_statements_until(&["fimescolha"], depth + 1)?;
                self.skip_newlines();
                if !self.at_keyword("fimescolha") {
                    return self.error_current("'outrocaso' deve ser o último bloco de 'escolha'");
                }
                break;
            } else {
                return self.error_current("esperado 'caso', 'outrocaso' ou 'fimescolha'");
            }
        }
        self.expect_keyword("fimescolha")?;
        Ok(Statement {
            kind: StatementKind::Choice {
                expression,
                cases,
                default,
            },
            line: token.line,
            column: token.column,
        })
    }

    fn parse_while(&mut self, token: Token, depth: usize) -> Result<Statement, Vec<Diagnostic>> {
        self.check_block_depth(&token, depth)?;
        let condition = self.parse_expression(0, depth + 1)?;
        self.expect_keyword("faca")?;
        self.finish_line()?;
        let body = self.parse_statements_until(&["fimenquanto"], depth + 1)?;
        self.expect_keyword("fimenquanto")?;
        Ok(Statement {
            kind: StatementKind::While { condition, body },
            line: token.line,
            column: token.column,
        })
    }

    fn parse_repeat(&mut self, token: Token, depth: usize) -> Result<Statement, Vec<Diagnostic>> {
        self.check_block_depth(&token, depth)?;
        self.finish_line()?;
        let body = self.parse_statements_until(&["ate", "fimrepita"], depth + 1)?;
        let condition = if self.match_keyword("ate") {
            Some(self.parse_expression(0, depth + 1)?)
        } else {
            self.expect_keyword("fimrepita")?;
            None
        };
        Ok(Statement {
            kind: StatementKind::Repeat { body, condition },
            line: token.line,
            column: token.column,
        })
    }

    fn parse_for(&mut self, token: Token, depth: usize) -> Result<Statement, Vec<Diagnostic>> {
        self.check_block_depth(&token, depth)?;
        let (variable, _) = self.expect_identifier()?;
        self.expect_keyword("de")?;
        let start = self.parse_expression(0, depth + 1)?;
        self.expect_keyword("ate")?;
        let end = self.parse_expression(0, depth + 1)?;
        let step = if self.match_keyword("passo") {
            Some(self.parse_expression(0, depth + 1)?)
        } else {
            None
        };
        self.expect_keyword("faca")?;
        self.finish_line()?;
        let body = self.parse_statements_until(&["fimpara"], depth + 1)?;
        self.expect_keyword("fimpara")?;
        Ok(Statement {
            kind: StatementKind::For {
                variable,
                start,
                end,
                step,
                body,
            },
            line: token.line,
            column: token.column,
        })
    }

    fn check_block_depth(&self, token: &Token, depth: usize) -> Result<(), Vec<Diagnostic>> {
        if depth >= self.max_depth {
            return Err(vec![Diagnostic::new(
                DiagnosticKind::Syntax,
                format!(
                    "profundidade máxima de blocos ({}) excedida",
                    self.max_depth
                ),
                token.line,
                token.column,
            )]);
        }
        Ok(())
    }

    fn parse_on_off(&mut self, token: &Token) -> Result<bool, Vec<Diagnostic>> {
        if self.match_keyword("on") {
            Ok(true)
        } else if self.match_keyword("off") {
            Ok(false)
        } else {
            Err(vec![Diagnostic::new(
                DiagnosticKind::Syntax,
                "esperado 'on' ou 'off'",
                token.line,
                token.column,
            )])
        }
    }

    fn parse_random_command(&mut self, token: Token) -> Result<Statement, Vec<Diagnostic>> {
        let (enabled, minimum, maximum) = if self.match_keyword("off") {
            (
                false,
                NumericConstant::Integer(0),
                NumericConstant::Integer(100),
            )
        } else if self.match_keyword("on") || self.at_newline() || self.at_eof() {
            (
                true,
                NumericConstant::Integer(0),
                NumericConstant::Integer(100),
            )
        } else {
            let first = self.parse_signed_number()?;
            let (minimum, maximum) = if self.match_symbol(',') {
                let second = self.parse_signed_number()?;
                if first.compare(second).is_gt() {
                    (second, first)
                } else {
                    (first, second)
                }
            } else {
                let zero = NumericConstant::Integer(0);
                if first.compare(zero).is_gt() {
                    (zero, first)
                } else {
                    (first, zero)
                }
            };
            (true, minimum, maximum)
        };
        Ok(Statement {
            kind: StatementKind::Aleatorio {
                enabled,
                minimum,
                maximum,
            },
            line: token.line,
            column: token.column,
        })
    }

    fn parse_signed_number(&mut self) -> Result<NumericConstant, Vec<Diagnostic>> {
        let negative = if self.match_operator("-") {
            true
        } else {
            self.match_operator("+");
            false
        };
        let token = self.current().clone();
        let value = match token.kind {
            TokenKind::Integer(value) => NumericConstant::Integer(if negative {
                value.checked_neg().ok_or_else(|| {
                    vec![Diagnostic::new(
                        DiagnosticKind::Syntax,
                        "limite inteiro fora do intervalo permitido",
                        token.line,
                        token.column,
                    )]
                })?
            } else {
                value
            }),
            TokenKind::Real(value) => NumericConstant::Real(if negative { -value } else { value }),
            _ => return self.error_current("esperado um limite numérico constante para aleatorio"),
        };
        self.advance();
        Ok(value)
    }

    fn parse_formatted_list(&mut self) -> Result<Vec<FormattedExpression>, Vec<Diagnostic>> {
        self.expect_symbol('(')?;
        let mut values = vec![self.parse_formatted_expression(0)?];
        while self.match_symbol(',') {
            values.push(self.parse_formatted_expression(0)?);
        }
        self.expect_symbol(')')?;
        Ok(values)
    }

    fn parse_formatted_expression(
        &mut self,
        depth: usize,
    ) -> Result<FormattedExpression, Vec<Diagnostic>> {
        let expression = self.parse_expression(0, depth + 1)?;
        let width = if self.match_symbol(':') {
            Some(self.parse_expression(0, depth + 1)?)
        } else {
            None
        };
        let precision = if self.match_symbol(':') {
            Some(self.parse_expression(0, depth + 1)?)
        } else {
            None
        };
        Ok(FormattedExpression {
            expression,
            width,
            precision,
        })
    }

    fn parse_argument_list(&mut self, opening: char) -> Result<Vec<Expression>, Vec<Diagnostic>> {
        if !self.match_symbol(opening) {
            return self.error_current("esperado '(' antes da lista de argumentos");
        }
        let mut arguments = Vec::new();
        if self.match_symbol(')') {
            return self.error_previous("a lista de variáveis não pode estar vazia");
        }
        loop {
            arguments.push(self.parse_expression(0, 1)?);
            if self.match_symbol(')') {
                break;
            }
            self.expect_symbol(',')?;
        }
        Ok(arguments)
    }

    fn parse_expression(
        &mut self,
        minimum_precedence: u8,
        depth: usize,
    ) -> Result<Expression, Vec<Diagnostic>> {
        self.check_depth(depth)?;
        let mut left = self.parse_prefix(depth)?;
        loop {
            let Some((operator, precedence, right_associative)) = self.current_infix() else {
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
            let right = self.parse_expression(next_minimum, depth + 1)?;
            let location = (left.line, left.column);
            let kind = ExpressionKind::Binary {
                left: Box::new(left),
                operator,
                right: Box::new(right),
            };
            left = Expression {
                kind,
                line: location.0,
                column: location.1,
            };
        }
        Ok(left)
    }

    fn parse_prefix(&mut self, depth: usize) -> Result<Expression, Vec<Diagnostic>> {
        let token = self.current().clone();
        let unary_operator = match &token.kind {
            TokenKind::Operator(operator) if matches!(operator.as_str(), "+" | "-") => {
                Some(operator.clone())
            }
            TokenKind::Identifier(operator) if operator == "nao" => Some(operator.clone()),
            _ => None,
        };
        if let Some(operator) = unary_operator {
            self.advance();
            let operand = self.parse_expression(7, depth + 1)?;
            return Ok(Expression {
                kind: ExpressionKind::Unary {
                    operator,
                    operand: Box::new(operand),
                },
                line: token.line,
                column: token.column,
            });
        }

        match token.kind {
            TokenKind::Integer(value) => {
                self.advance();
                Ok(Expression {
                    kind: ExpressionKind::Integer(value),
                    line: token.line,
                    column: token.column,
                })
            }
            TokenKind::Real(value) => {
                self.advance();
                Ok(Expression {
                    kind: ExpressionKind::Real(value),
                    line: token.line,
                    column: token.column,
                })
            }
            TokenKind::String(value) => {
                self.advance();
                Ok(Expression {
                    kind: ExpressionKind::String(value),
                    line: token.line,
                    column: token.column,
                })
            }
            TokenKind::Symbol('(') => {
                self.advance();
                let expression = self.parse_expression(0, depth + 1)?;
                self.expect_symbol(')')?;
                Ok(expression)
            }
            TokenKind::Identifier(name) if name == "verdadeiro" || name == "falso" => {
                self.advance();
                Ok(Expression {
                    kind: ExpressionKind::Logical(name == "verdadeiro"),
                    line: token.line,
                    column: token.column,
                })
            }
            TokenKind::Identifier(name) if !is_reserved_word(&name) => {
                self.advance();
                if self.match_symbol('(') {
                    let mut arguments = Vec::new();
                    if !self.match_symbol(')') {
                        loop {
                            arguments.push(self.parse_expression(0, depth + 1)?);
                            if self.match_symbol(')') {
                                break;
                            }
                            self.expect_symbol(',')?;
                        }
                    }
                    Ok(Expression {
                        kind: ExpressionKind::Call { name, arguments },
                        line: token.line,
                        column: token.column,
                    })
                } else if self.match_symbol('[') {
                    let mut indexes = Vec::new();
                    loop {
                        indexes.push(self.parse_expression(0, depth + 1)?);
                        if self.match_symbol(']') {
                            break;
                        }
                        self.expect_symbol(',')?;
                    }
                    Ok(Expression {
                        kind: ExpressionKind::Index { name, indexes },
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
            _ => self.error_current("esperada uma expressão"),
        }
    }

    fn current_infix(&self) -> Option<(String, u8, bool)> {
        let (operator, precedence, right_associative) = match &self.current().kind {
            TokenKind::Operator(operator) => match operator.as_str() {
                "=" | "<>" | "<" | "<=" | ">" | ">=" => (operator.as_str(), 3, false),
                "+" | "-" => (operator.as_str(), 4, false),
                "*" | "/" | "\\" | "%" => (operator.as_str(), 5, false),
                "^" => (operator.as_str(), 6, true),
                _ => return None,
            },
            TokenKind::Identifier(operator) => match operator.as_str() {
                "ou" | "xou" => (operator.as_str(), 1, false),
                "e" => (operator.as_str(), 2, false),
                "mod" => (operator.as_str(), 5, false),
                _ => return None,
            },
            _ => return None,
        };
        Some((operator.to_owned(), precedence, right_associative))
    }

    fn check_depth(&self, depth: usize) -> Result<(), Vec<Diagnostic>> {
        if depth > self.max_depth {
            return self.error_current(&format!(
                "profundidade máxima de análise ({}) excedida",
                self.max_depth
            ));
        }
        Ok(())
    }

    fn expect_identifier(&mut self) -> Result<(String, Token), Vec<Diagnostic>> {
        let token = self.current().clone();
        let TokenKind::Identifier(name) = &token.kind else {
            return self.error_current("esperado um identificador");
        };
        if is_reserved_word(name) {
            return self.error_current("palavra reservada não pode ser usada como identificador");
        }
        if name.chars().count() > 30 {
            return Err(vec![Diagnostic::new(
                DiagnosticKind::Syntax,
                "identificadores podem ter no máximo 30 caracteres",
                token.line,
                token.column,
            )]);
        }
        self.advance();
        Ok((name.clone(), token))
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<Token, Vec<Diagnostic>> {
        if self.at_keyword(keyword) {
            Ok(self.advance())
        } else {
            self.error_current(&format!("esperado '{keyword}'"))
        }
    }

    fn match_keyword(&mut self, keyword: &str) -> bool {
        if self.at_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at_keyword(&self, keyword: &str) -> bool {
        matches!(&self.current().kind, TokenKind::Identifier(name) if name == keyword)
    }

    fn expect_symbol(&mut self, symbol: char) -> Result<Token, Vec<Diagnostic>> {
        if self.match_symbol(symbol) {
            Ok(self.previous().clone())
        } else {
            self.error_current(&format!("esperado '{symbol}'"))
        }
    }

    fn match_symbol(&mut self, symbol: char) -> bool {
        if self.current().kind == TokenKind::Symbol(symbol) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_operator(&mut self, operator: &str) -> Result<Token, Vec<Diagnostic>> {
        if self.match_operator(operator) {
            Ok(self.previous().clone())
        } else {
            self.error_current(&format!("esperado '{operator}'"))
        }
    }

    fn match_operator(&mut self, operator: &str) -> bool {
        if matches!(&self.current().kind, TokenKind::Operator(value) if value == operator) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn finish_line(&mut self) -> Result<(), Vec<Diagnostic>> {
        if matches!(self.current().kind, TokenKind::Newline) {
            self.skip_newlines();
            return Ok(());
        }
        if self.at_eof() {
            return Ok(());
        }
        self.error_current("esperado o fim da linha")
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current().kind, TokenKind::Newline) {
            self.advance();
        }
    }

    fn at_newline(&self) -> bool {
        matches!(self.current().kind, TokenKind::Newline)
    }

    fn at_eof(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index.min(self.tokens.len() - 1)]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.index.saturating_sub(1)]
    }

    fn advance(&mut self) -> Token {
        let token = self.current().clone();
        if !matches!(token.kind, TokenKind::Eof) {
            self.index += 1;
        }
        token
    }

    fn error_current<T>(&self, message: &str) -> Result<T, Vec<Diagnostic>> {
        let token = self.current();
        Err(vec![Diagnostic::new(
            DiagnosticKind::Syntax,
            message,
            token.line,
            token.column,
        )])
    }

    fn error_previous<T>(&self, message: &str) -> Result<T, Vec<Diagnostic>> {
        let token = self.previous();
        Err(vec![Diagnostic::new(
            DiagnosticKind::Syntax,
            message,
            token.line,
            token.column,
        )])
    }
}

fn is_reserved_word(name: &str) -> bool {
    matches!(
        name,
        "algoritmo"
            | "var"
            | "arquivo"
            | "inicio"
            | "fimalgoritmo"
            | "procedimento"
            | "fimprocedimento"
            | "funcao"
            | "fimfuncao"
            | "se"
            | "entao"
            | "senao"
            | "fimse"
            | "escolha"
            | "caso"
            | "outrocaso"
            | "fimescolha"
            | "enquanto"
            | "faca"
            | "fimenquanto"
            | "repita"
            | "ate"
            | "fimrepita"
            | "para"
            | "passo"
            | "fimpara"
            | "interrompa"
            | "aleatorio"
            | "timer"
            | "pausa"
            | "debug"
            | "eco"
            | "cronometro"
            | "limpatela"
            | "escreva"
            | "escreval"
            | "leia"
            | "inteiro"
            | "real"
            | "caractere"
            | "logico"
            | "vetor"
            | "de"
            | "retorne"
            | "verdadeiro"
            | "falso"
            | "e"
            | "ou"
            | "xou"
            | "nao"
            | "mod"
    )
}

pub(crate) fn parse(source: &str, max_depth: usize) -> Result<Program, Vec<Diagnostic>> {
    Parser::with_max_depth(scan(source)?, max_depth).parse()
}

#[cfg(test)]
mod tests {
    use crate::ast::{ExpressionKind, RoutineKind, StatementKind, Type};
    use crate::lexer::scan;
    use crate::{Diagnostic, DiagnosticKind};

    use super::{Parser, parse};

    fn parse_default(source: &str) -> Result<crate::ast::Program, Vec<Diagnostic>> {
        Parser::with_max_depth(scan(source)?, 128).parse()
    }

    #[test]
    fn parses_existing_visualg_sample_and_algorithm_sections() {
        let program = parse_default(include_str!("../../../tests/fixtures/visualg-voting.alg"))
            .expect("the IDE sample should parse");

        assert_eq!(program.globals.len(), 1);
        assert_eq!(program.globals[0].name, "idade");
        assert_eq!(program.globals[0].data_type, Type::Integer);
        assert!(matches!(
            &program.statements[0].kind,
            StatementKind::Write { .. }
        ));
        assert!(matches!(
            &program.statements[1].kind,
            StatementKind::Read(_)
        ));
        let StatementKind::If {
            condition,
            then_branch,
            else_branch,
        } = &program.statements[2].kind
        else {
            panic!("the third statement should be a conditional");
        };
        assert!(matches!(&condition.kind, ExpressionKind::Binary { .. }));
        assert_eq!(then_branch.len(), 1);
        assert_eq!(else_branch.len(), 1);
    }

    #[test]
    fn parses_routines_and_one_or_two_dimensional_bounds() {
        let source = r#"algoritmo "matriz"
var
  dados: vetor[2..4, 0..1] de inteiro
procedimento troca(a, b: inteiro; var alvo: real)
var
  temporaria: caractere
inicio
  escreva(a)
fimprocedimento
funcao maior(x: inteiro): logico
inicio
  retorne x > 0
fimfuncao
inicio
  escreval(dados[2, 0])
fimalgoritmo
"#;
        let program = parse_default(source).expect("routine declarations should parse");

        assert_eq!(program.globals[0].dimensions.len(), 2);
        assert_eq!(program.globals[0].dimensions[0].lower, 2);
        assert_eq!(program.globals[0].dimensions[0].upper, 4);
        assert_eq!(program.globals[0].dimensions[1].lower, 0);
        assert_eq!(program.globals[0].dimensions[1].upper, 1);
        assert_eq!(program.routines.len(), 2);
        assert_eq!(program.routines[0].kind, RoutineKind::Procedure);
        assert_eq!(program.routines[0].parameters.len(), 3);
        assert!(program.routines[0].parameters[2].by_reference);
        assert_eq!(program.routines[0].locals[0].data_type, Type::Character);
        assert_eq!(program.routines[1].kind, RoutineKind::Function);
        assert_eq!(program.routines[1].return_type, Some(Type::Logical));
    }

    #[test]
    fn reports_malformed_endings_duplicate_names_and_invalid_bounds() {
        let missing_end = parse_default("algoritmo \"x\"\ninicio\nfimse\n");
        assert!(
            missing_end
                .expect_err("a missing algorithm terminator should fail")
                .iter()
                .any(|diagnostic| diagnostic.kind == DiagnosticKind::Syntax)
        );

        let duplicate =
            parse_default("algoritmo \"x\"\nvar\na: inteiro\na: real\ninicio\nfimalgoritmo\n");
        assert!(
            duplicate
                .expect_err("duplicate declarations should fail")
                .iter()
                .any(|diagnostic| diagnostic.message.contains("duplic"))
        );

        let invalid_bounds = parse_default(
            "algoritmo \"x\"\nvar\na: vetor[5..1] de inteiro\ninicio\nfimalgoritmo\n",
        );
        let diagnostics = invalid_bounds.expect_err("reversed bounds should fail");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Syntax);
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (3, 10));
    }

    #[test]
    fn rejects_parser_nesting_over_128() {
        let mut source = String::from("algoritmo \"fundo\"\ninicio\n");
        for _ in 0..129 {
            source.push_str("se verdadeiro entao\n");
        }
        source.push_str("escreval(1)\n");
        for _ in 0..129 {
            source.push_str("fimse\n");
        }
        source.push_str("fimalgoritmo\n");

        let diagnostics = parse_default(&source).expect_err("over-nested blocks should fail");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Syntax);
        assert!(diagnostics[0].message.contains("profundidade"));
    }

    #[test]
    fn parses_arquivo_declaration_and_preserves_the_declared_path() {
        let program = parse(
            "algoritmo \"x\"\nvar\na: inteiro\narquivo \"dados.txt\"\ninicio\nfimalgoritmo\n",
            128,
        )
        .expect("the data-file directive should parse");
        assert_eq!(
            program
                .arquivo
                .as_ref()
                .map(|arquivo| arquivo.path.as_str()),
            Some("dados.txt")
        );
    }

    #[test]
    fn sorts_the_endpoints_of_single_value_random_ranges() {
        let program = parse(
            "algoritmo \"aleatorio\"\ninicio\naleatorio -3\nfimalgoritmo\n",
            128,
        )
        .expect("a single negative maximum should form a valid range");
        let StatementKind::Aleatorio {
            enabled,
            minimum,
            maximum,
        } = program.statements[0].kind
        else {
            panic!("expected an aleatorio statement");
        };

        assert!(enabled);
        assert_eq!(
            (minimum, maximum),
            (NumericConstant::Integer(-3), NumericConstant::Integer(0))
        );
    }

    #[test]
    fn enforces_classic_identifier_spelling_and_length() {
        let valid_name = format!("a_{}", "b".repeat(28));
        let valid_source = format!(
            "algoritmo \"identificadores\"\nvar\n{valid_name}: inteiro\ninicio\nfimalgoritmo\n"
        );
        parse(&valid_source, 128).expect("30-character identifiers may contain underscores");

        let long_source = format!(
            "algoritmo \"identificadores\"\nvar\n{}: inteiro\ninicio\nfimalgoritmo\n",
            "a".repeat(31)
        );
        let long_error = parse(&long_source, 128)
            .expect_err("identifiers longer than 30 characters must be rejected");
        assert_eq!(long_error[0].kind, DiagnosticKind::Syntax);
        assert_eq!((long_error[0].line, long_error[0].column), (3, 1));

        let invalid_source =
            "algoritmo \"identificadores\"\nvar\n_invalido: inteiro\ninicio\nfimalgoritmo\n";
        let invalid_error =
            parse(invalid_source, 128).expect_err("identifiers must start with a letter");
        assert_eq!(invalid_error[0].kind, DiagnosticKind::Lexical);
        assert_eq!((invalid_error[0].line, invalid_error[0].column), (3, 1));
    }
}

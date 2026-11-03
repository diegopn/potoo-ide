use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::ast::{
    Expression, ExpressionKind, Program, Statement, StatementKind, Type, VariableDeclaration,
};
use crate::{
    ControlAction, DataType, Diagnostic, DiagnosticKind, ExecutionEvent, ExecutionOptions,
    InputPrompt, PauseReason, RuntimeIo, SourceLocation, VariableSnapshot,
};

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Integer(i64),
    Real(f64),
    Logical(bool),
    Chain(String),
    Character(char),
}

impl Value {
    fn data_type(&self) -> Type {
        match self {
            Self::Integer(_) => Type::Integer,
            Self::Real(_) => Type::Real,
            Self::Logical(_) => Type::Logical,
            Self::Chain(_) => Type::Chain,
            Self::Character(_) => Type::Character,
        }
    }

    fn display(&self) -> Cow<'_, str> {
        match self {
            Self::Integer(value) => Cow::Owned(value.to_string()),
            Self::Real(value) => Cow::Owned(value.to_string()),
            Self::Logical(true) => Cow::Borrowed("verdadeiro"),
            Self::Logical(false) => Cow::Borrowed("falso"),
            Self::Chain(value) => Cow::Borrowed(value),
            Self::Character(value) => Cow::Owned(value.to_string()),
        }
    }
}

struct VariableState {
    declaration: VariableDeclaration,
    values: Vec<Option<Value>>,
}

enum Flow {
    Normal,
    Stop,
}

pub(crate) struct Interpreter<'a> {
    variables: HashMap<String, VariableState>,
    modules: HashMap<String, Vec<Statement>>,
    io: &'a mut dyn RuntimeIo,
    options: ExecutionOptions,
    steps: u64,
    output_bytes: usize,
    call_depth: usize,
    debugging: bool,
    step_next: bool,
    stopped: bool,
}

impl<'a> Interpreter<'a> {
    pub(crate) fn new(
        program: Program,
        io: &'a mut dyn RuntimeIo,
        options: ExecutionOptions,
    ) -> Result<Self, Diagnostic> {
        let mut variables = HashMap::new();
        let mut variable_slots = 0usize;
        for declaration in program.variables {
            let length = match declaration.vector {
                Some(bounds) => usize::try_from(
                    bounds
                        .upper
                        .checked_sub(bounds.lower)
                        .and_then(|length| length.checked_add(1))
                        .ok_or_else(|| {
                            runtime_error(
                                declaration.line,
                                declaration.column,
                                "tamanho do vetor fora do intervalo permitido",
                            )
                        })?,
                )
                .map_err(|_| {
                    runtime_error(
                        declaration.line,
                        declaration.column,
                        "tamanho do vetor fora do intervalo permitido",
                    )
                })?,
                None => 1,
            };
            variable_slots = variable_slots.checked_add(length).ok_or_else(|| {
                runtime_error(
                    declaration.line,
                    declaration.column,
                    "o programa excede o limite de variáveis",
                )
            })?;
            if variable_slots > options.max_variable_slots {
                return Err(runtime_error(
                    declaration.line,
                    declaration.column,
                    &format!(
                        "o programa excede o limite de {} posições de variável",
                        options.max_variable_slots
                    ),
                ));
            }
            variables.insert(
                declaration.name.clone(),
                VariableState {
                    declaration,
                    values: vec![None; length],
                },
            );
        }

        let modules = program
            .modules
            .into_iter()
            .map(|module| (module.name, module.statements))
            .collect();
        Ok(Self {
            variables,
            modules,
            io,
            options,
            steps: 0,
            output_bytes: 0,
            call_depth: 0,
            debugging: false,
            step_next: false,
            stopped: false,
        })
    }

    pub(crate) fn execute(mut self, statements: &[Statement]) -> Result<(), Vec<Diagnostic>> {
        self.debugging = self.io.is_debugging();
        match self.statements(statements) {
            Ok(Flow::Normal | Flow::Stop) => Ok(()),
            Err(diagnostic) => Err(vec![diagnostic]),
        }
    }

    fn statements(&mut self, statements: &[Statement]) -> Result<Flow, Diagnostic> {
        for statement in statements {
            if matches!(self.statement(statement)?, Flow::Stop) {
                return Ok(Flow::Stop);
            }
        }
        Ok(Flow::Normal)
    }

    fn statement(&mut self, statement: &Statement) -> Result<Flow, Diagnostic> {
        if self.before_statement(statement)? {
            return Ok(Flow::Stop);
        }
        match &statement.kind {
            StatementKind::Write(expressions) => {
                let mut line = String::new();
                for expression in expressions {
                    let value = self.expression(expression)?;
                    let text = value.display();
                    let output_limit_error = || {
                        runtime_error(
                            statement.line,
                            statement.column,
                            "a saída excedeu o limite permitido",
                        )
                    };
                    let line_bytes = line
                        .len()
                        .checked_add(text.len())
                        .and_then(|bytes| bytes.checked_add(1))
                        .ok_or_else(output_limit_error)?;
                    let output_bytes = self
                        .output_bytes
                        .checked_add(line_bytes)
                        .ok_or_else(output_limit_error)?;
                    if output_bytes > self.options.max_output_bytes {
                        return Err(output_limit_error());
                    }
                    line.push_str(&text);
                }
                line.push('\n');
                self.write(&line, statement.line, statement.column)?;
                Ok(Flow::Normal)
            }
            StatementKind::Read(target) => {
                let (name, data_type) = self.input_target(target)?;
                let prompt = InputPrompt {
                    variable_name: name,
                    data_type,
                };
                let input = self
                    .io
                    .read_line(&prompt)
                    .map_err(|message| runtime_error(statement.line, statement.column, &message))?;
                if input.len() > self.options.max_string_bytes {
                    return Err(runtime_error(
                        statement.line,
                        statement.column,
                        "a entrada excede o limite de texto permitido",
                    ));
                }
                let value = parse_input(&input, prompt.data_type)
                    .map_err(|message| runtime_error(statement.line, statement.column, &message))?;
                self.assign(target, value)?;
                Ok(Flow::Normal)
            }
            StatementKind::Assign { target, value } => {
                let value = self.expression(value)?;
                self.assign(target, value)?;
                Ok(Flow::Normal)
            }
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => match self.expression(condition)? {
                Value::Logical(true) => self.statements(then_branch),
                Value::Logical(false) => self.statements(else_branch),
                _ => Err(runtime_error(
                    condition.line,
                    condition.column,
                    "a condição precisa ser lógica",
                )),
            },
            StatementKind::While { condition, body } => loop {
                if self.loop_tick(statement)? {
                    return Ok(Flow::Stop);
                }
                match self.expression(condition)? {
                    Value::Logical(true) => {}
                    Value::Logical(false) => return Ok(Flow::Normal),
                    _ => {
                        return Err(runtime_error(
                            condition.line,
                            condition.column,
                            "a condição precisa ser lógica",
                        ));
                    }
                }
                if matches!(self.statements(body)?, Flow::Stop) {
                    return Ok(Flow::Stop);
                }
            },
            StatementKind::Repeat { condition, body } => loop {
                if self.loop_tick(statement)? {
                    return Ok(Flow::Stop);
                }
                if matches!(self.statements(body)?, Flow::Stop) {
                    return Ok(Flow::Stop);
                }
                match self.expression(condition)? {
                    // Mapler repeats while the condition after `ate` is true.
                    Value::Logical(true) => {}
                    Value::Logical(false) => return Ok(Flow::Normal),
                    _ => {
                        return Err(runtime_error(
                            condition.line,
                            condition.column,
                            "a condição precisa ser lógica",
                        ));
                    }
                }
            },
            StatementKind::For {
                variable,
                start,
                end,
                step,
                body,
            } => {
                let Value::Integer(mut current) = self.expression(start)? else {
                    return Err(runtime_error(
                        start.line,
                        start.column,
                        "o início do 'para' precisa ser inteiro",
                    ));
                };
                let Value::Integer(end) = self.expression(end)? else {
                    return Err(runtime_error(
                        end.line,
                        end.column,
                        "o final do 'para' precisa ser inteiro",
                    ));
                };
                let Value::Integer(step) = self.expression(step)? else {
                    return Err(runtime_error(
                        step.line,
                        step.column,
                        "o passo do 'para' precisa ser inteiro",
                    ));
                };
                if step <= 0 {
                    return Err(runtime_error(
                        statement.line,
                        statement.column,
                        "o passo do 'para' precisa ser positivo",
                    ));
                }
                self.assign_named(variable, Value::Integer(current), statement)?;
                while current <= end {
                    if self.loop_tick(statement)? {
                        return Ok(Flow::Stop);
                    }
                    if matches!(self.statements(body)?, Flow::Stop) {
                        return Ok(Flow::Stop);
                    }
                    current = current.checked_add(step).ok_or_else(|| {
                        runtime_error(
                            statement.line,
                            statement.column,
                            "a variável de controle do 'para' excedeu o intervalo inteiro",
                        )
                    })?;
                    if current <= end {
                        self.assign_named(variable, Value::Integer(current), statement)?;
                    }
                }
                Ok(Flow::Normal)
            }
            StatementKind::CallModule(name) => {
                if self.call_depth >= self.options.max_call_depth {
                    return Err(runtime_error(
                        statement.line,
                        statement.column,
                        "módulos chamados recursivamente excedem o limite de chamadas",
                    ));
                }
                let body = self.modules.get(name).cloned().ok_or_else(|| {
                    runtime_error(statement.line, statement.column, "módulo sem definição")
                })?;
                self.call_depth += 1;
                let result = self.statements(&body);
                self.call_depth -= 1;
                result
            }
        }
    }

    fn expression(&mut self, expression: &Expression) -> Result<Value, Diagnostic> {
        match &expression.kind {
            ExpressionKind::Integer(value) => Ok(Value::Integer(*value)),
            ExpressionKind::Real(value) => Ok(Value::Real(*value)),
            ExpressionKind::String(value) => Ok(Value::Chain(value.clone())),
            ExpressionKind::Logical(value) => Ok(Value::Logical(*value)),
            ExpressionKind::Identifier(name) => self.read_value(name, None, expression),
            ExpressionKind::Index { name, index } => {
                let value = self.expression(index)?;
                let index = self.integer(value, expression)?;
                self.read_value(name, Some(index), expression)
            }
            ExpressionKind::Unary { operator, operand } => {
                let value = self.expression(operand)?;
                match (operator.as_str(), value) {
                    ("-", Value::Integer(value)) => {
                        value.checked_neg().map(Value::Integer).ok_or_else(|| {
                            runtime_error(expression.line, expression.column, "estouro de inteiro")
                        })
                    }
                    ("-", Value::Real(value)) => self.finite_real(-value, expression),
                    ("nao", Value::Logical(value)) => Ok(Value::Logical(!value)),
                    ("-", _) => Err(runtime_error(
                        expression.line,
                        expression.column,
                        "o sinal '-' exige um número",
                    )),
                    _ => Err(runtime_error(
                        expression.line,
                        expression.column,
                        "'nao' exige um valor lógico",
                    )),
                }
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let left = self.expression(left)?;
                if operator == "e" && left == Value::Logical(false) {
                    return Ok(Value::Logical(false));
                }
                if operator == "ou" && left == Value::Logical(true) {
                    return Ok(Value::Logical(true));
                }
                let right = self.expression(right)?;
                self.binary(left, operator, right, expression)
            }
        }
    }

    fn binary(
        &self,
        left: Value,
        operator: &str,
        right: Value,
        expression: &Expression,
    ) -> Result<Value, Diagnostic> {
        let error = || {
            runtime_error(
                expression.line,
                expression.column,
                "tipos incompatíveis na operação",
            )
        };
        match (operator, left, right) {
            ("+", Value::Integer(left), Value::Integer(right)) => {
                left.checked_add(right).map(Value::Integer).ok_or_else(|| {
                    runtime_error(expression.line, expression.column, "estouro de inteiro")
                })
            }
            ("-", Value::Integer(left), Value::Integer(right)) => {
                left.checked_sub(right).map(Value::Integer).ok_or_else(|| {
                    runtime_error(expression.line, expression.column, "estouro de inteiro")
                })
            }
            ("*", Value::Integer(left), Value::Integer(right)) => {
                left.checked_mul(right).map(Value::Integer).ok_or_else(|| {
                    runtime_error(expression.line, expression.column, "estouro de inteiro")
                })
            }
            ("/", Value::Integer(_), Value::Integer(0)) => Err(runtime_error(
                expression.line,
                expression.column,
                "divisão por zero",
            )),
            ("/", Value::Integer(left), Value::Integer(right)) => {
                left.checked_div(right).map(Value::Integer).ok_or_else(|| {
                    runtime_error(expression.line, expression.column, "estouro de inteiro")
                })
            }
            (operator @ ("+" | "-" | "*" | "/"), left, right) => {
                let (Some(left), Some(right)) = (as_real(left), as_real(right)) else {
                    return Err(error());
                };
                if operator == "/" && right == 0.0 {
                    return Err(runtime_error(
                        expression.line,
                        expression.column,
                        "divisão por zero",
                    ));
                }
                let value = match operator {
                    "+" => left + right,
                    "-" => left - right,
                    "*" => left * right,
                    "/" => left / right,
                    _ => unreachable!(),
                };
                if value.is_finite() {
                    Ok(Value::Real(value))
                } else {
                    Err(runtime_error(
                        expression.line,
                        expression.column,
                        "resultado real fora do intervalo permitido",
                    ))
                }
            }
            ("e", Value::Logical(left), Value::Logical(right)) => Ok(Value::Logical(left && right)),
            ("ou", Value::Logical(left), Value::Logical(right)) => {
                Ok(Value::Logical(left || right))
            }
            ("=", Value::Chain(left), Value::Chain(right)) => Ok(Value::Logical(left == right)),
            ("=", left, right) => {
                let ordering = compare_numeric(&left, &right).ok_or_else(error)?;
                Ok(Value::Logical(ordering == Ordering::Equal))
            }
            ("<" | "<=" | ">" | ">=", left, right) => {
                let ordering = compare_numeric(&left, &right).ok_or_else(error)?;
                Ok(Value::Logical(match operator {
                    "<" => ordering == Ordering::Less,
                    "<=" => ordering != Ordering::Greater,
                    ">" => ordering == Ordering::Greater,
                    ">=" => ordering != Ordering::Less,
                    _ => unreachable!(),
                }))
            }
            _ => Err(error()),
        }
    }

    fn input_target(&mut self, target: &Expression) -> Result<(String, DataType), Diagnostic> {
        let name = match &target.kind {
            ExpressionKind::Identifier(name) => name.clone(),
            ExpressionKind::Index { name, index } => {
                let value = self.expression(index)?;
                let index = self.integer(value, target)?;
                let _ = self.slot_index(name, Some(index), target)?;
                name.clone()
            }
            _ => {
                return Err(runtime_error(
                    target.line,
                    target.column,
                    "esperada uma variável para leitura",
                ));
            }
        };
        let state = self
            .variables
            .get(&name)
            .ok_or_else(|| runtime_error(target.line, target.column, "variável não declarada"))?;
        if state.declaration.vector.is_some()
            && matches!(target.kind, ExpressionKind::Identifier(_))
        {
            return Err(runtime_error(
                target.line,
                target.column,
                "o vetor precisa ser acessado por um índice",
            ));
        }
        let data_type = match state.declaration.data_type {
            Type::Integer => DataType::Integer,
            Type::Real => DataType::Real,
            Type::Logical => DataType::Logical,
            Type::Chain => DataType::Chain,
            Type::Character => DataType::Character,
        };
        Ok((name, data_type))
    }

    fn assign(&mut self, target: &Expression, value: Value) -> Result<(), Diagnostic> {
        let (name, index) = match &target.kind {
            ExpressionKind::Identifier(name) => (name.clone(), None),
            ExpressionKind::Index { name, index } => {
                let value = self.expression(index)?;
                (name.clone(), Some(self.integer(value, target)?))
            }
            _ => {
                return Err(runtime_error(
                    target.line,
                    target.column,
                    "esperada uma variável para atribuição",
                ));
            }
        };
        let slot = self.slot_index(&name, index, target)?;
        let state = self
            .variables
            .get_mut(&name)
            .ok_or_else(|| runtime_error(target.line, target.column, "variável não declarada"))?;
        let value = match (state.declaration.data_type, value) {
            (Type::Character, Value::Chain(text)) => {
                let mut characters = text.chars();
                let Some(character) = characters.next() else {
                    return Err(runtime_error(
                        target.line,
                        target.column,
                        "caractere precisa conter exatamente um caractere",
                    ));
                };
                if characters.next().is_some() {
                    return Err(runtime_error(
                        target.line,
                        target.column,
                        "caractere precisa conter exatamente um caractere",
                    ));
                }
                Value::Character(character)
            }
            (expected, value) if expected == value.data_type() => value,
            _ => {
                return Err(runtime_error(
                    target.line,
                    target.column,
                    "o valor não corresponde ao tipo da variável",
                ));
            }
        };
        state.values[slot] = Some(value);
        Ok(())
    }

    fn assign_named(
        &mut self,
        name: &str,
        value: Value,
        statement: &Statement,
    ) -> Result<(), Diagnostic> {
        let state = self.variables.get_mut(name).ok_or_else(|| {
            runtime_error(
                statement.line,
                statement.column,
                "variável de controle não declarada",
            )
        })?;
        if state.declaration.vector.is_some() || state.declaration.data_type != value.data_type() {
            return Err(runtime_error(
                statement.line,
                statement.column,
                "a variável de controle do 'para' precisa ser inteira",
            ));
        }
        state.values[0] = Some(value);
        Ok(())
    }

    fn read_value(
        &self,
        name: &str,
        index: Option<i64>,
        expression: &Expression,
    ) -> Result<Value, Diagnostic> {
        let slot = self.slot_index(name, index, expression)?;
        self.variables
            .get(name)
            .and_then(|state| state.values.get(slot))
            .and_then(Clone::clone)
            .ok_or_else(|| {
                runtime_error(
                    expression.line,
                    expression.column,
                    "variável ainda não inicializada",
                )
            })
    }

    fn slot_index(
        &self,
        name: &str,
        index: Option<i64>,
        expression: &Expression,
    ) -> Result<usize, Diagnostic> {
        let state = self.variables.get(name).ok_or_else(|| {
            runtime_error(expression.line, expression.column, "variável não declarada")
        })?;
        match (state.declaration.vector, index) {
            (None, None) => Ok(0),
            (None, Some(_)) => Err(runtime_error(
                expression.line,
                expression.column,
                "a variável não foi declarada como vetor",
            )),
            (Some(_), None) => Err(runtime_error(
                expression.line,
                expression.column,
                "o vetor precisa ser acessado por um índice",
            )),
            (Some(bounds), Some(index)) if index < bounds.lower || index > bounds.upper => {
                Err(runtime_error(
                    expression.line,
                    expression.column,
                    "índice fora dos limites do vetor",
                ))
            }
            (Some(bounds), Some(index)) => usize::try_from(index - bounds.lower).map_err(|_| {
                runtime_error(
                    expression.line,
                    expression.column,
                    "índice de vetor inválido",
                )
            }),
        }
    }

    fn integer(&self, value: Value, expression: &Expression) -> Result<i64, Diagnostic> {
        match value {
            Value::Integer(value) => Ok(value),
            _ => Err(runtime_error(
                expression.line,
                expression.column,
                "o índice do vetor precisa ser inteiro",
            )),
        }
    }

    fn before_statement(&mut self, statement: &Statement) -> Result<bool, Diagnostic> {
        if self.stopped {
            return Ok(true);
        }
        self.tick(statement.line, statement.column)?;
        if self.io.should_stop() {
            self.stopped = true;
            return Ok(true);
        }
        if self.debugging || self.step_next {
            let pause_reason = Some(if self.step_next {
                PauseReason::Step
            } else {
                PauseReason::Debug
            });
            let event = ExecutionEvent {
                location: SourceLocation {
                    line: statement.line,
                    column: statement.column,
                },
                variables: self.variable_snapshots(),
                pause_reason,
            };
            match self
                .io
                .on_execution_event(&event)
                .map_err(|message| runtime_error(statement.line, statement.column, &message))?
            {
                ControlAction::Continue => {
                    self.debugging = false;
                    self.step_next = false;
                }
                ControlAction::Step => {
                    self.debugging = true;
                    self.step_next = true;
                }
                ControlAction::Stop => {
                    self.stopped = true;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn loop_tick(&mut self, statement: &Statement) -> Result<bool, Diagnostic> {
        if self.stopped || self.io.should_stop() {
            self.stopped = true;
            return Ok(true);
        }
        self.tick(statement.line, statement.column)?;
        Ok(false)
    }

    fn tick(&mut self, line: usize, column: usize) -> Result<(), Diagnostic> {
        self.steps = self.steps.saturating_add(1);
        if self.steps > self.options.max_steps {
            return Err(runtime_error(
                line,
                column,
                "o programa excedeu o limite de execução",
            ));
        }
        Ok(())
    }

    fn write(&mut self, text: &str, line: usize, column: usize) -> Result<(), Diagnostic> {
        self.output_bytes = self
            .output_bytes
            .checked_add(text.len())
            .ok_or_else(|| runtime_error(line, column, "a saída excedeu o limite permitido"))?;
        if self.output_bytes > self.options.max_output_bytes {
            return Err(runtime_error(
                line,
                column,
                "a saída excedeu o limite permitido",
            ));
        }
        self.io.write(text);
        Ok(())
    }

    fn finite_real(&self, value: f64, expression: &Expression) -> Result<Value, Diagnostic> {
        if value.is_finite() {
            Ok(Value::Real(value))
        } else {
            Err(runtime_error(
                expression.line,
                expression.column,
                "resultado real fora do intervalo permitido",
            ))
        }
    }

    fn variable_snapshots(&self) -> Vec<VariableSnapshot> {
        self.variables
            .values()
            .map(|state| {
                let value = match state.declaration.vector {
                    None => state.values[0]
                        .as_ref()
                        .map(|value| value.display().into_owned())
                        .unwrap_or_else(|| "<não inicializada>".to_owned()),
                    Some(bounds) => state
                        .values
                        .iter()
                        .enumerate()
                        .map(|(offset, value)| {
                            let index = bounds.lower + offset as i64;
                            format!(
                                "{index}:{}",
                                value
                                    .as_ref()
                                    .map(|value| value.display().into_owned())
                                    .unwrap_or_else(|| "<não inicializado>".to_owned())
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                };
                VariableSnapshot {
                    name: state.declaration.name.clone(),
                    value: if state.declaration.vector.is_some() {
                        format!("[{value}]")
                    } else {
                        value
                    },
                }
            })
            .collect()
    }
}

fn parse_input(input: &str, data_type: DataType) -> Result<Value, String> {
    match data_type {
        DataType::Integer => input
            .trim()
            .parse::<i64>()
            .map(Value::Integer)
            .map_err(|_| "esperado um valor inteiro".to_owned()),
        DataType::Real => input
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(Value::Real)
            .ok_or_else(|| "esperado um valor real".to_owned()),
        DataType::Logical => match input.trim() {
            "verdadeiro" => Ok(Value::Logical(true)),
            "falso" => Ok(Value::Logical(false)),
            _ => Err("esperado 'verdadeiro' ou 'falso'".to_owned()),
        },
        DataType::Chain => Ok(Value::Chain(input.to_owned())),
        DataType::Character => {
            let mut characters = input.chars();
            let Some(character) = characters.next() else {
                return Err("esperado um único caractere".to_owned());
            };
            if characters.next().is_some() {
                Err("esperado um único caractere".to_owned())
            } else {
                Ok(Value::Character(character))
            }
        }
    }
}

fn as_real(value: Value) -> Option<f64> {
    match value {
        Value::Integer(value) => Some(value as f64),
        Value::Real(value) => Some(value),
        _ => None,
    }
}

fn compare_numeric(left: &Value, right: &Value) -> Option<Ordering> {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => Some(left.cmp(right)),
        (Value::Real(left), Value::Real(right)) => left.partial_cmp(right),
        (Value::Integer(left), Value::Real(right)) => Some(compare_integer_real(*left, *right)),
        (Value::Real(left), Value::Integer(right)) => {
            Some(compare_integer_real(*right, *left).reverse())
        }
        _ => None,
    }
}

fn compare_integer_real(integer: i64, real: f64) -> Ordering {
    if real < i64::MIN as f64 {
        return Ordering::Greater;
    }
    if real >= -(i64::MIN as f64) {
        return Ordering::Less;
    }
    let truncated = real as i64;
    match integer.cmp(&truncated) {
        Ordering::Equal if real > truncated as f64 => Ordering::Less,
        Ordering::Equal if real < truncated as f64 => Ordering::Greater,
        ordering => ordering,
    }
}

fn runtime_error(line: usize, column: usize, message: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticKind::Runtime, message, line, column)
}

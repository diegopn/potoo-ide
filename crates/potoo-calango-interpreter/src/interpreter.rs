use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::ast::{
    Expression, ExpressionKind, Routine, Statement, StatementKind, Type, VariableDeclaration,
};
use crate::parser::parse_with_options;
use crate::value::Value;
use crate::{
    ControlAction, DataType, Diagnostic, DiagnosticKind, ExecutionEvent, ExecutionOptions,
    InputPrompt, PauseReason, RuntimeIo, SourceLocation, VariableSnapshot,
};

type Cell = Rc<RefCell<Value>>;

#[derive(Clone)]
struct Binding {
    ty: Type,
    cell: Cell,
}

enum Flow {
    Continue,
    Break,
    Return(Option<Value>),
    Stop,
}

pub fn execute(source: &str, io: &mut dyn RuntimeIo) -> Result<(), Vec<Diagnostic>> {
    execute_with_options(source, io, ExecutionOptions::default())
}

pub fn execute_with_options(
    source: &str,
    io: &mut dyn RuntimeIo,
    options: ExecutionOptions,
) -> Result<(), Vec<Diagnostic>> {
    let program = parse_with_options(source, options)?;
    let mut interpreter = Interpreter::new(io, options, &program.routines);
    match interpreter.block(&program.main) {
        Ok(Flow::Continue) => Ok(()),
        Ok(Flow::Stop) => Ok(()),
        Ok(Flow::Break) => Err(vec![interpreter.error("'interrompa' sem laço", 1, 1)]),
        Ok(Flow::Return(_)) => Err(vec![interpreter.error(
            "'retorna' fora de uma função",
            1,
            1,
        )]),
        Err(_) if interpreter.stopped => Ok(()),
        Err(error) => Err(vec![error]),
    }
}

struct Interpreter<'a> {
    io: &'a mut dyn RuntimeIo,
    options: ExecutionOptions,
    routines: HashMap<String, Routine>,
    scopes: Vec<HashMap<String, Binding>>,
    steps: u64,
    output_bytes: usize,
    calls: usize,
    loop_depth: usize,
    stepping: bool,
    stopped: bool,
    random_state: u64,
}

impl<'a> Interpreter<'a> {
    fn new(io: &'a mut dyn RuntimeIo, options: ExecutionOptions, routines: &[Routine]) -> Self {
        Self {
            io,
            options,
            routines: routines
                .iter()
                .cloned()
                .map(|routine| (routine.name.clone(), routine))
                .collect(),
            scopes: vec![HashMap::new()],
            steps: 0,
            output_bytes: 0,
            calls: 0,
            loop_depth: 0,
            stepping: false,
            stopped: false,
            random_state: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0x9e37_79b9_7f4a_7c15, |duration| duration.as_nanos() as u64),
        }
    }

    fn block(&mut self, statements: &[Statement]) -> Result<Flow, Diagnostic> {
        for statement in statements {
            if self.before_statement(statement)? {
                return Ok(Flow::Stop);
            }
            let flow = self.statement(statement)?;
            if !matches!(flow, Flow::Continue) {
                return Ok(flow);
            }
        }
        Ok(Flow::Continue)
    }

    fn before_statement(&mut self, statement: &Statement) -> Result<bool, Diagnostic> {
        self.steps += 1;
        if self.steps > self.options.max_steps {
            return Err(self.runtime_error("limite de instruções excedido", statement));
        }
        if self.io.should_stop() {
            self.stopped = true;
            return Ok(true);
        }
        if self.io.is_debugging() || self.stepping {
            let pause_reason = Some(if self.stepping {
                PauseReason::Step
            } else {
                PauseReason::Debugger
            });
            let action = self
                .io
                .on_execution_event(&ExecutionEvent {
                    location: SourceLocation {
                        line: statement.line,
                        column: statement.column,
                    },
                    variables: self.visible_variables(),
                    pause_reason,
                })
                .map_err(|message| self.runtime_error(message, statement))?;
            match action {
                ControlAction::Continue => self.stepping = false,
                ControlAction::Step => self.stepping = true,
                ControlAction::Stop => {
                    self.stopped = true;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn visible_variables(&self) -> Vec<VariableSnapshot> {
        let mut visible = BTreeMap::new();
        for scope in &self.scopes {
            for (name, binding) in scope {
                visible.insert(name.clone(), binding.cell.borrow().display());
            }
        }
        visible
            .into_iter()
            .map(|(name, value)| VariableSnapshot { name, value })
            .collect()
    }

    fn statement(&mut self, statement: &Statement) -> Result<Flow, Diagnostic> {
        match &statement.kind {
            StatementKind::Declare(declarations) => {
                for declaration in declarations {
                    self.declare(declaration)?;
                }
            }
            StatementKind::Assign { target, value } => {
                let value = self.expression(value)?;
                self.assign(target, value, statement)?;
            }
            StatementKind::Write { newline, values } => {
                let mut output = String::new();
                for expression in values {
                    let value = self.expression(expression)?;
                    if matches!(&value, Value::Array { .. }) {
                        return Err(
                            self.type_error_at("não é possível escrever um vetor", expression)
                        );
                    }
                    output.push_str(&value.display());
                }
                if *newline {
                    output.push('\n');
                }
                self.write(&output, statement)?;
            }
            StatementKind::Read {
                targets,
                character_only,
            } => {
                for target in targets {
                    let cell = self.lvalue(target, statement)?;
                    let ty = self.binding_type(target, statement)?;
                    if *character_only && ty != Type::Character {
                        return Err(self
                            .type_error_at("'leiaCaracter' exige uma variável caracter", target));
                    }
                    if !character_only && ty.scalar() == &Type::Character {
                        return Err(self.type_error_at(
                            "'leia' não aceita variáveis caracter; use 'leiaCaracter'",
                            target,
                        ));
                    }
                    let data_type = if *character_only {
                        DataType::Character
                    } else {
                        data_type(&ty)
                    };
                    let variable_name = match &target.kind {
                        ExpressionKind::Variable(name) => Some(name.clone()),
                        _ => None,
                    };
                    let input = self
                        .io
                        .read_line_with_timeout(
                            &InputPrompt {
                                variable_name,
                                data_type,
                            },
                            Duration::from_millis(self.options.max_input_wait_millis),
                        )
                        .map_err(|message| self.runtime_error(message, statement))?;
                    let input = parse_input(&input, data_type, *character_only)
                        .map_err(|message| self.runtime_error(message, statement))?;
                    if matches!(&input, Value::Real(value) if !value.is_finite()) {
                        return Err(self.runtime_error("número real inválido", statement));
                    }
                    if matches!(&input, Value::Text(text) if text.len() > self.options.max_string_bytes)
                    {
                        return Err(
                            self.runtime_error("limite de tamanho de texto excedido", statement)
                        );
                    }
                    let ty = self.binding_type(target, statement)?;
                    *cell.borrow_mut() = coerce(input, &ty).ok_or_else(|| {
                        self.type_error("valor incompatível com a variável", statement)
                    })?;
                }
            }
            StatementKind::If {
                branches,
                otherwise,
            } => {
                for (condition, body) in branches {
                    let condition_value = self.expression(condition)?;
                    if self.logical(condition_value, condition)? {
                        return self.block(body);
                    }
                }
                return self.block(otherwise);
            }
            StatementKind::While { condition, body } => {
                self.loop_depth += 1;
                while {
                    let condition_value = self.expression(condition)?;
                    self.logical(condition_value, condition)?
                } {
                    if self.loop_tick(statement)? {
                        self.loop_depth -= 1;
                        return Ok(Flow::Stop);
                    }
                    match self.block(body)? {
                        Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Stop => {
                            self.loop_depth -= 1;
                            return Ok(Flow::Stop);
                        }
                        returned @ Flow::Return(_) => {
                            self.loop_depth -= 1;
                            return Ok(returned);
                        }
                    }
                    if self.io.should_stop() {
                        self.stopped = true;
                        break;
                    }
                }
                self.loop_depth -= 1;
                if self.stopped {
                    return Ok(Flow::Stop);
                }
            }
            StatementKind::DoWhile { body, condition } => {
                self.loop_depth += 1;
                loop {
                    if self.loop_tick(statement)? {
                        self.loop_depth -= 1;
                        return Ok(Flow::Stop);
                    }
                    match self.block(body)? {
                        Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Stop => {
                            self.loop_depth -= 1;
                            return Ok(Flow::Stop);
                        }
                        returned @ Flow::Return(_) => {
                            self.loop_depth -= 1;
                            return Ok(returned);
                        }
                    }
                    let condition_value = self.expression(condition)?;
                    if self.stopped || !self.logical(condition_value, condition)? {
                        break;
                    }
                }
                self.loop_depth -= 1;
                if self.stopped {
                    return Ok(Flow::Stop);
                }
            }
            StatementKind::For {
                variable,
                start,
                end,
                step,
                body,
            } => {
                let start_value = self.expression(start)?;
                let start = self.integer(start_value, start)?;
                let end_value = self.expression(end)?;
                let end = self.integer(end_value, end)?;
                let step_value = self.expression(step)?;
                let step = self.integer(step_value, step)?;
                if step == 0 {
                    return Err(self.runtime_error("o passo do laço não pode ser zero", statement));
                }
                let binding = self.lookup(variable).ok_or_else(|| {
                    self.runtime_error(format!("variável '{variable}' não declarada"), statement)
                })?;
                if binding.ty != Type::Integer {
                    return Err(self.type_error("a variável de 'para' deve ser inteira", statement));
                }
                self.loop_depth += 1;
                let mut current = start;
                while if step > 0 {
                    current <= end
                } else {
                    current >= end
                } {
                    if self.loop_tick(statement)? {
                        self.loop_depth -= 1;
                        return Ok(Flow::Stop);
                    }
                    *binding.cell.borrow_mut() = Value::Integer(current);
                    match self.block(body)? {
                        Flow::Continue => {}
                        Flow::Break => break,
                        Flow::Stop => {
                            self.loop_depth -= 1;
                            return Ok(Flow::Stop);
                        }
                        returned @ Flow::Return(_) => {
                            self.loop_depth -= 1;
                            return Ok(returned);
                        }
                    }
                    if self.stopped {
                        break;
                    }
                    let Some(next) = current.checked_add(step) else {
                        break;
                    };
                    if (step > 0 && next > end) || (step < 0 && next < end) {
                        break;
                    }
                    current = next;
                }
                self.loop_depth -= 1;
                if self.stopped {
                    return Ok(Flow::Stop);
                }
            }
            StatementKind::Choose {
                value,
                cases,
                otherwise,
            } => {
                let value = self.expression(value)?;
                for (candidate, body) in cases {
                    if value.same(&self.expression(candidate)?) {
                        return self.block(body);
                    }
                }
                return self.block(otherwise);
            }
            StatementKind::Call { name, arguments } => {
                let _ = self.call(name, arguments, statement)?;
            }
            StatementKind::Return(value) => {
                return Ok(Flow::Return(
                    value
                        .as_ref()
                        .map(|value| self.expression(value))
                        .transpose()?,
                ));
            }
            StatementKind::Break => {
                if self.loop_depth == 0 {
                    return Err(
                        self.runtime_error("'interrompa' só pode ser usado em um laço", statement)
                    );
                }
                return Ok(Flow::Break);
            }
            StatementKind::ClearScreen => self.io.clear_screen(),
        }
        Ok(Flow::Continue)
    }

    fn declare(&mut self, declaration: &VariableDeclaration) -> Result<(), Diagnostic> {
        if self.scopes.iter().map(HashMap::len).sum::<usize>() >= self.options.max_variable_slots {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                "limite de variáveis excedido",
                declaration.line,
                declaration.column,
            ));
        }
        if self
            .scopes
            .last()
            .is_some_and(|scope| scope.contains_key(&declaration.name))
        {
            return Err(Diagnostic::new(
                DiagnosticKind::Syntax,
                format!("a variável '{}' já foi declarada", declaration.name),
                declaration.line,
                declaration.column,
            ));
        }
        let value = default_value(&declaration.ty, self.options.max_array_length)
            .map_err(|message| self.error(message, declaration.line, declaration.column))?;
        let scope = self.scopes.last_mut().expect("there is always a scope");
        scope.insert(
            declaration.name.clone(),
            Binding {
                ty: declaration.ty.clone(),
                cell: Rc::new(RefCell::new(value)),
            },
        );
        Ok(())
    }

    fn assign(
        &mut self,
        target: &Expression,
        value: Value,
        statement: &Statement,
    ) -> Result<(), Diagnostic> {
        let ty = self.binding_type(target, statement)?;
        let value = coerce(value, &ty)
            .ok_or_else(|| self.type_error("valor incompatível com a variável", statement))?;
        *self.lvalue(target, statement)?.borrow_mut() = value;
        Ok(())
    }

    fn lvalue(
        &mut self,
        expression: &Expression,
        statement: &Statement,
    ) -> Result<Cell, Diagnostic> {
        match &expression.kind {
            ExpressionKind::Variable(name) => self
                .lookup(name)
                .map(|binding| binding.cell)
                .ok_or_else(|| {
                    self.runtime_error(format!("variável '{name}' não declarada"), statement)
                }),
            ExpressionKind::Index(array, index) => {
                let array = self.expression(array)?;
                let Value::Array { values, .. } = array else {
                    return Err(self.type_error("indexação exige um vetor ou matriz", statement));
                };
                let index_value = self.expression(index)?;
                let index = usize::try_from(self.integer(index_value, expression)?)
                    .map_err(|_| self.runtime_error("índice fora dos limites", statement))?;
                values.borrow().get(index).cloned().ok_or_else(|| {
                    self.runtime_error(format!("índice {index} fora dos limites"), statement)
                })
            }
            _ => Err(self.type_error("o destino precisa ser uma variável", statement)),
        }
    }

    fn binding_type(
        &mut self,
        expression: &Expression,
        statement: &Statement,
    ) -> Result<Type, Diagnostic> {
        match &expression.kind {
            ExpressionKind::Variable(name) => {
                self.lookup(name).map(|binding| binding.ty).ok_or_else(|| {
                    self.runtime_error(format!("variável '{name}' não declarada"), statement)
                })
            }
            ExpressionKind::Index(array, _) => self
                .binding_type(array, statement)?
                .element()
                .ok_or_else(|| self.type_error("indexação exige um vetor ou matriz", statement)),
            _ => Err(self.type_error("o destino precisa ser uma variável", statement)),
        }
    }

    fn expression(&mut self, expression: &Expression) -> Result<Value, Diagnostic> {
        let value = match &expression.kind {
            ExpressionKind::Integer(value) => Value::Integer(*value),
            ExpressionKind::Real(value) => Value::Real(*value),
            ExpressionKind::Logical(value) => Value::Logical(*value),
            ExpressionKind::Character(value) => Value::Character(*value),
            ExpressionKind::Text(value) => Value::Text(value.clone()),
            ExpressionKind::Variable(name) => self
                .lookup(name)
                .map(|binding| binding.cell.borrow().clone())
                .ok_or_else(|| {
                    self.error(
                        format!("variável '{name}' não declarada"),
                        expression.line,
                        expression.column,
                    )
                })?,
            ExpressionKind::Index(array, index) => {
                let array = self.expression(array)?;
                let Value::Array { values, .. } = array else {
                    return Err(
                        self.type_error_at("indexação exige um vetor ou matriz", expression)
                    );
                };
                let index_value = self.expression(index)?;
                let index = usize::try_from(self.integer(index_value, expression)?)
                    .map_err(|_| self.runtime_error_at("índice fora dos limites", expression))?;
                values
                    .borrow()
                    .get(index)
                    .map(|value| value.borrow().clone())
                    .ok_or_else(|| {
                        self.runtime_error_at(
                            format!("índice {index} fora dos limites"),
                            expression,
                        )
                    })?
            }
            ExpressionKind::Call { name, arguments } => {
                if self
                    .routines
                    .get(name)
                    .is_some_and(|routine| routine.return_type.is_none())
                {
                    return Err(self.type_error_at(
                        format!("procedimento '{name}' não pode ser usado como valor"),
                        expression,
                    ));
                }
                self.call(name, arguments, expression)?
            }
            ExpressionKind::Unary { operator, operand } => {
                let operand = self.expression(operand)?;
                match operator.as_str() {
                    "nao" => Value::Logical(!self.logical(operand, expression)?),
                    "-" => match operand {
                        Value::Integer(value) => {
                            Value::Integer(value.checked_neg().ok_or_else(|| {
                                self.runtime_error_at("estouro em negação", expression)
                            })?)
                        }
                        Value::Real(value) => Value::Real(-value),
                        _ => return Err(self.type_error_at("negação exige um número", expression)),
                    },
                    _ => return Err(self.type_error_at("operador unário desconhecido", expression)),
                }
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let left_value = self.expression(left)?;
                if matches!(
                    (operator.as_str(), &left_value),
                    ("e", Value::Logical(false))
                ) {
                    Value::Logical(false)
                } else if matches!(
                    (operator.as_str(), &left_value),
                    ("ou", Value::Logical(true))
                ) {
                    Value::Logical(true)
                } else {
                    let right_value = self.expression(right)?;
                    self.binary(left_value, operator, right_value, expression)?
                }
            }
            ExpressionKind::Formatted {
                value,
                width,
                decimals,
            } => {
                if *width > self.options.max_string_bytes
                    || decimals.is_some_and(|decimals| decimals > self.options.max_string_bytes)
                {
                    return Err(
                        self.runtime_error_at("formatação excede o limite de texto", expression)
                    );
                }
                let value = self.expression(value)?;
                let formatted = match (value, decimals) {
                    (Value::Real(value), Some(decimals)) => format!("{value:>width$.decimals$}"),
                    (Value::Real(value), None) => format!("{value:>width$.6}"),
                    (Value::Integer(value), None) => format!("{value:>width$}"),
                    (Value::Integer(_), Some(_)) => {
                        return Err(self
                            .type_error_at("formatação decimal exige um número real", expression));
                    }
                    _ => return Err(self.type_error_at("formatação exige um número", expression)),
                };
                Value::Text(formatted)
            }
        };
        if matches!(&value, Value::Text(text) if text.len() > self.options.max_string_bytes) {
            return Err(self.runtime_error_at("limite de tamanho de texto excedido", expression));
        }
        Ok(value)
    }

    fn binary(
        &self,
        left: Value,
        operator: &str,
        right: Value,
        expression: &Expression,
    ) -> Result<Value, Diagnostic> {
        let error = || self.type_error_at("operandos incompatíveis", expression);
        match operator {
            "e" | "ou" => Ok(Value::Logical(match (operator, left, right) {
                ("e", Value::Logical(left), Value::Logical(right)) => left && right,
                ("ou", Value::Logical(left), Value::Logical(right)) => left || right,
                _ => return Err(error()),
            })),
            "=" | "==" | "!=" => {
                if matches!((&left, &right), (Value::Text(_), _) | (_, Value::Text(_))) {
                    return Err(self.type_error_at(
                        "textos devem ser comparados com 'comparaTexto'",
                        expression,
                    ));
                }
                Ok(Value::Logical(if operator == "!=" {
                    !left.same(&right)
                } else {
                    left.same(&right)
                }))
            }
            "<" | "<=" | ">" | ">=" => {
                let ordering = match (&left, &right) {
                    (Value::Integer(left), Value::Integer(right)) => left.partial_cmp(right),
                    _ if left.as_real().is_some() && right.as_real().is_some() => {
                        left.as_real().and_then(|left| {
                            right.as_real().and_then(|right| left.partial_cmp(&right))
                        })
                    }
                    (Value::Text(_), _) | (_, Value::Text(_)) => {
                        return Err(self.type_error_at(
                            "textos devem ser comparados com 'comparaTexto'",
                            expression,
                        ));
                    }
                    (Value::Character(left), Value::Character(right)) => Some(left.cmp(right)),
                    _ => return Err(error()),
                };
                let ordering = ordering.ok_or_else(|| {
                    self.runtime_error_at("não é possível comparar valores indefinidos", expression)
                })?;
                Ok(Value::Logical(match operator {
                    "<" => ordering.is_lt(),
                    "<=" => !ordering.is_gt(),
                    ">" => ordering.is_gt(),
                    _ => !ordering.is_lt(),
                }))
            }
            "+" if matches!(
                (&left, &right),
                (Value::Text(_) | Value::Character(_), _)
                    | (_, Value::Text(_) | Value::Character(_))
            ) =>
            {
                Ok(Value::Text(format!(
                    "{}{}",
                    left.display(),
                    right.display()
                )))
            }
            "+" | "-" | "*" | "/" | "\\" | "%" | "mod" => {
                let (Some(left_number), Some(right_number)) = (left.as_real(), right.as_real())
                else {
                    return Err(error());
                };
                if matches!(operator, "/" | "\\" | "%" | "mod") && right_number == 0.0 {
                    return Err(self.runtime_error_at("divisão por zero", expression));
                }
                let integer =
                    matches!(left, Value::Integer(_)) && matches!(right, Value::Integer(_));
                if integer && operator != "/" {
                    let (left, right) = (left.as_integer().unwrap(), right.as_integer().unwrap());
                    let result = match operator {
                        "+" => left.checked_add(right),
                        "-" => left.checked_sub(right),
                        "*" => left.checked_mul(right),
                        "\\" => left.checked_div(right),
                        "%" | "mod" => left.checked_rem(right),
                        _ => None,
                    }
                    .ok_or_else(|| {
                        self.runtime_error_at("estouro na operação aritmética", expression)
                    })?;
                    Ok(Value::Integer(result))
                } else {
                    let result = match operator {
                        "+" => left_number + right_number,
                        "-" => left_number - right_number,
                        "*" => left_number * right_number,
                        "/" => left_number / right_number,
                        "\\" => (left_number / right_number).trunc(),
                        "%" | "mod" => left_number % right_number,
                        _ => unreachable!(),
                    };
                    if result.is_finite() {
                        Ok(Value::Real(result))
                    } else {
                        Err(self.runtime_error_at("resultado real fora do intervalo", expression))
                    }
                }
            }
            _ => Err(self.type_error_at("operador desconhecido", expression)),
        }
    }

    fn call(
        &mut self,
        name: &str,
        arguments: &[Expression],
        at: &impl Location,
    ) -> Result<Value, Diagnostic> {
        if let Some(routine) = self.routines.get(name).cloned() {
            return self.call_routine(&routine, arguments, at);
        }
        let values = arguments
            .iter()
            .map(|argument| self.expression(argument))
            .collect::<Result<Vec<_>, _>>()?;
        self.builtin(name, values, at)
    }

    fn call_routine(
        &mut self,
        routine: &Routine,
        arguments: &[Expression],
        at: &impl Location,
    ) -> Result<Value, Diagnostic> {
        if arguments.len() != routine.parameters.len() {
            return Err(self.error_at(
                format!(
                    "'{}' espera {} argumento(s)",
                    routine.name,
                    routine.parameters.len()
                ),
                at,
            ));
        }
        if self.calls >= self.options.max_call_depth {
            return Err(self.error_at("limite de chamadas recursivas excedido", at));
        }
        let mut scope = HashMap::new();
        for (parameter, argument) in routine.parameters.iter().zip(arguments) {
            let binding = if parameter.by_reference {
                let cell = self.lvalue(
                    argument,
                    &Statement {
                        kind: StatementKind::Break,
                        line: at.line(),
                        column: at.column(),
                    },
                )?;
                let actual = self.binding_type(
                    argument,
                    &Statement {
                        kind: StatementKind::Break,
                        line: at.line(),
                        column: at.column(),
                    },
                )?;
                if !compatible(&actual, &parameter.ty) {
                    return Err(self.error_at(
                        format!("tipo incompatível para o parâmetro '{}'", parameter.name),
                        at,
                    ));
                }
                Binding {
                    ty: parameter.ty.clone(),
                    cell,
                }
            } else {
                let value = self.expression(argument)?;
                let value = coerce(value, &parameter.ty).ok_or_else(|| {
                    self.error_at(
                        format!("tipo incompatível para o parâmetro '{}'", parameter.name),
                        at,
                    )
                })?;
                Binding {
                    ty: parameter.ty.clone(),
                    cell: Rc::new(RefCell::new(value)),
                }
            };
            scope.insert(parameter.name.clone(), binding);
        }
        self.calls += 1;
        let caller_loop_depth = self.loop_depth;
        self.loop_depth = 0;
        self.scopes.push(scope);
        let flow = self.block(&routine.body);
        self.scopes.pop();
        self.loop_depth = caller_loop_depth;
        self.calls -= 1;
        match flow? {
            Flow::Return(Some(value)) => {
                let Some(return_type) = &routine.return_type else {
                    return Err(self.error_at("procedimento não pode retornar valor", at));
                };
                coerce(value, return_type)
                    .ok_or_else(|| self.error_at("tipo de retorno incompatível", at))
            }
            Flow::Return(None) if routine.return_type.is_some() => {
                Err(self.error_at("função precisa retornar um valor", at))
            }
            Flow::Return(None) | Flow::Continue if routine.return_type.is_none() => Ok(Value::Unit),
            Flow::Return(None) => Err(self.error_at("função precisa retornar um valor", at)),
            Flow::Continue => Err(self.error_at("função terminou sem retornar um valor", at)),
            Flow::Break => Err(self.error_at("'interrompa' sem laço", at)),
            Flow::Stop => Err(self.error_at("execução interrompida", at)),
        }
    }

    fn builtin(
        &mut self,
        name: &str,
        mut args: Vec<Value>,
        at: &impl Location,
    ) -> Result<Value, Diagnostic> {
        let arity = |expected: usize| {
            if args.len() == expected {
                Ok(())
            } else {
                Err(self.error_at(format!("'{name}' espera {expected} argumento(s)"), at))
            }
        };
        let numeric = |value: &Value| value.as_real();
        match name {
            "pi" => {
                arity(0)?;
                Ok(Value::Real(std::f64::consts::PI))
            }
            "abs" | "absoluto" => {
                arity(1)?;
                match args.remove(0) {
                    Value::Integer(value) => value
                        .checked_abs()
                        .map(Value::Integer)
                        .ok_or_else(|| self.error_at("resultado fora do intervalo inteiro", at)),
                    Value::Real(value) => Ok(Value::Real(value.abs())),
                    _ => Err(self.error_at("'abs' exige um número", at)),
                }
            }
            "exp" => {
                arity(2)?;
                let base =
                    numeric(&args[0]).ok_or_else(|| self.error_at("'exp' exige números", at))?;
                let exponent =
                    numeric(&args[1]).ok_or_else(|| self.error_at("'exp' exige números", at))?;
                let result = base.powf(exponent);
                if result.is_finite() {
                    Ok(Value::Real(result))
                } else {
                    Err(self.error_at("resultado real fora do intervalo", at))
                }
            }
            "raizQuadrada" => {
                arity(1)?;
                let value = numeric(&args[0])
                    .ok_or_else(|| self.error_at("a função exige um número", at))?;
                let result = value.sqrt();
                if result.is_finite() {
                    Ok(Value::Real(result))
                } else {
                    Err(self.error_at("resultado real fora do intervalo", at))
                }
            }
            "aleatorio" => {
                arity(1)?;
                let upper = args[0]
                    .as_integer()
                    .filter(|upper| *upper > 0)
                    .ok_or_else(|| self.error_at("'aleatorio' exige um inteiro positivo", at))?;
                self.random_state ^= self.random_state << 13;
                self.random_state ^= self.random_state >> 7;
                self.random_state ^= self.random_state << 17;
                Ok(Value::Integer((self.random_state % upper as u64) as i64))
            }
            "asciiCaracter" => {
                arity(1)?;
                let Value::Integer(value) = args.remove(0) else {
                    return Err(self.error_at("'asciiCaracter' exige um inteiro", at));
                };
                u8::try_from(value)
                    .ok()
                    .filter(|value| value.is_ascii())
                    .map(|value| Value::Character(u16::from(value)))
                    .ok_or_else(|| self.error_at("código ASCII inválido", at))
            }
            "caracterAscii" => {
                arity(1)?;
                match args.remove(0) {
                    Value::Character(value) if value <= 127 => Ok(Value::Integer(i64::from(value))),
                    _ => Err(self.error_at("'caracterAscii' exige um caractere ASCII", at)),
                }
            }
            "caracterTexto" => {
                arity(2)?;
                let Value::Text(text) = args.remove(0) else {
                    return Err(self.error_at("'caracterTexto' exige um texto", at));
                };
                let index = args[0]
                    .as_integer()
                    .and_then(|index| usize::try_from(index).ok())
                    .ok_or_else(|| self.error_at("índice de texto inválido", at))?;
                text.encode_utf16()
                    .nth(index)
                    .map(Value::Character)
                    .ok_or_else(|| self.error_at("índice de texto fora dos limites", at))
            }
            "convCaractTexto" => {
                arity(1)?;
                let Value::Array { ty, values } = args.remove(0) else {
                    return Err(self.error_at("'convCaractTexto' exige um vetor", at));
                };
                if ty.scalar() != &Type::Character {
                    return Err(self.error_at("'convCaractTexto' exige um vetor de caracteres", at));
                }
                let units = values
                    .borrow()
                    .iter()
                    .map(|value| match &*value.borrow() {
                        Value::Character(character) => Ok(*character),
                        _ => Err(self.error_at("vetor de caracteres inválido", at)),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let text = String::from_utf16(&units)
                    .map_err(|_| self.error_at("vetor contém caracteres inválidos", at))?;
                Ok(Value::Text(text))
            }
            "convTextoCaract" => {
                arity(1)?;
                match args.remove(0) {
                    Value::Text(value) => {
                        let characters = value
                            .encode_utf16()
                            .map(|character| Rc::new(RefCell::new(Value::Character(character))))
                            .collect::<Vec<_>>();
                        let length = characters.len();
                        Ok(Value::Array {
                            ty: Type::Array(Box::new(Type::Character), vec![Some(length)]),
                            values: Rc::new(RefCell::new(characters)),
                        })
                    }
                    _ => Err(self.error_at("a função exige um texto", at)),
                }
            }
            "comparaTexto" => {
                arity(2)?;
                let (Value::Text(left), Value::Text(right)) = (args.remove(0), args.remove(0))
                else {
                    return Err(self.error_at("'comparaTexto' exige dois textos", at));
                };
                Ok(Value::Integer(
                    match left.encode_utf16().cmp(right.encode_utf16()) {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    },
                ))
            }
            "copia" => {
                arity(3)?;
                let (Value::Text(text), Some(start), Some(end)) =
                    (args.remove(0), args[0].as_integer(), args[1].as_integer())
                else {
                    return Err(self.error_at("'copia' espera texto e duas posições inteiras", at));
                };
                let start = usize::try_from(start)
                    .map_err(|_| self.error_at("posição inicial inválida em 'copia'", at))?;
                let end = usize::try_from(end)
                    .map_err(|_| self.error_at("posição final inválida em 'copia'", at))?;
                if start > end {
                    return Err(self.error_at("posição inicial maior que a final em 'copia'", at));
                }
                let units = text.encode_utf16().collect::<Vec<_>>();
                if end >= units.len() {
                    return Err(self.error_at("posição final fora dos limites em 'copia'", at));
                }
                let text = String::from_utf16(&units[start..=end])
                    .map_err(|_| self.error_at("'copia' divide um caractere Unicode", at))?;
                Ok(Value::Text(text))
            }
            "maiusculo" | "minusculo" => {
                arity(1)?;
                match args.remove(0) {
                    Value::Text(value) => Ok(Value::Text(if name == "maiusculo" {
                        value.to_uppercase()
                    } else {
                        value.to_lowercase()
                    })),
                    _ => Err(self.error_at("a função exige um texto", at)),
                }
            }
            "maiusculoCaracter" | "minusculoCaracter" => {
                arity(1)?;
                match args.remove(0) {
                    Value::Character(value) => {
                        let mapped = char::from_u32(u32::from(value)).and_then(|character| {
                            let mapped = if name == "maiusculoCaracter" {
                                character.to_uppercase().collect::<Vec<_>>()
                            } else {
                                character.to_lowercase().collect::<Vec<_>>()
                            };
                            (mapped.len() == 1).then(|| mapped[0].encode_utf16(&mut [0; 2])[0])
                        });
                        Ok(Value::Character(mapped.unwrap_or(value)))
                    }
                    _ => Err(self.error_at("a função exige um caractere", at)),
                }
            }
            "tamanhoTexto" => {
                arity(1)?;
                match args.remove(0) {
                    Value::Text(value) => Ok(Value::Integer(value.encode_utf16().count() as i64)),
                    _ => Err(self.error_at("'tamanhoTexto' exige um texto", at)),
                }
            }
            _ => Err(self.error_at(format!("função '{name}' não declarada"), at)),
        }
    }

    fn write(&mut self, text: &str, statement: &Statement) -> Result<(), Diagnostic> {
        self.output_bytes = self.output_bytes.saturating_add(text.len());
        if self.output_bytes > self.options.max_output_bytes {
            return Err(self.runtime_error("limite de saída excedido", statement));
        }
        self.io.write(text);
        Ok(())
    }

    fn lookup(&self, name: &str) -> Option<Binding> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
    }

    fn loop_tick(&mut self, statement: &Statement) -> Result<bool, Diagnostic> {
        self.steps += 1;
        if self.steps > self.options.max_steps {
            return Err(self.runtime_error("limite de instruções excedido", statement));
        }
        if self.io.should_stop() {
            self.stopped = true;
            return Ok(true);
        }
        Ok(false)
    }

    fn logical(&self, value: Value, at: &impl Location) -> Result<bool, Diagnostic> {
        match value {
            Value::Logical(value) => Ok(value),
            _ => Err(self.error_at("a condição deve ser lógica", at)),
        }
    }

    fn integer(&self, value: Value, at: &impl Location) -> Result<i64, Diagnostic> {
        value
            .as_integer()
            .ok_or_else(|| self.error_at("esperado um número inteiro", at))
    }

    fn error(&self, message: impl Into<String>, line: usize, column: usize) -> Diagnostic {
        Diagnostic::new(DiagnosticKind::Runtime, message, line, column)
    }

    fn error_at(&self, message: impl Into<String>, at: &impl Location) -> Diagnostic {
        self.error(message, at.line(), at.column())
    }

    fn runtime_error(&self, message: impl Into<String>, statement: &Statement) -> Diagnostic {
        self.error(message, statement.line, statement.column)
    }

    fn runtime_error_at(&self, message: impl Into<String>, expression: &Expression) -> Diagnostic {
        self.error(message, expression.line, expression.column)
    }

    fn type_error(&self, message: impl Into<String>, statement: &Statement) -> Diagnostic {
        Diagnostic::new(
            DiagnosticKind::Type,
            message,
            statement.line,
            statement.column,
        )
    }

    fn type_error_at(&self, message: impl Into<String>, expression: &Expression) -> Diagnostic {
        Diagnostic::new(
            DiagnosticKind::Type,
            message,
            expression.line,
            expression.column,
        )
    }
}

trait Location {
    fn line(&self) -> usize;
    fn column(&self) -> usize;
}

impl Location for Expression {
    fn line(&self) -> usize {
        self.line
    }
    fn column(&self) -> usize {
        self.column
    }
}

impl Location for Statement {
    fn line(&self) -> usize {
        self.line
    }
    fn column(&self) -> usize {
        self.column
    }
}

fn default_value(ty: &Type, max_array_length: usize) -> Result<Value, String> {
    default_value_with_count(ty, max_array_length, &mut 0)
}

fn default_value_with_count(
    ty: &Type,
    max_array_length: usize,
    array_elements: &mut usize,
) -> Result<Value, String> {
    match ty {
        Type::Integer => Ok(Value::Integer(0)),
        Type::Real => Ok(Value::Real(0.0)),
        Type::Logical => Ok(Value::Logical(false)),
        Type::Character => Ok(Value::Character(0)),
        Type::Text => Ok(Value::Text(String::new())),
        Type::Array(inner, dimensions) => {
            let length = dimensions.first().copied().flatten().unwrap_or(0);
            *array_elements = array_elements.saturating_add(length);
            if *array_elements > max_array_length {
                return Err("tamanho do vetor excede o limite".to_owned());
            }
            let child_type = if dimensions.len() > 1 {
                Type::Array(inner.clone(), dimensions[1..].to_vec())
            } else {
                (**inner).clone()
            };
            let values = (0..length)
                .map(|_| {
                    default_value_with_count(&child_type, max_array_length, array_elements)
                        .map(|value| Rc::new(RefCell::new(value)))
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Array {
                ty: ty.clone(),
                values: Rc::new(RefCell::new(values)),
            })
        }
    }
}

fn data_type(ty: &Type) -> DataType {
    match ty.scalar() {
        Type::Integer => DataType::Integer,
        Type::Real => DataType::Real,
        Type::Logical => DataType::Logical,
        Type::Character => DataType::Character,
        Type::Text => DataType::Text,
        Type::Array(_, _) => unreachable!(),
    }
}

fn parse_input(input: &str, ty: DataType, character_only: bool) -> Result<Value, String> {
    if character_only || ty == DataType::Character {
        return input
            .encode_utf16()
            .next()
            .map(Value::Character)
            .ok_or_else(|| "esperado um caractere".to_owned());
    }
    if ty == DataType::Text {
        return Ok(Value::Text(input.to_owned()));
    }
    let input = input.trim();
    match ty {
        DataType::Integer => input
            .parse()
            .map(Value::Integer)
            .map_err(|_| "entrada não é um inteiro válido".to_owned()),
        DataType::Real => input
            .parse()
            .map(Value::Real)
            .map_err(|_| "entrada não é um número real válido".to_owned()),
        DataType::Logical => match input.to_lowercase().as_str() {
            "verdadeiro" => Ok(Value::Logical(true)),
            "falso" => Ok(Value::Logical(false)),
            _ => Err("entrada lógica deve ser 'verdadeiro' ou 'falso'".to_owned()),
        },
        DataType::Text => unreachable!(),
        DataType::Character => unreachable!(),
    }
}

fn compatible(actual: &Type, expected: &Type) -> bool {
    match (actual, expected) {
        (Type::Array(actual, actual_dimensions), Type::Array(expected, expected_dimensions)) => {
            compatible(actual, expected)
                && actual_dimensions.len() == expected_dimensions.len()
                && actual_dimensions
                    .iter()
                    .zip(expected_dimensions)
                    .all(|(actual, expected)| {
                        expected.is_none_or(|expected| *actual == Some(expected))
                    })
        }
        (Type::Integer, Type::Real) => true,
        _ => actual == expected,
    }
}

fn coerce(value: Value, ty: &Type) -> Option<Value> {
    match (value, ty) {
        (value @ Value::Integer(_), Type::Integer)
        | (value @ Value::Real(_), Type::Real)
        | (value @ Value::Logical(_), Type::Logical)
        | (value @ Value::Character(_), Type::Character)
        | (value @ Value::Text(_), Type::Text) => Some(value),
        (Value::Integer(value), Type::Real) => Some(Value::Real(value as f64)),
        (Value::Array { ty, values }, expected @ Type::Array(_, _)) => {
            compatible(&ty, expected).then_some(Value::Array { ty, values })
        }
        _ => None,
    }
}

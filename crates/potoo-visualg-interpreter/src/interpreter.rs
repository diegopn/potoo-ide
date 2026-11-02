use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::ast::{
    Expression, ExpressionKind, FormattedExpression, Program, RoutineKind, Statement,
    StatementKind, Type,
};
use crate::environment::{Binding, Environment};
use crate::value::{ArrayValue, Value};
use crate::{
    ControlAction, DataType, Diagnostic, DiagnosticKind, ExecutionEvent, ExecutionOptions,
    InputPrompt, PauseReason, RuntimeIo, SourceLocation, VariableSnapshot,
};

enum Flow {
    Normal,
    Return(Value, usize, usize),
    Break(usize, usize),
    Stop,
}

pub(crate) struct Interpreter<'a> {
    program: Program,
    environment: Environment,
    io: &'a mut dyn RuntimeIo,
    options: ExecutionOptions,
    steps: u64,
    output_bytes: usize,
    call_depth: usize,
    loop_depth: usize,
    random_range: Option<(i64, i64)>,
    data_path: Option<String>,
    data_lines: Vec<String>,
    data_index: usize,
    record_data: bool,
    file_bytes: usize,
    echo_input: bool,
    timer_millis: Option<u64>,
    total_wait_millis: u64,
    chronometer: Option<Instant>,
    step_next: bool,
    stopped: bool,
}

impl<'a> Interpreter<'a> {
    pub(crate) fn new(
        program: Program,
        io: &'a mut dyn RuntimeIo,
        options: ExecutionOptions,
    ) -> Self {
        Self {
            program,
            environment: Environment::new(),
            io,
            options,
            steps: 0,
            output_bytes: 0,
            call_depth: 0,
            loop_depth: 0,
            random_range: None,
            data_path: None,
            data_lines: Vec::new(),
            data_index: 0,
            record_data: false,
            file_bytes: 0,
            echo_input: false,
            timer_millis: None,
            total_wait_millis: 0,
            chronometer: None,
            step_next: false,
            stopped: false,
        }
    }

    pub(crate) fn execute(mut self) -> Result<(), Vec<Diagnostic>> {
        if let Some(declaration) = self.program.arquivo.clone() {
            let path = declaration.path;
            self.data_path = Some(path.clone());
            let lines = self.io.read_data_file(&path).map_err(|message| {
                vec![Diagnostic::new(
                    DiagnosticKind::Runtime,
                    format!("não foi possível ler o arquivo de dados: {message}"),
                    declaration.line,
                    declaration.column,
                )]
            })?;
            if let Some(lines) = lines {
                self.file_bytes = lines.iter().fold(0_usize, |bytes, line| {
                    bytes.saturating_add(line.len().saturating_add(1))
                });
                if self.file_bytes > self.options.max_file_bytes {
                    return Err(vec![Diagnostic::new(
                        DiagnosticKind::Runtime,
                        format!(
                            "arquivo de dados excede o limite de {} bytes",
                            self.options.max_file_bytes
                        ),
                        declaration.line,
                        declaration.column,
                    )]);
                }
                self.data_lines = lines;
            } else {
                self.io.create_data_file(&path).map_err(|message| {
                    vec![Diagnostic::new(
                        DiagnosticKind::Runtime,
                        format!("não foi possível criar o arquivo de dados: {message}"),
                        declaration.line,
                        declaration.column,
                    )]
                })?;
                self.record_data = true;
            }
        }
        let globals = self.program.globals.clone();
        for variable in &globals {
            self.environment
                .declare(variable, self.options.max_variable_slots)
                .map_err(|diagnostic| vec![diagnostic])?;
        }
        match self.execute_statements(&self.program.statements.clone()) {
            Ok(Flow::Normal) => Ok(()),
            Ok(Flow::Return(_, line, column)) => Err(vec![Diagnostic::new(
                DiagnosticKind::Runtime,
                "'retorne' não pode aparecer fora de uma função",
                line,
                column,
            )]),
            Ok(Flow::Break(line, column)) => Err(vec![Diagnostic::new(
                DiagnosticKind::Runtime,
                "'interrompa' só pode ser usado dentro de um laço",
                line,
                column,
            )]),
            Ok(Flow::Stop) => Ok(()),
            Err(diagnostic) => Err(vec![diagnostic]),
        }
    }

    fn execute_statements(&mut self, statements: &[Statement]) -> Result<Flow, Diagnostic> {
        for statement in statements {
            match self.execute_statement(statement)? {
                Flow::Normal => {}
                flow @ (Flow::Return(_, _, _) | Flow::Break(_, _) | Flow::Stop) => return Ok(flow),
            }
        }
        Ok(Flow::Normal)
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<Flow, Diagnostic> {
        if self.stopped {
            return Ok(Flow::Stop);
        }
        self.tick(statement)?;
        if self.stopped {
            return Ok(Flow::Stop);
        }
        if self.before_statement(statement)? {
            return Ok(Flow::Stop);
        }
        match &statement.kind {
            StatementKind::Write { newline, values } => {
                for formatted in values {
                    self.write_formatted(formatted, statement)?;
                    if self.stopped {
                        return Ok(Flow::Stop);
                    }
                }
                if *newline && !self.stopped {
                    self.write_output("\n", statement)?;
                }
                Ok(Flow::Normal)
            }
            StatementKind::Read(expressions) => {
                for expression in expressions {
                    self.read_into(expression)?;
                    if self.stopped {
                        return Ok(Flow::Stop);
                    }
                }
                Ok(Flow::Normal)
            }
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let value = self.evaluate(condition)?;
                if self.stopped {
                    return Ok(Flow::Stop);
                }
                let Value::Logical(condition_value) = value else {
                    return Err(runtime_error(
                        condition,
                        "a condição de 'se' precisa ser lógica",
                    ));
                };
                if condition_value {
                    self.execute_statements(then_branch)
                } else {
                    self.execute_statements(else_branch)
                }
            }
            StatementKind::Choice {
                expression,
                cases,
                default,
            } => {
                let selected = self.evaluate(expression)?;
                if self.stopped {
                    return Ok(Flow::Stop);
                }
                for case in cases {
                    for value in &case.values {
                        let candidate = self.evaluate(value)?;
                        if self.stopped {
                            return Ok(Flow::Stop);
                        }
                        if matches!(
                            compare_values(selected.clone(), "=", candidate, expression)?,
                            Value::Logical(true)
                        ) {
                            return self.execute_statements(&case.body);
                        }
                    }
                }
                self.execute_statements(default)
            }
            StatementKind::While { condition, body } => {
                let mut first_iteration = true;
                loop {
                    if !first_iteration {
                        self.tick(statement)?;
                        if self.stopped || self.before_statement(statement)? {
                            return Ok(Flow::Stop);
                        }
                    }
                    first_iteration = false;
                    match self.evaluate(condition)? {
                        Value::Logical(true) => {}
                        Value::Logical(false) => break,
                        _ => {
                            return Err(runtime_error(
                                condition,
                                "a condição de 'enquanto' precisa ser lógica",
                            ));
                        }
                    }
                    self.loop_depth += 1;
                    let result = self.execute_statements(body);
                    self.loop_depth -= 1;
                    match result? {
                        Flow::Normal => {}
                        Flow::Break(_, _) => break,
                        Flow::Stop => return Ok(Flow::Stop),
                        flow @ Flow::Return(_, _, _) => return Ok(flow),
                    }
                }
                Ok(Flow::Normal)
            }
            StatementKind::Repeat { body, condition } => {
                let mut first_iteration = true;
                loop {
                    if !first_iteration {
                        self.tick(statement)?;
                        if self.stopped || self.before_statement(statement)? {
                            return Ok(Flow::Stop);
                        }
                    }
                    first_iteration = false;
                    self.loop_depth += 1;
                    let result = self.execute_statements(body);
                    self.loop_depth -= 1;
                    match result? {
                        Flow::Normal => {}
                        Flow::Break(_, _) => break,
                        Flow::Stop => return Ok(Flow::Stop),
                        flow @ Flow::Return(_, _, _) => return Ok(flow),
                    }
                    if let Some(condition) = condition {
                        match self.evaluate(condition)? {
                            Value::Logical(true) => break,
                            Value::Logical(false) => {}
                            _ => {
                                return Err(runtime_error(
                                    condition,
                                    "a condição de 'ate' precisa ser lógica",
                                ));
                            }
                        }
                    }
                }
                Ok(Flow::Normal)
            }
            StatementKind::For {
                variable,
                start,
                end,
                step,
                body,
            } => {
                let start = self.evaluate_integer(start)?;
                let end = self.evaluate_integer(end)?;
                let step = match step {
                    Some(step) => self.evaluate_integer(step)?,
                    None => 1,
                };
                if step == 0 {
                    return Err(runtime_error(
                        statement,
                        "o passo de 'para' não pode ser zero",
                    ));
                }
                let target = Expression {
                    kind: ExpressionKind::Identifier(variable.clone()),
                    line: statement.line,
                    column: statement.column,
                };
                let (cell, data_type) = self.locate_cell(&target)?;
                if data_type != Type::Integer {
                    return Err(runtime_error(
                        &target,
                        "a variável de 'para' precisa ser inteira",
                    ));
                }
                let mut value = start;
                let mut first_iteration = true;
                loop {
                    if !first_iteration {
                        self.tick(statement)?;
                        if self.stopped || self.before_statement(statement)? {
                            return Ok(Flow::Stop);
                        }
                    }
                    first_iteration = false;
                    if !(if step > 0 { value <= end } else { value >= end }) {
                        break;
                    }
                    *cell.borrow_mut() = Value::Integer(value);
                    self.loop_depth += 1;
                    let result = self.execute_statements(body);
                    self.loop_depth -= 1;
                    match result? {
                        Flow::Normal => {}
                        Flow::Break(_, _) => break,
                        Flow::Stop => return Ok(Flow::Stop),
                        flow @ Flow::Return(_, _, _) => return Ok(flow),
                    }
                    let Some(next_value) = value.checked_add(step) else {
                        break;
                    };
                    value = next_value;
                }
                Ok(Flow::Normal)
            }
            StatementKind::Break => {
                if self.loop_depth == 0 {
                    return Err(runtime_error(
                        statement,
                        "'interrompa' só pode ser usado dentro de um laço",
                    ));
                }
                Ok(Flow::Break(statement.line, statement.column))
            }
            StatementKind::Return(value) => {
                let value = match value {
                    Some(expression) => self.evaluate(expression)?,
                    None => Value::Void,
                };
                if self.stopped {
                    return Ok(Flow::Stop);
                }
                Ok(Flow::Return(value, statement.line, statement.column))
            }
            StatementKind::Expression(expression) => {
                self.evaluate(expression)?;
                if self.stopped {
                    Ok(Flow::Stop)
                } else {
                    Ok(Flow::Normal)
                }
            }
            StatementKind::Aleatorio {
                enabled,
                minimum,
                maximum,
            } => {
                self.random_range = enabled.then_some((*minimum, *maximum));
                Ok(Flow::Normal)
            }
            StatementKind::Timer(millis) => {
                self.timer_millis = *millis;
                Ok(Flow::Normal)
            }
            StatementKind::Pause | StatementKind::Debug(_) => Ok(Flow::Normal),
            StatementKind::Echo(enabled) => {
                self.echo_input = *enabled;
                Ok(Flow::Normal)
            }
            StatementKind::Chronometer(enabled) => {
                if *enabled {
                    self.chronometer = Some(Instant::now());
                    self.write_output("Cronômetro iniciado.\n", statement)?;
                } else if let Some(start) = self.chronometer.take() {
                    let elapsed = start.elapsed();
                    let seconds = elapsed.as_secs();
                    let millis = elapsed.subsec_millis();
                    self.write_output(&format!("Cronômetro terminado. Tempo decorrido: {seconds} segundo(s) e {millis} ms\n"), statement)?;
                }
                Ok(Flow::Normal)
            }
            StatementKind::ClearScreen => {
                self.io.clear_output();
                Ok(Flow::Normal)
            }
        }
    }

    fn before_statement(&mut self, statement: &Statement) -> Result<bool, Diagnostic> {
        if let Some(millis) = self.timer_millis {
            let total = self.total_wait_millis.saturating_add(millis);
            if total > self.options.max_wait_millis {
                return Err(runtime_error(
                    statement,
                    &format!(
                        "programa excede o limite total de espera ({} ms)",
                        self.options.max_wait_millis
                    ),
                ));
            }
            self.total_wait_millis = total;
            self.io
                .wait(Duration::from_millis(millis))
                .map_err(|message| {
                    runtime_error(statement, &format!("falha durante 'timer': {message}"))
                })?;
        }
        let pause_reason = match &statement.kind {
            StatementKind::Pause => Some(PauseReason::Pause),
            StatementKind::Debug(condition) => match self.evaluate(condition)? {
                Value::Logical(true) => Some(PauseReason::Debug),
                Value::Logical(false) => None,
                _ => {
                    return Err(runtime_error(
                        condition,
                        "a condição de 'debug' precisa ser lógica",
                    ));
                }
            },
            _ => None,
        };
        if self.stopped {
            return Ok(true);
        }
        let step_pause = self.step_next;
        if self.io.is_debugging() || pause_reason.is_some() || step_pause {
            let event = ExecutionEvent {
                location: SourceLocation {
                    line: statement.line,
                    column: statement.column,
                },
                variables: self
                    .environment
                    .snapshot()
                    .into_iter()
                    .map(|(name, value)| VariableSnapshot { name, value })
                    .collect(),
                pause_reason: pause_reason.or(step_pause.then_some(PauseReason::Step)),
            };
            let action = self.io.on_execution_event(&event).map_err(|message| {
                runtime_error(
                    statement,
                    &format!("falha no controle de execução: {message}"),
                )
            })?;
            match action {
                ControlAction::Continue => self.step_next = false,
                ControlAction::Step => self.step_next = true,
                ControlAction::Stop => {
                    self.stopped = true;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn tick(&mut self, statement: &Statement) -> Result<(), Diagnostic> {
        if self.io.should_stop() {
            self.stopped = true;
            return Ok(());
        }
        self.steps = self.steps.saturating_add(1);
        if self.steps > self.options.max_steps {
            return Err(runtime_error(
                statement,
                &format!(
                    "programa excedeu o limite de passos ({})",
                    self.options.max_steps
                ),
            ));
        }
        Ok(())
    }

    fn write_formatted(
        &mut self,
        formatted: &FormattedExpression,
        statement: &Statement,
    ) -> Result<(), Diagnostic> {
        let value = self.evaluate(&formatted.expression)?;
        if self.stopped {
            return Ok(());
        }
        let width = formatted
            .width
            .as_ref()
            .map(|expression| self.evaluate_integer(expression))
            .transpose()?;
        let precision = formatted
            .precision
            .as_ref()
            .map(|expression| self.evaluate_integer(expression))
            .transpose()?;
        if self.stopped {
            return Ok(());
        }
        let text = crate::format::format_value(
            &value,
            width,
            precision,
            self.options.max_string_bytes,
            &formatted.expression,
        )?;
        self.write_output(&text, statement)
    }

    fn write_output(&mut self, text: &str, statement: &impl HasLocation) -> Result<(), Diagnostic> {
        let output_bytes = self
            .output_bytes
            .checked_add(text.len())
            .ok_or_else(|| runtime_error(statement, "limite de saída excedido"))?;
        if output_bytes > self.options.max_output_bytes {
            return Err(runtime_error(
                statement,
                &format!(
                    "programa excede o limite de saída ({} bytes)",
                    self.options.max_output_bytes
                ),
            ));
        }
        self.output_bytes = output_bytes;
        self.io.write(text);
        Ok(())
    }

    fn read_into(&mut self, target: &Expression) -> Result<(), Diagnostic> {
        let (cell, data_type) = self.locate_cell(target)?;
        if self.stopped {
            return Ok(());
        }
        let variable_name = match &target.kind {
            ExpressionKind::Identifier(name) | ExpressionKind::Index { name, .. } => name.clone(),
            _ => "valor".to_owned(),
        };
        let prompt = InputPrompt {
            variable_name: variable_name.clone(),
            data_type: public_data_type(data_type),
        };
        let (value, echo) = if let Some((minimum, maximum)) = self.random_range {
            if data_type == Type::Logical {
                let input = self.io.read_line(&prompt).map_err(|message| {
                    runtime_error(
                        target,
                        &format!("não foi possível ler a entrada: {message}"),
                    )
                })?;
                (
                    parse_input(&input, data_type)
                        .map_err(|message| runtime_error(target, &message))?,
                    Some(input),
                )
            } else {
                (random_input(data_type, minimum, maximum), None)
            }
        } else if self.data_index < self.data_lines.len() {
            let input = self.data_lines[self.data_index].clone();
            self.data_index += 1;
            (
                parse_input(&input, data_type)
                    .map_err(|message| runtime_error(target, &message))?,
                Some(input),
            )
        } else {
            let input = self.io.read_line(&prompt).map_err(|message| {
                runtime_error(
                    target,
                    &format!("não foi possível ler a entrada: {message}"),
                )
            })?;
            let value = parse_input(&input, data_type)
                .map_err(|message| runtime_error(target, &message))?;
            if self.record_data {
                let next_bytes = self
                    .file_bytes
                    .saturating_add(input.len())
                    .saturating_add(1);
                if next_bytes > self.options.max_file_bytes {
                    return Err(runtime_error(
                        target,
                        &format!(
                            "arquivo de dados excede o limite de {} bytes",
                            self.options.max_file_bytes
                        ),
                    ));
                }
                let path = self.data_path.as_deref().unwrap_or_default();
                self.io
                    .append_data_file_line(path, &input)
                    .map_err(|message| {
                        runtime_error(
                            target,
                            &format!("não foi possível gravar o arquivo de dados: {message}"),
                        )
                    })?;
                self.file_bytes = next_bytes;
            }
            (value, Some(input))
        };
        self.ensure_value_size(&value, target)?;
        *cell.borrow_mut() = value;
        if self.echo_input {
            if echo.is_some() || self.random_range.is_some() {
                let value = cell.borrow().clone();
                let text = crate::format::format_value(
                    &value,
                    None,
                    None,
                    self.options.max_string_bytes,
                    target,
                )?;
                self.write_output(&text, target)?;
                self.write_output("\n", target)?;
            }
        }
        Ok(())
    }

    fn evaluate(&mut self, expression: &Expression) -> Result<Value, Diagnostic> {
        if self.stopped {
            return Ok(Value::Void);
        }
        match &expression.kind {
            ExpressionKind::Integer(value) => Ok(Value::Integer(*value)),
            ExpressionKind::Real(value) => Ok(Value::Real(*value)),
            ExpressionKind::String(value) => {
                self.ensure_string_size(value, expression)?;
                Ok(Value::Text(value.clone()))
            }
            ExpressionKind::Logical(value) => Ok(Value::Logical(*value)),
            ExpressionKind::Identifier(name) => {
                if crate::builtins::contains(name) {
                    if matches!(name.as_str(), "pi" | "rand") {
                        return crate::builtins::call(
                            name,
                            Vec::new(),
                            expression,
                            self.options.max_string_bytes,
                        );
                    }
                }
                let binding = self.environment.lookup(name, expression)?;
                let value = binding.value.borrow().clone();
                Ok(value)
            }
            ExpressionKind::Unary { operator, operand } => {
                let value = self.evaluate(operand)?;
                if self.stopped {
                    return Ok(Value::Void);
                }
                apply_unary(operator, value, expression)
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let left_value = self.evaluate(left)?;
                if self.stopped {
                    return Ok(Value::Void);
                }
                if operator == "e" {
                    let Value::Logical(left_value) = left_value else {
                        return Err(runtime_error(
                            left.as_ref(),
                            "o operador 'e' exige valores lógicos",
                        ));
                    };
                    if !left_value {
                        return Ok(Value::Logical(false));
                    }
                } else if operator == "ou" {
                    let Value::Logical(left_value) = left_value else {
                        return Err(runtime_error(
                            left.as_ref(),
                            "o operador 'ou' exige valores lógicos",
                        ));
                    };
                    if left_value {
                        return Ok(Value::Logical(true));
                    }
                }
                let right_value = self.evaluate(right)?;
                if self.stopped {
                    return Ok(Value::Void);
                }
                apply_binary(
                    left_value,
                    operator,
                    right_value,
                    expression,
                    self.options.max_string_bytes,
                )
            }
            ExpressionKind::Assign { target, value } => {
                let value = self.evaluate(value)?;
                if self.stopped {
                    return Ok(Value::Void);
                }
                let (cell, data_type) = self.locate_cell(target)?;
                if self.stopped {
                    return Ok(Value::Void);
                }
                let value = value
                    .coerce_to(data_type)
                    .map_err(|message| runtime_error(target.as_ref(), &message))?;
                self.ensure_value_size(&value, expression)?;
                *cell.borrow_mut() = value.clone();
                Ok(value)
            }
            ExpressionKind::Call { name, arguments } => {
                if crate::builtins::contains(name) {
                    let mut values = Vec::with_capacity(arguments.len());
                    for argument in arguments {
                        values.push(self.evaluate(argument)?);
                        if self.stopped {
                            return Ok(Value::Void);
                        }
                    }
                    return crate::builtins::call(
                        name,
                        values,
                        expression,
                        self.options.max_string_bytes,
                    );
                }
                self.call_routine(name, arguments, expression)
            }
            ExpressionKind::Index { .. } => {
                let (cell, _) = self.locate_cell(expression)?;
                if self.stopped {
                    return Ok(Value::Void);
                }
                Ok(cell.borrow().clone())
            }
        }
    }

    fn locate_cell(
        &mut self,
        expression: &Expression,
    ) -> Result<(Rc<RefCell<Value>>, Type), Diagnostic> {
        match &expression.kind {
            ExpressionKind::Identifier(name) => {
                let binding = self.environment.lookup(name, expression)?;
                if matches!(&*binding.value.borrow(), Value::Array(_)) {
                    return Err(runtime_error(
                        expression,
                        &format!("o vetor '{name}' exige índice"),
                    ));
                }
                Ok((binding.value, binding.data_type))
            }
            ExpressionKind::Index { name, indexes } => {
                let binding = self.environment.lookup(name, expression)?;
                let index_values = indexes
                    .iter()
                    .map(|index| self.evaluate_integer(index))
                    .collect::<Result<Vec<_>, _>>()?;
                if indexes.is_empty() {
                    return Err(runtime_error(expression, "esperado índice do vetor"));
                }
                let cell = self
                    .environment
                    .index_cell(name, &index_values, expression, indexes)?;
                Ok((cell, binding.data_type))
            }
            _ => Err(runtime_error(
                expression,
                "esperada uma variável para receber o valor",
            )),
        }
    }

    fn evaluate_integer(&mut self, expression: &Expression) -> Result<i64, Diagnostic> {
        if self.stopped {
            return Ok(0);
        }
        match self.evaluate(expression)? {
            Value::Integer(value) => Ok(value),
            value => Err(runtime_error(
                expression,
                &format!("esperado inteiro, recebido {}", value.type_name()),
            )),
        }
    }

    fn ensure_value_size(&self, value: &Value, expression: &Expression) -> Result<(), Diagnostic> {
        if let Value::Text(text) = value {
            self.ensure_string_size(text, expression)?;
        }
        Ok(())
    }

    fn ensure_string_size(&self, text: &str, expression: &Expression) -> Result<(), Diagnostic> {
        if text.len() > self.options.max_string_bytes {
            return Err(runtime_error(
                expression,
                &format!(
                    "texto excede o limite de {} bytes",
                    self.options.max_string_bytes
                ),
            ));
        }
        Ok(())
    }

    fn call_routine(
        &mut self,
        name: &str,
        arguments: &[Expression],
        call: &Expression,
    ) -> Result<Value, Diagnostic> {
        let routine = self
            .program
            .routines
            .iter()
            .find(|routine| routine.name == name)
            .cloned()
            .ok_or_else(|| {
                runtime_error(
                    call,
                    &format!("função ou procedimento desconhecido: '{name}'"),
                )
            })?;
        if arguments.len() != routine.parameters.len() {
            return Err(runtime_error(
                call,
                &format!(
                    "'{}' espera {} argumento(s), mas recebeu {}",
                    routine.name,
                    routine.parameters.len(),
                    arguments.len()
                ),
            ));
        }
        if self.call_depth >= self.options.max_call_depth {
            return Err(runtime_error(
                call,
                &format!(
                    "profundidade máxima de chamadas ({}) excedida",
                    self.options.max_call_depth
                ),
            ));
        }

        let mut prepared_parameters = Vec::with_capacity(arguments.len());
        for (parameter, argument) in routine.parameters.iter().zip(arguments) {
            let binding = if parameter.by_reference {
                let (binding, slots) = self.reference_argument(argument)?;
                if binding.data_type != parameter.data_type
                    || binding.dimensions.len() != parameter.dimensions
                {
                    return Err(runtime_error(
                        argument,
                        "o argumento de 'var' precisa ter o mesmo tipo e dimensões do parâmetro",
                    ));
                }
                (binding, slots)
            } else if parameter.dimensions > 0 {
                let ExpressionKind::Identifier(name) = &argument.kind else {
                    return Err(runtime_error(
                        argument,
                        "parâmetro de vetor precisa receber uma variável vetorial",
                    ));
                };
                let source = self.environment.lookup(name, argument)?;
                if source.data_type != parameter.data_type
                    || source.dimensions.len() != parameter.dimensions
                {
                    return Err(runtime_error(
                        argument,
                        "o argumento de vetor precisa ter o mesmo tipo e dimensões do parâmetro",
                    ));
                }
                let slots = match &*source.value.borrow() {
                    Value::Array(array) => array.elements.len(),
                    _ => {
                        return Err(runtime_error(
                            argument,
                            "parâmetro de vetor precisa receber uma variável vetorial",
                        ));
                    }
                };
                (
                    Binding {
                        data_type: source.data_type,
                        dimensions: source.dimensions,
                        value: Rc::new(RefCell::new(deep_copy_value(
                            source.value.borrow().clone(),
                        ))),
                    },
                    slots,
                )
            } else {
                let value = self.evaluate(argument)?;
                let value = value
                    .coerce_to(parameter.data_type)
                    .map_err(|message| runtime_error(argument, &message))?;
                (
                    Binding {
                        data_type: parameter.data_type,
                        dimensions: Vec::new(),
                        value: Rc::new(RefCell::new(deep_copy_value(value))),
                    },
                    1,
                )
            };
            prepared_parameters.push((parameter.name.clone(), binding));
        }

        self.environment.push_scope();
        self.call_depth += 1;
        let caller_loop_depth = self.loop_depth;
        self.loop_depth = 0;
        let result = (|| {
            for (name, (binding, slots)) in prepared_parameters {
                self.environment.insert_binding(
                    name,
                    binding,
                    slots,
                    self.options.max_variable_slots,
                    call.line,
                    call.column,
                )?;
            }
            for local in &routine.locals {
                self.environment
                    .declare(local, self.options.max_variable_slots)?;
            }
            match self.execute_statements(&routine.body)? {
                Flow::Normal => {
                    if routine.kind == RoutineKind::Function {
                        Err(Diagnostic::new(
                            DiagnosticKind::Runtime,
                            format!("a função '{}' terminou sem 'retorne'", routine.name),
                            routine.line,
                            routine.column,
                        ))
                    } else {
                        Ok(Value::Void)
                    }
                }
                Flow::Break(line, column) => Err(Diagnostic::new(
                    DiagnosticKind::Runtime,
                    "'interrompa' não pode sair de uma rotina",
                    line,
                    column,
                )),
                Flow::Stop => {
                    self.stopped = true;
                    Ok(Value::Void)
                }
                Flow::Return(value, line, column) => {
                    match (routine.kind, routine.return_type, value) {
                        (RoutineKind::Procedure, _, Value::Void) => Ok(Value::Void),
                        (RoutineKind::Procedure, _, _) => Err(Diagnostic::new(
                            DiagnosticKind::Runtime,
                            "procedimentos não podem retornar um valor",
                            line,
                            column,
                        )),
                        (RoutineKind::Function, Some(return_type), value) => {
                            value.coerce_to(return_type).map_err(|message| {
                                Diagnostic::new(DiagnosticKind::Runtime, message, line, column)
                            })
                        }
                        (RoutineKind::Function, None, _) => Err(Diagnostic::new(
                            DiagnosticKind::Runtime,
                            "função sem tipo de retorno",
                            line,
                            column,
                        )),
                    }
                }
            }
        })();
        self.environment.pop_scope();
        self.call_depth -= 1;
        self.loop_depth = caller_loop_depth;
        result
    }

    fn reference_argument(
        &mut self,
        expression: &Expression,
    ) -> Result<(Binding, usize), Diagnostic> {
        match &expression.kind {
            ExpressionKind::Identifier(name) => {
                let binding = self.environment.lookup(name, expression)?;
                Ok((binding, 0))
            }
            ExpressionKind::Index { name, indexes } => {
                let original = self.environment.lookup(name, expression)?;
                let values = indexes
                    .iter()
                    .map(|index| self.evaluate_integer(index))
                    .collect::<Result<Vec<_>, _>>()?;
                let cell = self
                    .environment
                    .index_cell(name, &values, expression, indexes)?;
                Ok((
                    Binding {
                        data_type: original.data_type,
                        dimensions: Vec::new(),
                        value: cell,
                    },
                    0,
                ))
            }
            _ => Err(runtime_error(
                expression,
                "argumento 'var' precisa ser uma variável",
            )),
        }
    }
}

fn deep_copy_value(value: Value) -> Value {
    match value {
        Value::Array(array) => Value::Array(ArrayValue {
            dimensions: array.dimensions,
            elements: array
                .elements
                .into_iter()
                .map(|cell| Rc::new(RefCell::new(deep_copy_value(cell.borrow().clone()))))
                .collect(),
        }),
        value => value,
    }
}

fn random_input(data_type: Type, minimum: i64, maximum: i64) -> Value {
    match data_type {
        Type::Integer => {
            let width = maximum.saturating_sub(minimum).saturating_add(1).max(1) as f64;
            let offset = (crate::builtins::random_unit() * width) as i64;
            Value::Integer(minimum.saturating_add(offset).min(maximum))
        }
        Type::Real => {
            let unit = crate::builtins::random_unit();
            Value::Real(minimum as f64 + (maximum as f64 - minimum as f64) * unit)
        }
        Type::Character => {
            let text = (0..5)
                .map(|_| (b'A' + (crate::builtins::random_unit() * 26.0) as u8) as char)
                .collect();
            Value::Text(text)
        }
        Type::Logical => Value::Logical(false),
    }
}

fn apply_unary(operator: &str, value: Value, expression: &Expression) -> Result<Value, Diagnostic> {
    match (operator, value) {
        ("+", value @ Value::Integer(_)) | ("+", value @ Value::Real(_)) => Ok(value),
        ("-", Value::Integer(value)) => value
            .checked_neg()
            .map(Value::Integer)
            .ok_or_else(|| runtime_error(expression, "estouro na operação inteira")),
        ("-", Value::Real(value)) => checked_real(-value, expression),
        ("nao" | "not", Value::Logical(value)) => Ok(Value::Logical(!value)),
        ("+" | "-", value) => Err(runtime_error(
            expression,
            &format!(
                "o operador '{operator}' exige um número, recebeu {}",
                value.type_name()
            ),
        )),
        ("nao" | "not", value) => Err(runtime_error(
            expression,
            &format!(
                "o operador 'nao' exige um valor lógico, recebeu {}",
                value.type_name()
            ),
        )),
        _ => Err(runtime_error(expression, "operador unário desconhecido")),
    }
}

fn apply_binary(
    left: Value,
    operator: &str,
    right: Value,
    expression: &Expression,
    max_string_bytes: usize,
) -> Result<Value, Diagnostic> {
    if matches!(operator, "=" | "<>" | "<" | "<=" | ">" | ">=") {
        return compare_values(left, operator, right, expression);
    }
    if matches!(operator, "e" | "ou" | "xou") {
        let (Value::Logical(left), Value::Logical(right)) = (left, right) else {
            return Err(runtime_error(
                expression,
                "operadores lógicos exigem valores lógicos",
            ));
        };
        return Ok(Value::Logical(match operator {
            "e" => left && right,
            "ou" => left || right,
            "xou" => left ^ right,
            _ => unreachable!(),
        }));
    }
    if operator == "+" {
        if let (Value::Text(left), Value::Text(right)) = (&left, &right) {
            let length = left
                .len()
                .checked_add(right.len())
                .ok_or_else(|| runtime_error(expression, "texto concatenado excede o limite"))?;
            if length > max_string_bytes {
                return Err(runtime_error(
                    expression,
                    &format!("texto concatenado excede o limite de {max_string_bytes} bytes"),
                ));
            }
            return Ok(Value::Text(format!("{left}{right}")));
        }
    }

    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => match operator {
            "+" => left
                .checked_add(right)
                .map(Value::Integer)
                .ok_or_else(|| runtime_error(expression, "estouro na operação inteira")),
            "-" => left
                .checked_sub(right)
                .map(Value::Integer)
                .ok_or_else(|| runtime_error(expression, "estouro na operação inteira")),
            "*" => left
                .checked_mul(right)
                .map(Value::Integer)
                .ok_or_else(|| runtime_error(expression, "estouro na operação inteira")),
            "/" if right != 0 => checked_real(left as f64 / right as f64, expression),
            "/" => Err(runtime_error(expression, "divisão por zero")),
            "\\" if right != 0 => left
                .checked_div(right)
                .map(Value::Integer)
                .ok_or_else(|| runtime_error(expression, "estouro na operação inteira")),
            "\\" => Err(runtime_error(expression, "divisão por zero")),
            "mod" | "%" if right != 0 => left
                .checked_rem(right)
                .map(Value::Integer)
                .ok_or_else(|| runtime_error(expression, "estouro na operação inteira")),
            "mod" | "%" => Err(runtime_error(expression, "divisão por zero")),
            "^" if right >= 0 => u32::try_from(right)
                .ok()
                .and_then(|exponent| left.checked_pow(exponent))
                .map(Value::Integer)
                .ok_or_else(|| runtime_error(expression, "estouro na potência inteira")),
            "^" => checked_real((left as f64).powf(right as f64), expression),
            _ => Err(runtime_error(expression, "operador numérico desconhecido")),
        },
        (left, right) if is_numeric(&left) && is_numeric(&right) => {
            if matches!(operator, "\\" | "mod" | "%") {
                return Err(runtime_error(
                    expression,
                    &format!("o operador '{operator}' exige operandos inteiros"),
                ));
            }
            let left = as_real(&left);
            let right = as_real(&right);
            if matches!(operator, "/" | "\\" | "mod" | "%") && right == 0.0 {
                return Err(runtime_error(expression, "divisão por zero"));
            }
            let result = match operator {
                "+" => left + right,
                "-" => left - right,
                "*" => left * right,
                "/" | "\\" => left / right,
                "mod" | "%" => left % right,
                "^" => left.powf(right),
                _ => return Err(runtime_error(expression, "operador numérico desconhecido")),
            };
            checked_real(result, expression)
        }
        (left, right) => Err(runtime_error(
            expression,
            &format!(
                "o operador '{operator}' não aceita {} e {}",
                left.type_name(),
                right.type_name()
            ),
        )),
    }
}

fn compare_values(
    left: Value,
    operator: &str,
    right: Value,
    expression: &Expression,
) -> Result<Value, Diagnostic> {
    let ordering = match (&left, &right) {
        (Value::Integer(left), Value::Integer(right)) => Some(left.cmp(right)),
        (left, right) if is_numeric(left) && is_numeric(right) => {
            as_real(left).partial_cmp(&as_real(right))
        }
        (Value::Text(left), Value::Text(right)) => {
            Some(left.to_lowercase().cmp(&right.to_lowercase()))
        }
        (Value::Logical(left), Value::Logical(right)) => Some(left.cmp(right)),
        _ => None,
    }
    .ok_or_else(|| {
        runtime_error(
            expression,
            &format!(
                "não é possível comparar {} e {}",
                left.type_name(),
                right.type_name()
            ),
        )
    })?;
    let result = match operator {
        "=" => ordering == Ordering::Equal,
        "<>" => ordering != Ordering::Equal,
        "<" => ordering == Ordering::Less,
        "<=" => ordering != Ordering::Greater,
        ">" => ordering == Ordering::Greater,
        ">=" => ordering != Ordering::Less,
        _ => unreachable!("only comparison operators enter this function"),
    };
    Ok(Value::Logical(result))
}

fn is_numeric(value: &Value) -> bool {
    matches!(value, Value::Integer(_) | Value::Real(_))
}

fn as_real(value: &Value) -> f64 {
    match value {
        Value::Integer(value) => *value as f64,
        Value::Real(value) => *value,
        _ => unreachable!("numeric values are checked before conversion"),
    }
}

fn checked_real(value: f64, expression: &Expression) -> Result<Value, Diagnostic> {
    if value.is_finite() {
        Ok(Value::Real(value))
    } else {
        Err(runtime_error(
            expression,
            "resultado real inválido ou fora do limite",
        ))
    }
}

fn parse_input(input: &str, data_type: Type) -> Result<Value, String> {
    let trimmed = input.trim();
    match data_type {
        Type::Integer => trimmed
            .parse::<i64>()
            .map(Value::Integer)
            .map_err(|_| format!("'{input}' não é um valor inteiro válido")),
        Type::Real => {
            let normalized = if trimmed.contains(',') && !trimmed.contains('.') {
                trimmed.replace(',', ".")
            } else {
                trimmed.to_owned()
            };
            normalized
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite())
                .map(Value::Real)
                .ok_or_else(|| format!("'{input}' não é um valor real válido"))
        }
        Type::Character => Ok(Value::Text(input.to_owned())),
        Type::Logical => match trimmed.to_lowercase().as_str() {
            "verdadeiro" | "true" => Ok(Value::Logical(true)),
            "falso" | "false" => Ok(Value::Logical(false)),
            _ => Err(format!("'{input}' não é um valor lógico válido")),
        },
    }
}

fn public_data_type(data_type: Type) -> DataType {
    match data_type {
        Type::Integer => DataType::Integer,
        Type::Real => DataType::Real,
        Type::Character => DataType::Character,
        Type::Logical => DataType::Logical,
    }
}

pub(crate) fn runtime_error(location: &impl HasLocation, message: &str) -> Diagnostic {
    Diagnostic::new(
        DiagnosticKind::Runtime,
        message,
        location.line(),
        location.column(),
    )
}

pub(crate) trait HasLocation {
    fn line(&self) -> usize;
    fn column(&self) -> usize;
}

impl HasLocation for Expression {
    fn line(&self) -> usize {
        self.line
    }
    fn column(&self) -> usize {
        self.column
    }
}

impl HasLocation for Statement {
    fn line(&self) -> usize {
        self.line
    }
    fn column(&self) -> usize {
        self.column
    }
}

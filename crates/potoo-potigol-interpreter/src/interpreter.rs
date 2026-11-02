use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::ast::{
    Block, Expr, ExprKind, ForGenerator, ForSource, FunctionBody, FunctionDecl, MatchCase, NodeId,
    Parameter, Pattern, PatternKind, Program, Stmt, StmtKind, StringPart, TypeDecl,
};
use crate::bigint;
use crate::environment::{Environment, EnvironmentRef};
use crate::value::{Callable, ObjectValue, Value};
use crate::{
    ControlAction, DataType, Diagnostic, DiagnosticKind, ExecutionEvent, ExecutionOptions,
    InputPrompt, PauseReason, RuntimeIo, SourceSpan, TypeInfo, ValueType, VariableSnapshot, check,
    parse_with_options,
};

type RuntimeResult<T> = Result<T, Vec<Diagnostic>>;

enum Flow {
    Next(Value),
    Return(Value),
}

enum TailOutcome {
    Value(Value),
    Call(Vec<Value>, SourceSpan),
}

struct Interpreter<'a> {
    io: &'a mut dyn RuntimeIo,
    options: ExecutionOptions,
    environment: EnvironmentRef,
    classes: HashMap<String, TypeDecl>,
    class_environments: HashMap<String, EnvironmentRef>,
    steps: u64,
    allocated_elements: usize,
    output_bytes: usize,
    waited: Duration,
    call_depth: usize,
    eof: bool,
    debugging: bool,
    stepping: bool,
    pause_reason: PauseReason,
    pending_return: Option<Value>,
    placeholder_values: Vec<Vec<Value>>,
    placeholder_indexes: Vec<usize>,
    random_state: u64,
    type_info: TypeInfo,
}

pub fn execute(source: &str, io: &mut dyn RuntimeIo) -> RuntimeResult<()> {
    execute_with_options(source, io, ExecutionOptions::default())
}

pub fn execute_with_options(
    source: &str,
    io: &mut dyn RuntimeIo,
    options: ExecutionOptions,
) -> RuntimeResult<()> {
    let program = parse_with_options(source, options)?;
    let type_info = check(&program)?;
    Interpreter::new(io, options, &program, type_info).execute_program(&program)
}

impl<'a> Interpreter<'a> {
    fn new(
        io: &'a mut dyn RuntimeIo,
        options: ExecutionOptions,
        program: &Program,
        type_info: TypeInfo,
    ) -> Self {
        let classes: HashMap<String, TypeDecl> = program
            .statements
            .iter()
            .filter_map(|statement| match &statement.kind {
                StmtKind::Type(declaration) => {
                    Some((declaration.name.clone(), declaration.clone()))
                }
                _ => None,
            })
            .collect();
        let environment = Environment::root();
        let class_environments = classes
            .keys()
            .map(|name| (name.clone(), environment.clone()))
            .collect();
        let debugging = io.is_debugging();
        let random_state = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        Self {
            io,
            options,
            environment,
            classes,
            class_environments,
            steps: 0,
            allocated_elements: 0,
            output_bytes: 0,
            waited: Duration::ZERO,
            call_depth: 0,
            eof: false,
            debugging,
            stepping: false,
            pause_reason: PauseReason::Debugger,
            pending_return: None,
            placeholder_values: Vec::new(),
            placeholder_indexes: Vec::new(),
            random_state: random_state.max(1),
            type_info,
        }
    }

    fn execute_program(&mut self, program: &Program) -> RuntimeResult<()> {
        self.predeclare(&program.statements)?;
        for statement in &program.statements {
            match self.execute_statement(statement)? {
                Flow::Next(_) => {}
                Flow::Return(_) => {
                    return Err(vec![
                        self.error("retorne não pode encerrar o programa", statement.span),
                    ]);
                }
            }
        }
        Ok(())
    }

    fn predeclare(&mut self, statements: &[Stmt]) -> RuntimeResult<()> {
        for statement in statements {
            let (name, value) = match &statement.kind {
                StmtKind::Function(declaration) => (
                    declaration.name.clone(),
                    Value::Function(Rc::new(Callable::Function {
                        declaration: declaration.clone(),
                        environment: self.environment.clone(),
                        owner: None,
                        receiver: None,
                    })),
                ),
                StmtKind::Type(declaration) => (
                    declaration.name.clone(),
                    Value::Function(Rc::new(Callable::Constructor {
                        declaration: declaration.clone(),
                        environment: self.environment.clone(),
                    })),
                ),
                _ => continue,
            };
            if let StmtKind::Type(declaration) = &statement.kind {
                self.class_environments
                    .insert(declaration.name.clone(), self.environment.clone());
            }
            Environment::define(&self.environment, name, value, false)
                .map_err(|message| vec![self.error(message, statement.span)])?;
        }
        Ok(())
    }

    fn execute_block(&mut self, block: &Block) -> RuntimeResult<Flow> {
        self.with_child_environment(|interpreter| {
            interpreter.predeclare(&block.statements)?;
            let mut last = Value::Nothing;
            for statement in &block.statements {
                match interpreter.execute_statement(statement)? {
                    Flow::Next(value) => last = value,
                    returned @ Flow::Return(_) => return Ok(returned),
                }
            }
            Ok(Flow::Next(last))
        })
    }

    fn with_child_environment<T>(
        &mut self,
        action: impl FnOnce(&mut Self) -> RuntimeResult<T>,
    ) -> RuntimeResult<T> {
        let child = Environment::child(self.environment.clone());
        let previous = std::mem::replace(&mut self.environment, child);
        let result = action(self);
        self.environment = previous;
        result
    }

    fn with_environment<T>(
        &mut self,
        environment: EnvironmentRef,
        action: impl FnOnce(&mut Self) -> RuntimeResult<T>,
    ) -> RuntimeResult<T> {
        let previous = std::mem::replace(&mut self.environment, environment);
        let result = action(self);
        self.environment = previous;
        result
    }

    fn execute_statement(&mut self, statement: &Stmt) -> RuntimeResult<Flow> {
        self.begin_statement(statement.span)?;
        let flow = match &statement.kind {
            StmtKind::Binding {
                pattern,
                mutable,
                value,
                ..
            } => {
                let value = self.evaluate_binding_value(pattern, value)?;
                if let Some(value) = self.pending_return.take() {
                    return Ok(Flow::Return(value));
                }
                self.bind_pattern(pattern, value, *mutable)?;
                Flow::Next(Value::Nothing)
            }
            StmtKind::Assignment { target, value } => {
                let value = self.evaluate_expression(value)?;
                if let Some(value) = self.pending_return.take() {
                    return Ok(Flow::Return(value));
                }
                self.assign_target(target, value)?;
                Flow::Next(Value::Nothing)
            }
            StmtKind::Function(_) | StmtKind::Type(_) => Flow::Next(Value::Nothing),
            StmtKind::Output { newline, value } => {
                let value = self.evaluate_expression(value)?;
                if let Some(value) = self.pending_return.take() {
                    return Ok(Flow::Return(value));
                }
                let mut output = value.display();
                if *newline {
                    output.push('\n');
                }
                self.write_output(&output, statement.span)?;
                Flow::Next(Value::Nothing)
            }
            StmtKind::Return(value) => {
                let value = if let Some(value) = value {
                    self.evaluate_expression(value)?
                } else {
                    Value::Nothing
                };
                if let Some(value) = self.pending_return.take() {
                    return Ok(Flow::Return(value));
                }
                Flow::Return(value)
            }
            StmtKind::While { condition, body } => {
                loop {
                    self.consume_step(statement.span)?;
                    let condition_value = self.evaluate_expression(condition)?;
                    if let Some(value) = self.pending_return.take() {
                        return Ok(Flow::Return(value));
                    }
                    if !self.expect_logical(condition_value, condition.span)? {
                        break;
                    }
                    match self.execute_block(body)? {
                        Flow::Next(_) => {}
                        returned @ Flow::Return(_) => return Ok(returned),
                    }
                }
                Flow::Next(Value::Nothing)
            }
            StmtKind::For { generators, body } => {
                match self.execute_generators(generators, 0, body)? {
                    returned @ Flow::Return(_) => return Ok(returned),
                    Flow::Next(_) => {}
                }
                Flow::Next(Value::Nothing)
            }
            StmtKind::Expression(expression) => Flow::Next(self.evaluate_expression(expression)?),
        };
        if let Some(value) = self.pending_return.take() {
            return Ok(Flow::Return(value));
        }
        Ok(flow)
    }

    fn execute_generators(
        &mut self,
        generators: &[ForGenerator],
        index: usize,
        body: &Block,
    ) -> RuntimeResult<Flow> {
        let Some(generator) = generators.get(index) else {
            return self.execute_block(body);
        };
        let values = self.generator_values(generator)?;
        if let Some(value) = self.pending_return.take() {
            return Ok(Flow::Return(value));
        }
        for value in values {
            self.consume_step(generator.span)?;
            let result = self.with_child_environment(|interpreter| {
                interpreter.bind_pattern(&generator.pattern, value, false)?;
                if let Some(condition) = &generator.condition {
                    let condition_value = interpreter.evaluate_expression(condition)?;
                    if let Some(value) = interpreter.pending_return.take() {
                        return Ok(Flow::Return(value));
                    }
                    if !interpreter.expect_logical(condition_value, condition.span)? {
                        return Ok(Flow::Next(Value::Nothing));
                    }
                }
                interpreter.execute_generators(generators, index + 1, body)
            })?;
            match result {
                returned @ Flow::Return(_) => return Ok(returned),
                Flow::Next(_) => {}
            }
        }
        Ok(Flow::Next(Value::Nothing))
    }

    fn generator_values(&mut self, generator: &ForGenerator) -> RuntimeResult<Vec<Value>> {
        match &generator.source {
            ForSource::Iterable(expression) => {
                let iterable = self.evaluate_expression(expression)?;
                if self.pending_return.is_some() {
                    return Ok(Vec::new());
                }
                match iterable {
                    Value::List(values) => Ok(values),
                    Value::MutableList(values) => {
                        let values = values.borrow().clone();
                        self.reserve_elements(values.len(), expression.span)?;
                        Ok(values)
                    }
                    Value::Tuple(values) => Ok(values),
                    Value::Text(value) => {
                        self.reserve_elements(value.chars().count(), expression.span)?;
                        Ok(value.chars().map(Value::Character).collect())
                    }
                    other => Err(vec![self.error(
                        format!("{} não pode ser percorrido", other.display()),
                        expression.span,
                    )]),
                }
            }
            ForSource::Range { start, end, step } => {
                let start_expression = self.evaluate_expression(start)?;
                if self.pending_return.is_some() {
                    return Ok(Vec::new());
                }
                let start_value = self.expect_integer(start_expression, start.span)?;
                let end_expression = self.evaluate_expression(end)?;
                if self.pending_return.is_some() {
                    return Ok(Vec::new());
                }
                let end_value = self.expect_integer(end_expression, end.span)?;
                let step_value = if let Some(step) = step {
                    let step_expression = self.evaluate_expression(step)?;
                    if self.pending_return.is_some() {
                        return Ok(Vec::new());
                    }
                    self.expect_integer(step_expression, step.span)?
                } else {
                    1
                };
                if step_value == 0 {
                    return Err(vec![
                        self.error("o passo do laço não pode ser zero", generator.span),
                    ]);
                }
                let mut values = Vec::new();
                let mut current = start_value;
                while if step_value > 0 {
                    current <= end_value
                } else {
                    current >= end_value
                } {
                    self.reserve_elements(1, generator.span)?;
                    values.push(Value::Integer(current));
                    let Some(next) = current.checked_add(step_value) else {
                        break;
                    };
                    current = next;
                }
                Ok(values)
            }
        }
    }

    fn evaluate_binding_value(
        &mut self,
        pattern: &Pattern,
        expression: &Expr,
    ) -> RuntimeResult<Value> {
        if let (PatternKind::Tuple(_), ExprKind::Identifier(name)) =
            (&pattern.kind, &expression.kind)
            && let Some(data_type) = input_data_type(name)
        {
            return self.read_pattern_input(pattern, data_type, expression.span);
        }
        self.evaluate_expression(expression)
    }

    fn read_pattern_input(
        &mut self,
        pattern: &Pattern,
        data_type: DataType,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if let PatternKind::Tuple(patterns) = &pattern.kind {
            let values = patterns
                .iter()
                .map(|pattern| self.read_pattern_input(pattern, data_type, span))
                .collect::<RuntimeResult<Vec<_>>>()?;
            self.reserve_elements(values.len(), span)?;
            Ok(Value::Tuple(values))
        } else {
            self.read_scalar(data_type, span)
        }
    }

    fn bind_pattern(
        &mut self,
        pattern: &Pattern,
        value: Value,
        mutable: bool,
    ) -> RuntimeResult<()> {
        let value = broadcast_pattern_value(pattern, value);
        let mut bindings = Vec::new();
        if !self.collect_pattern_bindings(pattern, &value, &mut bindings)? {
            return Err(vec![
                self.error("valor incompatível com o padrão", pattern.span),
            ]);
        }
        let mut binding_types = HashMap::new();
        self.collect_pattern_types(pattern, &mut binding_types);
        for (name, value) in bindings {
            let value = if let Some(value_type) = binding_types.get(&name) {
                self.coerce_value(value, value_type, pattern.span)?
            } else {
                value
            };
            Environment::define(&self.environment, name, value, mutable)
                .map_err(|message| vec![self.error(message, pattern.span)])?;
        }
        Ok(())
    }

    fn collect_pattern_types(&self, pattern: &Pattern, types: &mut HashMap<String, ValueType>) {
        match &pattern.kind {
            PatternKind::Binding(name) => {
                if let Some(value_type) = self.type_info.node_types.get(&pattern.id) {
                    types.insert(name.clone(), value_type.clone());
                }
            }
            PatternKind::Tuple(patterns)
            | PatternKind::List(patterns)
            | PatternKind::Constructor {
                fields: patterns, ..
            } => {
                for pattern in patterns {
                    self.collect_pattern_types(pattern, types);
                }
            }
            PatternKind::Cons(head, tail) => {
                self.collect_pattern_types(head, types);
                self.collect_pattern_types(tail, types);
            }
            _ => {}
        }
    }

    fn coerce_value(
        &mut self,
        value: Value,
        expected: &ValueType,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if !runtime_needs_coercion(&value, expected) {
            return Ok(value);
        }
        match (value, expected) {
            (Value::Character(character), ValueType::Integer) => {
                Ok(Value::Integer(i64::from(u32::from(character))))
            }
            (Value::Character(character), ValueType::Real) => {
                Ok(Value::Real(u32::from(character) as f64))
            }
            (Value::Integer(number), ValueType::Real) => Ok(Value::Real(number as f64)),
            (
                value @ (Value::Character(_)
                | Value::Integer(_)
                | Value::Real(_)
                | Value::BigInteger(_)
                | Value::Logical(_)),
                ValueType::Text,
            ) => Ok(Value::Text(value.display())),
            (Value::Tuple(values), ValueType::Tuple(expected))
                if values.len() == expected.len() =>
            {
                let values = values
                    .into_iter()
                    .zip(expected)
                    .map(|(value, expected)| self.coerce_value(value, expected, span))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                Ok(Value::Tuple(values))
            }
            (Value::List(values), ValueType::List(expected)) => {
                self.reserve_elements(values.len(), span)?;
                let values = values
                    .into_iter()
                    .map(|value| self.coerce_value(value, expected, span))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                Ok(Value::List(values))
            }
            (Value::MutableList(values), ValueType::MutableList(expected)) => {
                let values = values.borrow().clone();
                self.reserve_elements(values.len(), span)?;
                let values = values
                    .into_iter()
                    .map(|value| self.coerce_value(value, expected, span))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                Ok(Value::MutableList(Rc::new(RefCell::new(values))))
            }
            (value, _) => Ok(value),
        }
    }

    fn coerce_to_node(
        &mut self,
        value: Value,
        node: NodeId,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let Some(expected) = self.type_info.node_types.get(&node).cloned() else {
            return Ok(value);
        };
        self.coerce_value(value, &expected, span)
    }

    fn collect_pattern_bindings(
        &self,
        pattern: &Pattern,
        value: &Value,
        bindings: &mut Vec<(String, Value)>,
    ) -> RuntimeResult<bool> {
        let matches = match (&pattern.kind, value) {
            (PatternKind::Wildcard, _) => true,
            (PatternKind::Binding(name), _) => {
                bindings.push((name.clone(), value.clone()));
                true
            }
            (PatternKind::Integer(expected), Value::Integer(actual)) => expected == actual,
            (PatternKind::Real(expected), Value::Real(actual)) => expected == actual,
            (PatternKind::Character(expected), Value::Character(actual)) => expected == actual,
            (PatternKind::Text(expected), Value::Text(actual)) => expected == actual,
            (PatternKind::Logical(expected), Value::Logical(actual)) => expected == actual,
            (PatternKind::Tuple(patterns), Value::Tuple(values))
                if patterns.len() == values.len() =>
            {
                for (pattern, value) in patterns.iter().zip(values) {
                    if !self.collect_pattern_bindings(pattern, value, bindings)? {
                        return Ok(false);
                    }
                }
                true
            }
            (PatternKind::List(patterns), Value::List(values))
                if patterns.len() == values.len() =>
            {
                for (pattern, value) in patterns.iter().zip(values) {
                    if !self.collect_pattern_bindings(pattern, value, bindings)? {
                        return Ok(false);
                    }
                }
                true
            }
            (PatternKind::List(patterns), Value::MutableList(values)) => {
                let values = values.borrow();
                if patterns.len() != values.len() {
                    false
                } else {
                    for (pattern, value) in patterns.iter().zip(values.iter()) {
                        if !self.collect_pattern_bindings(pattern, value, bindings)? {
                            return Ok(false);
                        }
                    }
                    true
                }
            }
            (PatternKind::Cons(head, tail), Value::List(values)) if !values.is_empty() => {
                self.collect_pattern_bindings(head, &values[0], bindings)?
                    && self.collect_pattern_bindings(
                        tail,
                        &Value::List(values[1..].to_vec()),
                        bindings,
                    )?
            }
            (PatternKind::Cons(head, tail), Value::MutableList(values)) => {
                let values = values.borrow();
                !values.is_empty()
                    && self.collect_pattern_bindings(head, &values[0], bindings)?
                    && self.collect_pattern_bindings(
                        tail,
                        &Value::List(values[1..].to_vec()),
                        bindings,
                    )?
            }
            (PatternKind::Constructor { name, fields }, Value::Object(object)) => {
                let object = object.borrow();
                if &object.class_name != name {
                    false
                } else if let Some(declaration) = self.classes.get(name) {
                    let field_names = declaration
                        .fields
                        .iter()
                        .filter(|field| field.constructor_parameter)
                        .map(|field| field.name.as_str())
                        .collect::<Vec<_>>();
                    if fields.len() != field_names.len() {
                        false
                    } else {
                        for (pattern, field_name) in fields.iter().zip(field_names) {
                            let Some(value) = object.fields.get(field_name) else {
                                return Ok(false);
                            };
                            if !self.collect_pattern_bindings(pattern, value, bindings)? {
                                return Ok(false);
                            }
                        }
                        true
                    }
                } else {
                    false
                }
            }
            _ => false,
        };
        Ok(matches)
    }

    fn assign_target(&mut self, target: &Expr, value: Value) -> RuntimeResult<()> {
        let value = self.coerce_to_node(value, target.id, target.span)?;
        match &target.kind {
            ExprKind::Identifier(name) => {
                let Some(binding) = Environment::lookup(&self.environment, name) else {
                    return Err(vec![self.error(
                        format!("identificador '{name}' não foi declarado"),
                        target.span,
                    )]);
                };
                if let Value::FieldReference(object, field) = binding.value {
                    if !binding.mutable {
                        return Err(vec![
                            self.error(format!("atributo '{field}' é imutável"), target.span),
                        ]);
                    }
                    object.borrow_mut().fields.insert(field, value);
                    return Ok(());
                }
                Environment::assign(&self.environment, name, value)
                    .map_err(|message| vec![self.error(message, target.span)])
            }
            ExprKind::Tuple(targets) => {
                let values = match value {
                    Value::Tuple(values) if values.len() == targets.len() => values,
                    Value::Tuple(_) => {
                        return Err(vec![self.error(
                            "quantidades diferentes na atribuição paralela",
                            target.span,
                        )]);
                    }
                    value => vec![value; targets.len()],
                };
                for (target, value) in targets.iter().zip(values) {
                    self.assign_target(target, value)?;
                }
                Ok(())
            }
            ExprKind::Member { receiver, name } => {
                let Value::Object(object) = self.evaluate_expression(receiver)? else {
                    return Err(vec![self.error(
                        "somente atributos de objetos podem receber atribuição",
                        target.span,
                    )]);
                };
                let class_name = object.borrow().class_name.clone();
                let mutable = self
                    .classes
                    .get(&class_name)
                    .and_then(|declaration| {
                        declaration.fields.iter().find(|field| field.name == *name)
                    })
                    .is_some_and(|field| field.mutable);
                if !mutable {
                    return Err(vec![
                        self.error(format!("atributo '{name}' é imutável"), target.span),
                    ]);
                }
                object.borrow_mut().fields.insert(name.clone(), value);
                Ok(())
            }
            ExprKind::Index { receiver, indices } => {
                if indices.len() != 1 {
                    return Err(vec![
                        self.error("um acesso aceita exatamente um índice", target.span),
                    ]);
                }
                let current = self.evaluate_expression(receiver)?;
                let index_value = self.evaluate_expression(&indices[0])?;
                let index = self.expect_integer(index_value, indices[0].span)?;
                if index <= 0 {
                    return Err(vec![self.error("os índices começam em 1", indices[0].span)]);
                }
                match current {
                    Value::List(mut values) => {
                        let Some(slot) = values.get_mut(index as usize - 1) else {
                            return Err(vec![self.error("índice fora dos limites", target.span)]);
                        };
                        *slot = value;
                        self.assign_target(receiver, Value::List(values))
                    }
                    Value::MutableList(values) => {
                        let mut values = values.borrow_mut();
                        let Some(slot) = values.get_mut(index as usize - 1) else {
                            return Err(vec![self.error("índice fora dos limites", target.span)]);
                        };
                        *slot = value;
                        Ok(())
                    }
                    other => Err(vec![self.error(
                        format!("{} não permite atribuição por índice", other.display()),
                        target.span,
                    )]),
                }
            }
            _ => Err(vec![self.error("alvo de atribuição inválido", target.span)]),
        }
    }

    fn begin_statement(&mut self, span: SourceSpan) -> RuntimeResult<()> {
        self.consume_step(span)?;
        if !self.debugging && !self.stepping {
            return Ok(());
        }
        let variables = Environment::visible_values(&self.environment)
            .into_iter()
            .map(|(name, value)| VariableSnapshot {
                name,
                value: self.resolve_field_reference(value).display(),
            })
            .collect();
        let event = ExecutionEvent {
            location: span.start,
            variables,
            pause_reason: Some(self.pause_reason),
        };
        match self
            .io
            .on_execution_event(&event)
            .map_err(|message| vec![self.error(message, span)])?
        {
            ControlAction::Continue => {
                self.debugging = false;
                self.stepping = false;
            }
            ControlAction::Step => {
                self.debugging = false;
                self.stepping = true;
                self.pause_reason = PauseReason::Step;
            }
            ControlAction::Stop => {
                return Err(vec![
                    self.error("execução interrompida pelo depurador", span),
                ]);
            }
        }
        Ok(())
    }

    fn consume_step(&mut self, span: SourceSpan) -> RuntimeResult<()> {
        self.consume_work(1, span)
    }

    fn consume_work(&mut self, steps: u64, span: SourceSpan) -> RuntimeResult<()> {
        if self.io.should_stop() {
            return Err(vec![self.error("execução interrompida", span)]);
        }
        self.steps = self.steps.saturating_add(steps);
        if self.steps > self.options.max_steps {
            return Err(vec![self.error("limite de passos excedido", span)]);
        }
        Ok(())
    }

    fn reserve_elements(&mut self, count: usize, span: SourceSpan) -> RuntimeResult<()> {
        self.allocated_elements = self.allocated_elements.saturating_add(count);
        if self.allocated_elements > self.options.max_collection_elements {
            return Err(vec![self.error("limite de elementos excedido", span)]);
        }
        Ok(())
    }

    fn write_output(&mut self, text: &str, span: SourceSpan) -> RuntimeResult<()> {
        self.output_bytes = self.output_bytes.saturating_add(text.len());
        if self.output_bytes > self.options.max_output_bytes {
            return Err(vec![self.error("limite de saída excedido", span)]);
        }
        self.io.write(text);
        Ok(())
    }

    fn read_scalar(&mut self, data_type: DataType, span: SourceSpan) -> RuntimeResult<Value> {
        if self.io.should_stop() {
            return Err(vec![self.error("execução interrompida", span)]);
        }
        let prompt = InputPrompt {
            data_type,
            source: span.start,
        };
        let line = match self.io.read_line(&prompt) {
            Ok(line) => line,
            Err(message) => {
                self.eof = true;
                return Err(vec![self.error(message, span)]);
            }
        };
        let line = line.trim_end_matches(['\r', '\n']);
        self.check_string_size(line.len(), span)?;
        let parsed = match data_type {
            DataType::Integer => line
                .trim()
                .parse::<i64>()
                .map(Value::Integer)
                .map_err(|_| "entrada não é um Inteiro".to_owned()),
            DataType::Real => line
                .trim()
                .parse::<f64>()
                .map(Value::Real)
                .map_err(|_| "entrada não é um Real".to_owned()),
            DataType::Text => Ok(Value::Text(line.to_owned())),
            DataType::Character => {
                let mut characters = line.chars();
                match (characters.next(), characters.next()) {
                    (Some(character), None) => Ok(Value::Character(character)),
                    _ => Err("entrada deve conter exatamente um Caractere".to_owned()),
                }
            }
            DataType::Logical => match line.trim().to_lowercase().as_str() {
                "verdadeiro" | "true" => Ok(Value::Logical(true)),
                "falso" | "false" => Ok(Value::Logical(false)),
                _ => Err("entrada não é Lógico".to_owned()),
            },
        };
        parsed.map_err(|message| vec![self.error(message, span)])
    }

    fn expect_integer(&self, value: Value, span: SourceSpan) -> RuntimeResult<i64> {
        if let Value::Integer(value) = value {
            Ok(value)
        } else {
            Err(vec![self.error("esperava um Inteiro", span)])
        }
    }

    fn expect_logical(&self, value: Value, span: SourceSpan) -> RuntimeResult<bool> {
        if let Value::Logical(value) = value {
            Ok(value)
        } else {
            Err(vec![self.error("esperava um Lógico", span)])
        }
    }

    fn resolve_field_reference(&self, value: Value) -> Value {
        if let Value::FieldReference(object, field) = value {
            object
                .borrow()
                .fields
                .get(&field)
                .cloned()
                .unwrap_or(Value::Nothing)
        } else {
            value
        }
    }

    fn error(&self, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
        Diagnostic::new(DiagnosticKind::Runtime, message, span)
    }
}

impl<'a> Interpreter<'a> {
    fn evaluate_expression(&mut self, expression: &Expr) -> RuntimeResult<Value> {
        if self.pending_return.is_some() {
            return Ok(Value::Nothing);
        }
        let result = self.evaluate_expression_inner(expression);
        if self.pending_return.is_some() {
            Ok(Value::Nothing)
        } else {
            result.and_then(|value| self.coerce_to_node(value, expression.id, expression.span))
        }
    }

    fn evaluate_expression_inner(&mut self, expression: &Expr) -> RuntimeResult<Value> {
        self.consume_step(expression.span)?;
        match &expression.kind {
            ExprKind::Integer(value) => Ok(Value::Integer(*value)),
            ExprKind::Real(value) => Ok(Value::Real(*value)),
            ExprKind::Character(value) => Ok(Value::Character(*value)),
            ExprKind::Logical(value) => Ok(Value::Logical(*value)),
            ExprKind::Text(parts) => {
                let mut text = String::new();
                for part in parts {
                    match part {
                        StringPart::Text(part) => text.push_str(part),
                        StringPart::Expression(expression) => {
                            text.push_str(&self.evaluate_expression(expression)?.display());
                        }
                    }
                    self.check_string_size(text.len(), expression.span)?;
                }
                Ok(Value::Text(text))
            }
            ExprKind::Identifier(name) => self.identifier_value(name, expression.span),
            ExprKind::Placeholder => {
                let Some(values) = self.placeholder_values.last() else {
                    return Err(vec![self.error(
                        "'_' só pode aparecer como parâmetro de uma função de ordem superior",
                        expression.span,
                    )]);
                };
                let index = self.placeholder_indexes.last_mut().unwrap();
                let Some(value) = values.get(*index).cloned() else {
                    return Err(vec![
                        self.error("faltou um argumento para '_'", expression.span),
                    ]);
                };
                *index += 1;
                Ok(value)
            }
            ExprKind::Tuple(elements) => {
                let values = elements
                    .iter()
                    .map(|element| self.evaluate_expression(element))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                self.reserve_elements(values.len(), expression.span)?;
                Ok(Value::Tuple(values))
            }
            ExprKind::List(elements) => {
                let values = elements
                    .iter()
                    .map(|element| self.evaluate_expression(element))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                self.reserve_elements(values.len(), expression.span)?;
                Ok(Value::List(values))
            }
            ExprKind::Unary { operator, operand } => {
                let value = self.evaluate_expression(operand)?;
                self.evaluate_unary(operator, value, expression.span)
            }
            ExprKind::Binary {
                left,
                operator,
                right,
            } => self.evaluate_binary(left, operator, right, expression.span),
            ExprKind::Call { callee, arguments } => {
                self.evaluate_call(callee, arguments, expression.span)
            }
            ExprKind::Member { receiver, name } => {
                let receiver = self.evaluate_expression(receiver)?;
                self.member_value(receiver, name, expression.span)
            }
            ExprKind::Index { receiver, indices } => {
                if let ExprKind::Member {
                    receiver: factory,
                    name,
                } = &receiver.kind
                    && name == "vazia"
                    && matches!(&factory.kind, ExprKind::Identifier(name) if name == "Lista")
                {
                    if indices.len() != 1 {
                        return Err(vec![self.error(
                            "Lista.vazia precisa de exatamente um tipo",
                            expression.span,
                        )]);
                    }
                    return Ok(Value::List(Vec::new()));
                }
                let receiver = self.evaluate_expression(receiver)?;
                if indices.len() != 1 {
                    return Err(vec![
                        self.error("um acesso aceita exatamente um índice", expression.span),
                    ]);
                }
                let index_value = self.evaluate_expression(&indices[0])?;
                let index = self.expect_integer(index_value, indices[0].span)?;
                self.index_value(receiver, index, expression.span)
            }
            ExprKind::Lambda { parameters, body } => {
                Ok(Value::Function(Rc::new(Callable::Lambda {
                    id: expression.id,
                    parameters: parameters.clone(),
                    body: body.as_ref().clone(),
                    environment: self.environment.clone(),
                })))
            }
            ExprKind::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition_value = self.evaluate_expression(condition)?;
                if self.expect_logical(condition_value, condition.span)? {
                    match self.execute_block(then_branch)? {
                        Flow::Next(value) => Ok(value),
                        Flow::Return(value) => {
                            self.pending_return = Some(value);
                            Ok(Value::Nothing)
                        }
                    }
                } else if let Some(else_branch) = else_branch {
                    match self.execute_block(else_branch)? {
                        Flow::Next(value) => Ok(value),
                        Flow::Return(value) => {
                            self.pending_return = Some(value);
                            Ok(Value::Nothing)
                        }
                    }
                } else {
                    Ok(Value::Nothing)
                }
            }
            ExprKind::Match { value, cases } => self.evaluate_match(value, cases, expression.span),
            ExprKind::Comprehension { generators, value } => {
                let mut values = Vec::new();
                self.evaluate_comprehension(generators, 0, value, &mut values)?;
                self.reserve_elements(values.len(), expression.span)?;
                Ok(Value::List(values))
            }
        }
    }

    fn identifier_value(&mut self, name: &str, span: SourceSpan) -> RuntimeResult<Value> {
        if let Some(data_type) = input_data_type(name) {
            return self.read_scalar(data_type, span);
        }
        match name.to_lowercase().as_str() {
            "eof" => return Ok(Value::Logical(self.eof || self.io.is_eof())),
            "pi" => return Ok(Value::Real(std::f64::consts::PI)),
            _ => {}
        }
        match name {
            "Lista" | "Matriz" | "Cubo" => return Ok(Value::TypeName(name.to_owned())),
            _ => {}
        }
        let Some(binding) = Environment::lookup(&self.environment, name) else {
            return Err(vec![
                self.error(format!("identificador '{name}' não foi declarado"), span),
            ]);
        };
        Ok(self.resolve_field_reference(binding.value))
    }

    fn evaluate_call(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if let ExprKind::Identifier(name) = &callee.kind
            && let Some(value) = self.call_builtin(name, arguments, span)?
        {
            return Ok(value);
        }
        if let ExprKind::Member { receiver, name } = &callee.kind {
            let receiver = self.evaluate_expression(receiver)?;
            if self.pending_return.is_some() {
                return Ok(Value::Nothing);
            }
            let values = arguments
                .iter()
                .map(|argument| self.evaluate_argument(argument))
                .collect::<RuntimeResult<Vec<_>>>()?;
            if self.pending_return.is_some() {
                return Ok(Value::Nothing);
            }
            return self.call_member(receiver, name, values, span);
        }
        let callee = self.evaluate_expression(callee)?;
        if self.pending_return.is_some() {
            return Ok(Value::Nothing);
        }
        let values = arguments
            .iter()
            .map(|argument| self.evaluate_argument(argument))
            .collect::<RuntimeResult<Vec<_>>>()?;
        if self.pending_return.is_some() {
            return Ok(Value::Nothing);
        }
        self.apply(callee, values, span)
    }

    fn evaluate_argument(&mut self, expression: &Expr) -> RuntimeResult<Value> {
        let arity = count_placeholders(expression);
        if arity > 0 {
            Ok(Value::Function(Rc::new(Callable::Placeholder {
                expression: expression.clone(),
                environment: self.environment.clone(),
                arity,
            })))
        } else {
            self.evaluate_expression(expression)
        }
    }

    fn call_builtin(
        &mut self,
        name: &str,
        arguments: &[Expr],
        span: SourceSpan,
    ) -> RuntimeResult<Option<Value>> {
        let value = match name {
            "Lista" | "lista" => {
                if arguments.len() != 2 {
                    return Err(vec![
                        self.error("Lista recebe tamanho e valor inicial", span),
                    ]);
                }
                let length_value = self.evaluate_expression(&arguments[0])?;
                let length = self.expect_integer(length_value, arguments[0].span)?;
                if length < 0 {
                    return Err(vec![
                        self.error("o tamanho da lista não pode ser negativo", span),
                    ]);
                }
                let length = usize::try_from(length)
                    .map_err(|_| vec![self.error("tamanho de lista inválido", span)])?;
                self.reserve_elements(length, span)?;
                let initial = self.evaluate_expression(&arguments[1])?;
                Value::List(vec![initial; length])
            }
            "BigInt" | "bigint" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("BigInt recebe um valor", span)]);
                }
                let source = self.evaluate_expression(&arguments[0])?;
                let value = match source {
                    Value::Integer(value) => value.to_string(),
                    Value::BigInteger(value) => value,
                    Value::Text(value) => {
                        let normalized = normalize_big_integer(&value).ok_or_else(|| {
                            vec![self.error("Texto inválido para BigInt", arguments[0].span)]
                        })?;
                        normalized
                    }
                    _ => {
                        return Err(vec![
                            self.error("BigInt recebe Inteiro ou Texto", arguments[0].span),
                        ]);
                    }
                };
                self.check_string_size(value.len(), span)?;
                Value::BigInteger(value)
            }
            "leia_inteiros" => {
                return self
                    .read_input_list(arguments, DataType::Integer, span)
                    .map(Some);
            }
            "leia_reais" => {
                return self
                    .read_input_list(arguments, DataType::Real, span)
                    .map(Some);
            }
            "leia_textos" => {
                return self
                    .read_input_list(arguments, DataType::Text, span)
                    .map(Some);
            }
            "espera" => {
                if arguments.len() != 1 {
                    return Err(vec![
                        self.error("espera recebe um número de segundos", span),
                    ]);
                }
                let value = self.evaluate_expression(&arguments[0])?;
                let seconds = numeric_f64(&value)
                    .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
                    .ok_or_else(|| {
                        vec![self.error(
                            "espera exige um número finito e não negativo",
                            arguments[0].span,
                        )]
                    })?;
                let duration = Duration::try_from_secs_f64(seconds).map_err(|_| {
                    vec![self.error("duração de espera inválida", arguments[0].span)]
                })?;
                let total = self
                    .waited
                    .checked_add(duration)
                    .ok_or_else(|| vec![self.error("limite total de espera excedido", span)])?;
                if total > Duration::from_millis(self.options.max_wait_millis) {
                    return Err(vec![self.error("limite total de espera excedido", span)]);
                }
                if self.io.should_stop() {
                    return Err(vec![self.error("execução interrompida", span)]);
                }
                self.io
                    .wait(duration)
                    .map_err(|message| vec![self.error(message, span)])?;
                self.waited = total;
                Value::Nothing
            }
            "min" | "menor" | "max" | "maior" => {
                if arguments.len() != 2 {
                    return Err(vec![
                        self.error(format!("{name} recebe dois argumentos"), span),
                    ]);
                }
                let left = self.evaluate_expression(&arguments[0])?;
                let right = self.evaluate_expression(&arguments[1])?;
                let order = compare_values(&left, &right)
                    .ok_or_else(|| vec![self.error("valores não podem ser comparados", span)])?;
                if matches!(name, "min" | "menor") {
                    if order.is_le() { left } else { right }
                } else if order.is_ge() {
                    left
                } else {
                    right
                }
            }
            "aleatório" | "aleatorio" => {
                let args = arguments
                    .iter()
                    .map(|argument| self.evaluate_expression(argument))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                match args.as_slice() {
                    [] => Value::Real(self.random_unit()),
                    [Value::Integer(maximum)] => {
                        if *maximum < 1 {
                            return Err(vec![self.error("aleatório(n) exige n >= 1", span)]);
                        }
                        Value::Integer(self.random_integer(1, *maximum, span)?)
                    }
                    [Value::List(values)] => {
                        let Some(index) = self.random_index(values.len()) else {
                            return Err(vec![self.error("aleatório não aceita lista vazia", span)]);
                        };
                        values[index].clone()
                    }
                    [Value::MutableList(values)] => {
                        let values = values.borrow();
                        let Some(index) = self.random_index(values.len()) else {
                            return Err(vec![self.error("aleatório não aceita lista vazia", span)]);
                        };
                        values[index].clone()
                    }
                    [other] => {
                        return Err(vec![
                            self.error(format!("aleatório não aceita {}", other.display()), span),
                        ]);
                    }
                    [Value::Integer(minimum), Value::Integer(maximum)] => {
                        Value::Integer(self.random_integer(*minimum, *maximum, span)?)
                    }
                    _ => {
                        return Err(vec![
                            self.error("argumentos inválidos para aleatório", span),
                        ]);
                    }
                }
            }
            "abs" | "raiz" | "sqrt" | "sen" | "cos" | "tan" | "tg" | "arcsen" | "arccos"
            | "arctg" | "log" | "log10" => {
                if arguments.len() != 1 {
                    return Err(vec![
                        self.error(format!("{name} recebe um argumento"), span),
                    ]);
                }
                let value = self.evaluate_expression(&arguments[0])?;
                let number = numeric_f64(&value)
                    .ok_or_else(|| vec![self.error("esperava um número", arguments[0].span)])?;
                match name {
                    "abs" => match value {
                        Value::Integer(number) => Value::Integer(
                            number
                                .checked_abs()
                                .ok_or_else(|| vec![self.error("estouro aritmético", span)])?,
                        ),
                        Value::Real(number) => Value::Real(number.abs()),
                        Value::BigInteger(number) => Value::BigInteger(
                            number.strip_prefix('-').unwrap_or(&number).to_owned(),
                        ),
                        _ => Value::Real(number.abs()),
                    },
                    "raiz" | "sqrt" => Value::Real(number.sqrt()),
                    "sen" => Value::Real(number.sin()),
                    "cos" => Value::Real(number.cos()),
                    "tan" | "tg" => Value::Real(number.tan()),
                    "arcsen" => Value::Real(number.asin()),
                    "arccos" => Value::Real(number.acos()),
                    "arctg" => Value::Real(number.atan()),
                    "log" => Value::Real(number.ln()),
                    "log10" => Value::Real(number.log10()),
                    _ => unreachable!(),
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }

    fn read_input_list(
        &mut self,
        arguments: &[Expr],
        data_type: DataType,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if arguments.len() > 1 {
            return Err(vec![
                self.error("leitura de lista aceita no máximo um argumento", span),
            ]);
        }
        let argument = if let Some(argument) = arguments.first() {
            Some(self.evaluate_expression(argument)?)
        } else {
            None
        };
        let values = match argument {
            Some(Value::Integer(count)) => {
                if count < 0 {
                    return Err(vec![self.error("a quantidade não pode ser negativa", span)]);
                }
                let count = usize::try_from(count)
                    .map_err(|_| vec![self.error("quantidade inválida", span)])?;
                self.reserve_elements(count, span)?;
                let mut values = Vec::with_capacity(count);
                for _ in 0..count {
                    values.push(self.read_scalar(data_type, span)?);
                }
                values
            }
            Some(Value::Text(separator)) => {
                let line = self.read_line(DataType::Text, span)?;
                let fields: Vec<String> = if separator.is_empty() {
                    line.chars()
                        .map(|character| character.to_string())
                        .collect()
                } else {
                    line.split(&separator).map(str::to_owned).collect()
                };
                self.reserve_elements(fields.len(), span)?;
                fields
                    .into_iter()
                    .map(|field| self.parse_input_value(data_type, &field, span))
                    .collect::<RuntimeResult<Vec<_>>>()?
            }
            None => {
                let line = self.read_line(DataType::Text, span)?;
                let fields = line
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                self.reserve_elements(fields.len(), span)?;
                fields
                    .into_iter()
                    .map(|field| self.parse_input_value(data_type, &field, span))
                    .collect::<RuntimeResult<Vec<_>>>()?
            }
            Some(other) => {
                return Err(vec![self.error(
                    format!("argumento de leitura incompatível: {}", other.display()),
                    span,
                )]);
            }
        };
        Ok(Value::List(values))
    }

    fn read_line(&mut self, data_type: DataType, span: SourceSpan) -> RuntimeResult<String> {
        if self.io.should_stop() {
            return Err(vec![self.error("execução interrompida", span)]);
        }
        let prompt = InputPrompt {
            data_type,
            source: span.start,
        };
        let line = self
            .io
            .read_line(&prompt)
            .map(|line| line.trim_end_matches(['\r', '\n']).to_owned())
            .map_err(|message| {
                self.eof = true;
                vec![self.error(message, span)]
            })?;
        self.check_string_size(line.len(), span)?;
        Ok(line)
    }

    fn parse_input_value(
        &self,
        data_type: DataType,
        value: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let parsed = match data_type {
            DataType::Integer => value
                .trim()
                .parse::<i64>()
                .map(Value::Integer)
                .map_err(|_| "entrada contém um valor que não é Inteiro".to_owned()),
            DataType::Real => value
                .trim()
                .parse::<f64>()
                .map(Value::Real)
                .map_err(|_| "entrada contém um valor que não é Real".to_owned()),
            DataType::Text => Ok(Value::Text(value.to_owned())),
            DataType::Character => {
                let mut characters = value.chars();
                match (characters.next(), characters.next()) {
                    (Some(character), None) => Ok(Value::Character(character)),
                    _ => Err("entrada contém texto onde se esperava Caractere".to_owned()),
                }
            }
            DataType::Logical => match value.trim().to_lowercase().as_str() {
                "verdadeiro" | "true" => Ok(Value::Logical(true)),
                "falso" | "false" => Ok(Value::Logical(false)),
                _ => Err("entrada contém um valor que não é Lógico".to_owned()),
            },
        };
        parsed.map_err(|message| vec![self.error(message, span)])
    }

    fn check_string_size(&self, size: usize, span: SourceSpan) -> RuntimeResult<()> {
        if size > self.options.max_string_bytes {
            Err(vec![
                self.error("limite de tamanho de texto excedido", span),
            ])
        } else {
            Ok(())
        }
    }

    fn random_next(&mut self) -> u64 {
        let mut state = self.random_state;
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        self.random_state = state.max(1);
        state
    }

    fn random_unit(&mut self) -> f64 {
        (self.random_next() >> 11) as f64 / (1_u64 << 53) as f64
    }

    fn random_index(&mut self, length: usize) -> Option<usize> {
        if length == 0 {
            None
        } else {
            Some((self.random_next() as u128 % length as u128) as usize)
        }
    }

    fn random_integer(
        &mut self,
        minimum: i64,
        maximum: i64,
        span: SourceSpan,
    ) -> RuntimeResult<i64> {
        if minimum > maximum {
            return Err(vec![self.error("limite inferior excede a superior", span)]);
        }
        let width = (i128::from(maximum) - i128::from(minimum) + 1) as u128;
        let offset = (self.random_next() as u128 % width) as i128;
        Ok((i128::from(minimum) + offset) as i64)
    }

    fn bigint_operation(
        &mut self,
        left: &str,
        right: &str,
        operation: char,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let left_digits = left.len() as u64;
        let right_digits = right.len() as u64;
        let work = if matches!(operation, '*' | '/' | '%') {
            left_digits.saturating_mul(right_digits)
        } else {
            left_digits.saturating_add(right_digits)
        };
        self.consume_work(work.max(1), span)?;
        let result = match operation {
            '+' => bigint::add(left, right),
            '-' => bigint::subtract(left, right),
            '*' => bigint::multiply(left, right),
            '/' | '%' => {
                let (quotient, remainder) = bigint::div_rem(left, right)
                    .ok_or_else(|| vec![self.error("divisão por zero", span)])?;
                if operation == '/' {
                    quotient
                } else {
                    remainder
                }
            }
            _ => return Err(vec![self.error("operação BigInt desconhecida", span)]),
        };
        self.check_string_size(result.len(), span)?;
        Ok(Value::BigInteger(result))
    }

    fn bigint_power(
        &mut self,
        base: &str,
        exponent: u64,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let mut result = String::from("1");
        let mut base = base.to_owned();
        let mut exponent = exponent;
        while exponent > 0 {
            if exponent & 1 == 1 {
                result = match self.bigint_operation(&result, &base, '*', span)? {
                    Value::BigInteger(result) => result,
                    _ => unreachable!(),
                };
            }
            exponent >>= 1;
            if exponent > 0 {
                base = match self.bigint_operation(&base, &base, '*', span)? {
                    Value::BigInteger(base) => base,
                    _ => unreachable!(),
                };
            }
        }
        Ok(Value::BigInteger(result))
    }
}

impl<'a> Interpreter<'a> {
    fn evaluate_unary(
        &self,
        operator: &str,
        value: Value,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match (operator, value) {
            ("+", Value::Character(value)) => Ok(Value::Integer(i64::from(u32::from(value)))),
            ("+", value @ (Value::Integer(_) | Value::Real(_) | Value::BigInteger(_))) => Ok(value),
            ("-", Value::Character(value)) => Ok(Value::Integer(-i64::from(u32::from(value)))),
            ("-", Value::Integer(value)) => value
                .checked_neg()
                .map(Value::Integer)
                .ok_or_else(|| vec![self.error("estouro aritmético", span)]),
            ("-", Value::Real(value)) => Ok(Value::Real(-value)),
            ("-", Value::BigInteger(value)) => Ok(Value::BigInteger(bigint::negate(&value))),
            ("não" | "nao", Value::Logical(value)) => Ok(Value::Logical(!value)),
            _ => Err(vec![
                self.error("operando inválido para operador unário", span),
            ]),
        }
    }

    fn evaluate_binary(
        &mut self,
        left: &Expr,
        operator: &str,
        right: &Expr,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let left = self.evaluate_expression(left)?;
        if operator == "e" {
            let left = self.expect_logical(left, span)?;
            return if left {
                self.evaluate_expression(right)
            } else {
                Ok(Value::Logical(false))
            };
        }
        if operator == "ou" {
            let left = self.expect_logical(left, span)?;
            return if left {
                Ok(Value::Logical(true))
            } else {
                self.evaluate_expression(right)
            };
        }
        let right = self.evaluate_expression(right)?;
        match operator {
            "==" => Ok(Value::Logical(left == right)),
            "<>" => Ok(Value::Logical(left != right)),
            "<" | "<=" | ">" | ">=" => {
                let order = compare_values(&left, &right)
                    .ok_or_else(|| vec![self.error("valores não podem ser ordenados", span)])?;
                Ok(Value::Logical(match operator {
                    "<" => order.is_lt(),
                    "<=" => order.is_le(),
                    ">" => order.is_gt(),
                    _ => order.is_ge(),
                }))
            }
            "::" => {
                let mut values = match right {
                    Value::List(values) => values,
                    Value::MutableList(values) => values.borrow().clone(),
                    _ => {
                        return Err(vec![
                            self.error("o lado direito de '::' deve ser lista", span),
                        ]);
                    }
                };
                values.insert(0, left);
                self.reserve_elements(1, span)?;
                Ok(Value::List(values))
            }
            "formato" => {
                let Value::Text(format) = right else {
                    return Err(vec![self.error("formato espera um Texto", span)]);
                };
                let result = format_value(&left, &format)
                    .ok_or_else(|| vec![self.error("formato inválido para este valor", span)])?;
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "+" => self.add_values(left, right, span),
            "-" => self.subtract_values(left, right, span),
            "*" => self.multiply_values(left, right, span),
            "/" => {
                let left = numeric_f64(&left)
                    .ok_or_else(|| vec![self.error("operação exige números", span)])?;
                let right = numeric_f64(&right)
                    .ok_or_else(|| vec![self.error("operação exige números", span)])?;
                if right == 0.0 {
                    return Err(vec![self.error("divisão por zero", span)]);
                }
                Ok(Value::Real(left / right))
            }
            "div" | "mod" => {
                let left = numericize_character(left);
                let right = numericize_character(right);
                if matches!(left, Value::BigInteger(_)) || matches!(right, Value::BigInteger(_)) {
                    let left = bigint_repr(&left)
                        .ok_or_else(|| vec![self.error("div e mod exigem inteiros", span)])?;
                    let right = bigint_repr(&right)
                        .ok_or_else(|| vec![self.error("div e mod exigem inteiros", span)])?;
                    self.bigint_operation(
                        &left,
                        &right,
                        if operator == "div" { '/' } else { '%' },
                        span,
                    )
                } else if operator == "mod"
                    && (matches!(left, Value::Real(_)) || matches!(right, Value::Real(_)))
                {
                    let left = numeric_f64(&left)
                        .ok_or_else(|| vec![self.error("mod exige números", span)])?;
                    let right = numeric_f64(&right)
                        .ok_or_else(|| vec![self.error("mod exige números", span)])?;
                    if right == 0.0 {
                        return Err(vec![self.error("divisão por zero", span)]);
                    }
                    Ok(Value::Real(left % right))
                } else {
                    let left = self.expect_integer(left, span)?;
                    let right = self.expect_integer(right, span)?;
                    if right == 0 {
                        return Err(vec![self.error("divisão por zero", span)]);
                    }
                    let value = if operator == "div" {
                        left.checked_div(right)
                    } else {
                        left.checked_rem(right)
                    }
                    .ok_or_else(|| vec![self.error("estouro aritmético", span)])?;
                    Ok(Value::Integer(value))
                }
            }
            "^" => self.power_values(left, right, span),
            _ => Err(vec![
                self.error(format!("operador '{operator}' desconhecido"), span),
            ]),
        }
    }

    fn add_values(&mut self, left: Value, right: Value, span: SourceSpan) -> RuntimeResult<Value> {
        if let Value::Text(mut text) = left {
            text.push_str(&right.display());
            self.check_string_size(text.len(), span)?;
            return Ok(Value::Text(text));
        }
        let left = numericize_character(left);
        let right = numericize_character(right);
        match (left, right) {
            (Value::Integer(left), Value::Integer(right)) => left
                .checked_add(right)
                .map(Value::Integer)
                .ok_or_else(|| vec![self.error("estouro aritmético", span)]),
            (Value::Real(left), Value::Real(right)) => Ok(Value::Real(left + right)),
            (Value::Integer(left), Value::Real(right)) => Ok(Value::Real(left as f64 + right)),
            (Value::Real(left), Value::Integer(right)) => Ok(Value::Real(left + right as f64)),
            (Value::BigInteger(left), Value::Real(right)) => Ok(Value::Real(
                left.parse::<f64>().unwrap_or(f64::INFINITY) + right,
            )),
            (Value::Real(left), Value::BigInteger(right)) => Ok(Value::Real(
                left + right.parse::<f64>().unwrap_or(f64::INFINITY),
            )),
            (Value::BigInteger(left), Value::Integer(right)) => {
                self.bigint_operation(&left, &right.to_string(), '+', span)
            }
            (Value::Integer(left), Value::BigInteger(right)) => {
                self.bigint_operation(&left.to_string(), &right, '+', span)
            }
            (Value::BigInteger(left), Value::BigInteger(right)) => {
                self.bigint_operation(&left, &right, '+', span)
            }
            (Value::List(mut left), Value::List(right)) => {
                self.reserve_elements(right.len(), span)?;
                left.extend(right);
                Ok(Value::List(left))
            }
            (Value::MutableList(left), Value::List(right)) => {
                self.reserve_elements(right.len(), span)?;
                let mut values = left.borrow().clone();
                values.extend(right);
                Ok(Value::List(values))
            }
            (Value::MutableList(left), Value::MutableList(right)) => {
                let mut values = left.borrow().clone();
                let right = right.borrow();
                self.reserve_elements(right.len(), span)?;
                values.extend(right.iter().cloned());
                Ok(Value::List(values))
            }
            (Value::List(mut left), Value::MutableList(right)) => {
                let right = right.borrow();
                self.reserve_elements(right.len(), span)?;
                left.extend(right.iter().cloned());
                Ok(Value::List(left))
            }
            _ => Err(vec![self.error("valores incompatíveis para '+'", span)]),
        }
    }

    fn subtract_values(
        &mut self,
        left: Value,
        right: Value,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if let (Value::Text(left_text), Value::Text(remove)) = (&left, &right) {
            let mut text = left_text.clone();
            for character in remove.chars() {
                if let Some(index) = text.find(character) {
                    text.remove(index);
                }
            }
            self.check_string_size(text.len(), span)?;
            return Ok(Value::Text(text));
        }
        let left = numericize_character(left);
        let right = numericize_character(right);
        match (left, right) {
            (Value::Integer(left), Value::Integer(right)) => left
                .checked_sub(right)
                .map(Value::Integer)
                .ok_or_else(|| vec![self.error("estouro aritmético", span)]),
            (Value::Real(left), Value::Real(right)) => Ok(Value::Real(left - right)),
            (Value::Integer(left), Value::Real(right)) => Ok(Value::Real(left as f64 - right)),
            (Value::Real(left), Value::Integer(right)) => Ok(Value::Real(left - right as f64)),
            (Value::BigInteger(left), Value::Real(right)) => Ok(Value::Real(
                left.parse::<f64>().unwrap_or(f64::INFINITY) - right,
            )),
            (Value::Real(left), Value::BigInteger(right)) => Ok(Value::Real(
                left - right.parse::<f64>().unwrap_or(f64::INFINITY),
            )),
            (Value::BigInteger(left), Value::Integer(right)) => {
                self.bigint_operation(&left, &right.to_string(), '-', span)
            }
            (Value::Integer(left), Value::BigInteger(right)) => {
                self.bigint_operation(&left.to_string(), &right, '-', span)
            }
            (Value::BigInteger(left), Value::BigInteger(right)) => {
                self.bigint_operation(&left, &right, '-', span)
            }
            _ => Err(vec![self.error("valores incompatíveis para '-'", span)]),
        }
    }

    fn multiply_values(
        &mut self,
        left: Value,
        right: Value,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let left = numericize_character(left);
        let right = numericize_character(right);
        match (left, right) {
            (Value::Integer(left), Value::Integer(right)) => left
                .checked_mul(right)
                .map(Value::Integer)
                .ok_or_else(|| vec![self.error("estouro aritmético", span)]),
            (Value::Real(left), Value::Real(right)) => Ok(Value::Real(left * right)),
            (Value::Integer(left), Value::Real(right)) => Ok(Value::Real(left as f64 * right)),
            (Value::Real(left), Value::Integer(right)) => Ok(Value::Real(left * right as f64)),
            (Value::BigInteger(left), Value::Real(right)) => Ok(Value::Real(
                left.parse::<f64>().unwrap_or(f64::INFINITY) * right,
            )),
            (Value::Real(left), Value::BigInteger(right)) => Ok(Value::Real(
                left * right.parse::<f64>().unwrap_or(f64::INFINITY),
            )),
            (Value::BigInteger(left), Value::Integer(right)) => {
                self.bigint_operation(&left, &right.to_string(), '*', span)
            }
            (Value::Integer(left), Value::BigInteger(right)) => {
                self.bigint_operation(&left.to_string(), &right, '*', span)
            }
            (Value::BigInteger(left), Value::BigInteger(right)) => {
                self.bigint_operation(&left, &right, '*', span)
            }
            (Value::Text(text), Value::Integer(count))
            | (Value::Integer(count), Value::Text(text)) => {
                if count < 0 {
                    return Err(vec![self.error("repetição de texto negativa", span)]);
                }
                let count = usize::try_from(count)
                    .map_err(|_| vec![self.error("repetição de texto inválida", span)])?;
                let size = text.len().saturating_mul(count);
                self.check_string_size(size, span)?;
                Ok(Value::Text(text.repeat(count)))
            }
            _ => Err(vec![self.error("valores incompatíveis para '*'", span)]),
        }
    }

    fn power_values(
        &mut self,
        left: Value,
        right: Value,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let left = numericize_character(left);
        let right = numericize_character(right);
        match (left, right) {
            (Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Real((left as f64).powf(right as f64)))
            }
            (Value::BigInteger(base), Value::BigInteger(exponent)) => {
                let exponent = exponent.parse::<u64>().map_err(|_| {
                    vec![self.error("expoente de BigInt inválido ou negativo", span)]
                })?;
                self.bigint_power(&base, exponent, span)
            }
            (Value::BigInteger(base), Value::Integer(exponent)) if exponent >= 0 => {
                self.bigint_power(&base, exponent as u64, span)
            }
            (Value::Integer(base), Value::BigInteger(exponent)) => {
                let exponent = exponent.parse::<u64>().map_err(|_| {
                    vec![self.error("expoente de BigInt inválido ou negativo", span)]
                })?;
                self.bigint_power(&base.to_string(), exponent, span)
            }
            (left, right) => {
                let left = numeric_f64(&left)
                    .ok_or_else(|| vec![self.error("base numérica inválida", span)])?;
                let right = numeric_f64(&right)
                    .ok_or_else(|| vec![self.error("expoente numérico inválido", span)])?;
                Ok(Value::Real(left.powf(right)))
            }
        }
    }

    fn member_value(
        &mut self,
        receiver: Value,
        name: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if name == "texto" {
            return Ok(Value::Text(receiver.display()));
        }
        if name == "qual_tipo" {
            return Ok(Value::Text(runtime_type_name(&receiver)));
        }
        match receiver.clone() {
            Value::TypeName(_) => Ok(Value::Function(Rc::new(Callable::Member {
                receiver,
                name: name.to_owned(),
            }))),
            Value::List(_) | Value::MutableList(_) => self.list_member_value(receiver, name, span),
            Value::Text(text) => self.text_member_value(&text, name, span),
            Value::Integer(_) | Value::Real(_) | Value::BigInteger(_) | Value::Character(_) => {
                self.numeric_member_value(receiver, name, span)
            }
            Value::Tuple(values) => {
                let index = tuple_member_index(name).ok_or_else(|| {
                    vec![self.error(format!("tupla não possui membro '{name}'"), span)]
                })?;
                values.get(index).cloned().ok_or_else(|| {
                    vec![self.error(format!("tupla não possui membro '{name}'"), span)]
                })
            }
            Value::Object(object) => {
                if let Some(value) = object.borrow().fields.get(name).cloned() {
                    return Ok(value);
                }
                let class_name = object.borrow().class_name.clone();
                let Some(declaration) = self.classes.get(&class_name).cloned() else {
                    return Err(vec![self.error("tipo do objeto não foi encontrado", span)]);
                };
                let Some(method) = declaration
                    .methods
                    .iter()
                    .find(|method| method.name == name)
                else {
                    return Err(vec![self.error(
                        format!("membro '{name}' não existe em '{class_name}'"),
                        span,
                    )]);
                };
                let callable = Value::Function(Rc::new(Callable::Function {
                    declaration: method.clone(),
                    environment: self
                        .class_environments
                        .get(&class_name)
                        .cloned()
                        .unwrap_or_else(|| self.environment.clone()),
                    owner: Some(declaration.name),
                    receiver: Some(Value::Object(object)),
                }));
                if method.parameters.is_empty() {
                    self.apply(callable, Vec::new(), span)
                } else {
                    Ok(callable)
                }
            }
            _ => Err(vec![self.error(
                format!("tipo {} não possui membro '{name}'", receiver.display()),
                span,
            )]),
        }
    }

    fn list_member_value(
        &mut self,
        receiver: Value,
        name: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let values = list_values(&receiver).unwrap_or_default();
        match name {
            "tamanho" | "size" => Ok(Value::Integer(values.len() as i64)),
            "cabeça" | "cabeca" | "primeiro" => values
                .first()
                .cloned()
                .ok_or_else(|| vec![self.error("lista vazia não possui cabeça", span)]),
            "último" | "ultimo" => values
                .last()
                .cloned()
                .ok_or_else(|| vec![self.error("lista vazia não possui último elemento", span)]),
            "cauda" => Ok(Value::List(values.into_iter().skip(1).collect())),
            "vazia" => Ok(Value::Logical(values.is_empty())),
            "inverta" => {
                let mut values = values;
                values.reverse();
                self.collection_value_like(&receiver, values)
            }
            "ordene" => self.sort_values(values, None, span).map(Value::List),
            "junte" => {
                let text = values.iter().map(Value::display).collect::<String>();
                self.check_string_size(text.len(), span)?;
                Ok(Value::Text(text))
            }
            "mutável" | "mutavel" => {
                self.reserve_elements(values.len(), span)?;
                Ok(Value::MutableList(Rc::new(RefCell::new(values))))
            }
            "imutável" | "imutavel" => Ok(Value::List(values)),
            _ => Ok(Value::Function(Rc::new(Callable::Member {
                receiver,
                name: name.to_owned(),
            }))),
        }
    }

    fn collection_value_like(&self, source: &Value, values: Vec<Value>) -> RuntimeResult<Value> {
        match source {
            Value::MutableList(_) => Ok(Value::MutableList(Rc::new(RefCell::new(values)))),
            _ => Ok(Value::List(values)),
        }
    }

    fn text_member_value(
        &mut self,
        text: &str,
        name: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match name {
            "tamanho" => Ok(Value::Integer(text.chars().count() as i64)),
            "maiúsculo" | "maiusculo" => {
                let result = text.to_uppercase();
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "minúsculo" | "minusculo" => {
                let result = text.to_lowercase();
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "inverta" => Ok(Value::Text(text.chars().rev().collect())),
            "inteiro" => Ok(Value::Integer(parse_integer_prefix(text))),
            "real" => Ok(Value::Real(parse_real_prefix(text))),
            "lista" => {
                let characters = text.chars().map(Value::Character).collect::<Vec<_>>();
                self.reserve_elements(characters.len(), span)?;
                Ok(Value::List(characters))
            }
            "cabeça" | "cabeca" => text
                .chars()
                .next()
                .map(Value::Character)
                .ok_or_else(|| vec![self.error("Texto vazio não possui cabeça", span)]),
            "último" | "ultimo" => text
                .chars()
                .next_back()
                .map(Value::Character)
                .ok_or_else(|| vec![self.error("Texto vazio não possui último caractere", span)]),
            "cauda" => {
                let result = text.chars().skip(1).collect::<String>();
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "ordene" => {
                let mut characters = text.chars().collect::<Vec<_>>();
                characters.sort_unstable();
                Ok(Value::Text(characters.into_iter().collect()))
            }
            "divida" | "pegue" | "descarte" | "junte" | "posição" | "posicao" | "contém"
            | "contem" | "selecione" | "injete" | "pegue_enquanto" | "descarte_enquanto"
            | "remova" | "insira" => Ok(Value::Function(Rc::new(Callable::Member {
                receiver: Value::Text(text.to_owned()),
                name: name.to_owned(),
            }))),
            _ => Err(vec![
                self.error(format!("Texto não possui membro '{name}'"), span),
            ]),
        }
    }

    fn numeric_member_value(
        &self,
        value: Value,
        name: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match name {
            "inteiro" => match &value {
                Value::BigInteger(value) => value
                    .parse::<i64>()
                    .map(Value::Integer)
                    .map_err(|_| vec![self.error("BigInt não cabe em Inteiro", span)]),
                _ => numeric_f64(&value)
                    .map(|number| Value::Integer(number as i64))
                    .ok_or_else(|| vec![self.error("conversão para Inteiro inválida", span)]),
            },
            "real" => numeric_f64(&value)
                .map(Value::Real)
                .ok_or_else(|| vec![self.error("conversão para Real inválida", span)]),
            "piso" => numeric_f64(&value)
                .map(|number| Value::Real(number.floor()))
                .ok_or_else(|| vec![self.error("conversão para Real inválida", span)]),
            "teto" => numeric_f64(&value)
                .map(|number| Value::Real(number.ceil()))
                .ok_or_else(|| vec![self.error("conversão para Real inválida", span)]),
            "arredonde" => numeric_f64(&value)
                .map(|number| Value::Integer(number.round() as i64))
                .ok_or_else(|| vec![self.error("conversão para Inteiro inválida", span)]),
            "pow" if matches!(&value, Value::BigInteger(_)) => {
                Ok(Value::Function(Rc::new(Callable::Member {
                    receiver: value,
                    name: name.to_owned(),
                })))
            }
            "caractere" | "caracter" => {
                let Some(number) = numeric_f64(&value) else {
                    return Err(vec![self.error("conversão para Caractere inválida", span)]);
                };
                char::from_u32(number as u32)
                    .map(Value::Character)
                    .ok_or_else(|| vec![self.error("código de Caractere inválido", span)])
            }
            _ => Err(vec![self.error(
                format!("tipo {} não possui membro '{name}'", value.display()),
                span,
            )]),
        }
    }

    fn call_numeric_member(
        &mut self,
        receiver: Value,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match (receiver, name, arguments.as_slice()) {
            (Value::BigInteger(base), "pow", [Value::Integer(exponent)]) if *exponent >= 0 => {
                self.bigint_power(&base, *exponent as u64, span)
            }
            (Value::Real(value), "arredonde", [Value::Integer(digits)]) => {
                let factor = 10_f64.powi((*digits).clamp(-308, 308) as i32);
                let rounded = if factor.is_finite() && (value * factor).is_finite() {
                    (value * factor).round() / factor
                } else {
                    value
                };
                Ok(Value::Real(rounded))
            }
            (Value::Real(value), "arredonde", []) => Ok(Value::Integer(value.round() as i64)),
            (receiver, _, _) => Err(vec![self.error(
                format!("{} não possui método '{name}'", receiver.display()),
                span,
            )]),
        }
    }

    fn index_value(&self, receiver: Value, index: i64, span: SourceSpan) -> RuntimeResult<Value> {
        let values = match receiver {
            Value::List(values) => values,
            Value::MutableList(values) => values.borrow().clone(),
            Value::Tuple(values) => values,
            Value::Text(text) => {
                let characters = text.chars().map(Value::Character).collect::<Vec<_>>();
                let index = normalize_index(index, characters.len())
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                return Ok(characters[index].clone());
            }
            other => {
                return Err(vec![
                    self.error(format!("{} não aceita índice", other.display()), span),
                ]);
            }
        };
        let index = normalize_index(index, values.len())
            .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
        Ok(values[index].clone())
    }

    fn evaluate_match(
        &mut self,
        value: &Expr,
        cases: &[MatchCase],
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let value = self.evaluate_expression(value)?;
        if self.pending_return.is_some() {
            return Ok(Value::Nothing);
        }
        for case in cases {
            let mut bindings = Vec::new();
            if !self.collect_pattern_bindings(&case.pattern, &value, &mut bindings)? {
                continue;
            }
            let matched = self.with_child_environment(|interpreter| {
                for (name, value) in bindings {
                    Environment::define(&interpreter.environment, name, value, false)
                        .map_err(|message| vec![interpreter.error(message, case.pattern.span)])?;
                }
                if let Some(guard) = &case.guard {
                    let guard_value = interpreter.evaluate_expression(guard)?;
                    if interpreter.pending_return.is_some() {
                        return Ok(None);
                    }
                    if !interpreter.expect_logical(guard_value, guard.span)? {
                        return Ok(None);
                    }
                }
                match interpreter.execute_block(&case.value)? {
                    Flow::Next(value) => Ok(Some(value)),
                    Flow::Return(value) => {
                        interpreter.pending_return = Some(value);
                        Ok(Some(Value::Nothing))
                    }
                }
            })?;
            if let Some(value) = matched {
                return Ok(value);
            }
        }
        Err(vec![
            self.error("nenhum caso de escolha corresponde ao valor", span),
        ])
    }

    fn evaluate_comprehension(
        &mut self,
        generators: &[ForGenerator],
        index: usize,
        value: &Expr,
        results: &mut Vec<Value>,
    ) -> RuntimeResult<()> {
        if self.pending_return.is_some() {
            return Ok(());
        }
        let Some(generator) = generators.get(index) else {
            results.push(self.evaluate_expression(value)?);
            if results.len() > self.options.max_collection_elements {
                return Err(vec![self.error("limite de elementos excedido", value.span)]);
            }
            return Ok(());
        };
        for item in self.generator_values(generator)? {
            self.consume_step(generator.span)?;
            self.with_child_environment(|interpreter| {
                interpreter.bind_pattern(&generator.pattern, item, false)?;
                if let Some(condition) = &generator.condition {
                    let condition = interpreter.evaluate_expression(condition)?;
                    if interpreter.pending_return.is_some() {
                        return Ok(());
                    }
                    if !interpreter.expect_logical(condition, generator.span)? {
                        return Ok(());
                    }
                }
                interpreter.evaluate_comprehension(generators, index + 1, value, results)
            })?;
            if self.pending_return.is_some() {
                return Ok(());
            }
        }
        Ok(())
    }

    fn call_member(
        &mut self,
        receiver: Value,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match receiver.clone() {
            Value::List(_) | Value::MutableList(_) => {
                self.call_list_member(receiver, name, arguments, span)
            }
            Value::Text(text) => self.call_text_member(&text, name, arguments, span),
            Value::Integer(_) | Value::Real(_) | Value::BigInteger(_) | Value::Character(_) => {
                self.call_numeric_member(receiver, name, arguments, span)
            }
            Value::TypeName(type_name) => self.call_type_factory(&type_name, name, arguments, span),
            Value::Object(object) => {
                let class_name = object.borrow().class_name.clone();
                let Some(declaration) = self.classes.get(&class_name).cloned() else {
                    return Err(vec![self.error("tipo do objeto não foi encontrado", span)]);
                };
                let Some(method) = declaration
                    .methods
                    .iter()
                    .find(|method| method.name == name)
                else {
                    return Err(vec![self.error(
                        format!("método '{name}' não existe em '{class_name}'"),
                        span,
                    )]);
                };
                let callable = Value::Function(Rc::new(Callable::Function {
                    declaration: method.clone(),
                    environment: self
                        .class_environments
                        .get(&class_name)
                        .cloned()
                        .unwrap_or_else(|| self.environment.clone()),
                    owner: Some(class_name),
                    receiver: Some(Value::Object(object)),
                }));
                self.apply(callable, arguments, span)
            }
            _ => Err(vec![self.error(
                format!("tipo {} não possui método '{name}'", receiver.display()),
                span,
            )]),
        }
    }

    fn call_type_factory(
        &mut self,
        type_name: &str,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let (dimensions, mutable) = match (type_name, name) {
            ("Lista", "imutável" | "imutavel") => (1, false),
            ("Lista", "mutável" | "mutavel") => (1, true),
            ("Matriz", "imutável" | "imutavel") => (2, false),
            ("Cubo", "imutável" | "imutavel") => (3, false),
            _ => {
                return Err(vec![self.error(
                    format!("{type_name} não possui construtor '{name}'"),
                    span,
                )]);
            }
        };
        if arguments.len() != dimensions + 1 {
            return Err(vec![self.error(
                format!("{type_name}.{name} recebeu quantidade incorreta de argumentos"),
                span,
            )]);
        }
        let mut sizes = Vec::with_capacity(dimensions);
        for argument in arguments.iter().take(dimensions) {
            let size = self.expect_integer(argument.clone(), span)?;
            if size < 0 {
                return Err(vec![self.error("dimensão não pode ser negativa", span)]);
            }
            sizes.push(
                usize::try_from(size).map_err(|_| vec![self.error("dimensão inválida", span)])?,
            );
        }
        let total = sizes
            .iter()
            .try_fold(1usize, |product, size| product.checked_mul(*size))
            .ok_or_else(|| vec![self.error("dimensões excedem o limite", span)])?;
        self.reserve_elements(total, span)?;
        let initial = arguments.last().cloned().unwrap_or(Value::Nothing);
        let values = make_nested_list(&sizes, initial);
        if mutable {
            Ok(Value::MutableList(Rc::new(RefCell::new(values))))
        } else {
            Ok(Value::List(values))
        }
    }

    fn call_text_member(
        &mut self,
        text: &str,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match name {
            "divida" => {
                if arguments.len() > 1 {
                    return Err(vec![
                        self.error("divida aceita no máximo um separador", span),
                    ]);
                }
                let pieces: Vec<String> = if let Some(Value::Text(separator)) = arguments.first() {
                    if separator.is_empty() {
                        text.chars()
                            .map(|character| character.to_string())
                            .collect()
                    } else {
                        text.split(separator).map(str::to_owned).collect()
                    }
                } else if arguments.is_empty() {
                    text.split_whitespace().map(str::to_owned).collect()
                } else {
                    return Err(vec![self.error("o separador deve ser Texto", span)]);
                };
                self.reserve_elements(pieces.len(), span)?;
                Ok(Value::List(pieces.into_iter().map(Value::Text).collect()))
            }
            "pegue" | "descarte" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error(format!("{name} recebe um número"), span)]);
                }
                let count = self.expect_integer(arguments[0].clone(), span)?.max(0);
                let chars = text.chars().collect::<Vec<_>>();
                let count = usize::try_from(count)
                    .unwrap_or(usize::MAX)
                    .min(chars.len());
                let result: String = if name == "pegue" {
                    chars.into_iter().take(count).collect()
                } else {
                    chars.into_iter().skip(count).collect()
                };
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "junte" => {
                let (prefix, separator, suffix) = match arguments.as_slice() {
                    [Value::Text(separator)] => ("", separator.as_str(), ""),
                    [
                        Value::Text(prefix),
                        Value::Text(separator),
                        Value::Text(suffix),
                    ] => (prefix.as_str(), separator.as_str(), suffix.as_str()),
                    _ => {
                        return Err(vec![self.error(
                            "junte recebe um separador ou prefixo, separador e sufixo",
                            span,
                        )]);
                    }
                };
                self.reserve_elements(text.chars().count(), span)?;
                let joined = text
                    .chars()
                    .map(|character| character.to_string())
                    .collect::<Vec<_>>()
                    .join(separator);
                let mut result = String::from(prefix);
                result.push_str(&joined);
                result.push_str(suffix);
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "posição" | "posicao" | "contém" | "contem" => {
                if arguments.len() != 1 {
                    return Err(vec![
                        self.error(format!("{name} recebe um caractere ou texto"), span),
                    ]);
                }
                let needle = match &arguments[0] {
                    Value::Character(character) => character.to_string(),
                    Value::Text(text) => text.clone(),
                    other => {
                        return Err(vec![
                            self.error(format!("{name} não aceita {}", other.display()), span),
                        ]);
                    }
                };
                if matches!(name, "posição" | "posicao") {
                    let position = text.find(&needle).map_or(0, |byte_index| {
                        text[..byte_index].chars().count() as i64 + 1
                    });
                    Ok(Value::Integer(position))
                } else {
                    Ok(Value::Logical(text.contains(&needle)))
                }
            }
            "selecione" | "pegue_enquanto" | "descarte_enquanto" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error(format!("{name} recebe uma função"), span)]);
                }
                let characters = text.chars().collect::<Vec<_>>();
                if name == "selecione" {
                    let mut result = String::new();
                    for character in characters {
                        self.consume_step(span)?;
                        let selected = self.apply(
                            arguments[0].clone(),
                            vec![Value::Character(character)],
                            span,
                        )?;
                        if self.expect_logical(selected, span)? {
                            result.push(character);
                        }
                    }
                    self.check_string_size(result.len(), span)?;
                    Ok(Value::Text(result))
                } else {
                    let mut count = 0;
                    for character in &characters {
                        self.consume_step(span)?;
                        let selected = self.apply(
                            arguments[0].clone(),
                            vec![Value::Character(*character)],
                            span,
                        )?;
                        if !self.expect_logical(selected, span)? {
                            break;
                        }
                        count += 1;
                    }
                    let result: String = if name == "pegue_enquanto" {
                        characters.into_iter().take(count).collect()
                    } else {
                        characters.into_iter().skip(count).collect()
                    };
                    self.check_string_size(result.len(), span)?;
                    Ok(Value::Text(result))
                }
            }
            "injete" => {
                if arguments.len() == 1 {
                    return Ok(Value::Function(Rc::new(Callable::Partial {
                        callable: Rc::new(Callable::Member {
                            receiver: Value::Text(text.to_owned()),
                            name: name.to_owned(),
                        }),
                        arguments,
                    })));
                }
                let [initial, reducer] = arguments.as_slice() else {
                    return Err(vec![self.error("injete recebe acumulador e função", span)]);
                };
                let mut accumulator = initial.clone();
                for character in text.chars() {
                    self.consume_step(span)?;
                    accumulator = self.apply(
                        reducer.clone(),
                        vec![accumulator, Value::Character(character)],
                        span,
                    )?;
                }
                Ok(accumulator)
            }
            "remova" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("remova recebe uma posição", span)]);
                }
                let mut characters = text.chars().collect::<Vec<_>>();
                let index = self.expect_integer(arguments[0].clone(), span)?;
                let index = normalize_index(index, characters.len())
                    .ok_or_else(|| vec![self.error("posição fora dos limites", span)])?;
                characters.remove(index);
                Ok(Value::Text(characters.into_iter().collect()))
            }
            "insira" => {
                if arguments.len() != 2 {
                    return Err(vec![self.error("insira recebe posição e valor", span)]);
                }
                let position = self.expect_integer(arguments[0].clone(), span)?;
                let mut characters = text.chars().collect::<Vec<_>>();
                if position <= 0 || position as usize > characters.len() + 1 {
                    return Err(vec![self.error("posição de inserção inválida", span)]);
                }
                let insert_at = position as usize - 1;
                let insertion = match &arguments[1] {
                    Value::Character(character) => character.to_string(),
                    Value::Text(text) => text.clone(),
                    other => {
                        return Err(vec![
                            self.error(format!("insira não aceita {}", other.display()), span),
                        ]);
                    }
                };
                characters.splice(insert_at..insert_at, insertion.chars());
                let result = characters.into_iter().collect::<String>();
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            _ => Err(vec![
                self.error(format!("Texto não possui método '{name}'"), span),
            ]),
        }
    }

    fn call_list_member(
        &mut self,
        receiver: Value,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let values = list_values(&receiver).unwrap_or_default();
        match name {
            "mapeie" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("mapeie recebe uma função", span)]);
                }
                let mut result = Vec::with_capacity(values.len());
                for value in values {
                    result.push(self.apply(arguments[0].clone(), vec![value], span)?);
                }
                self.reserve_elements(result.len(), span)?;
                Ok(Value::List(result))
            }
            "selecione" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("selecione recebe uma função", span)]);
                }
                let mut result = Vec::new();
                for value in values {
                    let selected = self.apply(arguments[0].clone(), vec![value.clone()], span)?;
                    if self.expect_logical(selected, span)? {
                        result.push(value);
                    }
                }
                self.reserve_elements(result.len(), span)?;
                Ok(Value::List(result))
            }
            "pegue_enquanto" | "descarte_enquanto" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error(format!("{name} recebe uma função"), span)]);
                }
                let mut count = 0;
                for value in &values {
                    let predicate = self.apply(arguments[0].clone(), vec![value.clone()], span)?;
                    let matches = self.expect_logical(predicate, span)?;
                    if !matches {
                        break;
                    }
                    count += 1;
                }
                let result = if name == "pegue_enquanto" {
                    values.into_iter().take(count).collect()
                } else {
                    values.into_iter().skip(count).collect()
                };
                Ok(Value::List(result))
            }
            "injete" => {
                if arguments.len() == 1 && !matches!(arguments[0], Value::Function(_)) {
                    return Ok(Value::Function(Rc::new(Callable::Partial {
                        callable: Rc::new(Callable::Member {
                            receiver,
                            name: name.to_owned(),
                        }),
                        arguments,
                    })));
                }
                let (mut accumulator, reducer, remaining) = match arguments.as_slice() {
                    [Value::Function(_)] => {
                        let Some(first) = values.first().cloned() else {
                            return Err(vec![
                                self.error("injete sem acumulador exige lista não vazia", span),
                            ]);
                        };
                        (
                            first,
                            arguments[0].clone(),
                            values.into_iter().skip(1).collect::<Vec<_>>(),
                        )
                    }
                    [initial, Value::Function(reducer)] => {
                        (initial.clone(), Value::Function(reducer.clone()), values)
                    }
                    _ => return Err(vec![self.error("injete recebe acumulador e função", span)]),
                };
                for value in remaining {
                    accumulator = self.apply(reducer.clone(), vec![accumulator, value], span)?;
                }
                Ok(accumulator)
            }
            "ordene" => {
                if arguments.len() > 1 {
                    return Err(vec![self.error("ordene aceita no máximo uma função", span)]);
                }
                let sorted = self.sort_values(values, arguments.first(), span)?;
                self.collection_value_like(&receiver, sorted)
            }
            "posição" | "posicao" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("posição recebe um valor", span)]);
                }
                let position = values
                    .iter()
                    .position(|value| value == &arguments[0])
                    .map_or(0, |index| index as i64 + 1);
                Ok(Value::Integer(position))
            }
            "contém" | "contem" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("contém recebe um valor", span)]);
                }
                Ok(Value::Logical(values.contains(&arguments[0])))
            }
            "remova" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("remova recebe uma posição", span)]);
                }
                let index = self.expect_integer(arguments[0].clone(), span)?;
                let index = normalize_index(index, values.len())
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                let mut result = values;
                result.remove(index);
                self.return_collection_mutation(receiver, result, span)
            }
            "insira" | "atualize" => {
                if arguments.len() != 2 {
                    return Err(vec![
                        self.error(format!("{name} recebe posição e valor"), span),
                    ]);
                }
                let index = self.expect_integer(arguments[0].clone(), span)?;
                let mut result = values;
                if name == "insira" {
                    if index <= 0 || index as usize > result.len() + 1 {
                        return Err(vec![self.error("posição de inserção inválida", span)]);
                    }
                    result.insert(index as usize - 1, arguments[1].clone());
                    self.reserve_elements(1, span)?;
                } else {
                    let index = normalize_index(index, result.len())
                        .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                    result[index] = arguments[1].clone();
                }
                self.return_collection_mutation(receiver, result, span)
            }
            "junte" => {
                if arguments.len() > 3 || arguments.len() == 2 {
                    return Err(vec![
                        self.error("junte recebe zero, um ou três argumentos", span),
                    ]);
                }
                let (prefix, separator, suffix) = match arguments.as_slice() {
                    [] => ("", "", ""),
                    [Value::Text(separator)] => ("", separator.as_str(), ""),
                    [
                        Value::Text(prefix),
                        Value::Text(separator),
                        Value::Text(suffix),
                    ] => (prefix.as_str(), separator.as_str(), suffix.as_str()),
                    _ => {
                        return Err(vec![
                            self.error("argumentos de junte devem ser Texto", span),
                        ]);
                    }
                };
                let mut result = String::from(prefix);
                result.push_str(
                    &values
                        .iter()
                        .map(Value::display)
                        .collect::<Vec<_>>()
                        .join(separator),
                );
                result.push_str(suffix);
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "divida_quando" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("divida_quando recebe uma função", span)]);
                }
                let mut groups = Vec::<Vec<Value>>::new();
                for value in values {
                    let split = if let Some(previous) = groups.last().and_then(|group| group.last())
                    {
                        let split_result = self.apply(
                            arguments[0].clone(),
                            vec![previous.clone(), value.clone()],
                            span,
                        )?;
                        self.expect_logical(split_result, span)?
                    } else {
                        false
                    };
                    if split || groups.is_empty() {
                        groups.push(Vec::new());
                    }
                    groups.last_mut().unwrap().push(value);
                }
                self.reserve_elements(groups.len(), span)?;
                Ok(Value::List(
                    groups.into_iter().map(Value::List).collect::<Vec<_>>(),
                ))
            }
            "pegue" | "descarte" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error(format!("{name} recebe um número"), span)]);
                }
                let count = self.expect_integer(arguments[0].clone(), span)?.max(0) as usize;
                let result = if name == "pegue" {
                    values.into_iter().take(count).collect()
                } else {
                    values.into_iter().skip(count).collect()
                };
                Ok(Value::List(result))
            }
            _ => Err(vec![
                self.error(format!("lista não possui método '{name}'"), span),
            ]),
        }
    }

    fn return_collection_mutation(
        &self,
        receiver: Value,
        values: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match receiver {
            Value::MutableList(target) => {
                *target.borrow_mut() = values;
                Ok(Value::MutableList(target))
            }
            Value::List(_) => Ok(Value::List(values)),
            _ => Err(vec![self.error("esperava uma lista", span)]),
        }
    }

    fn sort_values(
        &mut self,
        mut values: Vec<Value>,
        key_or_comparator: Option<&Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Vec<Value>> {
        let Some(function) = key_or_comparator else {
            let mut invalid = false;
            values.sort_by(|left, right| {
                compare_values(left, right).unwrap_or_else(|| {
                    invalid = true;
                    std::cmp::Ordering::Equal
                })
            });
            return if invalid {
                Err(vec![
                    self.error("elementos da lista não podem ser ordenados", span),
                ])
            } else {
                Ok(values)
            };
        };
        let Value::Function(callable) = function else {
            return Err(vec![self.error("ordene espera uma função", span)]);
        };
        let arity = self.callable_arity(&callable, &[]).unwrap_or(1);
        if arity >= 2 {
            let mut failure = None;
            values.sort_by(|left, right| {
                if failure.is_some() {
                    return std::cmp::Ordering::Equal;
                }
                match self
                    .apply(
                        Value::Function(callable.clone()),
                        vec![left.clone(), right.clone()],
                        span,
                    )
                    .and_then(|result| self.expect_logical(result, span))
                {
                    Ok(true) => std::cmp::Ordering::Less,
                    Ok(false) => match self
                        .apply(
                            Value::Function(callable.clone()),
                            vec![right.clone(), left.clone()],
                            span,
                        )
                        .and_then(|result| self.expect_logical(result, span))
                    {
                        Ok(true) => std::cmp::Ordering::Greater,
                        Ok(false) => std::cmp::Ordering::Equal,
                        Err(error) => {
                            failure = Some(error);
                            std::cmp::Ordering::Equal
                        }
                    },
                    Err(error) => {
                        failure = Some(error);
                        std::cmp::Ordering::Equal
                    }
                }
            });
            if let Some(error) = failure {
                return Err(error);
            }
            return Ok(values);
        }
        let mut keyed = values
            .into_iter()
            .map(|value| {
                self.apply(Value::Function(callable.clone()), vec![value.clone()], span)
                    .map(|key| (key, value))
            })
            .collect::<RuntimeResult<Vec<_>>>()?;
        let mut invalid = false;
        keyed.sort_by(|(left, _), (right, _)| {
            compare_values(left, right).unwrap_or_else(|| {
                invalid = true;
                std::cmp::Ordering::Equal
            })
        });
        if invalid {
            return Err(vec![
                self.error("chaves da ordenação não podem ser comparadas", span),
            ]);
        }
        Ok(keyed.into_iter().map(|(_, value)| value).collect())
    }

    fn apply(
        &mut self,
        function: Value,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let Value::Function(callable) = function else {
            return Err(vec![self.error(
                format!("valor do tipo {} não pode ser chamado", function.display()),
                span,
            )]);
        };
        let (callable, arguments) = match callable.as_ref() {
            Callable::Partial {
                callable,
                arguments: previous,
            } => {
                let mut combined = previous.clone();
                combined.extend(arguments);
                (callable.clone(), combined)
            }
            _ => (callable, arguments),
        };
        let arity = self
            .callable_arity(&callable, &arguments)
            .ok_or_else(|| vec![self.error("assinatura de função inválida", span)])?;
        if arguments.len() < arity {
            return Ok(Value::Function(Rc::new(Callable::Partial {
                callable,
                arguments,
            })));
        }
        let mut arguments = arguments;
        let extra = arguments.split_off(arity);
        let result = self.invoke_callable(callable, arguments, span)?;
        if extra.is_empty() {
            Ok(result)
        } else {
            self.apply(result, extra, span)
        }
    }

    fn callable_arity(&self, callable: &Callable, arguments: &[Value]) -> Option<usize> {
        match callable {
            Callable::Function { declaration, .. } => Some(declaration.parameters.len()),
            Callable::Lambda { parameters, .. } => Some(parameters.len()),
            Callable::Placeholder { arity, .. } => Some(*arity),
            Callable::Constructor { declaration, .. } => Some(
                declaration
                    .fields
                    .iter()
                    .filter(|field| field.constructor_parameter)
                    .count(),
            ),
            Callable::Partial {
                callable,
                arguments: previous,
            } => self.callable_arity(callable, previous),
            Callable::Member { receiver, name } => match receiver {
                Value::List(_) | Value::MutableList(_) => match name.as_str() {
                    "mapeie" | "selecione" | "pegue_enquanto" | "descarte_enquanto" | "posição"
                    | "posicao" | "contém" | "contem" | "remova" | "divida_quando" => Some(1),
                    "insira" | "atualize" => Some(2),
                    "injete" => Some(if matches!(arguments.first(), Some(Value::Function(_))) {
                        1
                    } else {
                        2
                    }),
                    "ordene" | "junte" => Some(arguments.len()),
                    "pegue" | "descarte" => Some(1),
                    _ => None,
                },
                Value::Text(_) => match name.as_str() {
                    "divida" => Some(arguments.len().min(1)),
                    "pegue" | "descarte" | "posição" | "posicao" | "contém" | "contem"
                    | "selecione" | "pegue_enquanto" | "descarte_enquanto" | "remova" => Some(1),
                    "injete" => Some(2),
                    "insira" => Some(2),
                    "junte" => Some(arguments.len()),
                    _ => None,
                },
                Value::BigInteger(_) if name == "pow" => Some(1),
                Value::TypeName(type_name) => match (type_name.as_str(), name.as_str()) {
                    ("Lista", "imutável" | "imutavel" | "mutável" | "mutavel") => Some(2),
                    ("Matriz", "imutável" | "imutavel") => Some(3),
                    ("Cubo", "imutável" | "imutavel") => Some(4),
                    _ => None,
                },
                Value::Object(object) => self
                    .classes
                    .get(&object.borrow().class_name)
                    .and_then(|class| class.methods.iter().find(|method| method.name == *name))
                    .map(|method| method.parameters.len()),
                _ => None,
            },
        }
    }

    fn invoke_callable(
        &mut self,
        callable: Rc<Callable>,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match callable.as_ref() {
            Callable::Function {
                declaration,
                environment,
                owner,
                receiver,
            } => self.invoke_function(
                declaration,
                environment.clone(),
                owner.as_deref(),
                receiver.clone(),
                arguments,
                span,
            ),
            Callable::Lambda {
                id,
                parameters,
                body,
                environment,
            } => self.invoke_lambda(*id, parameters, body, environment.clone(), arguments, span),
            Callable::Constructor {
                declaration,
                environment,
            } => self.construct(declaration, environment.clone(), arguments, span),
            Callable::Member { receiver, name } => {
                self.call_member(receiver.clone(), name, arguments, span)
            }
            Callable::Placeholder {
                expression,
                environment,
                ..
            } => {
                self.placeholder_values.push(arguments);
                self.placeholder_indexes.push(0);
                let result = self.with_environment(environment.clone(), |interpreter| {
                    interpreter.evaluate_expression(expression)
                });
                self.placeholder_values.pop();
                self.placeholder_indexes.pop();
                result
            }
            Callable::Partial { .. } => Err(vec![self.error("função parcial inválida", span)]),
        }
    }

    fn evaluate_tail_expression(
        &mut self,
        expression: &Expr,
        function_name: &str,
    ) -> RuntimeResult<TailOutcome> {
        match &expression.kind {
            ExprKind::Call { callee, arguments } if matches!(&callee.kind, ExprKind::Identifier(name) if name == function_name) =>
            {
                let arguments = arguments
                    .iter()
                    .map(|argument| self.evaluate_argument(argument))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                if let Some(value) = self.pending_return.take() {
                    return Ok(TailOutcome::Value(value));
                }
                Ok(TailOutcome::Call(arguments, expression.span))
            }
            ExprKind::Conditional {
                condition,
                then_branch,
                else_branch,
            } => {
                let condition = self.evaluate_expression(condition)?;
                if let Some(value) = self.pending_return.take() {
                    return Ok(TailOutcome::Value(value));
                }
                if self.expect_logical(condition, expression.span)? {
                    self.evaluate_tail_block(then_branch, function_name)
                } else if let Some(else_branch) = else_branch {
                    self.evaluate_tail_block(else_branch, function_name)
                } else {
                    Ok(TailOutcome::Value(Value::Nothing))
                }
            }
            ExprKind::Match { value, cases } => {
                let value = self.evaluate_expression(value)?;
                if let Some(value) = self.pending_return.take() {
                    return Ok(TailOutcome::Value(value));
                }
                for case in cases {
                    let mut bindings = Vec::new();
                    if !self.collect_pattern_bindings(&case.pattern, &value, &mut bindings)? {
                        continue;
                    }
                    let outcome = self.with_child_environment(|interpreter| {
                        for (name, value) in bindings {
                            Environment::define(&interpreter.environment, name, value, false)
                                .map_err(|message| {
                                    vec![interpreter.error(message, case.pattern.span)]
                                })?;
                        }
                        if let Some(guard) = &case.guard {
                            let guard = interpreter.evaluate_expression(guard)?;
                            if let Some(value) = interpreter.pending_return.take() {
                                return Ok(Some(TailOutcome::Value(value)));
                            }
                            if !interpreter.expect_logical(guard, case.span)? {
                                return Ok(None);
                            }
                        }
                        interpreter
                            .evaluate_tail_block(&case.value, function_name)
                            .map(Some)
                    })?;
                    if let Some(outcome) = outcome {
                        return Ok(outcome);
                    }
                }
                Err(vec![self.error(
                    "nenhum caso de escolha corresponde ao valor",
                    expression.span,
                )])
            }
            _ => {
                let value = self.evaluate_expression(expression)?;
                Ok(TailOutcome::Value(
                    self.pending_return.take().unwrap_or(value),
                ))
            }
        }
    }

    fn evaluate_tail_block(
        &mut self,
        block: &Block,
        function_name: &str,
    ) -> RuntimeResult<TailOutcome> {
        self.with_child_environment(|interpreter| {
            interpreter.predeclare(&block.statements)?;
            let last_index = block.statements.len().checked_sub(1);
            let mut last = Value::Nothing;
            for (index, statement) in block.statements.iter().enumerate() {
                if Some(index) == last_index {
                    match &statement.kind {
                        StmtKind::Expression(expression) => {
                            interpreter.begin_statement(statement.span)?;
                            return interpreter.evaluate_tail_expression(expression, function_name);
                        }
                        StmtKind::Return(Some(expression)) => {
                            interpreter.begin_statement(statement.span)?;
                            return interpreter.evaluate_tail_expression(expression, function_name);
                        }
                        _ => {}
                    }
                }
                match interpreter.execute_statement(statement)? {
                    Flow::Next(value) => last = value,
                    Flow::Return(value) => return Ok(TailOutcome::Value(value)),
                }
            }
            Ok(TailOutcome::Value(last))
        })
    }

    fn invoke_function(
        &mut self,
        declaration: &FunctionDecl,
        environment: EnvironmentRef,
        owner: Option<&str>,
        receiver: Option<Value>,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if self.call_depth >= self.options.max_call_depth {
            return Err(vec![
                self.error("limite de chamadas recursivas excedido", span),
            ]);
        }
        let outer_return = self.pending_return.take();
        self.call_depth += 1;
        let mut arguments = arguments;
        let result = loop {
            let call_environment = Environment::child(environment.clone());
            let setup = (|| {
                if let (Some(owner), Some(Value::Object(object))) = (owner, receiver.as_ref()) {
                    let Some(class) = self.classes.get(owner).cloned() else {
                        return Err(vec![self.error("tipo da função não foi encontrado", span)]);
                    };
                    for field in &class.fields {
                        Environment::define(
                            &call_environment,
                            field.name.clone(),
                            Value::FieldReference(object.clone(), field.name.clone()),
                            field.mutable,
                        )
                        .map_err(|message| vec![self.error(message, span)])?;
                    }
                    for method in &class.methods {
                        Environment::define(
                            &call_environment,
                            method.name.clone(),
                            Value::Function(Rc::new(Callable::Function {
                                declaration: method.clone(),
                                environment: environment.clone(),
                                owner: Some(owner.to_owned()),
                                receiver: receiver.clone(),
                            })),
                            false,
                        )
                        .map_err(|message| vec![self.error(message, span)])?;
                    }
                }
                for (parameter, value) in declaration.parameters.iter().zip(arguments.iter()) {
                    let value = self.coerce_to_node(value.clone(), parameter.id, parameter.span)?;
                    Environment::define(
                        &call_environment,
                        parameter.name.clone(),
                        value,
                        parameter.mutable,
                    )
                    .map_err(|message| vec![self.error(message, parameter.span)])?;
                }
                Ok(())
            })();
            if let Err(error) = setup {
                break Err(error);
            }
            let outcome =
                self.with_environment(call_environment, |interpreter| match &declaration.body {
                    FunctionBody::Expression(expression) => {
                        interpreter.evaluate_tail_expression(expression, &declaration.name)
                    }
                    FunctionBody::Block(block) => {
                        interpreter.evaluate_tail_block(block, &declaration.name)
                    }
                });
            match outcome {
                Ok(TailOutcome::Value(value)) => break Ok(value),
                Ok(TailOutcome::Call(next_arguments, tail_span)) => {
                    if next_arguments.len() != declaration.parameters.len() {
                        break Err(vec![
                            self.error("chamada recursiva incompatível", tail_span),
                        ]);
                    }
                    if let Err(error) = self.consume_step(tail_span) {
                        break Err(error);
                    }
                    arguments = next_arguments;
                }
                Err(error) => break Err(error),
            }
        };
        let result = self.pending_return.take().map_or(result, |value| Ok(value));
        self.pending_return = outer_return;
        self.call_depth -= 1;
        let return_type =
            self.type_info.node_types.get(&declaration.id).and_then(
                |value_type| match value_type {
                    ValueType::Function(_, result) => Some((**result).clone()),
                    _ => None,
                },
            );
        if let Some(return_type) = return_type {
            result.and_then(|value| self.coerce_value(value, &return_type, span))
        } else {
            result
        }
    }

    fn invoke_lambda(
        &mut self,
        id: NodeId,
        parameters: &[Parameter],
        body: &Expr,
        environment: EnvironmentRef,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if self.call_depth >= self.options.max_call_depth {
            return Err(vec![
                self.error("limite de chamadas recursivas excedido", span),
            ]);
        }
        let outer_return = self.pending_return.take();
        self.call_depth += 1;
        let call_environment = Environment::child(environment);
        let result = (|| {
            for (parameter, value) in parameters.iter().zip(arguments.iter()) {
                let value = self.coerce_to_node(value.clone(), parameter.id, parameter.span)?;
                Environment::define(
                    &call_environment,
                    parameter.name.clone(),
                    value,
                    parameter.mutable,
                )
                .map_err(|message| vec![self.error(message, parameter.span)])?;
            }
            self.with_environment(call_environment, |interpreter| {
                interpreter.evaluate_expression(body)
            })
        })();
        let result = self.pending_return.take().map_or(result, |value| Ok(value));
        self.pending_return = outer_return;
        self.call_depth -= 1;
        let return_type =
            self.type_info
                .node_types
                .get(&id)
                .and_then(|value_type| match value_type {
                    ValueType::Function(_, result) => Some((**result).clone()),
                    _ => None,
                });
        if let Some(return_type) = return_type {
            result.and_then(|value| self.coerce_value(value, &return_type, span))
        } else {
            result
        }
    }

    fn construct(
        &mut self,
        declaration: &TypeDecl,
        environment: EnvironmentRef,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let fields = declaration
            .fields
            .iter()
            .filter(|field| field.constructor_parameter)
            .collect::<Vec<_>>();
        if arguments.len() != fields.len() {
            return Err(vec![self.error(
                format!("{} espera {} argumentos", declaration.name, fields.len()),
                span,
            )]);
        }
        let object = Rc::new(RefCell::new(ObjectValue {
            class_name: declaration.name.clone(),
            fields: HashMap::new(),
        }));
        let instance_environment = Environment::child(environment);
        for field in &declaration.fields {
            Environment::define(
                &instance_environment,
                field.name.clone(),
                Value::FieldReference(object.clone(), field.name.clone()),
                field.mutable,
            )
            .map_err(|message| vec![self.error(message, field.span)])?;
        }
        for (field, value) in fields.into_iter().zip(arguments) {
            let value = self.coerce_to_node(value, field.id, field.span)?;
            object.borrow_mut().fields.insert(field.name.clone(), value);
        }
        self.with_environment(instance_environment, |interpreter| {
            for field in &declaration.fields {
                if let Some(default) = &field.default_value {
                    let value = interpreter.evaluate_expression(default)?;
                    let value = interpreter.coerce_to_node(value, field.id, field.span)?;
                    object.borrow_mut().fields.insert(field.name.clone(), value);
                }
            }
            Ok(Value::Object(object.clone()))
        })
    }
}

fn list_values(value: &Value) -> Option<Vec<Value>> {
    match value {
        Value::List(values) => Some(values.clone()),
        Value::MutableList(values) => Some(values.borrow().clone()),
        _ => None,
    }
}

fn broadcast_pattern_value(pattern: &Pattern, value: Value) -> Value {
    match &pattern.kind {
        PatternKind::Tuple(patterns) if !matches!(&value, Value::Tuple(_)) => Value::Tuple(
            patterns
                .iter()
                .map(|pattern| broadcast_pattern_value(pattern, value.clone()))
                .collect(),
        ),
        _ => value,
    }
}

fn normalize_index(index: i64, length: usize) -> Option<usize> {
    let length = i64::try_from(length).ok()?;
    let index = if index > 0 {
        index.checked_sub(1)?
    } else if index < 0 {
        length.checked_add(index)?
    } else {
        return None;
    };
    usize::try_from(index)
        .ok()
        .filter(|index| *index < length as usize)
}

fn tuple_member_index(name: &str) -> Option<usize> {
    match name {
        "primeiro" => Some(0),
        "segundo" => Some(1),
        "terceiro" => Some(2),
        "quarto" => Some(3),
        "quinto" => Some(4),
        "sexto" => Some(5),
        "sétimo" | "setimo" => Some(6),
        "oitavo" => Some(7),
        "nono" => Some(8),
        "décimo" | "decimo" => Some(9),
        _ => None,
    }
}

fn numeric_f64(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(value) => Some(*value as f64),
        Value::Real(value) => Some(*value),
        Value::BigInteger(value) => value.parse().ok(),
        Value::Character(value) => Some(u32::from(*value) as f64),
        _ => None,
    }
}

fn numericize_character(value: Value) -> Value {
    match value {
        Value::Character(character) => Value::Integer(i64::from(u32::from(character))),
        value => value,
    }
}

fn runtime_needs_coercion(value: &Value, expected: &ValueType) -> bool {
    match (value, expected) {
        (Value::Character(_), ValueType::Integer | ValueType::Real | ValueType::Text)
        | (Value::Integer(_), ValueType::Real | ValueType::Text)
        | (Value::Real(_) | Value::BigInteger(_) | Value::Logical(_), ValueType::Text) => true,
        (Value::Tuple(values), ValueType::Tuple(expected)) if values.len() == expected.len() => {
            values
                .iter()
                .zip(expected)
                .any(|(value, expected)| runtime_needs_coercion(value, expected))
        }
        (Value::List(values), ValueType::List(expected)) => values
            .iter()
            .any(|value| runtime_needs_coercion(value, expected)),
        (Value::MutableList(values), ValueType::MutableList(expected)) => values
            .borrow()
            .iter()
            .any(|value| runtime_needs_coercion(value, expected)),
        _ => false,
    }
}

fn runtime_type_name(value: &Value) -> String {
    match value {
        Value::Integer(_) => "Inteiro".to_owned(),
        Value::Real(_) => "Real".to_owned(),
        Value::BigInteger(_) => "BigInt".to_owned(),
        Value::Text(_) => "Texto".to_owned(),
        Value::Character(_) => "Caractere".to_owned(),
        Value::Logical(_) => "Lógico".to_owned(),
        Value::Nothing => "Nada".to_owned(),
        Value::Tuple(_) => "Tupla".to_owned(),
        Value::List(_) | Value::MutableList(_) => "Lista".to_owned(),
        Value::Function(_) => "Função".to_owned(),
        Value::TypeName(name) => name.clone(),
        Value::Object(object) => object.borrow().class_name.clone(),
        Value::FieldReference(_, _) => "Atributo".to_owned(),
    }
}

fn parse_integer_prefix(source: &str) -> i64 {
    let source = source.trim_start();
    let bytes = source.as_bytes();
    let mut end = if bytes
        .first()
        .is_some_and(|byte| matches!(byte, b'+' | b'-'))
    {
        1
    } else {
        0
    };
    let digits_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if end == digits_start {
        0
    } else {
        source[..end].parse().unwrap_or(0)
    }
}

fn parse_real_prefix(source: &str) -> f64 {
    let source = source.trim_start();
    let bytes = source.as_bytes();
    let mut end = 0;
    if bytes
        .first()
        .is_some_and(|byte| matches!(byte, b'+' | b'-'))
    {
        end = 1;
    }
    let integer_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    let mut has_digits = end > integer_start;
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        let fraction_start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        has_digits |= end > fraction_start;
    }
    if !has_digits {
        return 0.0;
    }
    let mantissa_end = end;
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let exponent_start = end;
        end += 1;
        if matches!(bytes.get(end), Some(b'+' | b'-')) {
            end += 1;
        }
        let digits_start = end;
        while bytes.get(end).is_some_and(u8::is_ascii_digit) {
            end += 1;
        }
        if end == digits_start {
            end = mantissa_end;
        } else if end == exponent_start {
            end = mantissa_end;
        }
    }
    source[..end].parse().unwrap_or(0.0)
}

fn normalize_big_integer(value: &str) -> Option<String> {
    bigint::normalize(value)
}

fn bigint_repr(value: &Value) -> Option<String> {
    match value {
        Value::Integer(value) => Some(value.to_string()),
        Value::BigInteger(value) => Some(value.clone()),
        _ => None,
    }
}

fn compare_values(left: &Value, right: &Value) -> Option<std::cmp::Ordering> {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => Some(left.cmp(right)),
        (Value::Real(left), Value::Real(right)) => left.partial_cmp(right),
        (Value::Integer(left), Value::Real(right)) => (*left as f64).partial_cmp(right),
        (Value::Real(left), Value::Integer(right)) => left.partial_cmp(&(*right as f64)),
        (Value::Character(left), Value::Integer(right)) => {
            Some(i64::from(u32::from(*left)).cmp(right))
        }
        (Value::Integer(left), Value::Character(right)) => {
            Some(left.cmp(&i64::from(u32::from(*right))))
        }
        (Value::Character(left), Value::Real(right)) => {
            (u32::from(*left) as f64).partial_cmp(right)
        }
        (Value::Real(left), Value::Character(right)) => {
            left.partial_cmp(&(u32::from(*right) as f64))
        }
        (Value::Logical(left), Value::Logical(right)) => Some(left.cmp(right)),
        (Value::BigInteger(left), Value::BigInteger(right)) => Some(bigint::compare(left, right)),
        (Value::Integer(left), Value::BigInteger(right)) => {
            Some(bigint::compare(&left.to_string(), right))
        }
        (Value::BigInteger(left), Value::Integer(right)) => {
            Some(bigint::compare(left, &right.to_string()))
        }
        (Value::Character(left), Value::Character(right)) => Some(left.cmp(right)),
        (Value::Text(left), Value::Text(right)) => Some(left.cmp(right)),
        (Value::Tuple(left), Value::Tuple(right)) | (Value::List(left), Value::List(right)) => {
            compare_slices(left, right)
        }
        (Value::MutableList(left), Value::MutableList(right)) => {
            compare_slices(&left.borrow(), &right.borrow())
        }
        (Value::List(left), Value::MutableList(right)) => compare_slices(left, &right.borrow()),
        (Value::MutableList(left), Value::List(right)) => compare_slices(&left.borrow(), right),
        _ => None,
    }
}

fn compare_slices(left: &[Value], right: &[Value]) -> Option<std::cmp::Ordering> {
    for (left, right) in left.iter().zip(right) {
        let order = compare_values(left, right)?;
        if order != std::cmp::Ordering::Equal {
            return Some(order);
        }
    }
    Some(left.len().cmp(&right.len()))
}

fn make_nested_list(sizes: &[usize], initial: Value) -> Vec<Value> {
    let Some((&size, remainder)) = sizes.split_first() else {
        return Vec::new();
    };
    if remainder.is_empty() {
        vec![initial; size]
    } else {
        (0..size)
            .map(|_| Value::List(make_nested_list(remainder, initial.clone())))
            .collect()
    }
}

fn format_value(value: &Value, format: &str) -> Option<String> {
    let start = format.find('%')?;
    let (prefix, rest) = format.split_at(start);
    let conversion_index = rest
        .char_indices()
        .skip(1)
        .find(|(_, character)| matches!(character, 'd' | 'i' | 'f' | 's'))?
        .0;
    let conversion = rest[conversion_index..].chars().next()?;
    let suffix = &rest[conversion_index + conversion.len_utf8()..];
    let specification = &rest[1..conversion_index];
    let left_aligned = specification.contains('-');
    let zero_padded = specification.contains('0') && !left_aligned;
    let precision = specification.split_once('.').and_then(|(_, precision)| {
        precision
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
            .parse::<usize>()
            .ok()
    });
    let width = specification
        .split_once('.')
        .map_or(specification, |(width, _)| width)
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse::<usize>()
        .unwrap_or(0);
    let raw = match conversion {
        'd' | 'i' => match value {
            Value::Integer(value) => value.to_string(),
            Value::BigInteger(value) => value.clone(),
            Value::Real(value) => (*value as i64).to_string(),
            _ => return None,
        },
        'f' => {
            let number = numeric_f64(value)?;
            match precision {
                Some(precision) => format!("{number:.precision$}"),
                None => number.to_string(),
            }
        }
        's' => value.display(),
        _ => return None,
    };
    let padding = width.saturating_sub(raw.chars().count());
    let padded = if left_aligned {
        format!("{raw}{}", " ".repeat(padding))
    } else if zero_padded && conversion != 's' {
        if raw.starts_with('-') {
            format!("-{}{}", "0".repeat(padding), &raw[1..])
        } else {
            format!("{}{}", "0".repeat(padding), raw)
        }
    } else {
        format!("{}{raw}", " ".repeat(padding))
    };
    Some(format!("{prefix}{padded}{suffix}"))
}

fn count_placeholders(expression: &Expr) -> usize {
    match &expression.kind {
        ExprKind::Placeholder => 1,
        ExprKind::Text(parts) => parts
            .iter()
            .map(|part| match part {
                StringPart::Text(_) => 0,
                StringPart::Expression(expression) => count_placeholders(expression),
            })
            .sum(),
        ExprKind::Tuple(elements) | ExprKind::List(elements) => {
            elements.iter().map(count_placeholders).sum()
        }
        ExprKind::Unary { operand, .. } => count_placeholders(operand),
        ExprKind::Binary { left, right, .. } => {
            count_placeholders(left) + count_placeholders(right)
        }
        ExprKind::Call { callee, arguments } => {
            count_placeholders(callee) + arguments.iter().map(count_placeholders).sum::<usize>()
        }
        ExprKind::Member { receiver, .. } => count_placeholders(receiver),
        ExprKind::Index { receiver, indices } => {
            count_placeholders(receiver) + indices.iter().map(count_placeholders).sum::<usize>()
        }
        ExprKind::Lambda { body, .. } => count_placeholders(body),
        ExprKind::Conditional {
            condition,
            then_branch,
            else_branch,
        } => {
            count_placeholders(condition)
                + count_block_placeholders(then_branch)
                + else_branch.as_ref().map_or(0, count_block_placeholders)
        }
        ExprKind::Match { value, cases } => {
            count_placeholders(value)
                + cases
                    .iter()
                    .map(|case| {
                        case.guard.as_ref().map_or(0, count_placeholders)
                            + count_block_placeholders(&case.value)
                    })
                    .sum::<usize>()
        }
        ExprKind::Comprehension { generators, value } => {
            generators
                .iter()
                .map(count_generator_placeholders)
                .sum::<usize>()
                + count_placeholders(value)
        }
        _ => 0,
    }
}

fn count_block_placeholders(block: &Block) -> usize {
    block
        .statements
        .iter()
        .map(|statement| match &statement.kind {
            StmtKind::Binding { value, .. }
            | StmtKind::Output { value, .. }
            | StmtKind::Expression(value) => count_placeholders(value),
            StmtKind::Assignment { target, value } => {
                count_placeholders(target) + count_placeholders(value)
            }
            StmtKind::Return(value) => value.as_ref().map_or(0, count_placeholders),
            StmtKind::While { condition, body } => {
                count_placeholders(condition) + count_block_placeholders(body)
            }
            StmtKind::For { generators, body } => {
                generators
                    .iter()
                    .map(count_generator_placeholders)
                    .sum::<usize>()
                    + count_block_placeholders(body)
            }
            StmtKind::Function(function) => match &function.body {
                FunctionBody::Expression(expression) => count_placeholders(expression),
                FunctionBody::Block(block) => count_block_placeholders(block),
            },
            StmtKind::Type(declaration) => declaration
                .fields
                .iter()
                .filter_map(|field| field.default_value.as_ref())
                .map(count_placeholders)
                .sum(),
        })
        .sum()
}

fn count_generator_placeholders(generator: &ForGenerator) -> usize {
    match &generator.source {
        ForSource::Range { start, end, step } => {
            count_placeholders(start)
                + count_placeholders(end)
                + step.as_ref().map_or(0, count_placeholders)
        }
        ForSource::Iterable(expression) => count_placeholders(expression),
    }
}

fn input_data_type(name: &str) -> Option<DataType> {
    match name {
        "leia_inteiro" => Some(DataType::Integer),
        "leia_real" => Some(DataType::Real),
        "leia_texto" => Some(DataType::Text),
        "leia_caractere" => Some(DataType::Character),
        "leia_logico" => Some(DataType::Logical),
        _ => None,
    }
}

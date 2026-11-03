use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use fancy_regex::RegexBuilder;

use crate::ast::{
    Block, Expr, ExprKind, ForGenerator, ForSource, FunctionBody, FunctionDecl, MatchCase, NodeId,
    Parameter, Pattern, PatternKind, Program, Stmt, StmtKind, StringPart, TypeDecl,
};
use crate::bigint;
use crate::environment::{Binding, Environment, EnvironmentRef};
use crate::jerimum;
use crate::value::{Callable, JerimumObject, ObjectValue, Value};
use crate::{
    ControlAction, DataType, Diagnostic, DiagnosticKind, ExecutionEvent, ExecutionOptions,
    InputPrompt, JerimumRequest, JerimumValue, PauseReason, RuntimeIo, SourceSpan, TypeInfo,
    ValueType, VariableSnapshot, check, parse_with_options,
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

#[derive(Clone)]
struct JerimumDrawCommand {
    color: Option<Value>,
    function: String,
    arguments: Vec<JerimumValue>,
}

#[derive(Clone)]
enum JerimumDrawItem {
    Command(JerimumDrawCommand),
    Callback(Value),
}

struct Interpreter<'a> {
    io: &'a mut dyn RuntimeIo,
    options: ExecutionOptions,
    environment: EnvironmentRef,
    classes: HashMap<String, TypeDecl>,
    class_environments: HashMap<String, EnvironmentRef>,
    steps: u64,
    allocated_elements: usize,
    allocated_collection_bytes: usize,
    pending_input_tokens: VecDeque<String>,
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
    jerimum_enabled: bool,
    jerimum_started: std::time::Instant,
    jerimum_game_title: String,
    jerimum_game_width: i32,
    jerimum_game_height: i32,
    jerimum_game_fps: i32,
    jerimum_next_screen_id: u64,
    jerimum_drawings: BTreeMap<i32, Vec<JerimumDrawItem>>,
    jerimum_image_cache: HashMap<String, Value>,
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
            allocated_collection_bytes: 0,
            pending_input_tokens: VecDeque::new(),
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
            jerimum_enabled: program.statements.iter().any(
                |statement| matches!(&statement.kind, StmtKind::Import(path) if path == "jerimum"),
            ),
            jerimum_started: std::time::Instant::now(),
            jerimum_game_title: "Sem Nome".to_owned(),
            jerimum_game_width: 640,
            jerimum_game_height: 480,
            jerimum_game_fps: 60,
            jerimum_next_screen_id: 1,
            jerimum_drawings: BTreeMap::new(),
            jerimum_image_cache: HashMap::new(),
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
            StmtKind::Function(_)
            | StmtKind::Type(_)
            | StmtKind::TypeAlias { .. }
            | StmtKind::Import(_) => Flow::Next(Value::Nothing),
            StmtKind::Block(block) => match self.execute_block(block)? {
                Flow::Next(value) => Flow::Next(value),
                returned @ Flow::Return(_) => return Ok(returned),
            },
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
                        let values = values.borrow();
                        self.copy_value_vec(&values, expression.span)
                    }
                    Value::Tuple(values) => Ok(values),
                    Value::Text(value) => {
                        let units = utf16_units(&value);
                        self.reserve_elements(units.len(), expression.span)?;
                        Ok(units.into_iter().map(Value::Character).collect())
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
                let start_value = self.expect_integer(&start_expression, start.span)?;
                let end_expression = self.evaluate_expression(end)?;
                if self.pending_return.is_some() {
                    return Ok(Vec::new());
                }
                let end_value = self.expect_integer(&end_expression, end.span)?;
                let step_value = if let Some(step) = step {
                    let step_expression = self.evaluate_expression(step)?;
                    if self.pending_return.is_some() {
                        return Ok(Vec::new());
                    }
                    self.expect_integer(&step_expression, step.span)?
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
        if let (PatternKind::Tuple(patterns), ExprKind::Call { callee, arguments }) =
            (&pattern.kind, &expression.kind)
            && let ExprKind::Identifier(name) = &callee.kind
            && let Some(data_type) = input_list_data_type(name)
        {
            let lists = patterns
                .iter()
                .map(|_| self.read_input_list(arguments, data_type, expression.span))
                .collect::<RuntimeResult<Vec<_>>>()?;
            self.reserve_elements(lists.len(), expression.span)?;
            return Ok(Value::Tuple(lists));
        }
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
        let value = self.broadcast_pattern_value(pattern, value)?;
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

    fn broadcast_pattern_value(&mut self, pattern: &Pattern, value: Value) -> RuntimeResult<Value> {
        if let PatternKind::Tuple(patterns) = &pattern.kind
            && !matches!(&value, Value::Tuple(_))
        {
            self.reserve_elements(patterns.len(), pattern.span)?;
            let values = patterns
                .iter()
                .map(|pattern| {
                    let copy = self.copy_value(&value, pattern.span)?;
                    self.broadcast_pattern_value(pattern, copy)
                })
                .collect::<RuntimeResult<Vec<_>>>()?;
            Ok(Value::Tuple(values))
        } else {
            Ok(value)
        }
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
                Ok(Value::Integer(i32::from(character)))
            }
            (Value::Character(character), ValueType::Real) => Ok(Value::Real(f64::from(character))),
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
                let values = values.borrow();
                let values = self.copy_value_vec(&values, span)?;
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
        &mut self,
        pattern: &Pattern,
        value: &Value,
        bindings: &mut Vec<(String, Value)>,
    ) -> RuntimeResult<bool> {
        let matches = match (&pattern.kind, value) {
            (PatternKind::Wildcard, _) => true,
            (PatternKind::Binding(name), _) => {
                bindings.push((name.clone(), self.copy_value(value, pattern.span)?));
                true
            }
            (PatternKind::Integer(expected), Value::Integer(actual)) => expected == actual,
            (PatternKind::Real(expected), Value::Real(actual)) => expected == actual,
            (PatternKind::Character(expected), Value::Character(actual)) => {
                u16::try_from(u32::from(*expected)).ok() == Some(*actual)
            }
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
                if !self.collect_pattern_bindings(head, &values[0], bindings)? {
                    false
                } else {
                    let tail_values = &values[1..];
                    self.reserve_elements(tail_values.len(), pattern.span)?;
                    self.reserve_cloned_values_payload(tail_values, pattern.span)?;
                    let tail_value = Value::List(tail_values.to_vec());
                    self.collect_pattern_bindings(tail, &tail_value, bindings)?
                }
            }
            (PatternKind::Cons(head, tail), Value::MutableList(values)) => {
                let values = values.borrow();
                if values.is_empty() {
                    false
                } else if !self.collect_pattern_bindings(head, &values[0], bindings)? {
                    false
                } else {
                    let tail_values = &values[1..];
                    self.reserve_elements(tail_values.len(), pattern.span)?;
                    self.reserve_cloned_values_payload(tail_values, pattern.span)?;
                    let tail_value = Value::List(tail_values.to_vec());
                    self.collect_pattern_bindings(tail, &tail_value, bindings)?
                }
            }
            (PatternKind::Constructor { name, fields }, Value::Object(object)) => {
                let object = object.borrow();
                if &object.class_name != name {
                    false
                } else if let Some(field_names) = self.classes.get(name).map(|declaration| {
                    declaration
                        .fields
                        .iter()
                        .filter(|field| field.constructor_parameter)
                        .map(|field| field.name.clone())
                        .collect::<Vec<_>>()
                }) {
                    if fields.len() != field_names.len() {
                        false
                    } else {
                        for (pattern, field_name) in fields.iter().zip(field_names) {
                            let Some(value) = object.fields.get(&field_name) else {
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
                let Some(binding) = self.lookup_assignment_target(name, target.span)? else {
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
                let receiver_value = self.evaluate_expression(receiver)?;
                if let Value::TypeName(owner) = &receiver_value
                    && owner == "JerimumModule:Jogo"
                {
                    match (name.as_str(), value) {
                        ("titulo", Value::Text(title)) => self.jerimum_game_title = title,
                        ("largura", Value::Integer(width)) => self.jerimum_game_width = width,
                        ("altura", Value::Integer(height)) => self.jerimum_game_height = height,
                        ("fps", Value::Integer(fps)) => self.jerimum_game_fps = fps,
                        _ => {
                            return Err(vec![self.error(
                                format!("atributo '{name}' não pode receber esse valor"),
                                target.span,
                            )]);
                        }
                    }
                    return Ok(());
                }
                let Value::Object(object) = receiver_value else {
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
                let index = self.expect_integer(&index_value, indices[0].span)?;
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
        let environment = self.environment.clone();
        let values =
            Environment::visible_values_with(&environment, |value| self.copy_value(value, span))?;
        let variables = values
            .into_iter()
            .map(|(name, value)| {
                Ok(VariableSnapshot {
                    name,
                    value: self.resolve_field_reference(value, span)?.display(),
                })
            })
            .collect::<RuntimeResult<Vec<_>>>()?;
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
        let elements = self
            .allocated_elements
            .checked_add(count)
            .ok_or_else(|| vec![self.error("limite de elementos excedido", span)])?;
        if elements > self.options.max_collection_elements {
            return Err(vec![self.error("limite de elementos excedido", span)]);
        }
        let bytes = count
            .checked_mul(std::mem::size_of::<Value>())
            .ok_or_else(|| vec![self.error("limite de memória de coleções excedido", span)])?;
        self.reserve_collection_bytes(bytes, span)?;
        self.allocated_elements = elements;
        Ok(())
    }

    fn reserve_collection_bytes(&mut self, bytes: usize, span: SourceSpan) -> RuntimeResult<()> {
        let allocated = self
            .allocated_collection_bytes
            .checked_add(bytes)
            .ok_or_else(|| vec![self.error("limite de memória de coleções excedido", span)])?;
        if allocated > self.options.max_collection_bytes {
            return Err(vec![
                self.error("limite de memória de coleções excedido", span),
            ]);
        }
        self.allocated_collection_bytes = allocated;
        Ok(())
    }

    fn reserve_value_payload(&mut self, value: &Value, span: SourceSpan) -> RuntimeResult<()> {
        let bytes = value_direct_payload_bytes(value);
        self.reserve_collection_bytes(bytes, span)
    }

    fn reserve_values_payload(&mut self, values: &[Value], span: SourceSpan) -> RuntimeResult<()> {
        let bytes = values.iter().try_fold(0usize, |total, value| {
            total.checked_add(value_direct_payload_bytes(value))
        });
        let bytes = bytes
            .ok_or_else(|| vec![self.error("limite de memória de coleções excedido", span)])?;
        self.reserve_collection_bytes(bytes, span)
    }

    fn reserve_cloned_value_payload(
        &mut self,
        value: &Value,
        copies: usize,
        span: SourceSpan,
    ) -> RuntimeResult<()> {
        let bytes = value_clone_bytes(value)
            .and_then(|bytes| bytes.checked_mul(copies))
            .ok_or_else(|| vec![self.error("limite de memória de coleções excedido", span)])?;
        let elements = value_clone_elements(value)
            .and_then(|elements| elements.checked_mul(copies))
            .ok_or_else(|| vec![self.error("limite de elementos excedido", span)])?;
        self.reserve_elements(elements, span)?;
        self.reserve_collection_bytes(bytes, span)
    }

    fn reserve_cloned_values_payload(
        &mut self,
        values: &[Value],
        span: SourceSpan,
    ) -> RuntimeResult<()> {
        let bytes = values.iter().try_fold(0usize, |total, value| {
            total.checked_add(value_clone_bytes(value)?)
        });
        let bytes = bytes
            .ok_or_else(|| vec![self.error("limite de memória de coleções excedido", span)])?;
        let elements = values.iter().try_fold(0usize, |total, value| {
            total.checked_add(value_clone_elements(value)?)
        });
        let elements =
            elements.ok_or_else(|| vec![self.error("limite de elementos excedido", span)])?;
        self.reserve_elements(elements, span)?;
        self.reserve_collection_bytes(bytes, span)
    }

    fn reserve_value_vec_copy(&mut self, values: &[Value], span: SourceSpan) -> RuntimeResult<()> {
        self.reserve_elements(values.len(), span)?;
        self.reserve_cloned_values_payload(values, span)
    }

    fn copy_value(&mut self, value: &Value, span: SourceSpan) -> RuntimeResult<Value> {
        self.reserve_cloned_value_payload(value, 1, span)?;
        Ok(value.clone())
    }

    fn copy_value_vec(&mut self, values: &[Value], span: SourceSpan) -> RuntimeResult<Vec<Value>> {
        self.reserve_value_vec_copy(values, span)?;
        Ok(values.to_vec())
    }

    fn lookup_binding(&mut self, name: &str, span: SourceSpan) -> RuntimeResult<Option<Binding>> {
        let environment = self.environment.clone();
        Environment::lookup_with(&environment, name, |binding| {
            self.reserve_cloned_value_payload(&binding.value, 1, span)?;
            Ok(binding.clone())
        })
    }

    fn lookup_assignment_target(
        &mut self,
        name: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Option<Binding>> {
        let environment = self.environment.clone();
        Environment::lookup_with(&environment, name, |binding| {
            let value = if matches!(binding.value, Value::FieldReference(_, _)) {
                self.reserve_cloned_value_payload(&binding.value, 1, span)?;
                binding.value.clone()
            } else {
                Value::Nothing
            };
            Ok(Binding {
                value,
                mutable: binding.mutable,
            })
        })
    }

    fn copy_list_values(&mut self, value: &Value, span: SourceSpan) -> RuntimeResult<Vec<Value>> {
        match value {
            Value::List(values) => self.copy_value_vec(values, span),
            Value::MutableList(values) => {
                let values = values.borrow();
                self.copy_value_vec(&values, span)
            }
            _ => Ok(Vec::new()),
        }
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
        if matches!(data_type, DataType::Integer | DataType::Real) {
            let token = self.read_input_token(data_type, span)?;
            return self.parse_input_value(data_type, &token, span);
        }
        let line = self.read_line(data_type, span)?;
        self.parse_input_value(data_type, &line, span)
    }

    fn read_input_token(&mut self, data_type: DataType, span: SourceSpan) -> RuntimeResult<String> {
        while self.pending_input_tokens.is_empty() {
            let line = self.read_line(data_type, span)?;
            self.pending_input_tokens
                .extend(line.split_whitespace().map(str::to_owned));
        }
        Ok(self.pending_input_tokens.pop_front().unwrap())
    }

    fn read_line(&mut self, data_type: DataType, span: SourceSpan) -> RuntimeResult<String> {
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
        let line = line.trim_end_matches(['\r', '\n']).to_owned();
        self.check_string_size(line.len(), span)?;
        Ok(line)
    }

    fn expect_integer(&self, value: &Value, span: SourceSpan) -> RuntimeResult<i32> {
        if let Value::Integer(value) = value {
            Ok(*value)
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

    fn resolve_field_reference(&mut self, value: Value, span: SourceSpan) -> RuntimeResult<Value> {
        if let Value::FieldReference(object, field) = value {
            let object = object.borrow();
            if let Some(value) = object.fields.get(&field) {
                self.copy_value(value, span)
            } else {
                Ok(Value::Nothing)
            }
        } else {
            Ok(value)
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
            ExprKind::Character(value) => u16::try_from(u32::from(*value))
                .map(Value::Character)
                .map_err(|_| {
                    vec![self.error("Caractere fora do intervalo UTF-16", expression.span)]
                }),
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
            ExprKind::This => self.identifier_value("\0isto", expression.span),
            ExprKind::Placeholder => {
                let Some(values) = self.placeholder_values.last() else {
                    return Err(vec![self.error(
                        "'_' só pode aparecer como parâmetro de uma função de ordem superior",
                        expression.span,
                    )]);
                };
                let index = *self.placeholder_indexes.last().unwrap();
                if values.get(index).is_none() {
                    return Err(vec![
                        self.error("faltou um argumento para '_'", expression.span),
                    ]);
                }
                let value = std::mem::replace(
                    &mut self.placeholder_values.last_mut().unwrap()[index],
                    Value::Nothing,
                );
                *self.placeholder_indexes.last_mut().unwrap() += 1;
                Ok(value)
            }
            ExprKind::Tuple(elements) => {
                self.reserve_elements(elements.len(), expression.span)?;
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    let value = self.evaluate_expression(element)?;
                    self.reserve_value_payload(&value, expression.span)?;
                    values.push(value);
                }
                Ok(Value::Tuple(values))
            }
            ExprKind::List(elements) => {
                self.reserve_elements(elements.len(), expression.span)?;
                let mut values = Vec::with_capacity(elements.len());
                for element in elements {
                    let value = self.evaluate_expression(element)?;
                    self.reserve_value_payload(&value, expression.span)?;
                    values.push(value);
                }
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
                let index = self.expect_integer(&index_value, indices[0].span)?;
                self.index_value(receiver, index, expression.span)
            }
            ExprKind::Lambda { parameters, body } => {
                Ok(Value::Function(Rc::new(Callable::Lambda {
                    id: expression.id,
                    parameters: parameters.clone(),
                    body: body.clone(),
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
            ExprKind::Comprehension { generators, body } => {
                let mut values = Vec::new();
                self.evaluate_comprehension(generators, 0, body, &mut values)?;
                Ok(Value::List(values))
            }
            ExprKind::Block(block) => match self.execute_block(block)? {
                Flow::Next(value) => Ok(value),
                Flow::Return(value) => {
                    self.pending_return = Some(value);
                    Ok(Value::Nothing)
                }
            },
            ExprKind::While { condition, body } => {
                loop {
                    self.consume_step(expression.span)?;
                    let condition = self.evaluate_expression(condition)?;
                    if self.pending_return.is_some() {
                        return Ok(Value::Nothing);
                    }
                    if !self.expect_logical(condition, expression.span)? {
                        break;
                    }
                    if let Flow::Return(value) = self.execute_block(body)? {
                        self.pending_return = Some(value);
                        return Ok(Value::Nothing);
                    }
                }
                Ok(Value::Nothing)
            }
            ExprKind::For { generators, body } => {
                if let Flow::Return(value) = self.execute_generators(generators, 0, body)? {
                    self.pending_return = Some(value);
                }
                Ok(Value::Nothing)
            }
        }
    }

    fn identifier_value(&mut self, name: &str, span: SourceSpan) -> RuntimeResult<Value> {
        if let Some(binding) = self.lookup_binding(name, span)? {
            return self.resolve_field_reference(binding.value, span);
        }
        if self.jerimum_enabled
            && let Some(imported) = jerimum::imported_name(name)
        {
            return Ok(Value::TypeName(imported));
        }
        if let Some(data_type) = input_data_type(name) {
            return self.read_scalar(data_type, span);
        }
        let lower = name.to_lowercase();
        match lower.as_str() {
            "eof" => return Ok(Value::Logical(self.eof || self.io.is_eof())),
            "pi" => return Ok(Value::Real(std::f64::consts::PI)),
            "abs" | "sqrt" | "sen" | "cos" | "tan" | "tg" | "arcsen" | "arccos" | "arctg"
            | "log" | "log10" => {
                return Ok(Value::Function(Rc::new(Callable::Builtin(lower))));
            }
            _ => {}
        }
        match name {
            "Lista" | "Matriz" | "Cubo" => return Ok(Value::TypeName(name.to_owned())),
            _ => {}
        }
        Err(vec![self.error(
            format!("identificador '{name}' não foi declarado"),
            span,
        )])
    }

    fn evaluate_call(
        &mut self,
        callee: &Expr,
        arguments: &[Expr],
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if let ExprKind::Identifier(name) = &callee.kind {
            if self.lookup_binding(name, span)?.is_none()
                && let Some(value) = self.call_builtin(name, arguments, span)?
            {
                return Ok(value);
            }
        }
        if let ExprKind::Member { receiver, name } = &callee.kind {
            let receiver = self.evaluate_expression(receiver)?;
            if self.pending_return.is_some() {
                return Ok(Value::Nothing);
            }
            if let Value::TypeName(type_name) = &receiver {
                if type_name.starts_with("Jerimum") {
                    let values = arguments
                        .iter()
                        .map(|argument| self.evaluate_argument(argument))
                        .collect::<RuntimeResult<Vec<_>>>()?;
                    if self.pending_return.is_some() {
                        return Ok(Value::Nothing);
                    }
                    return self.call_member(receiver, name, values, span);
                }
                return self.call_type_factory_expressions(type_name, name, arguments, span);
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
        if self.jerimum_enabled
            && let Some(value) = self.call_jerimum_constructor(name, arguments, span)?
        {
            return Ok(Some(value));
        }
        let value = match name {
            "Lista" | "lista" => {
                let dimensions = arguments.len().saturating_sub(1);
                if dimensions == 0 {
                    return Err(vec![
                        self.error("Lista recebe dimensões e valor inicial", span),
                    ]);
                }
                let mut sizes = Vec::with_capacity(dimensions);
                for argument in arguments.iter().take(dimensions) {
                    let value = self.evaluate_expression(argument)?;
                    let size = self.expect_integer(&value, argument.span)?.max(0) as usize;
                    sizes.push(size);
                }
                let (_, storage_elements) = nested_list_counts(&sizes)
                    .ok_or_else(|| vec![self.error("dimensões excedem o limite", span)])?;
                self.reserve_elements(storage_elements, span)?;
                Value::List(self.evaluate_nested_list(&sizes, arguments.last().unwrap(), span)?)
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
            "leia_reais" | "leia_numeros" | "leia_números" => {
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
                        self.copy_value(&values[index], span)?
                    }
                    [Value::MutableList(values)] => {
                        let values = values.borrow();
                        let Some(index) = self.random_index(values.len()) else {
                            return Err(vec![self.error("aleatório não aceita lista vazia", span)]);
                        };
                        self.copy_value(&values[index], span)?
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
            "abs" | "sqrt" | "sen" | "cos" | "tan" | "tg" | "arcsen" | "arccos" | "arctg"
            | "log" | "log10" => {
                if arguments.len() != 1 {
                    return Err(vec![
                        self.error(format!("{name} recebe um argumento"), span),
                    ]);
                }
                let value = self.evaluate_expression(&arguments[0])?;
                self.evaluate_math_builtin(name, value, arguments[0].span)?
            }
            "raiz" => {
                if !matches!(arguments.len(), 1 | 2) {
                    return Err(vec![self.error("raiz recebe um ou dois argumentos", span)]);
                }
                let value = self.evaluate_expression(&arguments[0])?;
                let number = numeric_f64(&value)
                    .ok_or_else(|| vec![self.error("esperava um número", arguments[0].span)])?;
                let degree = if let Some(argument) = arguments.get(1) {
                    let degree = self.evaluate_expression(argument)?;
                    numeric_f64(&degree)
                        .ok_or_else(|| vec![self.error("esperava um número", argument.span)])?
                } else {
                    2.0
                };
                Value::Real(number.powf(1.0 / degree))
            }
            _ => return Ok(None),
        };
        Ok(Some(value))
    }

    fn evaluate_math_builtin(
        &self,
        name: &str,
        value: Value,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if name == "abs" {
            return match value {
                Value::Integer(value) => value
                    .checked_abs()
                    .map(Value::Integer)
                    .ok_or_else(|| vec![self.error("estouro aritmético", span)]),
                Value::Real(value) => Ok(Value::Real(value.abs())),
                Value::BigInteger(value) => Ok(Value::BigInteger(
                    value.strip_prefix('-').unwrap_or(&value).to_owned(),
                )),
                value => numeric_f64(&value)
                    .map(|value| Value::Real(value.abs()))
                    .ok_or_else(|| vec![self.error("esperava um número", span)]),
            };
        }
        let number =
            numeric_f64(&value).ok_or_else(|| vec![self.error("esperava um número", span)])?;
        let result = match name {
            "sqrt" => number.sqrt(),
            "sen" => number.sin(),
            "cos" => number.cos(),
            "tan" | "tg" => number.tan(),
            "arcsen" => number.asin(),
            "arccos" => number.acos(),
            "arctg" => number.atan(),
            "log" => number.ln(),
            "log10" => number.log10(),
            _ => return Err(vec![self.error("função matemática desconhecida", span)]),
        };
        Ok(Value::Real(result))
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
                    let value = self.read_scalar(data_type, span)?;
                    self.reserve_value_payload(&value, span)?;
                    values.push(value);
                }
                if count > 0 {
                    self.pending_input_tokens.clear();
                }
                values
            }
            Some(Value::Text(separator)) => {
                let line = self.read_line(DataType::Text, span)?;
                let fields = if separator.is_empty() {
                    utf16_units(&line)
                        .into_iter()
                        .map(|unit| text_from_utf16(&[unit]))
                        .collect::<Vec<_>>()
                } else {
                    line.split(|character| separator.contains(character))
                        .map(str::to_owned)
                        .collect::<Vec<_>>()
                };
                let count = fields.len();
                self.reserve_elements(count, span)?;
                let values = fields
                    .into_iter()
                    .map(|field| self.parse_input_value(data_type, &field, span))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                self.reserve_values_payload(&values, span)?;
                values
            }
            None => {
                let line = self.read_line(DataType::Text, span)?;
                self.reserve_elements(line.split_whitespace().count(), span)?;
                let fields = line
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                let values = fields
                    .into_iter()
                    .map(|field| self.parse_input_value(data_type, &field, span))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                self.reserve_values_payload(&values, span)?;
                values
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

    fn parse_input_value(
        &self,
        data_type: DataType,
        value: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let parsed = match data_type {
            DataType::Integer => parse_integer_input(value.trim())
                .map(Value::Integer)
                .map_err(|_| "entrada contém um valor que não é Inteiro".to_owned()),
            DataType::Real => parse_real_input(value.trim())
                .map(Value::Real)
                .map_err(|_| "entrada contém um valor que não é Real".to_owned()),
            DataType::Text => Ok(Value::Text(value.to_owned())),
            DataType::Character => {
                let mut characters = value.encode_utf16();
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
        minimum: i32,
        maximum: i32,
        span: SourceSpan,
    ) -> RuntimeResult<i32> {
        if minimum > maximum {
            return Err(vec![self.error("limite inferior excede a superior", span)]);
        }
        let width = (i64::from(maximum) - i64::from(minimum) + 1) as u128;
        let offset = (self.random_next() as u128 % width) as i128;
        Ok((i128::from(minimum) + offset) as i32)
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
            ("+", Value::Character(value)) => Ok(Value::Integer(i32::from(value))),
            ("+", value @ (Value::Integer(_) | Value::Real(_) | Value::BigInteger(_))) => Ok(value),
            ("-", Value::Character(value)) => Ok(Value::Integer(-i32::from(value))),
            ("-", Value::Integer(value)) => Ok(Value::Integer(value.wrapping_neg())),
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
                let values = match right {
                    Value::List(mut values) => {
                        self.reserve_elements(1, span)?;
                        self.reserve_value_payload(&left, span)?;
                        values.insert(0, left);
                        values
                    }
                    Value::MutableList(values) => {
                        let values = values.borrow();
                        self.reserve_value_vec_copy(&values, span)?;
                        self.reserve_elements(1, span)?;
                        self.reserve_value_payload(&left, span)?;
                        let mut copy = values.clone();
                        copy.insert(0, left);
                        copy
                    }
                    _ => {
                        return Err(vec![
                            self.error("o lado direito de '::' deve ser lista", span),
                        ]);
                    }
                };
                Ok(Value::List(values))
            }
            "formato" => {
                let Value::Text(format) = right else {
                    return Err(vec![self.error("formato espera um Texto", span)]);
                };
                let result = format_value(&left, &format, self.options.max_string_bytes)
                    .map_err(|()| vec![self.error("limite de tamanho de texto excedido", span)])?
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
                    let left = self.expect_integer(&left, span)?;
                    let right = self.expect_integer(&right, span)?;
                    if right == 0 {
                        return Err(vec![self.error("divisão por zero", span)]);
                    }
                    let value = if operator == "div" {
                        left.wrapping_div(right)
                    } else {
                        left.wrapping_rem(right)
                    };
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
        if matches!(&left, Value::Character(_)) && matches!(&right, Value::Text(_)) {
            let Value::Character(character) = left else {
                unreachable!()
            };
            let Value::Text(text) = right else {
                unreachable!()
            };
            let mut result = String::from_utf16_lossy(&[character]);
            result.push_str(&text);
            self.check_string_size(result.len(), span)?;
            return Ok(Value::Text(result));
        }
        let left = numericize_character(left);
        let right = numericize_character(right);
        match (left, right) {
            (Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Integer(left.wrapping_add(right)))
            }
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
                let left = left.borrow();
                self.reserve_value_vec_copy(&left, span)?;
                self.reserve_elements(right.len(), span)?;
                let mut values = left.clone();
                values.extend(right);
                Ok(Value::List(values))
            }
            (Value::MutableList(left), Value::MutableList(right)) => {
                let left = left.borrow();
                let right = right.borrow();
                self.reserve_value_vec_copy(&left, span)?;
                self.reserve_elements(right.len(), span)?;
                self.reserve_cloned_values_payload(&right, span)?;
                let mut values = left.clone();
                values.extend(right.iter().cloned());
                Ok(Value::List(values))
            }
            (Value::List(mut left), Value::MutableList(right)) => {
                let right = right.borrow();
                self.reserve_elements(right.len(), span)?;
                self.reserve_cloned_values_payload(&right, span)?;
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
        if matches!(&left, Value::List(_) | Value::MutableList(_))
            && matches!(&right, Value::List(_) | Value::MutableList(_))
        {
            let mut values = self.copy_list_values(&left, span)?;
            for removed in self.copy_list_values(&right, span)? {
                if let Some(index) = values.iter().position(|value| value == &removed) {
                    values.remove(index);
                }
            }
            return Ok(Value::List(values));
        }
        if let (Value::Text(left_text), Value::Text(remove)) = (&left, &right) {
            let mut units = utf16_units(left_text);
            for unit in utf16_units(remove) {
                if let Some(index) = units.iter().position(|candidate| *candidate == unit) {
                    units.remove(index);
                }
            }
            let text = text_from_utf16(&units);
            self.check_string_size(text.len(), span)?;
            return Ok(Value::Text(text));
        }
        let left = numericize_character(left);
        let right = numericize_character(right);
        match (left, right) {
            (Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Integer(left.wrapping_sub(right)))
            }
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
            (Value::Integer(left), Value::Integer(right)) => {
                Ok(Value::Integer(left.wrapping_mul(right)))
            }
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
        match receiver {
            Value::TypeName(type_name) if type_name.starts_with("Jerimum") => {
                self.jerimum_member_value(&type_name, name, span)
            }
            value @ Value::TypeName(_) => Ok(Value::Function(Rc::new(Callable::Member {
                receiver: value,
                name: name.to_owned(),
            }))),
            Value::JerimumObject(object) => self.jerimum_object_member_value(object, name, span),
            Value::List(values) => self.list_member_value(Value::List(values), name, span),
            Value::MutableList(values) => {
                self.list_member_value(Value::MutableList(values), name, span)
            }
            Value::Text(text) => self.text_member_value(&text, name, span),
            value @ (Value::Integer(_)
            | Value::Real(_)
            | Value::BigInteger(_)
            | Value::Character(_)) => self.numeric_member_value(value, name, span),
            Value::Tuple(values) => {
                let index = tuple_member_index(name).ok_or_else(|| {
                    vec![self.error(format!("tupla não possui membro '{name}'"), span)]
                })?;
                let value = values.get(index).ok_or_else(|| {
                    vec![self.error(format!("tupla não possui membro '{name}'"), span)]
                })?;
                self.copy_value(value, span)
            }
            Value::Object(object) => {
                let object_fields = object.borrow();
                if let Some(value) = object_fields.fields.get(name) {
                    return self.copy_value(value, span);
                }
                drop(object_fields);
                let class_name = object.borrow().class_name.clone();
                let Some((declaration, method)) = self.method_declaration(&class_name, name) else {
                    return Err(vec![self.error(
                        format!("membro '{name}' não existe em '{class_name}'"),
                        span,
                    )]);
                };
                let callable = Value::Function(Rc::new(Callable::Function {
                    declaration: method.clone(),
                    environment: self
                        .class_environments
                        .get(&declaration.name)
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
            other => Err(vec![self.error(
                format!("tipo {} não possui membro '{name}'", other.display()),
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
        let values = self.copy_list_values(&receiver, span)?;
        match name {
            "tamanho" | "size" => Ok(Value::Integer(values.len() as i32)),
            "cabeça" | "cabeca" | "primeiro" => match values.first() {
                Some(value) => self.copy_value(value, span),
                None => Err(vec![self.error("lista vazia não possui cabeça", span)]),
            },
            "último" | "ultimo" => match values.last() {
                Some(value) => self.copy_value(value, span),
                None => Err(vec![
                    self.error("lista vazia não possui último elemento", span),
                ]),
            },
            "cauda" => {
                self.reserve_elements(values.len().saturating_sub(1), span)?;
                self.collection_value_like(&receiver, values.into_iter().skip(1).collect())
            }
            "vazia" => Ok(Value::Logical(values.is_empty())),
            "inverta" => {
                let mut values = values;
                values.reverse();
                self.collection_value_like(&receiver, values)
            }
            "ordene" => {
                let sorted = self.sort_values(values, None, span)?;
                self.collection_value_like(&receiver, sorted)
            }
            "junte" => {
                let text = values.iter().map(Value::display).collect::<String>();
                self.check_string_size(text.len(), span)?;
                Ok(Value::Text(text))
            }
            "mutável" | "mutavel" => Ok(Value::MutableList(Rc::new(RefCell::new(values)))),
            "imutável" | "imutavel" => Ok(Value::List(values)),
            "lista" | "para_lista" => self.collection_value_like(&receiver, values),
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
            "tamanho" => Ok(Value::Integer(text.encode_utf16().count() as i32)),
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
                let units = utf16_units(text);
                self.reserve_elements(units.len(), span)?;
                let characters = units.into_iter().map(Value::Character).collect::<Vec<_>>();
                Ok(Value::List(characters))
            }
            "cabeça" | "cabeca" => text
                .encode_utf16()
                .next()
                .map(Value::Character)
                .ok_or_else(|| vec![self.error("Texto vazio não possui cabeça", span)]),
            "primeiro" => text
                .encode_utf16()
                .next()
                .map(Value::Character)
                .ok_or_else(|| vec![self.error("Texto vazio não possui primeiro caractere", span)]),
            "último" | "ultimo" => utf16_units(text)
                .last()
                .copied()
                .map(Value::Character)
                .ok_or_else(|| vec![self.error("Texto vazio não possui último caractere", span)]),
            "cauda" => {
                let units = utf16_units(text);
                let result = text_from_utf16(&units[usize::from(!units.is_empty())..]);
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "ordene" => {
                let mut characters = utf16_units(text);
                self.reserve_elements(characters.len(), span)?;
                let mut failure = None;
                characters.sort_unstable_by(|left, right| {
                    if failure.is_some() {
                        return std::cmp::Ordering::Equal;
                    }
                    if let Err(error) = self.consume_step(span) {
                        failure = Some(error);
                        return std::cmp::Ordering::Equal;
                    }
                    left.cmp(right)
                });
                if let Some(error) = failure {
                    return Err(error);
                }
                let result = text_from_utf16(&characters);
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "divida" => self.call_text_member(text, name, Vec::new(), span),
            "get" | "pegue" | "descarte" | "junte" | "posição" | "posicao" | "contém"
            | "contem" | "selecione" | "ache" | "find" | "injete" | "pegue_enquanto"
            | "descarte_enquanto" | "remova" | "insira" | "mapeie" | "map" | "divida_quando" => {
                Ok(Value::Function(Rc::new(Callable::Member {
                    receiver: Value::Text(text.to_owned()),
                    name: name.to_owned(),
                })))
            }
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
                Value::BigInteger(value) => {
                    let negative = value.starts_with('-');
                    let digits = value.strip_prefix('-').unwrap_or(value);
                    let integer = digits.bytes().fold(0_i32, |integer, digit| {
                        integer
                            .wrapping_mul(10)
                            .wrapping_add(i32::from(digit - b'0'))
                    });
                    Ok(Value::Integer(if negative {
                        integer.wrapping_neg()
                    } else {
                        integer
                    }))
                }
                _ => numeric_f64(&value)
                    .map(|number| Value::Integer(number as i32))
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
                .map(|number| Value::Integer(jvm_math_round(number) as i32))
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
                Ok(Value::Character(number as i32 as u16))
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
                    (jvm_math_round(value * factor) as f64) / factor
                } else {
                    value
                };
                Ok(Value::Real(rounded))
            }
            (Value::Real(value), "arredonde", []) => {
                Ok(Value::Integer(jvm_math_round(value) as i32))
            }
            (receiver, _, _) => Err(vec![self.error(
                format!("{} não possui método '{name}'", receiver.display()),
                span,
            )]),
        }
    }

    fn index_value(
        &mut self,
        receiver: Value,
        index: i32,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match receiver {
            Value::List(values) | Value::Tuple(values) => {
                let index = normalize_index(index, values.len())
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                let value = values
                    .get(index)
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                self.copy_value(value, span)
            }
            Value::MutableList(values) => {
                let values = values.borrow();
                let index = normalize_index(index, values.len())
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                let value = values
                    .get(index)
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                self.copy_value(value, span)
            }
            Value::Text(text) => {
                let units = utf16_units(&text);
                let index = normalize_index(index, units.len())
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                Ok(Value::Character(units[index]))
            }
            other => {
                return Err(vec![
                    self.error(format!("{} não aceita índice", other.display()), span),
                ]);
            }
        }
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
        body: &Block,
        results: &mut Vec<Value>,
    ) -> RuntimeResult<()> {
        if self.pending_return.is_some() {
            return Ok(());
        }
        let Some(generator) = generators.get(index) else {
            let result = match self.execute_block(body)? {
                Flow::Next(value) => value,
                Flow::Return(value) => {
                    self.pending_return = Some(value);
                    return Ok(());
                }
            };
            self.reserve_elements(1, body.span)?;
            self.reserve_value_payload(&result, body.span)?;
            results.push(result);
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
                interpreter.evaluate_comprehension(generators, index + 1, body, results)
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
        match receiver {
            Value::List(values) => {
                self.call_list_member(Value::List(values), name, arguments, span)
            }
            Value::MutableList(values) => {
                self.call_list_member(Value::MutableList(values), name, arguments, span)
            }
            Value::Text(text) => self.call_text_member(&text, name, arguments, span),
            value @ (Value::Integer(_)
            | Value::Real(_)
            | Value::BigInteger(_)
            | Value::Character(_)) => self.call_numeric_member(value, name, arguments, span),
            Value::TypeName(type_name) if type_name.starts_with("Jerimum") => {
                self.call_jerimum_member(&type_name, name, arguments, span)
            }
            Value::TypeName(type_name) => self.call_type_factory(&type_name, name, arguments, span),
            Value::JerimumObject(object) => self.call_jerimum_object(object, name, arguments, span),
            Value::Object(object) => {
                let class_name = object.borrow().class_name.clone();
                let Some((declaration, method)) = self.method_declaration(&class_name, name) else {
                    return Err(vec![self.error(
                        format!("método '{name}' não existe em '{class_name}'"),
                        span,
                    )]);
                };
                let callable = Value::Function(Rc::new(Callable::Function {
                    declaration: method.clone(),
                    environment: self
                        .class_environments
                        .get(&declaration.name)
                        .cloned()
                        .unwrap_or_else(|| self.environment.clone()),
                    owner: Some(declaration.name),
                    receiver: Some(Value::Object(object)),
                }));
                self.apply(callable, arguments, span)
            }
            other => Err(vec![self.error(
                format!("tipo {} não possui método '{name}'", other.display()),
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
            ("Lista", "imutável" | "imutavel") => (arguments.len().saturating_sub(1), false),
            ("Lista", "mutável" | "mutavel") => (arguments.len().saturating_sub(1), true),
            ("Matriz", "imutável" | "imutavel") => (2, false),
            ("Matriz", "mutável" | "mutavel") => (2, true),
            ("Cubo", "imutável" | "imutavel") => (3, false),
            ("Cubo", "mutável" | "mutavel") => (3, true),
            _ => {
                return Err(vec![self.error(
                    format!("{type_name} não possui construtor '{name}'"),
                    span,
                )]);
            }
        };
        if dimensions == 0 || arguments.len() != dimensions + 1 {
            return Err(vec![self.error(
                format!("{type_name}.{name} recebeu quantidade incorreta de argumentos"),
                span,
            )]);
        }
        let mut sizes = Vec::with_capacity(dimensions);
        for argument in arguments.iter().take(dimensions) {
            let size = self.expect_integer(argument, span)?;
            sizes.push(size.max(0) as usize);
        }
        let (_, storage_elements) = nested_list_counts(&sizes)
            .ok_or_else(|| vec![self.error("dimensões excedem o limite", span)])?;
        self.reserve_elements(storage_elements, span)?;
        let initial = match arguments.last() {
            Some(value) => self.copy_value(value, span)?,
            None => Value::Nothing,
        };
        let leaf_count = sizes.iter().product();
        self.reserve_cloned_value_payload(&initial, leaf_count, span)?;
        let values = make_nested_list(&sizes, &initial);
        if mutable {
            Ok(Value::MutableList(Rc::new(RefCell::new(values))))
        } else {
            Ok(Value::List(values))
        }
    }

    fn call_type_factory_expressions(
        &mut self,
        type_name: &str,
        name: &str,
        arguments: &[Expr],
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let mutable = match (type_name, name) {
            ("Lista" | "Matriz", "imutável" | "imutavel") => false,
            ("Lista" | "Matriz", "mutável" | "mutavel") => true,
            ("Cubo", "imutável" | "imutavel") => false,
            ("Cubo", "mutável" | "mutavel") => true,
            _ => {
                return Err(vec![self.error(
                    format!("{type_name} não possui construtor '{name}'"),
                    span,
                )]);
            }
        };
        let dimensions = match type_name {
            "Lista" => arguments.len().saturating_sub(1),
            "Matriz" => 2,
            "Cubo" => 3,
            _ => 0,
        };
        if dimensions == 0 || arguments.len() != dimensions + 1 {
            return Err(vec![self.error(
                format!("{type_name}.{name} recebeu quantidade incorreta de argumentos"),
                span,
            )]);
        }
        let mut sizes = Vec::with_capacity(dimensions);
        for argument in arguments.iter().take(dimensions) {
            let value = self.evaluate_expression(argument)?;
            let size = self.expect_integer(&value, argument.span)?.max(0) as usize;
            sizes.push(size);
        }
        let (_, storage_elements) = nested_list_counts(&sizes)
            .ok_or_else(|| vec![self.error("dimensões excedem o limite", span)])?;
        self.reserve_elements(storage_elements, span)?;
        let values = self.evaluate_nested_list(&sizes, arguments.last().unwrap(), span)?;
        if mutable {
            Ok(Value::MutableList(Rc::new(RefCell::new(values))))
        } else {
            Ok(Value::List(values))
        }
    }

    fn evaluate_nested_list(
        &mut self,
        sizes: &[usize],
        initializer: &Expr,
        span: SourceSpan,
    ) -> RuntimeResult<Vec<Value>> {
        let Some((&size, remainder)) = sizes.split_first() else {
            return Ok(Vec::new());
        };
        let mut values = Vec::with_capacity(size);
        if remainder.is_empty() {
            for _ in 0..size {
                let value = self.evaluate_expression(initializer)?;
                self.reserve_value_payload(&value, span)?;
                values.push(value);
            }
        } else {
            for _ in 0..size {
                values.push(Value::List(self.evaluate_nested_list(
                    remainder,
                    initializer,
                    span,
                )?));
            }
        }
        Ok(values)
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
                let separator = match arguments.as_slice() {
                    [] => None,
                    [Value::Text(separator)] => Some(separator.as_str()),
                    [_] => return Err(vec![self.error("o separador deve ser Texto", span)]),
                    _ => {
                        return Err(vec![
                            self.error("divida aceita no máximo um separador", span),
                        ]);
                    }
                };
                let normalized = separator
                    .is_none()
                    .then(|| normalize_default_text_split(text));
                let input = normalized.as_deref().unwrap_or(text);
                let pattern = separator.unwrap_or(" ");
                let pieces = split_text_regex(input, pattern)
                    .ok_or_else(|| vec![self.error("expressão regular inválida", span)])?;
                let count = pieces.len();
                self.reserve_elements(count, span)?;
                self.reserve_collection_bytes(text.len(), span)?;
                Ok(Value::List(pieces.into_iter().map(Value::Text).collect()))
            }
            "get" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("get recebe um índice", span)]);
                }
                let index = self.expect_integer(&arguments[0], span)?;
                let characters = utf16_units(text);
                let index = normalize_index(index, characters.len())
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                Ok(Value::Character(characters[index]))
            }
            "pegue" | "descarte" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error(format!("{name} recebe um número"), span)]);
                }
                let count = self.expect_integer(&arguments[0], span)?.max(0);
                let chars = utf16_units(text);
                let count = usize::try_from(count)
                    .unwrap_or(usize::MAX)
                    .min(chars.len());
                let result: Vec<u16> = if name == "pegue" {
                    chars.into_iter().take(count).collect()
                } else {
                    chars.into_iter().skip(count).collect()
                };
                let result = text_from_utf16(&result);
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "junte" => {
                let (prefix, separator, suffix) = match arguments.as_slice() {
                    [] => ("", "", ""),
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
                let units = utf16_units(text);
                self.reserve_elements(units.len(), span)?;
                let joined = join_utf16_units(&units, separator);
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
                    Value::Character(character) => vec![*character],
                    Value::Text(text) => utf16_units(text),
                    other => {
                        return Err(vec![
                            self.error(format!("{name} não aceita {}", other.display()), span),
                        ]);
                    }
                };
                let units = utf16_units(text);
                if matches!(name, "posição" | "posicao") {
                    let position =
                        unit_subslice_position(&units, &needle).map_or(0, |index| index as i32 + 1);
                    Ok(Value::Integer(position))
                } else {
                    Ok(Value::Logical(
                        unit_subslice_position(&units, &needle).is_some(),
                    ))
                }
            }
            "ache" | "find" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("ache recebe uma função", span)]);
                }
                for character in utf16_units(text) {
                    let function = self.copy_value(&arguments[0], span)?;
                    let selected = self.apply(function, vec![Value::Character(character)], span)?;
                    if self.expect_logical(selected, span)? {
                        self.reserve_collection_bytes(std::mem::size_of::<Value>(), span)?;
                        return Ok(Value::Option(Some(Box::new(Value::Character(character)))));
                    }
                }
                Ok(Value::Option(None))
            }
            "mapeie" | "map" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("mapeie recebe uma função", span)]);
                }
                let mut mapped = Vec::new();
                for character in utf16_units(text) {
                    self.consume_step(span)?;
                    let function = self.copy_value(&arguments[0], span)?;
                    let value = self.apply(function, vec![Value::Character(character)], span)?;
                    let Value::Character(character) = value else {
                        return Err(vec![self.error("mapeie deve retornar Caractere", span)]);
                    };
                    mapped.push(character);
                }
                let result = text_from_utf16(&mapped);
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "divida_quando" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("divida_quando recebe uma função", span)]);
                }
                let mut groups = Vec::<Vec<u16>>::new();
                for character in utf16_units(text) {
                    self.consume_step(span)?;
                    let split = if let Some(previous) = groups.last().and_then(|group| group.last())
                    {
                        let function = self.copy_value(&arguments[0], span)?;
                        let result = self.apply(
                            function,
                            vec![Value::Character(*previous), Value::Character(character)],
                            span,
                        )?;
                        self.expect_logical(result, span)?
                    } else {
                        false
                    };
                    if split || groups.is_empty() {
                        groups.push(Vec::new());
                    }
                    groups.last_mut().unwrap().push(character);
                }
                self.reserve_elements(groups.len(), span)?;
                self.reserve_collection_bytes(text.len(), span)?;
                Ok(Value::List(
                    groups
                        .into_iter()
                        .map(|group| Value::Text(text_from_utf16(&group)))
                        .collect(),
                ))
            }
            "selecione" | "pegue_enquanto" | "descarte_enquanto" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error(format!("{name} recebe uma função"), span)]);
                }
                let characters = utf16_units(text);
                if name == "selecione" {
                    let mut result = Vec::new();
                    for character in characters {
                        self.consume_step(span)?;
                        let function = self.copy_value(&arguments[0], span)?;
                        let selected =
                            self.apply(function, vec![Value::Character(character)], span)?;
                        if self.expect_logical(selected, span)? {
                            result.push(character);
                        }
                    }
                    let result = text_from_utf16(&result);
                    self.check_string_size(result.len(), span)?;
                    Ok(Value::Text(result))
                } else {
                    let mut count = 0;
                    for character in &characters {
                        self.consume_step(span)?;
                        let function = self.copy_value(&arguments[0], span)?;
                        let selected =
                            self.apply(function, vec![Value::Character(*character)], span)?;
                        if !self.expect_logical(selected, span)? {
                            break;
                        }
                        count += 1;
                    }
                    let result: Vec<u16> = if name == "pegue_enquanto" {
                        characters.into_iter().take(count).collect()
                    } else {
                        characters.into_iter().skip(count).collect()
                    };
                    let result = text_from_utf16(&result);
                    self.check_string_size(result.len(), span)?;
                    Ok(Value::Text(result))
                }
            }
            "injete" => {
                if let [Value::Function(reducer)] = arguments.as_slice() {
                    let mut characters = utf16_units(text).into_iter();
                    let Some(first) = characters.next() else {
                        return Err(vec![
                            self.error("injete sem acumulador exige Texto não vazio", span),
                        ]);
                    };
                    let mut accumulator = Value::Character(first);
                    for character in characters {
                        self.consume_step(span)?;
                        accumulator = self.apply(
                            Value::Function(reducer.clone()),
                            vec![accumulator, Value::Character(character)],
                            span,
                        )?;
                    }
                    return Ok(accumulator);
                }
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
                let mut accumulator = self.copy_value(initial, span)?;
                for character in utf16_units(text) {
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
                let mut characters = utf16_units(text);
                let index = self.expect_integer(&arguments[0], span)?;
                if let Some(index) = index
                    .checked_sub(1)
                    .and_then(|index| usize::try_from(index).ok())
                    .filter(|index| *index < characters.len())
                {
                    characters.remove(index);
                }
                let result = text_from_utf16(&characters);
                self.check_string_size(result.len(), span)?;
                Ok(Value::Text(result))
            }
            "insira" => {
                if arguments.len() != 2 {
                    return Err(vec![self.error("insira recebe posição e valor", span)]);
                }
                let position = self.expect_integer(&arguments[0], span)?;
                let mut characters = utf16_units(text);
                let insert_at = usize::try_from(position.wrapping_sub(1))
                    .unwrap_or(0)
                    .min(characters.len());
                let insertion = match &arguments[1] {
                    Value::Character(character) => vec![*character],
                    Value::Text(text) => utf16_units(text),
                    other => {
                        return Err(vec![
                            self.error(format!("insira não aceita {}", other.display()), span),
                        ]);
                    }
                };
                characters.splice(insert_at..insert_at, insertion);
                let result = text_from_utf16(&characters);
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
        let values = self.copy_list_values(&receiver, span)?;
        match name {
            "mapeie" | "map" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("mapeie recebe uma função", span)]);
                }
                self.reserve_elements(values.len(), span)?;
                let mut result = Vec::with_capacity(values.len());
                for value in values {
                    let function = self.copy_value(&arguments[0], span)?;
                    let mapped = self.apply(function, vec![value], span)?;
                    self.reserve_value_payload(&mapped, span)?;
                    result.push(mapped);
                }
                self.collection_value_like(&receiver, result)
            }
            "selecione" | "filtre" | "filter" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("selecione recebe uma função", span)]);
                }
                let mut result = Vec::new();
                for value in values {
                    let value_copy = self.copy_value(&value, span)?;
                    let function = self.copy_value(&arguments[0], span)?;
                    let selected = self.apply(function, vec![value_copy], span)?;
                    if self.expect_logical(selected, span)? {
                        self.reserve_elements(1, span)?;
                        result.push(value);
                    }
                }
                self.collection_value_like(&receiver, result)
            }
            "pegue_enquanto" | "descarte_enquanto" | "takeWhile" | "dropWhile" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error(format!("{name} recebe uma função"), span)]);
                }
                let mut count = 0;
                for value in &values {
                    let value_copy = self.copy_value(value, span)?;
                    let function = self.copy_value(&arguments[0], span)?;
                    let predicate = self.apply(function, vec![value_copy], span)?;
                    let matches = self.expect_logical(predicate, span)?;
                    if !matches {
                        break;
                    }
                    count += 1;
                }
                let result = if matches!(name, "pegue_enquanto" | "takeWhile") {
                    self.reserve_elements(count, span)?;
                    values.into_iter().take(count).collect()
                } else {
                    self.reserve_elements(values.len().saturating_sub(count), span)?;
                    values.into_iter().skip(count).collect()
                };
                self.collection_value_like(&receiver, result)
            }
            "ache" | "find" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("ache recebe uma função", span)]);
                }
                for value in values {
                    let predicate = self.copy_value(&arguments[0], span)?;
                    let candidate = self.copy_value(&value, span)?;
                    let selected = self.apply(predicate, vec![candidate], span)?;
                    if self.expect_logical(selected, span)? {
                        return Ok(Value::Option(Some(Box::new(value))));
                    }
                }
                Ok(Value::Option(None))
            }
            "get" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("get recebe um índice", span)]);
                }
                let index = self.expect_integer(&arguments[0], span)?;
                let index = normalize_index(index, values.len())
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                values
                    .into_iter()
                    .nth(index)
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])
            }
            "zip" => {
                if arguments.len() != 1 {
                    return Err(vec![self.error("zip recebe uma coleção", span)]);
                }
                let right = match &arguments[0] {
                    Value::List(_) | Value::MutableList(_) => {
                        self.copy_list_values(&arguments[0], span)?
                    }
                    Value::Text(text) => {
                        let characters = utf16_units(text);
                        self.reserve_elements(characters.len(), span)?;
                        characters.into_iter().map(Value::Character).collect()
                    }
                    Value::Tuple(values) => self.copy_value_vec(values, span)?,
                    other => {
                        return Err(vec![
                            self.error(format!("zip não aceita {}", other.display()), span),
                        ]);
                    }
                };
                let count = values.len().min(right.len());
                self.reserve_elements(count.saturating_mul(3), span)?;
                let zipped = values
                    .into_iter()
                    .zip(right)
                    .take(count)
                    .map(|(left, right)| Value::Tuple(vec![left, right]))
                    .collect();
                self.collection_value_like(&receiver, zipped)
            }
            "reduza" | "reduce" => {
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
                    [Value::Function(reducer)] => {
                        let Some(first) = values.first() else {
                            return Err(vec![self.error("reduza não aceita lista vazia", span)]);
                        };
                        (
                            self.copy_value(first, span)?,
                            Value::Function(reducer.clone()),
                            values.into_iter().skip(1).collect::<Vec<_>>(),
                        )
                    }
                    [initial, Value::Function(reducer)] => (
                        self.copy_value(initial, span)?,
                        Value::Function(reducer.clone()),
                        values,
                    ),
                    _ => return Err(vec![self.error("reduza recebe acumulador e função", span)]),
                };
                for value in remaining {
                    self.consume_step(span)?;
                    accumulator = self.apply(reducer.clone(), vec![accumulator, value], span)?;
                }
                Ok(accumulator)
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
                        let Some(first) = values.first() else {
                            return Err(vec![
                                self.error("injete sem acumulador exige lista não vazia", span),
                            ]);
                        };
                        let first = self.copy_value(first, span)?;
                        self.reserve_elements(values.len().saturating_sub(1), span)?;
                        let reducer = self.copy_value(&arguments[0], span)?;
                        (
                            first,
                            reducer,
                            values.into_iter().skip(1).collect::<Vec<_>>(),
                        )
                    }
                    [initial, Value::Function(reducer)] => (
                        self.copy_value(initial, span)?,
                        Value::Function(reducer.clone()),
                        values,
                    ),
                    _ => return Err(vec![self.error("injete recebe acumulador e função", span)]),
                };
                for value in remaining {
                    self.consume_step(span)?;
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
                    .map_or(0, |index| index as i32 + 1);
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
                let index = self.expect_integer(&arguments[0], span)?;
                let mut result = values;
                if let Some(index) = index
                    .checked_sub(1)
                    .and_then(|index| usize::try_from(index).ok())
                    .filter(|index| *index < result.len())
                {
                    result.remove(index);
                }
                self.collection_value_like(&receiver, result)
            }
            "insira" | "atualize" => {
                if arguments.len() != 2 {
                    return Err(vec![
                        self.error(format!("{name} recebe posição e valor"), span),
                    ]);
                }
                let index = self.expect_integer(&arguments[0], span)?;
                if name == "insira" {
                    self.reserve_elements(1, span)?;
                    self.reserve_cloned_value_payload(&arguments[1], 1, span)?;
                    let mut result = values;
                    let insert_at = usize::try_from(index.wrapping_sub(1))
                        .unwrap_or(0)
                        .min(result.len());
                    result.insert(insert_at, arguments[1].clone());
                    return self.collection_value_like(&receiver, result);
                }
                let index = usize::try_from(index)
                    .ok()
                    .filter(|index| *index < values.len())
                    .ok_or_else(|| vec![self.error("índice fora dos limites", span)])?;
                let replacement = if matches!(arguments[1], Value::Function(_)) {
                    let current = self.copy_value(&values[index], span)?;
                    self.apply(arguments[1].clone(), vec![current], span)?
                } else {
                    self.copy_value(&arguments[1], span)?
                };
                let mut result = values;
                result[index] = replacement;
                if let Value::MutableList(target) = receiver {
                    *target.borrow_mut() = result;
                    Ok(Value::MutableList(target))
                } else {
                    Ok(Value::List(result))
                }
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
                        let previous = self.copy_value(previous, span)?;
                        let value_copy = self.copy_value(&value, span)?;
                        let function = self.copy_value(&arguments[0], span)?;
                        let split_result =
                            self.apply(function, vec![previous, value_copy], span)?;
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
                let count = self.expect_integer(&arguments[0], span)?.max(0) as usize;
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

    fn sort_values(
        &mut self,
        mut values: Vec<Value>,
        key_or_comparator: Option<&Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Vec<Value>> {
        let Some(function) = key_or_comparator else {
            let mut invalid = false;
            let mut failure = None;
            values.sort_by(|left, right| {
                if failure.is_some() {
                    return std::cmp::Ordering::Equal;
                }
                if let Err(error) = self.consume_step(span) {
                    failure = Some(error);
                    return std::cmp::Ordering::Equal;
                }
                compare_values(left, right).unwrap_or_else(|| {
                    invalid = true;
                    std::cmp::Ordering::Equal
                })
            });
            if let Some(error) = failure {
                return Err(error);
            }
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
                if let Err(error) = self.consume_step(span) {
                    failure = Some(error);
                    return std::cmp::Ordering::Equal;
                }
                let left_copy = match self.copy_value(left, span) {
                    Ok(value) => value,
                    Err(error) => {
                        failure = Some(error);
                        return std::cmp::Ordering::Equal;
                    }
                };
                let right_copy = match self.copy_value(right, span) {
                    Ok(value) => value,
                    Err(error) => {
                        failure = Some(error);
                        return std::cmp::Ordering::Equal;
                    }
                };
                match self
                    .apply(
                        Value::Function(callable.clone()),
                        vec![left_copy, right_copy],
                        span,
                    )
                    .and_then(|result| self.expect_logical(result, span))
                {
                    Ok(true) => std::cmp::Ordering::Less,
                    Ok(false) => {
                        let right_copy = match self.copy_value(right, span) {
                            Ok(value) => value,
                            Err(error) => {
                                failure = Some(error);
                                return std::cmp::Ordering::Equal;
                            }
                        };
                        let left_copy = match self.copy_value(left, span) {
                            Ok(value) => value,
                            Err(error) => {
                                failure = Some(error);
                                return std::cmp::Ordering::Equal;
                            }
                        };
                        match self
                            .apply(
                                Value::Function(callable.clone()),
                                vec![right_copy, left_copy],
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
                        }
                    }
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
        let keyed_bytes = values
            .len()
            .checked_mul(std::mem::size_of::<(Value, Value)>())
            .ok_or_else(|| vec![self.error("limite de memória de coleções excedido", span)])?;
        self.reserve_collection_bytes(keyed_bytes, span)?;
        let mut keyed = Vec::with_capacity(values.len());
        for value in values {
            let value_copy = self.copy_value(&value, span)?;
            let key = self.apply(Value::Function(callable.clone()), vec![value_copy], span)?;
            self.reserve_value_payload(&key, span)?;
            keyed.push((key, value));
        }
        let mut invalid = false;
        let mut failure = None;
        keyed.sort_by(|(left, _), (right, _)| {
            if failure.is_some() {
                return std::cmp::Ordering::Equal;
            }
            if let Err(error) = self.consume_step(span) {
                failure = Some(error);
                return std::cmp::Ordering::Equal;
            }
            compare_values(left, right).unwrap_or_else(|| {
                invalid = true;
                std::cmp::Ordering::Equal
            })
        });
        if let Some(error) = failure {
            return Err(error);
        }
        if invalid {
            return Err(vec![
                self.error("chaves da ordenação não podem ser comparadas", span),
            ]);
        }
        self.reserve_elements(keyed.len(), span)?;
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
                let mut combined = self.copy_value_vec(previous, span)?;
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
            Callable::Builtin(_) => Some(1),
            Callable::Constructor { declaration, .. } => Some(
                self.effective_type_fields(&declaration.name)
                    .ok()?
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
                    "get" | "pegue" | "descarte" => Some(1),
                    "insira" | "atualize" => Some(2),
                    "reduza" | "reduce" => {
                        Some(if matches!(arguments.first(), Some(Value::Function(_))) {
                            1
                        } else {
                            2
                        })
                    }
                    "injete" => Some(if matches!(arguments.first(), Some(Value::Function(_))) {
                        1
                    } else {
                        2
                    }),
                    "ordene" | "junte" => Some(arguments.len()),
                    _ => None,
                },
                Value::Text(_) => match name.as_str() {
                    "divida" => Some(arguments.len().min(1)),
                    "get" | "pegue" | "descarte" | "posição" | "posicao" | "contém" | "contem"
                    | "selecione" | "pegue_enquanto" | "descarte_enquanto" | "remova" => Some(1),
                    "mapeie" | "map" | "divida_quando" => Some(1),
                    "injete" => Some(if matches!(arguments.first(), Some(Value::Function(_))) {
                        1
                    } else {
                        2
                    }),
                    "insira" => Some(2),
                    "junte" => Some(arguments.len()),
                    _ => None,
                },
                Value::BigInteger(_) if name == "pow" => Some(1),
                Value::TypeName(type_name) => {
                    jerimum::max_arity(type_name, name).or_else(|| {
                        match (type_name.as_str(), name.as_str()) {
                            ("Lista", "imutável" | "imutavel" | "mutável" | "mutavel") => Some(2),
                            ("Matriz", "imutável" | "imutavel" | "mutável" | "mutavel") => {
                                Some(3)
                            }
                            ("Cubo", "imutável" | "imutavel" | "mutável" | "mutavel") => Some(4),
                            _ => None,
                        }
                    })
                }
                Value::JerimumObject(object) => {
                    jerimum::max_arity(jerimum_object_owner(&object.borrow()), name)
                }
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
            } => {
                let receiver = receiver
                    .as_ref()
                    .map(|value| self.copy_value(value, span))
                    .transpose()?;
                self.invoke_function(
                    declaration,
                    environment.clone(),
                    owner.as_deref(),
                    receiver,
                    arguments,
                    span,
                )
            }
            Callable::Lambda {
                id,
                parameters,
                body,
                environment,
            } => self.invoke_lambda(*id, parameters, body, environment.clone(), arguments, span),
            Callable::Builtin(name) => {
                let [value] = arguments.as_slice() else {
                    return Err(vec![
                        self.error(format!("{name} recebe um argumento"), span),
                    ]);
                };
                let value = self.copy_value(value, span)?;
                self.evaluate_math_builtin(name, value, span)
            }
            Callable::Constructor {
                declaration,
                environment,
            } => self.construct(declaration, environment.clone(), arguments, span),
            Callable::Member { receiver, name } => {
                let receiver = self.copy_value(receiver, span)?;
                self.call_member(receiver, name, arguments, span)
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
                    Environment::define(
                        &call_environment,
                        "\0isto",
                        Value::Object(object.clone()),
                        false,
                    )
                    .map_err(|message| vec![self.error(message, span)])?;
                    let Some(class) = self.type_info.classes.get(owner).cloned() else {
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
                    let runtime_class = object.borrow().class_name.clone();
                    for method_name in class.methods.keys() {
                        let Some((method_owner, method)) =
                            self.method_declaration(&runtime_class, method_name)
                        else {
                            continue;
                        };
                        Environment::define(
                            &call_environment,
                            method_name.clone(),
                            Value::Function(Rc::new(Callable::Function {
                                declaration: method,
                                environment: self
                                    .class_environments
                                    .get(&method_owner.name)
                                    .cloned()
                                    .unwrap_or_else(|| environment.clone()),
                                owner: Some(method_owner.name),
                                receiver: receiver.clone(),
                            })),
                            false,
                        )
                        .map_err(|message| vec![self.error(message, span)])?;
                    }
                }
                for (parameter, value) in declaration.parameters.iter().zip(arguments.iter()) {
                    let value = self.copy_value(value, parameter.span)?;
                    let value = self.coerce_to_node(value, parameter.id, parameter.span)?;
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
                    FunctionBody::Abstract => Err(vec![interpreter.error(
                        format!(
                            "método abstrato '{}' não foi implementado",
                            declaration.name
                        ),
                        declaration.span,
                    )]),
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
        body: &FunctionBody,
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
                let value = self.copy_value(value, parameter.span)?;
                let value = self.coerce_to_node(value, parameter.id, parameter.span)?;
                Environment::define(
                    &call_environment,
                    parameter.name.clone(),
                    value,
                    parameter.mutable,
                )
                .map_err(|message| vec![self.error(message, parameter.span)])?;
            }
            self.with_environment(call_environment, |interpreter| match body {
                FunctionBody::Expression(expression) => interpreter.evaluate_expression(expression),
                FunctionBody::Block(block) => match interpreter.execute_block(block)? {
                    Flow::Next(value) => Ok(value),
                    Flow::Return(value) => {
                        interpreter.pending_return = Some(value);
                        Ok(Value::Nothing)
                    }
                },
                FunctionBody::Abstract => Err(vec![
                    interpreter.error("lambda abstrata não pode ser chamada", span),
                ]),
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

    fn effective_type_fields(&self, type_name: &str) -> RuntimeResult<Vec<crate::ast::TypeField>> {
        let declaration = self.classes.get(type_name).ok_or_else(|| {
            vec![self.error(
                format!("tipo '{type_name}' não foi encontrado"),
                SourceSpan::default(),
            )]
        })?;
        let mut fields = Vec::<crate::ast::TypeField>::new();
        for parent in &declaration.parents {
            for field in self.effective_type_fields(parent)? {
                if let Some(existing) = fields.iter_mut().find(|item| item.name == field.name) {
                    existing.mutable |= field.mutable;
                    existing.constructor_parameter |= field.constructor_parameter;
                } else {
                    fields.push(field);
                }
            }
        }
        for field in &declaration.fields {
            if let Some(inherited) = fields.iter_mut().find(|item| item.name == field.name) {
                let mut field = field.clone();
                field.mutable |= inherited.mutable;
                *inherited = field;
            } else {
                fields.push(field.clone());
            }
        }
        Ok(fields)
    }

    fn method_declaration(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> Option<(TypeDecl, FunctionDecl)> {
        let declaration = self.classes.get(type_name)?.clone();
        if let Some(method) = declaration
            .methods
            .iter()
            .find(|method| method.name == method_name)
            .cloned()
        {
            return Some((declaration, method));
        }
        for parent in &declaration.parents {
            if let Some(method) = self.method_declaration(parent, method_name) {
                return Some(method);
            }
        }
        None
    }

    fn construct(
        &mut self,
        declaration: &TypeDecl,
        environment: EnvironmentRef,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if declaration.abstract_type {
            return Err(vec![self.error(
                format!(
                    "o tipo abstrato '{}' não pode ser instanciado",
                    declaration.name
                ),
                span,
            )]);
        }
        let all_fields = self.effective_type_fields(&declaration.name)?;
        let fields = all_fields
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
            constructor_fields: fields.iter().map(|field| field.name.clone()).collect(),
            fields: HashMap::new(),
        }));
        let instance_environment = Environment::child(environment);
        for field in &all_fields {
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
            for field in &all_fields {
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

impl<'a> Interpreter<'a> {
    fn call_jerimum_constructor(
        &mut self,
        name: &str,
        expressions: &[Expr],
        span: SourceSpan,
    ) -> RuntimeResult<Option<Value>> {
        let Some(kind) = jerimum::constructor_name(name) else {
            return Ok(None);
        };
        let arguments = expressions
            .iter()
            .map(|expression| self.evaluate_argument(expression))
            .collect::<RuntimeResult<Vec<_>>>()?;
        let object = match kind {
            "Cor" => {
                let [red, green, blue] = arguments.as_slice() else {
                    return Err(vec![self.error("Cor recebe vermelho, verde e azul", span)]);
                };
                let red = self.expect_integer(red, span)?;
                let green = self.expect_integer(green, span)?;
                let blue = self.expect_integer(blue, span)?;
                if [red, green, blue]
                    .iter()
                    .any(|channel| !(0..=255).contains(channel))
                {
                    return Err(vec![
                        self.error("cada canal de cor deve estar entre 0 e 255", span),
                    ]);
                }
                JerimumObject::Color { red, green, blue }
            }
            "Imagem" => {
                let [Value::Text(path)] = arguments.as_slice() else {
                    return Err(vec![
                        self.error("Imagem recebe um caminho de arquivo", span),
                    ]);
                };
                return self.load_jerimum_image(path, span).map(Some);
            }
            "Animacao" => {
                let [Value::Integer(speed), images] = arguments.as_slice() else {
                    return Err(vec![
                        self.error("Animacao recebe velocidade e lista de imagens", span),
                    ]);
                };
                if *speed <= 0 {
                    return Err(vec![
                        self.error("a velocidade da animação deve ser positiva", span),
                    ]);
                }
                let images = self.copy_list_values(images, span)?;
                if images.is_empty()
                    || images.iter().any(|image| {
                        !matches!(image, Value::JerimumObject(object) if matches!(&*object.borrow(), JerimumObject::Image { .. }))
                    })
                {
                    return Err(vec![self.error("Animacao precisa de uma lista não vazia de imagens", span)]);
                }
                JerimumObject::Animation {
                    speed: *speed,
                    images,
                    started: std::time::Instant::now(),
                }
            }
            "Fonte" => {
                let [Value::Integer(size)] = arguments.as_slice() else {
                    return Err(vec![self.error("Fonte recebe um tamanho inteiro", span)]);
                };
                if !(1..=512).contains(size) {
                    return Err(vec![
                        self.error("o tamanho da fonte deve estar entre 1 e 512", span),
                    ]);
                }
                JerimumObject::Font { size: *size }
            }
            "Som" | "Musica" => {
                let [Value::Text(path)] = arguments.as_slice() else {
                    return Err(vec![
                        self.error("o áudio recebe um caminho de arquivo", span),
                    ]);
                };
                let handle = self.jerimum_host_integer(
                    JerimumRequest::Audio {
                        function: if kind == "Musica" {
                            "carregar_musica".to_owned()
                        } else {
                            "carregar_som".to_owned()
                        },
                        arguments: vec![JerimumValue::String(path.clone())],
                    },
                    span,
                )?;
                JerimumObject::Sound {
                    path: path.clone(),
                    handle,
                    music: kind == "Musica",
                }
            }
            "Tela" => {
                let [
                    Value::Text(title),
                    Value::Integer(width),
                    Value::Integer(height),
                ] = arguments.as_slice()
                else {
                    return Err(vec![
                        self.error("Tela recebe título, largura e altura", span),
                    ]);
                };
                self.jerimum_start_window(title, *width, *height, span)?;
                let id = self.jerimum_next_screen_id;
                self.jerimum_next_screen_id = id
                    .checked_add(1)
                    .ok_or_else(|| vec![self.error("limite de telas Jerimum excedido", span)])?;
                JerimumObject::Screen {
                    id,
                    title: title.clone(),
                    width: *width,
                    height: *height,
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(Value::JerimumObject(Rc::new(RefCell::new(object)))))
    }

    fn call_jerimum_member(
        &mut self,
        owner: &str,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if owner == "JerimumModule:Jogo" {
            return match name {
                "distância" | "distancia" => {
                    let [x1, y1, x2, y2] = arguments.as_slice() else {
                        return Err(vec![
                            self.error("distância recebe quatro coordenadas", span),
                        ]);
                    };
                    let number = |value| {
                        numeric_f64(value)
                            .ok_or_else(|| vec![self.error("esperava um número", span)])
                    };
                    let x1 = number(x1)?;
                    let y1 = number(y1)?;
                    let x2 = number(x2)?;
                    let y2 = number(y2)?;
                    Ok(Value::Real(((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt()))
                }
                "projeção_X" | "projecao_X" | "projeçao_X" | "projecão_X" | "projeção_Y"
                | "projecao_Y" | "projeçao_Y" | "projecão_Y" => {
                    let [angle, magnitude] = arguments.as_slice() else {
                        return Err(vec![self.error("projeção recebe ângulo e valor", span)]);
                    };
                    let angle = numeric_f64(angle)
                        .ok_or_else(|| vec![self.error("esperava um ângulo numérico", span)])?;
                    let magnitude = numeric_f64(magnitude)
                        .ok_or_else(|| vec![self.error("esperava um valor numérico", span)])?;
                    let radians = angle.to_radians();
                    if name.ends_with('X') {
                        Ok(Value::Real(radians.sin() * magnitude))
                    } else {
                        Ok(Value::Real(-radians.cos() * magnitude))
                    }
                }
                "iniciar" => self.call_jerimum_game(arguments, span),
                _ => {
                    Err(vec![self.error(
                        format!("membro Jerimum desconhecido: '{name}'"),
                        span,
                    )])
                }
            };
        }
        if owner == "JerimumModule:Arquivo" && name == "leia" {
            let [Value::Text(path)] = arguments.as_slice() else {
                return Err(vec![self.error("Arquivo.leia recebe um caminho", span)]);
            };
            return self
                .jerimum_request_value(JerimumRequest::ReadFileLines { path: path.clone() }, span);
        }
        if matches!(owner, "JerimumModule:Imagem" | "JerimumType:Imagem") && name == "fatie" {
            let [
                Value::Text(path),
                Value::Integer(width),
                Value::Integer(height),
            ] = arguments.as_slice()
            else {
                return Err(vec![
                    self.error("Imagem.fatie recebe caminho, largura e altura", span),
                ]);
            };
            let image = self.load_jerimum_image(path, span)?;
            return self.slice_jerimum_image(&image, *width, *height, span);
        }
        if owner == "JerimumModule:Desenho" && name == "incluir" {
            let [Value::Integer(z), callback] = arguments.as_slice() else {
                return Err(vec![
                    self.error("Desenho.incluir recebe camada e função", span),
                ]);
            };
            self.jerimum_drawings
                .entry(*z)
                .or_default()
                .push(JerimumDrawItem::Callback(callback.clone()));
            return Ok(Value::Nothing);
        }
        if owner == "JerimumModule:Desenho" && name == "desenhe" {
            let [context] = arguments.as_slice() else {
                return Err(vec![
                    self.error("Desenho.desenhe recebe um contexto gráfico", span),
                ]);
            };
            return self
                .flush_jerimum_drawings_with_context(context.clone(), span)
                .map(|()| Value::Nothing);
        }
        Err(vec![self.error(
            format!("método Jerimum desconhecido: '{name}'"),
            span,
        )])
    }

    fn call_jerimum_object(
        &mut self,
        object: Rc<RefCell<JerimumObject>>,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let kind = object.borrow().clone();
        match kind {
            JerimumObject::Image {
                handle,
                width,
                height,
                ..
            } => match name {
                "fatie" => {
                    let [Value::Integer(frame_width), Value::Integer(frame_height)] =
                        arguments.as_slice()
                    else {
                        return Err(vec![
                            self.error("Imagem.fatie recebe largura e altura", span),
                        ]);
                    };
                    self.slice_jerimum_image(
                        &Value::JerimumObject(object),
                        *frame_width,
                        *frame_height,
                        span,
                    )
                }
                "desenhe" | "desenhe_centralizado" => {
                    if !(3..=6).contains(&arguments.len()) {
                        return Err(vec![
                            self.error("Imagem.desenhe recebe de três a seis argumentos", span),
                        ]);
                    }
                    let mut x = numeric_f64(&arguments[0])
                        .ok_or_else(|| vec![self.error("esperava uma coordenada X", span)])?;
                    let mut y = numeric_f64(&arguments[1])
                        .ok_or_else(|| vec![self.error("esperava uma coordenada Y", span)])?;
                    let z = self.expect_integer(&arguments[2], span)?;
                    let angle = arguments.get(3).and_then(numeric_f64).unwrap_or(0.0);
                    let scale_x = arguments.get(4).and_then(numeric_f64).unwrap_or(1.0);
                    let scale_y = arguments.get(5).and_then(numeric_f64).unwrap_or(1.0);
                    if name == "desenhe_centralizado" {
                        x -= width as f64 / 2.0;
                        y -= height as f64 / 2.0;
                    }
                    self.enqueue_jerimum_drawing(
                        z,
                        None,
                        "jerimum_desenhar_imagem",
                        vec![
                            JerimumValue::Real(x),
                            JerimumValue::Real(y),
                            JerimumValue::Integer(handle),
                            JerimumValue::Real(angle),
                            JerimumValue::Real(scale_x),
                            JerimumValue::Real(scale_y),
                        ],
                    );
                    Ok(Value::Nothing)
                }
                _ => Err(vec![
                    self.error(format!("Imagem não possui método '{name}'"), span),
                ]),
            },
            JerimumObject::Animation {
                speed,
                images,
                started,
            } if name == "imagem" => {
                if !arguments.is_empty() || images.is_empty() {
                    return Err(vec![
                        self.error("Animacao.imagem não recebe argumentos", span),
                    ]);
                }
                let frame = (started.elapsed().as_millis() / speed as u128) % images.len() as u128;
                self.copy_value(&images[frame as usize], span)
            }
            JerimumObject::Font { size } if matches!(name, "desenhe" | "desenhe_centralizado") => {
                if !(4..=5).contains(&arguments.len()) {
                    return Err(vec![self.error(
                        "Fonte.desenhe recebe texto, posição, camada e cor opcional",
                        span,
                    )]);
                }
                let [Value::Text(text), x, y, z] = &arguments[..4] else {
                    return Err(vec![
                        self.error("argumentos inválidos para Fonte.desenhe", span),
                    ]);
                };
                let x = numeric_f64(x)
                    .ok_or_else(|| vec![self.error("esperava uma coordenada X", span)])?;
                let y = numeric_f64(y)
                    .ok_or_else(|| vec![self.error("esperava uma coordenada Y", span)])?;
                let z = self.expect_integer(z, span)?;
                let color = arguments
                    .get(4)
                    .cloned()
                    .unwrap_or_else(white_jerimum_color);
                if !matches!(&color, Value::JerimumObject(value) if matches!(&*value.borrow(), JerimumObject::Color { .. }))
                {
                    return Err(vec![self.error("a cor precisa ser um valor Cor", span)]);
                }
                self.enqueue_jerimum_drawing(
                    z,
                    Some(color),
                    "jerimum_desenhar_texto",
                    vec![
                        JerimumValue::String(text.clone()),
                        JerimumValue::Real(x),
                        JerimumValue::Real(y),
                        JerimumValue::Integer(size as i64),
                        JerimumValue::Logical(name == "desenhe_centralizado"),
                    ],
                );
                Ok(Value::Nothing)
            }
            JerimumObject::Sound { handle, music, .. } => {
                self.call_jerimum_audio_object(handle, music, name, arguments, span)
            }
            JerimumObject::AwtColor {
                red,
                green,
                blue,
                alpha,
            } => self.call_jerimum_color_member(red, green, blue, alpha, name, arguments, span),
            JerimumObject::ImageBuffer {
                handle,
                width,
                height,
            } => {
                self.call_jerimum_image_buffer_member(handle, width, height, name, arguments, span)
            }
            JerimumObject::Frame {
                title,
                width,
                height,
                ..
            } => self.call_jerimum_frame_member(&title, width, height, name, arguments, span),
            JerimumObject::Canvas { width, height, .. } => {
                self.call_jerimum_canvas_member(width, height, name, arguments, span)
            }
            JerimumObject::GraphicsContext { .. } => {
                self.call_jerimum_graphics_context_member(object, name, arguments, span)
            }
            JerimumObject::Color { .. } | JerimumObject::Screen { .. } => {
                Err(vec![self.error(
                    format!("objeto Jerimum não possui método '{name}'"),
                    span,
                )])
            }
            JerimumObject::Font { .. } => Err(vec![
                self.error(format!("Fonte não possui método '{name}'"), span),
            ]),
            JerimumObject::Animation { .. } => {
                Err(vec![self.error(
                    format!("Animacao não possui método '{name}'"),
                    span,
                )])
            }
        }
    }

    fn jerimum_member_value(
        &mut self,
        owner: &str,
        name: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if owner == "JerimumType:Cor"
            && let Some((red, green, blue)) = jerimum::color_constant(name)
        {
            return Ok(Value::JerimumObject(Rc::new(RefCell::new(
                JerimumObject::Color { red, green, blue },
            ))));
        }
        if owner == "JerimumType:Imagem" && name == "fatie" {
            return Ok(Value::Function(Rc::new(Callable::Member {
                receiver: Value::TypeName("JerimumModule:Imagem".to_owned()),
                name: name.to_owned(),
            })));
        }
        if owner.starts_with("JerimumType:") {
            return Err(vec![self.error(
                format!("tipo Jerimum não possui membro estático '{name}'"),
                span,
            )]);
        }
        if jerimum::property_type(owner, name).is_some() {
            return match owner {
                "JerimumModule:Jogo" => Ok(match name {
                    "titulo" => Value::Text(self.jerimum_game_title.clone()),
                    "largura" => Value::Integer(self.jerimum_game_width),
                    "altura" => Value::Integer(self.jerimum_game_height),
                    "fps" => Value::Integer(self.jerimum_game_fps),
                    _ => Value::Nothing,
                }),
                "JerimumModule:Relogio" => Ok(match name {
                    "agora" => Value::Real(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as f64,
                    ),
                    "milisegundos" => {
                        Value::Integer(self.jerimum_started.elapsed().as_millis() as i32)
                    }
                    _ => Value::Nothing,
                }),
                "JerimumModule:Teclado" => {
                    let key = jerimum_key_code(name)
                        .ok_or_else(|| vec![self.error("tecla Jerimum desconhecida", span)])?;
                    self.jerimum_request_value(JerimumRequest::KeyPressed { key }, span)
                }
                "JerimumModule:Mouse" if matches!(name, "BOTAO_ESQUERDO" | "BOTAO_DIREITO") => self
                    .jerimum_request_value(
                        JerimumRequest::MouseButtonPressed {
                            button: if name == "BOTAO_ESQUERDO" { 1 } else { 3 },
                        },
                        span,
                    ),
                "JerimumModule:Mouse" if matches!(name, "x" | "y") => {
                    let Value::List(position) =
                        self.jerimum_request_value(JerimumRequest::MousePosition, span)?
                    else {
                        return Err(vec![self.error("posição do mouse inválida", span)]);
                    };
                    let index = usize::from(name == "y");
                    position
                        .get(index)
                        .cloned()
                        .ok_or_else(|| vec![self.error("posição do mouse inválida", span)])
                }
                _ => Err(vec![self.error(
                    format!("membro Jerimum '{name}' não está disponível"),
                    span,
                )]),
            };
        }
        if let Some(value_type) = jerimum::object_property_type(owner, name) {
            if value_type == ValueType::Nothing {
                return self.call_jerimum_member_no_receiver(owner, name, span);
            }
        }
        if jerimum::member_type(owner, name).is_some() {
            return Ok(Value::Function(Rc::new(Callable::Member {
                receiver: Value::TypeName(owner.to_owned()),
                name: name.to_owned(),
            })));
        }
        Err(vec![self.error(
            format!("membro Jerimum desconhecido: '{name}'"),
            span,
        )])
    }

    fn call_jerimum_member_no_receiver(
        &mut self,
        owner: &str,
        name: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        Err(vec![self.error(
            format!("membro Jerimum '{owner}.{name}' requer um objeto"),
            span,
        )])
    }

    fn jerimum_object_member_value(
        &mut self,
        object: Rc<RefCell<JerimumObject>>,
        name: &str,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let kind = object.borrow().clone();
        match kind {
            JerimumObject::Color { red, green, blue } => match name {
                "vermelho" => Ok(Value::Integer(red)),
                "verde" => Ok(Value::Integer(green)),
                "azul" => Ok(Value::Integer(blue)),
                "color" => Ok(Value::JerimumObject(Rc::new(RefCell::new(
                    JerimumObject::AwtColor {
                        red,
                        green,
                        blue,
                        alpha: 255,
                    },
                )))),
                _ => Err(vec![
                    self.error(format!("Cor não possui membro '{name}'"), span),
                ]),
            },
            JerimumObject::Image {
                handle,
                path,
                width,
                height,
            } => match name {
                "largura" => Ok(Value::Integer(width as i32)),
                "altura" => Ok(Value::Integer(height as i32)),
                "caminho" => Ok(Value::Text(path)),
                "buffer" => Ok(Value::JerimumObject(Rc::new(RefCell::new(
                    JerimumObject::ImageBuffer {
                        handle,
                        width,
                        height,
                    },
                )))),
                _ => Ok(Value::Function(Rc::new(Callable::Member {
                    receiver: Value::JerimumObject(object),
                    name: name.to_owned(),
                }))),
            },
            JerimumObject::Animation { speed, images, .. } => match name {
                "velocidade" => Ok(Value::Integer(speed)),
                "imagens" => self.copy_value(&Value::List(images), span),
                "imagem" => self.call_jerimum_object(object, name, Vec::new(), span),
                _ => {
                    Err(vec![self.error(
                        format!("Animacao não possui membro '{name}'"),
                        span,
                    )])
                }
            },
            JerimumObject::Font { size } if name == "tamanho" => Ok(Value::Integer(size)),
            JerimumObject::Sound { path, .. } if name == "arquivo" => Ok(Value::Text(path)),
            JerimumObject::Screen {
                id,
                title,
                width,
                height,
            } => match name {
                "frame" => Ok(Value::JerimumObject(Rc::new(RefCell::new(
                    JerimumObject::Frame {
                        screen_id: id,
                        title,
                        width,
                        height,
                    },
                )))),
                "canvas" => Ok(Value::JerimumObject(Rc::new(RefCell::new(
                    JerimumObject::Canvas {
                        screen_id: id,
                        width,
                        height,
                    },
                )))),
                _ => Err(vec![
                    self.error(format!("Tela não possui membro '{name}'"), span),
                ]),
            },
            JerimumObject::Sound { music, .. }
                if jerimum::object_property_type(
                    if music {
                        "JerimumType:Musica"
                    } else {
                        "JerimumType:Som"
                    },
                    name,
                )
                .is_some() =>
            {
                self.call_jerimum_object(object, name, Vec::new(), span)
            }
            _ => {
                let owner = jerimum_object_owner(&object.borrow());
                if jerimum::member_type(owner, name).is_some() {
                    Ok(Value::Function(Rc::new(Callable::Member {
                        receiver: Value::JerimumObject(object),
                        name: name.to_owned(),
                    })))
                } else {
                    Err(vec![self.error(
                        format!("membro Jerimum desconhecido: '{name}'"),
                        span,
                    )])
                }
            }
        }
    }

    fn call_jerimum_color_member(
        &self,
        red: i32,
        green: i32,
        blue: i32,
        alpha: i32,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if !arguments.is_empty() {
            return Err(vec![
                self.error("Color não recebe argumentos neste método", span),
            ]);
        }
        match name {
            "getRed" => Ok(Value::Integer(red)),
            "getGreen" => Ok(Value::Integer(green)),
            "getBlue" => Ok(Value::Integer(blue)),
            "getAlpha" => Ok(Value::Integer(alpha)),
            "getRGB" => Ok(Value::Integer(i32::from_be_bytes([
                alpha as u8,
                red as u8,
                green as u8,
                blue as u8,
            ]))),
            _ => Err(vec![
                self.error(format!("Color não possui método '{name}'"), span),
            ]),
        }
    }

    fn call_jerimum_frame_member(
        &self,
        title: &str,
        width: i32,
        height: i32,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if !arguments.is_empty() {
            return Err(vec![
                self.error("JFrame não recebe argumentos neste método", span),
            ]);
        }
        match name {
            "getTitle" => Ok(Value::Text(title.to_owned())),
            "getWidth" => Ok(Value::Integer(width)),
            "getHeight" => Ok(Value::Integer(height)),
            _ => Err(vec![
                self.error(format!("JFrame não possui método '{name}'"), span),
            ]),
        }
    }

    fn call_jerimum_canvas_member(
        &self,
        width: i32,
        height: i32,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if !arguments.is_empty() {
            return Err(vec![
                self.error("Canvas não recebe argumentos neste método", span),
            ]);
        }
        match name {
            "getWidth" => Ok(Value::Integer(width)),
            "getHeight" => Ok(Value::Integer(height)),
            _ => Err(vec![
                self.error(format!("Canvas não possui método '{name}'"), span),
            ]),
        }
    }

    fn call_jerimum_image_buffer_member(
        &mut self,
        handle: i64,
        width: i64,
        height: i64,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        match (name, arguments.as_slice()) {
            ("getWidth", []) => i32::try_from(width)
                .map(Value::Integer)
                .map_err(|_| vec![self.error("largura da imagem excede 32 bits", span)]),
            ("getHeight", []) => i32::try_from(height)
                .map(Value::Integer)
                .map_err(|_| vec![self.error("altura da imagem excede 32 bits", span)]),
            ("getRGB", [x, y]) => {
                let x = self.expect_integer(x, span)?;
                let y = self.expect_integer(y, span)?;
                let color = self.jerimum_graphics_integer(
                    "obter_cor_pixel",
                    vec![
                        JerimumValue::Integer(handle),
                        JerimumValue::Integer(i64::from(x)),
                        JerimumValue::Integer(i64::from(y)),
                    ],
                    span,
                )?;
                i32::try_from(color)
                    .map(Value::Integer)
                    .map_err(|_| vec![self.error("cor do pixel excede 32 bits", span)])
            }
            ("getSubimage", [x, y, frame_width, frame_height]) => {
                let x = self.expect_integer(x, span)?;
                let y = self.expect_integer(y, span)?;
                let frame_width = self.expect_integer(frame_width, span)?;
                let frame_height = self.expect_integer(frame_height, span)?;
                let x2 = i64::from(x) + i64::from(frame_width);
                let y2 = i64::from(y) + i64::from(frame_height);
                if x < 0
                    || y < 0
                    || frame_width <= 0
                    || frame_height <= 0
                    || x2 > width
                    || y2 > height
                {
                    return Err(vec![self.error("recorte fora dos limites da imagem", span)]);
                }
                let subimage = self.jerimum_graphics_integer(
                    "transformar_porcao_imagem",
                    vec![
                        JerimumValue::Integer(handle),
                        JerimumValue::Integer(i64::from(x)),
                        JerimumValue::Integer(i64::from(y)),
                        JerimumValue::Integer(i64::from(frame_width)),
                        JerimumValue::Integer(i64::from(frame_height)),
                        JerimumValue::Logical(false),
                        JerimumValue::Logical(false),
                        JerimumValue::Integer(0),
                        JerimumValue::Integer(0),
                    ],
                    span,
                )?;
                Ok(Value::JerimumObject(Rc::new(RefCell::new(
                    JerimumObject::ImageBuffer {
                        handle: subimage,
                        width: i64::from(frame_width),
                        height: i64::from(frame_height),
                    },
                ))))
            }
            _ => Err(vec![self.error(
                format!("BufferedImage não possui método '{name}' com esses argumentos"),
                span,
            )]),
        }
    }

    fn call_jerimum_graphics_context_member(
        &mut self,
        context: Rc<RefCell<JerimumObject>>,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let (red, green, blue, active) = match &*context.borrow() {
            JerimumObject::GraphicsContext {
                red,
                green,
                blue,
                active,
            } => (*red, *green, *blue, *active),
            _ => return Err(vec![self.error("contexto gráfico inválido", span)]),
        };
        if !active {
            return Err(vec![
                self.error("o contexto gráfico já foi encerrado", span),
            ]);
        }
        if name == "setColor" {
            let [Value::JerimumObject(color)] = arguments.as_slice() else {
                return Err(vec![self.error("setColor recebe uma cor", span)]);
            };
            let (red, green, blue) = match &*color.borrow() {
                JerimumObject::AwtColor {
                    red, green, blue, ..
                }
                | JerimumObject::Color { red, green, blue } => (*red, *green, *blue),
                _ => return Err(vec![self.error("setColor recebe uma cor", span)]),
            };
            let mut context = context.borrow_mut();
            let JerimumObject::GraphicsContext {
                red: context_red,
                green: context_green,
                blue: context_blue,
                ..
            } = &mut *context
            else {
                return Err(vec![self.error("contexto gráfico inválido", span)]);
            };
            *context_red = red;
            *context_green = green;
            *context_blue = blue;
            return Ok(Value::Nothing);
        }
        if name == "getColor" {
            if !arguments.is_empty() {
                return Err(vec![self.error("getColor não recebe argumentos", span)]);
            }
            return Ok(Value::JerimumObject(Rc::new(RefCell::new(
                JerimumObject::AwtColor {
                    red,
                    green,
                    blue,
                    alpha: 255,
                },
            ))));
        }

        let color = i64::from(u32::from_be_bytes([255, red as u8, green as u8, blue as u8]) as i32);
        self.jerimum_graphics("definir_cor", vec![JerimumValue::Integer(color)], span)?;
        match name {
            "drawLine" => {
                let [x1, y1, x2, y2] = arguments.as_slice() else {
                    return Err(vec![self.error("drawLine recebe quatro coordenadas", span)]);
                };
                self.jerimum_graphics(
                    "desenhar_linha",
                    vec![x1, y1, x2, y2]
                        .into_iter()
                        .map(|value| {
                            numeric_f64(value)
                                .map(JerimumValue::Real)
                                .ok_or_else(|| vec![self.error("coordenada inválida", span)])
                        })
                        .collect::<RuntimeResult<Vec<_>>>()?,
                    span,
                )?;
                Ok(Value::Nothing)
            }
            "drawRect" | "fillRect" => {
                let [x, y, width, height] = arguments.as_slice() else {
                    return Err(vec![self.error("retângulo recebe quatro medidas", span)]);
                };
                let coordinates = [x, y, width, height]
                    .into_iter()
                    .map(|value| {
                        numeric_f64(value)
                            .map(JerimumValue::Real)
                            .ok_or_else(|| vec![self.error("medida inválida", span)])
                    })
                    .collect::<RuntimeResult<Vec<_>>>()?;
                self.jerimum_graphics(
                    "desenhar_retangulo",
                    [
                        coordinates,
                        vec![
                            JerimumValue::Logical(false),
                            JerimumValue::Logical(name == "fillRect"),
                        ],
                    ]
                    .concat(),
                    span,
                )?;
                Ok(Value::Nothing)
            }
            "drawOval" | "fillOval" => {
                let [x, y, width, height] = arguments.as_slice() else {
                    return Err(vec![self.error("elipse recebe quatro medidas", span)]);
                };
                let coordinates = [x, y, width, height]
                    .into_iter()
                    .map(|value| {
                        numeric_f64(value)
                            .map(JerimumValue::Real)
                            .ok_or_else(|| vec![self.error("medida inválida", span)])
                    })
                    .collect::<RuntimeResult<Vec<_>>>()?;
                self.jerimum_graphics(
                    "desenhar_elipse",
                    [coordinates, vec![JerimumValue::Logical(name == "fillOval")]].concat(),
                    span,
                )?;
                Ok(Value::Nothing)
            }
            "drawString" => {
                let [Value::Text(text), x, y] = arguments.as_slice() else {
                    return Err(vec![self.error("drawString recebe texto e posição", span)]);
                };
                let x = numeric_f64(x)
                    .ok_or_else(|| vec![self.error("coordenada X inválida", span)])?;
                let y = numeric_f64(y)
                    .ok_or_else(|| vec![self.error("coordenada Y inválida", span)])?;
                self.jerimum_graphics(
                    "desenhar_texto",
                    vec![
                        JerimumValue::Real(x),
                        JerimumValue::Real(y),
                        JerimumValue::String(text.clone()),
                    ],
                    span,
                )?;
                Ok(Value::Nothing)
            }
            "drawImage" => {
                if !(3..=6).contains(&arguments.len()) {
                    return Err(vec![self.error("drawImage recebeu aridade inválida", span)]);
                }
                let Value::JerimumObject(image) = &arguments[0] else {
                    return Err(vec![self.error("drawImage recebe BufferedImage", span)]);
                };
                let (handle, image_width, image_height) = match &*image.borrow() {
                    JerimumObject::ImageBuffer {
                        handle,
                        width,
                        height,
                    }
                    | JerimumObject::Image {
                        handle,
                        width,
                        height,
                        ..
                    } => (*handle, *width, *height),
                    _ => return Err(vec![self.error("drawImage recebe BufferedImage", span)]),
                };
                let x = self.expect_integer(&arguments[1], span)?;
                let y = self.expect_integer(&arguments[2], span)?;
                if arguments.len() == 3 || arguments.len() == 4 {
                    self.jerimum_graphics(
                        "desenhar_imagem",
                        vec![
                            JerimumValue::Real(f64::from(x)),
                            JerimumValue::Real(f64::from(y)),
                            JerimumValue::Integer(handle),
                        ],
                        span,
                    )?;
                } else {
                    let width = self.expect_integer(&arguments[3], span)?;
                    let height = self.expect_integer(&arguments[4], span)?;
                    let scale_x = f64::from(width) / image_width as f64;
                    let scale_y = f64::from(height) / image_height as f64;
                    self.jerimum_graphics(
                        "jerimum_desenhar_imagem",
                        vec![
                            JerimumValue::Real(f64::from(x)),
                            JerimumValue::Real(f64::from(y)),
                            JerimumValue::Integer(handle),
                            JerimumValue::Real(0.0),
                            JerimumValue::Real(scale_x),
                            JerimumValue::Real(scale_y),
                        ],
                        span,
                    )?;
                }
                Ok(Value::Logical(true))
            }
            _ => {
                Err(vec![self.error(
                    format!("Graphics2D não possui método '{name}'"),
                    span,
                )])
            }
        }
    }

    fn call_jerimum_audio_object(
        &mut self,
        handle: i64,
        music: bool,
        name: &str,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let host_call = match (music, name, arguments.as_slice()) {
            (false, "toque", []) => (
                "reproduzir_som",
                vec![JerimumValue::Integer(handle), JerimumValue::Logical(false)],
            ),
            (false, "pare", []) => ("pausar_som", vec![JerimumValue::Integer(handle)]),
            (true, "pare", []) => ("interromper_som", vec![JerimumValue::Integer(handle)]),
            (true, "toque", []) => (
                "reproduzir_som",
                vec![JerimumValue::Integer(handle), JerimumValue::Logical(true)],
            ),
            (true, "toque", [Value::Logical(looped)]) => (
                "reproduzir_som",
                vec![
                    JerimumValue::Integer(handle),
                    JerimumValue::Logical(*looped),
                ],
            ),
            (true, "pause", []) => ("pausar_som", vec![JerimumValue::Integer(handle)]),
            (true, "continue", []) => ("continuar_som", vec![JerimumValue::Integer(handle)]),
            (true, "feche", []) => ("liberar_som", vec![JerimumValue::Integer(handle)]),
            (false, "tocando", []) | (true, "tocando", []) => {
                ("som_tocando", vec![JerimumValue::Integer(handle)])
            }
            (true, "pausado", []) => ("musica_pausada", vec![JerimumValue::Integer(handle)]),
            _ => {
                return Err(vec![self.error(
                    format!(
                        "chamada inválida para {}.{name}",
                        if music { "Musica" } else { "Som" }
                    ),
                    span,
                )]);
            }
        };
        let result = self.jerimum_request_value(
            JerimumRequest::Audio {
                function: host_call.0.to_owned(),
                arguments: host_call.1,
            },
            span,
        )?;
        if matches!(host_call.0, "som_tocando" | "musica_pausada") {
            Ok(result)
        } else {
            Ok(Value::Nothing)
        }
    }

    fn call_jerimum_game(
        &mut self,
        arguments: Vec<Value>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        if arguments.len() > 6 {
            return Err(vec![
                self.error("Jogo.iniciar recebe até seis argumentos", span),
            ]);
        }
        let title = match arguments.first() {
            Some(Value::Text(value)) => value.clone(),
            Some(_) => return Err(vec![self.error("o título do jogo deve ser Texto", span)]),
            None => "Potigol com Jerimum".to_owned(),
        };
        let width = match arguments.get(1) {
            Some(value) => self.expect_integer(value, span)?,
            None => 640,
        };
        let height = match arguments.get(2) {
            Some(value) => self.expect_integer(value, span)?,
            None => 480,
        };
        let fps = match arguments.get(5) {
            Some(value) => self.expect_integer(value, span)?,
            None => 60,
        };
        if fps <= 0 {
            return Err(vec![
                self.error("a taxa de quadros deve ser positiva", span),
            ]);
        }
        let update = arguments.get(3).cloned();
        let draw = arguments.get(4).cloned();
        self.jerimum_game_title = title.clone();
        self.jerimum_game_width = width;
        self.jerimum_game_height = height;
        self.jerimum_game_fps = fps;
        self.jerimum_start_window(&title, width, height, span)?;
        let frame_duration = std::time::Duration::from_secs_f64(1.0 / fps as f64);
        loop {
            if self.io.should_stop() {
                break;
            }
            self.consume_step(span)?;
            if !self.jerimum_request_logical(JerimumRequest::WindowIsOpen, span)? {
                break;
            }
            if let Some(update) = &update {
                self.apply(update.clone(), Vec::new(), span)?;
            }
            let black = i64::from(u32::from_be_bytes([255, 0, 0, 0]) as i32);
            self.jerimum_graphics("definir_cor", vec![JerimumValue::Integer(black)], span)?;
            self.jerimum_graphics("limpar", Vec::new(), span)?;
            self.flush_jerimum_drawings(span)?;
            self.jerimum_graphics("renderizar", Vec::new(), span)?;
            if let Some(draw) = &draw {
                self.apply(draw.clone(), Vec::new(), span)?;
            }
            if self.stepping {
                self.jerimum_graphics("definir_cor", vec![JerimumValue::Integer(black)], span)?;
                self.jerimum_graphics("limpar", Vec::new(), span)?;
                self.flush_jerimum_drawings(span)?;
                self.jerimum_graphics("renderizar", Vec::new(), span)?;
                break;
            }
            self.io
                .wait(frame_duration)
                .map_err(|message| vec![self.error(message, span)])?;
        }
        self.jerimum_drawings.clear();
        Ok(Value::Nothing)
    }

    fn jerimum_start_window(
        &mut self,
        title: &str,
        width: i32,
        height: i32,
        span: SourceSpan,
    ) -> RuntimeResult<()> {
        self.jerimum_graphics(
            "iniciar_modo_grafico",
            vec![JerimumValue::Logical(true)],
            span,
        )?;
        self.jerimum_graphics(
            "definir_dimensoes_janela",
            vec![
                JerimumValue::Integer(width as i64),
                JerimumValue::Integer(height as i64),
            ],
            span,
        )?;
        self.jerimum_graphics(
            "definir_titulo_janela",
            vec![JerimumValue::String(title.to_owned())],
            span,
        )?;
        Ok(())
    }

    fn load_jerimum_image(&mut self, path: &str, span: SourceSpan) -> RuntimeResult<Value> {
        self.check_string_size(path.len(), span)?;
        if let Some(image) = self.jerimum_image_cache.get(path).cloned() {
            return self.copy_value(&image, span);
        }
        let handle = self.jerimum_host_integer(
            JerimumRequest::Graphics {
                function: "carregar_imagem".to_owned(),
                arguments: vec![JerimumValue::String(path.to_owned())],
            },
            span,
        )?;
        let width = self.jerimum_graphics_integer(
            "largura_imagem",
            vec![JerimumValue::Integer(handle)],
            span,
        )?;
        let height = self.jerimum_graphics_integer(
            "altura_imagem",
            vec![JerimumValue::Integer(handle)],
            span,
        )?;
        let image = Value::JerimumObject(Rc::new(RefCell::new(JerimumObject::Image {
            path: path.to_owned(),
            handle,
            width,
            height,
        })));
        self.jerimum_image_cache
            .insert(path.to_owned(), image.clone());
        Ok(image)
    }

    fn slice_jerimum_image(
        &mut self,
        image: &Value,
        frame_width: i32,
        frame_height: i32,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let Value::JerimumObject(object) = image else {
            return Err(vec![self.error("fatie precisa de uma imagem", span)]);
        };
        let JerimumObject::Image {
            handle,
            width,
            height,
            ..
        } = &*object.borrow()
        else {
            return Err(vec![self.error("fatie precisa de uma imagem", span)]);
        };
        if frame_width <= 0 || frame_height <= 0 {
            return Err(vec![
                self.error("as dimensões dos quadros devem ser positivas", span),
            ]);
        }
        let columns = (*width / i64::from(frame_width)) as usize;
        let rows = (*height / i64::from(frame_height)) as usize;
        let count = rows
            .checked_mul(columns)
            .ok_or_else(|| vec![self.error("quantidade de quadros excede o limite", span)])?;
        self.reserve_elements(count, span)?;
        let (handle, width, height) = (*handle, *width, *height);
        let mut images = Vec::with_capacity(count);
        for y in (0..height).step_by(frame_height as usize) {
            if y + i64::from(frame_height) > height {
                break;
            }
            for x in (0..width).step_by(frame_width as usize) {
                if x + i64::from(frame_width) > width {
                    break;
                }
                let frame_handle = self.jerimum_graphics_integer(
                    "transformar_porcao_imagem",
                    vec![
                        JerimumValue::Integer(handle),
                        JerimumValue::Integer(x),
                        JerimumValue::Integer(y),
                        JerimumValue::Integer(i64::from(frame_width)),
                        JerimumValue::Integer(i64::from(frame_height)),
                        JerimumValue::Logical(false),
                        JerimumValue::Logical(false),
                        JerimumValue::Integer(0),
                        JerimumValue::Integer(0),
                    ],
                    span,
                )?;
                images.push(Value::JerimumObject(Rc::new(RefCell::new(
                    JerimumObject::Image {
                        path: String::new(),
                        handle: frame_handle,
                        width: i64::from(frame_width),
                        height: i64::from(frame_height),
                    },
                ))));
            }
        }
        Ok(Value::List(images))
    }

    fn enqueue_jerimum_drawing(
        &mut self,
        z: i32,
        color: Option<Value>,
        function: &str,
        arguments: Vec<JerimumValue>,
    ) {
        self.jerimum_drawings
            .entry(z)
            .or_default()
            .push(JerimumDrawItem::Command(JerimumDrawCommand {
                color,
                function: function.to_owned(),
                arguments,
            }));
    }

    fn flush_jerimum_drawings(&mut self, span: SourceSpan) -> RuntimeResult<()> {
        let context = Value::JerimumObject(Rc::new(RefCell::new(JerimumObject::GraphicsContext {
            red: 0,
            green: 0,
            blue: 0,
            active: true,
        })));
        let result = self.flush_jerimum_drawings_with_context(context.clone(), span);
        if let Value::JerimumObject(object) = context
            && let JerimumObject::GraphicsContext { active, .. } = &mut *object.borrow_mut()
        {
            *active = false;
        }
        result
    }

    fn flush_jerimum_drawings_with_context(
        &mut self,
        context: Value,
        span: SourceSpan,
    ) -> RuntimeResult<()> {
        let drawings = std::mem::take(&mut self.jerimum_drawings);
        for items in drawings.into_values() {
            for item in items.iter().rev() {
                match item {
                    JerimumDrawItem::Callback(callback) => {
                        let result = self.apply(callback.clone(), vec![context.clone()], span);
                        result?;
                    }
                    JerimumDrawItem::Command(command) => {
                        if let Some(color) = &command.color {
                            let packed = jerimum_color_value(color, span)
                                .ok_or_else(|| vec![self.error("valor Cor inválido", span)])?;
                            self.jerimum_graphics(
                                "definir_cor",
                                vec![JerimumValue::Integer(packed)],
                                span,
                            )?;
                        }
                        self.jerimum_graphics(&command.function, command.arguments.clone(), span)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn jerimum_request_value(
        &mut self,
        request: JerimumRequest,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        let value = self
            .io
            .jerimum_request(request)
            .map_err(|message| vec![self.error(message, span)])?;
        self.jerimum_value(value, span)
    }

    fn jerimum_value(&mut self, value: JerimumValue, span: SourceSpan) -> RuntimeResult<Value> {
        match value {
            JerimumValue::Null => Ok(Value::Nothing),
            JerimumValue::Integer(value) => i32::try_from(value)
                .map(Value::Integer)
                .map_err(|_| vec![self.error("inteiro Jerimum excede 32 bits", span)]),
            JerimumValue::Real(value) => Ok(Value::Real(value)),
            JerimumValue::Logical(value) => Ok(Value::Logical(value)),
            JerimumValue::String(value) => {
                self.check_string_size(value.len(), span)?;
                Ok(Value::Text(value))
            }
            JerimumValue::Array(values) => {
                self.reserve_elements(values.len(), span)?;
                let values = values
                    .into_iter()
                    .map(|value| self.jerimum_value(value, span))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                Ok(Value::List(values))
            }
        }
    }

    fn jerimum_host_integer(
        &mut self,
        request: JerimumRequest,
        span: SourceSpan,
    ) -> RuntimeResult<i64> {
        match self
            .io
            .jerimum_request(request)
            .map_err(|message| vec![self.error(message, span)])?
        {
            JerimumValue::Integer(value) => Ok(value),
            _ => Err(vec![
                self.error("resposta inteira inválida do host Jerimum", span),
            ]),
        }
    }

    fn jerimum_request_logical(
        &mut self,
        request: JerimumRequest,
        span: SourceSpan,
    ) -> RuntimeResult<bool> {
        match self
            .io
            .jerimum_request(request)
            .map_err(|message| vec![self.error(message, span)])?
        {
            JerimumValue::Logical(value) => Ok(value),
            _ => Err(vec![
                self.error("resposta lógica inválida do host Jerimum", span),
            ]),
        }
    }

    fn jerimum_graphics(
        &mut self,
        function: &str,
        arguments: Vec<JerimumValue>,
        span: SourceSpan,
    ) -> RuntimeResult<Value> {
        self.jerimum_request_value(
            JerimumRequest::Graphics {
                function: function.to_owned(),
                arguments,
            },
            span,
        )
    }

    fn jerimum_graphics_integer(
        &mut self,
        function: &str,
        arguments: Vec<JerimumValue>,
        span: SourceSpan,
    ) -> RuntimeResult<i64> {
        self.jerimum_host_integer(
            JerimumRequest::Graphics {
                function: function.to_owned(),
                arguments,
            },
            span,
        )
    }
}

fn jerimum_object_owner(object: &JerimumObject) -> &'static str {
    match object {
        JerimumObject::Color { .. } => "JerimumType:Cor",
        JerimumObject::AwtColor { .. } => "JerimumType:AwtColor",
        JerimumObject::Image { .. } => "JerimumType:Imagem",
        JerimumObject::ImageBuffer { .. } => "JerimumType:BufferedImage",
        JerimumObject::Animation { .. } => "JerimumType:Animacao",
        JerimumObject::Font { .. } => "JerimumType:Fonte",
        JerimumObject::Sound { music: true, .. } => "JerimumType:Musica",
        JerimumObject::Sound { music: false, .. } => "JerimumType:Som",
        JerimumObject::Screen { .. } => "JerimumType:Tela",
        JerimumObject::Frame { .. } => "JerimumType:Frame",
        JerimumObject::Canvas { .. } => "JerimumType:Canvas",
        JerimumObject::GraphicsContext { .. } => "JerimumType:Graphics2D",
    }
}

fn white_jerimum_color() -> Value {
    Value::JerimumObject(Rc::new(RefCell::new(JerimumObject::Color {
        red: 255,
        green: 255,
        blue: 255,
    })))
}

fn jerimum_color_value(value: &Value, span: SourceSpan) -> Option<i64> {
    let Value::JerimumObject(object) = value else {
        return None;
    };
    let color = object.borrow();
    let (red, green, blue) = match &*color {
        JerimumObject::Color { red, green, blue }
        | JerimumObject::AwtColor {
            red, green, blue, ..
        } => (red, green, blue),
        _ => return None,
    };
    if !(0..=255).contains(red) || !(0..=255).contains(green) || !(0..=255).contains(blue) {
        return None;
    }
    let _ = span;
    Some(i64::from(
        u32::from_be_bytes([255, *red as u8, *green as u8, *blue as u8]) as i32,
    ))
}

fn jerimum_key_code(name: &str) -> Option<i64> {
    match name {
        "TECLA_ESPACO" | "TECLA_ESPAÇO" => Some(32),
        "TECLA_ENTER" => Some(10),
        "TECLA_PARA_CIMA" => Some(38),
        "TECLA_PARA_BAIXO" => Some(40),
        "TECLA_PARA_ESQUERDA" => Some(37),
        "TECLA_PARA_DIREITA" => Some(39),
        _ => name
            .strip_prefix("TECLA_")
            .and_then(|key| (key.len() == 1).then(|| key.as_bytes()[0]))
            .filter(u8::is_ascii_alphanumeric)
            .map(i64::from),
    }
}

fn value_direct_payload_bytes(value: &Value) -> usize {
    match value {
        Value::BigInteger(value) | Value::Text(value) | Value::TypeName(value) => value.len(),
        Value::Option(Some(value)) => {
            std::mem::size_of::<Value>() + value_direct_payload_bytes(value)
        }
        Value::FieldReference(_, name) => name.len(),
        _ => 0,
    }
}

fn value_clone_bytes(value: &Value) -> Option<usize> {
    match value {
        Value::BigInteger(value) | Value::Text(value) | Value::TypeName(value) => Some(value.len()),
        Value::Tuple(values) | Value::List(values) => values
            .len()
            .checked_mul(std::mem::size_of::<Value>())
            .and_then(|bytes| {
                values.iter().try_fold(bytes, |total, value| {
                    total.checked_add(value_clone_bytes(value)?)
                })
            }),
        Value::Option(Some(value)) => {
            std::mem::size_of::<Value>().checked_add(value_clone_bytes(value)?)
        }
        Value::FieldReference(_, name) => Some(name.len()),
        Value::Integer(_)
        | Value::Real(_)
        | Value::Character(_)
        | Value::Logical(_)
        | Value::Nothing
        | Value::Option(None)
        | Value::MutableList(_)
        | Value::Function(_)
        | Value::Object(_)
        | Value::JerimumObject(_) => Some(0),
    }
}

fn value_clone_elements(value: &Value) -> Option<usize> {
    match value {
        Value::Tuple(values) | Value::List(values) => {
            values.iter().try_fold(values.len(), |total, value| {
                total.checked_add(value_clone_elements(value)?)
            })
        }
        Value::Option(Some(value)) => value_clone_elements(value),
        Value::Integer(_)
        | Value::Real(_)
        | Value::BigInteger(_)
        | Value::Text(_)
        | Value::Character(_)
        | Value::Logical(_)
        | Value::Nothing
        | Value::Option(None)
        | Value::MutableList(_)
        | Value::Function(_)
        | Value::TypeName(_)
        | Value::Object(_)
        | Value::JerimumObject(_)
        | Value::FieldReference(_, _) => Some(0),
    }
}

fn normalize_index(index: i32, length: usize) -> Option<usize> {
    let length = i32::try_from(length).ok()?;
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
        Value::Character(value) => Some(f64::from(*value)),
        _ => None,
    }
}

fn numericize_character(value: Value) -> Value {
    match value {
        Value::Character(character) => Value::Integer(i32::from(character)),
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
        Value::BigInteger(_) => "InteiroGrande".to_owned(),
        Value::Text(_) => "Texto".to_owned(),
        Value::Character(_) => "Character".to_owned(),
        Value::Logical(_) => "Logico".to_owned(),
        Value::Nothing => "BoxedUnit".to_owned(),
        Value::Tuple(_) => "Tupla".to_owned(),
        Value::List(_) => "Lista".to_owned(),
        Value::MutableList(_) => "Vetor".to_owned(),
        Value::Option(Some(_)) => "Some".to_owned(),
        Value::Option(None) => "None".to_owned(),
        Value::Function(_) => "Função".to_owned(),
        Value::TypeName(name) => name.clone(),
        Value::Object(object) => object.borrow().class_name.clone(),
        Value::JerimumObject(object) => match &*object.borrow() {
            JerimumObject::Color { .. } => "Cor".to_owned(),
            JerimumObject::AwtColor { .. } => "Color".to_owned(),
            JerimumObject::Image { .. } => "Imagem".to_owned(),
            JerimumObject::ImageBuffer { .. } => "BufferedImage".to_owned(),
            JerimumObject::Animation { .. } => "Animacao".to_owned(),
            JerimumObject::Font { .. } => "Fonte".to_owned(),
            JerimumObject::Sound { music: true, .. } => "Musica".to_owned(),
            JerimumObject::Sound { music: false, .. } => "Som".to_owned(),
            JerimumObject::Screen { .. } => "Tela".to_owned(),
            JerimumObject::Frame { .. } => "JFrame".to_owned(),
            JerimumObject::Canvas { .. } => "Canvas".to_owned(),
            JerimumObject::GraphicsContext { .. } => "Graphics2D".to_owned(),
        },
        Value::FieldReference(_, _) => "Atributo".to_owned(),
    }
}

fn utf16_units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

fn text_from_utf16(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

fn unit_subslice_position(haystack: &[u16], needle: &[u16]) -> Option<usize> {
    if needle.is_empty() {
        Some(0)
    } else {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
    }
}

fn join_utf16_units(units: &[u16], separator: &str) -> String {
    let separator = utf16_units(separator);
    let mut joined =
        Vec::with_capacity(units.len() + separator.len() * units.len().saturating_sub(1));
    for (index, unit) in units.iter().enumerate() {
        if index != 0 {
            joined.extend_from_slice(&separator);
        }
        joined.push(*unit);
    }
    text_from_utf16(&joined)
}

fn jvm_math_round(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

fn parse_integer_prefix(source: &str) -> i32 {
    let bytes = source.as_bytes();
    let mut end = if bytes.first().is_some_and(|byte| *byte == b'-') {
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

fn parse_integer_input(source: &str) -> Result<i32, ()> {
    let bytes = source.as_bytes();
    let mut end = usize::from(
        bytes
            .first()
            .is_some_and(|byte| matches!(byte, b'+' | b'-')),
    );
    let digits_start = end;
    while bytes.get(end).is_some_and(u8::is_ascii_digit) {
        end += 1;
    }
    if end == digits_start {
        Err(())
    } else {
        source[..end].parse().map_err(|_| ())
    }
}

fn parse_real_input(source: &str) -> Result<f64, ()> {
    let bytes = source.as_bytes();
    let mut end = usize::from(
        bytes
            .first()
            .is_some_and(|byte| matches!(byte, b'+' | b'-')),
    );
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
    if has_digits {
        source[..end].parse().map_err(|_| ())
    } else {
        Err(())
    }
}

fn parse_real_prefix(source: &str) -> f64 {
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
        (Value::Character(left), Value::Integer(right)) => Some(i32::from(*left).cmp(right)),
        (Value::Integer(left), Value::Character(right)) => Some(left.cmp(&i32::from(*right))),
        (Value::Character(left), Value::Real(right)) => f64::from(*left).partial_cmp(right),
        (Value::Real(left), Value::Character(right)) => left.partial_cmp(&f64::from(*right)),
        (Value::Logical(left), Value::Logical(right)) => Some(left.cmp(right)),
        (Value::BigInteger(left), Value::BigInteger(right)) => Some(bigint::compare(left, right)),
        (Value::Integer(left), Value::BigInteger(right)) => {
            Some(bigint::compare(&left.to_string(), right))
        }
        (Value::BigInteger(left), Value::Integer(right)) => {
            Some(bigint::compare(left, &right.to_string()))
        }
        (Value::BigInteger(left), Value::Character(right)) => {
            Some(bigint::compare(left, &i32::from(*right).to_string()))
        }
        (Value::Character(left), Value::BigInteger(right)) => {
            Some(bigint::compare(&i32::from(*left).to_string(), right))
        }
        (Value::BigInteger(left), Value::Real(right)) => compare_big_integer_real(left, *right),
        (Value::Real(left), Value::BigInteger(right)) => {
            compare_big_integer_real(right, *left).map(std::cmp::Ordering::reverse)
        }
        (Value::Character(left), Value::Character(right)) => Some(left.cmp(right)),
        (Value::Text(left), Value::Text(right)) => {
            Some(left.encode_utf16().cmp(right.encode_utf16()))
        }
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

fn compare_big_integer_real(integer: &str, real: f64) -> Option<std::cmp::Ordering> {
    if real.is_nan() {
        return None;
    }
    if real.is_infinite() {
        return Some(if real.is_sign_positive() {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        });
    }
    let truncated = real.trunc();
    let truncated = if truncated == 0.0 {
        "0".to_owned()
    } else {
        format!("{truncated:.0}")
    };
    let order = bigint::compare(integer, &truncated);
    Some(if order.is_eq() && real.fract() != 0.0 {
        if real.is_sign_positive() {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    } else {
        order
    })
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

fn make_nested_list(sizes: &[usize], initial: &Value) -> Vec<Value> {
    let Some((&size, remainder)) = sizes.split_first() else {
        return Vec::new();
    };
    if remainder.is_empty() {
        (0..size).map(|_| initial.clone()).collect()
    } else {
        (0..size)
            .map(|_| Value::List(make_nested_list(remainder, initial)))
            .collect()
    }
}

fn nested_list_counts(sizes: &[usize]) -> Option<(usize, usize)> {
    sizes
        .iter()
        .try_fold((1usize, 0usize), |(product, total), size| {
            let product = product.checked_mul(*size)?;
            Some((product, total.checked_add(product)?))
        })
}

fn normalize_default_text_split(text: &str) -> String {
    let mut normalized = String::with_capacity(text.len());
    let mut in_separator = false;
    for character in text.chars() {
        if matches!(character, ' ' | '\n') {
            if !in_separator {
                normalized.push(' ');
                in_separator = true;
            }
        } else {
            normalized.push(character);
            in_separator = false;
        }
    }
    normalized
}

fn split_text_regex(text: &str, pattern: &str) -> Option<Vec<String>> {
    if text.is_empty() {
        return Some(vec![String::new()]);
    }
    let mut builder = RegexBuilder::new(pattern);
    // shortcut: backtracking is capped at one million steps; raise only with an equivalent execution budget.
    builder.backtrack_limit(1_000_000);
    let regex = builder.build().ok()?;
    let mut pieces = Vec::new();
    let mut field_start = 0;
    let mut found_match = false;
    for result in regex.find_iter(text) {
        let separator = result.ok()?;
        if separator.start() == 0 && separator.end() == 0 && !found_match {
            continue;
        }
        pieces.push(text[field_start..separator.start()].to_owned());
        field_start = separator.end();
        found_match = true;
    }
    if !found_match {
        return Some(vec![text.to_owned()]);
    }
    pieces.push(text[field_start..].to_owned());
    while pieces.last().is_some_and(String::is_empty) {
        pieces.pop();
    }
    Some(pieces)
}

fn format_value(value: &Value, format: &str, maximum_bytes: usize) -> Result<Option<String>, ()> {
    let Some(start) = format.find('%') else {
        return Ok(None);
    };
    let (prefix, rest) = format.split_at(start);
    let Some((conversion_index, _)) = rest
        .char_indices()
        .skip(1)
        .find(|(_, character)| matches!(character, 'd' | 'i' | 'f' | 's'))
    else {
        return Ok(None);
    };
    let Some(conversion) = rest[conversion_index..].chars().next() else {
        return Ok(None);
    };
    let suffix = &rest[conversion_index + conversion.len_utf8()..];
    let specification = &rest[1..conversion_index];
    let left_aligned = specification.contains('-');
    let zero_padded = specification.contains('0') && !left_aligned;
    let width_text = specification
        .split_once('.')
        .map_or(specification, |(width, _)| width);
    let width_digits = width_text
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>();
    let width = if width_digits.is_empty() {
        0
    } else {
        width_digits.parse::<usize>().map_err(|_| ())?
    };
    let precision_text = specification.split_once('.').map(|(_, precision)| {
        precision
            .chars()
            .take_while(char::is_ascii_digit)
            .collect::<String>()
    });
    let precision = precision_text
        .filter(|precision| !precision.is_empty())
        .map(|precision| precision.parse::<usize>().map_err(|_| ()))
        .transpose()?;
    if precision.is_some_and(|precision| precision > maximum_bytes) {
        return Err(());
    }
    let raw = match conversion {
        'd' | 'i' => match value {
            Value::Integer(value) => value.to_string(),
            Value::BigInteger(value) => value.clone(),
            Value::Real(value) => (*value as i64).to_string(),
            _ => return Ok(None),
        },
        'f' => {
            let Some(number) = numeric_f64(value) else {
                return Ok(None);
            };
            match precision {
                Some(precision) => format_fixed_real(number, precision, maximum_bytes)?,
                None => number.to_string(),
            }
        }
        's' => match value {
            Value::Logical(value) => value.to_string(),
            _ => value.display(),
        },
        _ => return Ok(None),
    };
    let padding = width.saturating_sub(raw.chars().count());
    let output_bytes = prefix
        .len()
        .checked_add(raw.len())
        .and_then(|size| size.checked_add(padding))
        .and_then(|size| size.checked_add(suffix.len()))
        .ok_or(())?;
    if output_bytes > maximum_bytes {
        return Err(());
    }
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
    Ok(Some(format!("{prefix}{padded}{suffix}")))
}

fn format_fixed_real(number: f64, precision: usize, maximum_bytes: usize) -> Result<String, ()> {
    if !number.is_finite() {
        return Ok(number.to_string());
    }

    let shortest = number.to_string();
    let (negative, shortest) = shortest
        .strip_prefix('-')
        .map_or((false, shortest.as_str()), |digits| (true, digits));
    let (mantissa, exponent) =
        shortest
            .split_once(['e', 'E'])
            .map_or(Ok((shortest, 0)), |(mantissa, exponent)| {
                exponent
                    .parse::<i64>()
                    .map(|exponent| (mantissa, exponent))
                    .map_err(|_| ())
            })?;
    let (integer, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut digits = integer.bytes().chain(fraction.bytes()).collect::<Vec<_>>();
    let decimal_exponent = exponent
        .checked_sub(i64::try_from(fraction.len()).map_err(|_| ())?)
        .ok_or(())?;
    let decimal_point = i64::try_from(digits.len())
        .map_err(|_| ())?
        .checked_add(decimal_exponent)
        .ok_or(())?;
    let integer_digits = usize::try_from(decimal_point.max(1)).map_err(|_| ())?;
    let decimal_bytes = if precision == 0 {
        0
    } else {
        precision.checked_add(1).ok_or(())?
    };
    let minimum_output_bytes = usize::from(negative)
        .checked_add(integer_digits)
        .and_then(|size| size.checked_add(decimal_bytes))
        .ok_or(())?;
    if minimum_output_bytes > maximum_bytes {
        return Err(());
    }

    let scale = decimal_exponent
        .checked_add(i64::try_from(precision).map_err(|_| ())?)
        .ok_or(())?;
    let mut rounded = if scale >= 0 {
        digits.resize(
            digits
                .len()
                .checked_add(usize::try_from(scale).map_err(|_| ())?)
                .ok_or(())?,
            b'0',
        );
        digits
    } else {
        let discarded_count = usize::try_from(scale.checked_neg().ok_or(())?).map_err(|_| ())?;
        let retained_count = digits.len().saturating_sub(discarded_count);
        let discarded = if discarded_count < digits.len() {
            &digits[retained_count..]
        } else {
            &[]
        };
        let mut retained = if retained_count == 0 {
            vec![b'0']
        } else {
            digits[..retained_count].to_vec()
        };
        let first_discarded = discarded.first().copied().unwrap_or(b'0');
        let remaining_nonzero = discarded
            .get(1..)
            .is_some_and(|digits| digits.iter().any(|digit| *digit != b'0'));
        let round_up = first_discarded > b'5'
            || (first_discarded == b'5'
                && (remaining_nonzero || retained.last().is_some_and(|digit| digit % 2 == 1)));
        if round_up {
            increment_decimal_digits(&mut retained);
        }
        retained
    };

    let minimum_digits = precision.checked_add(1).ok_or(())?;
    if rounded.len() < minimum_digits {
        rounded.splice(
            0..0,
            std::iter::repeat(b'0').take(minimum_digits - rounded.len()),
        );
    }
    let output_bytes = usize::from(negative)
        .checked_add(rounded.len())
        .and_then(|size| size.checked_add(usize::from(precision > 0)))
        .ok_or(())?;
    if output_bytes > maximum_bytes {
        return Err(());
    }

    let digits = String::from_utf8(rounded).map_err(|_| ())?;
    let raw = if precision == 0 {
        digits
    } else {
        let split = digits.len() - precision;
        format!("{}.{}", &digits[..split], &digits[split..])
    };
    Ok(if negative { format!("-{raw}") } else { raw })
}

fn increment_decimal_digits(digits: &mut Vec<u8>) {
    for digit in digits.iter_mut().rev() {
        if *digit < b'9' {
            *digit += 1;
            return;
        }
        *digit = b'0';
    }
    digits.insert(0, b'1');
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
        ExprKind::Lambda { body, .. } => match body {
            FunctionBody::Expression(expression) => count_placeholders(expression),
            FunctionBody::Block(block) => count_block_placeholders(block),
            FunctionBody::Abstract => 0,
        },
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
        ExprKind::Comprehension { generators, body } => {
            generators
                .iter()
                .map(count_generator_placeholders)
                .sum::<usize>()
                + count_block_placeholders(body)
        }
        ExprKind::Block(block) => count_block_placeholders(block),
        ExprKind::While { condition, body } => {
            count_placeholders(condition) + count_block_placeholders(body)
        }
        ExprKind::For { generators, body } => {
            generators
                .iter()
                .map(count_generator_placeholders)
                .sum::<usize>()
                + count_block_placeholders(body)
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
            StmtKind::Block(block) => count_block_placeholders(block),
            StmtKind::Function(function) => match &function.body {
                FunctionBody::Expression(expression) => count_placeholders(expression),
                FunctionBody::Block(block) => count_block_placeholders(block),
                FunctionBody::Abstract => 0,
            },
            StmtKind::Type(declaration) => declaration
                .fields
                .iter()
                .filter_map(|field| field.default_value.as_ref())
                .map(count_placeholders)
                .sum(),
            StmtKind::TypeAlias { .. } | StmtKind::Import(_) => 0,
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
        "leia_real" | "leia_numero" | "leia_número" => Some(DataType::Real),
        "leia_texto" => Some(DataType::Text),
        "leia_caractere" => Some(DataType::Character),
        "leia_logico" => Some(DataType::Logical),
        _ => None,
    }
}

fn input_list_data_type(name: &str) -> Option<DataType> {
    match name {
        "leia_inteiros" => Some(DataType::Integer),
        "leia_reais" | "leia_numeros" | "leia_números" => Some(DataType::Real),
        "leia_textos" => Some(DataType::Text),
        _ => None,
    }
}

#[cfg(test)]
mod format_value_tests {
    use super::{format_value, value_clone_bytes, value_direct_payload_bytes};
    use crate::value::Value;

    #[test]
    fn format_width_and_precision_respect_the_string_limit_before_padding() {
        assert_eq!(
            format_value(&Value::Integer(7), "%1000000000d", 16),
            Err(())
        );
        assert_eq!(
            format_value(&Value::Real(1.25), "%.1000000000f", 16),
            Err(())
        );
        assert_eq!(
            format_value(&Value::Integer(7), "%4d", 16),
            Ok(Some("   7".to_owned()))
        );
    }

    #[test]
    fn moving_a_nested_collection_does_not_count_its_payload_twice() {
        let nested = Value::List(vec![Value::Integer(1), Value::Text("a".to_owned())]);

        assert_eq!(value_direct_payload_bytes(&nested), 0);
        assert_eq!(
            value_clone_bytes(&nested),
            Some(std::mem::size_of::<Value>() * 2 + 1)
        );
    }
}

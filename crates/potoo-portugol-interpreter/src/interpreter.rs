use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::HashMap,
    f64::consts::PI,
    rc::Rc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    ControlAction, Diagnostic, DiagnosticKind, ExecutionOptions, ExecutionPoint, HostValue,
    HttpRequest, RuntimeIo, SourceLocation, VariableSnapshot,
    ast::{
        AssignmentOperator, BinaryOperator, Expression, ExpressionKind, Function, Library,
        PostfixOperator, Program, Statement, StatementKind, Type, UnaryOperator, Variable,
    },
    objects::{ObjectStore, PropertyValue},
};

const MAX_CALL_DEPTH: usize = 32;
const MAX_ARRAY_DIMENSIONS: usize = 16;
const MAX_HTTP_RESPONSE_BYTES: usize = 10 * 1024 * 1024;

#[derive(Clone, Debug)]
struct CustomHttpRequest {
    url: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

#[derive(Clone, Debug)]
enum Value {
    Integer(i64),
    Real(f64),
    Logical(bool),
    Character(char),
    String(String),
    Array(Rc<RefCell<Vec<Rc<RefCell<Value>>>>>),
    Null,
}

impl Value {
    fn default_for(ty: Type) -> Self {
        match ty {
            Type::Integer => Self::Integer(0),
            Type::Real => Self::Real(0.0),
            Type::Logical => Self::Logical(false),
            Type::Character => Self::Character('\0'),
            Type::String => Self::String(String::new()),
            Type::Void => Self::Null,
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Self::Integer(_) => "inteiro",
            Self::Real(_) => "real",
            Self::Logical(_) => "logico",
            Self::Character(_) => "caracter",
            Self::String(_) => "cadeia",
            Self::Array(_) => "vetor",
            Self::Null => "vazio",
        }
    }

    fn as_number(&self, expression: &Expression) -> Result<f64, Diagnostic> {
        match self {
            Self::Integer(value) => Ok(*value as f64),
            Self::Real(value) => Ok(*value),
            value => Err(runtime_error(
                expression,
                format!("esperado um número, recebido {}", value.type_name()),
            )),
        }
    }

    fn as_integer(&self, expression: &Expression) -> Result<i64, Diagnostic> {
        match self {
            Self::Integer(value) => Ok(*value),
            value => Err(runtime_error(
                expression,
                format!("esperado um inteiro, recebido {}", value.type_name()),
            )),
        }
    }

    fn as_logical(&self, expression: &Expression) -> Result<bool, Diagnostic> {
        match self {
            Self::Logical(value) => Ok(*value),
            value => Err(runtime_error(
                expression,
                format!("esperado um valor lógico, recebido {}", value.type_name()),
            )),
        }
    }

    fn to_text(&self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Real(value) => java_double_to_string(*value),
            Self::Logical(true) => "verdadeiro".to_owned(),
            Self::Logical(false) => "falso".to_owned(),
            Self::Character(value) => value.to_string(),
            Self::String(value) => value.clone(),
            Self::Array(_) => "[vetor]".to_owned(),
            Self::Null => String::new(),
        }
    }

    fn matches_type(&self, ty: Type) -> bool {
        matches!(
            (self, ty),
            (Self::Integer(_), Type::Integer)
                | (Self::Real(_), Type::Real)
                | (Self::Integer(_), Type::Real)
                | (Self::Logical(_), Type::Logical)
                | (Self::Character(_), Type::Character)
                | (Self::String(_), Type::String)
        )
    }

    fn coerce_to(self, ty: Type, expression: &Expression) -> Result<Self, Diagnostic> {
        match (self, ty) {
            (Self::Integer(value), Type::Real) => Ok(Self::Real(value as f64)),
            (value, ty) if value.matches_type(ty) => Ok(value),
            (value, ty) => Err(runtime_error(
                expression,
                format!(
                    "não é possível atribuir {} a uma variável do tipo {}",
                    value.type_name(),
                    type_name(ty)
                ),
            )),
        }
    }
}

#[derive(Clone, Debug)]
struct Binding {
    ty: Type,
    dimensions: usize,
    is_constant: bool,
    value: Rc<RefCell<Value>>,
}

#[derive(Clone, Debug)]
struct PreparedTarget {
    value: Rc<RefCell<Value>>,
    ty: Type,
    dimensions: usize,
    is_constant: bool,
}

#[derive(Clone, Debug)]
enum PreparedArgument {
    Value {
        value: Value,
        ty: Type,
        dimensions: usize,
    },
    Reference {
        value: Rc<RefCell<Value>>,
        ty: Type,
        dimensions: usize,
    },
}

#[derive(Clone, Debug)]
enum Flow {
    Normal,
    Return(Value),
    Break,
    Stop,
}

pub struct Interpreter<'a> {
    libraries: Vec<Library>,
    functions: HashMap<String, Vec<Function>>,
    global_statements: Vec<Statement>,
    scopes: Vec<HashMap<String, Binding>>,
    function_scope_indices: Vec<usize>,
    io: &'a mut dyn RuntimeIo,
    steps: u64,
    step_limit: u64,
    output_bytes: usize,
    output_limit: usize,
    string_limit: usize,
    wait_limit_millis: u64,
    wait_millis_used: u64,
    array_elements_allocated: usize,
    array_element_limit: usize,
    call_depth: usize,
    util_start_time: SystemTime,
    random_state: u64,
    objects: ObjectStore,
    internet_timeout_millis: u64,
    custom_http_request: Option<CustomHttpRequest>,
    step_next: bool,
    stopped: bool,
}

impl<'a> Interpreter<'a> {
    pub fn new(program: Program, io: &'a mut dyn RuntimeIo, options: ExecutionOptions) -> Self {
        let mut functions: HashMap<String, Vec<Function>> = HashMap::new();
        for function in program.functions {
            functions
                .entry(function.name.clone())
                .or_default()
                .push(function);
        }
        let random_seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let random_state = (random_seed ^ 0x5DEECE66D) & ((1_u64 << 48) - 1);
        Self {
            libraries: program.libraries,
            functions,
            global_statements: program.statements,
            scopes: vec![HashMap::new()],
            function_scope_indices: Vec::new(),
            io,
            steps: 0,
            step_limit: options.max_steps,
            output_bytes: 0,
            output_limit: options.max_output_bytes,
            string_limit: options.max_string_bytes,
            wait_limit_millis: options.max_wait_millis,
            wait_millis_used: 0,
            array_elements_allocated: 0,
            array_element_limit: options.max_array_elements,
            call_depth: 0,
            util_start_time: SystemTime::now(),
            random_state: random_state.max(1),
            objects: ObjectStore::default(),
            internet_timeout_millis: 2_000,
            custom_http_request: None,
            step_next: false,
            stopped: false,
        }
    }

    pub fn execute(mut self) -> Result<(), Vec<Diagnostic>> {
        self.run()
            .map(|_| ())
            .map_err(|diagnostic| vec![diagnostic])
    }

    fn run(&mut self) -> Result<Value, Diagnostic> {
        match self.execute_statements(&self.global_statements.clone())? {
            Flow::Normal => {}
            Flow::Return(_) => {
                return Err(Diagnostic::new(
                    DiagnosticKind::Runtime,
                    "'retorne' não pode aparecer fora de uma função",
                    1,
                    1,
                ));
            }
            Flow::Break => {
                return Err(Diagnostic::new(
                    DiagnosticKind::Runtime,
                    "'pare' não pode aparecer fora de um laço ou 'escolha'",
                    1,
                    1,
                ));
            }
            Flow::Stop => return Ok(Value::Null),
        }
        if !self.functions.contains_key("inicio") {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                "função 'inicio' não encontrada",
                1,
                1,
            ));
        }
        self.call_function("inicio", &[], 1, 1)
    }

    fn execute_statement(&mut self, statement: &Statement) -> Result<Flow, Diagnostic> {
        if self.stopped {
            return Ok(Flow::Stop);
        }
        self.tick(statement.line, statement.column)?;
        if self.stopped {
            return Ok(Flow::Stop);
        }
        if self.before_statement(statement)? {
            return Ok(Flow::Stop);
        }
        match &statement.kind {
            StatementKind::Block(statements) => {
                self.scopes.push(HashMap::new());
                let result = self.execute_statements(statements);
                self.scopes.pop();
                result
            }
            StatementKind::Variables(variables) => {
                for variable in variables {
                    self.declare_variable(variable, statement)?;
                }
                Ok(Flow::Normal)
            }
            StatementKind::Expression(expression) => {
                self.evaluate(expression)?;
                if self.stopped {
                    Ok(Flow::Stop)
                } else {
                    Ok(Flow::Normal)
                }
            }
            StatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                if self.evaluate(condition)?.as_logical(condition)? {
                    self.execute_statement(then_branch)
                } else if let Some(else_branch) = else_branch {
                    self.execute_statement(else_branch)
                } else {
                    Ok(Flow::Normal)
                }
            }
            StatementKind::While { condition, body } => {
                while self.evaluate(condition)?.as_logical(condition)? {
                    match self.execute_statement(body)? {
                        Flow::Normal => {}
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                        Flow::Break => break,
                        Flow::Stop => return Ok(Flow::Stop),
                    }
                    self.tick(statement.line, statement.column)?;
                    if self.stopped {
                        return Ok(Flow::Stop);
                    }
                }
                Ok(Flow::Normal)
            }
            StatementKind::DoWhile { body, condition } => {
                loop {
                    match self.execute_statement(body)? {
                        Flow::Normal => {}
                        Flow::Return(value) => return Ok(Flow::Return(value)),
                        Flow::Break => break,
                        Flow::Stop => return Ok(Flow::Stop),
                    }
                    self.tick(statement.line, statement.column)?;
                    if self.stopped {
                        return Ok(Flow::Stop);
                    }
                    if !self.evaluate(condition)?.as_logical(condition)? {
                        break;
                    }
                }
                Ok(Flow::Normal)
            }
            StatementKind::For {
                initializer,
                condition,
                increment,
                body,
            } => {
                self.scopes.push(HashMap::new());
                let loop_result = (|| {
                    if let Some(initializer) = initializer {
                        match self.execute_statement(initializer)? {
                            Flow::Return(value) => return Ok(Flow::Return(value)),
                            Flow::Stop => return Ok(Flow::Stop),
                            Flow::Break => return Ok(Flow::Break),
                            Flow::Normal => {}
                        }
                    }
                    loop {
                        if let Some(condition) = condition
                            && !self.evaluate(condition)?.as_logical(condition)?
                        {
                            break;
                        }
                        match self.execute_statement(body)? {
                            Flow::Normal => {}
                            Flow::Return(value) => return Ok(Flow::Return(value)),
                            Flow::Break => break,
                            Flow::Stop => return Ok(Flow::Stop),
                        }
                        if let Some(increment) = increment {
                            self.evaluate(increment)?;
                        }
                        self.tick(statement.line, statement.column)?;
                        if self.stopped {
                            return Ok(Flow::Stop);
                        }
                    }
                    Ok(Flow::Normal)
                })();
                self.scopes.pop();
                loop_result
            }
            StatementKind::Switch { expression, cases } => {
                let value = self.evaluate(expression)?;
                let mut matching_case = None;
                for (index, case) in cases.iter().enumerate() {
                    if let Some(case_value) = &case.value {
                        let candidate = self.evaluate(case_value)?;
                        if values_equal(&value, &candidate) {
                            matching_case = Some(index);
                            break;
                        }
                    }
                }
                let start =
                    matching_case.or_else(|| cases.iter().position(|case| case.value.is_none()));
                if let Some(start) = start {
                    for case in &cases[start..] {
                        match self.execute_statements(&case.body)? {
                            Flow::Normal => {}
                            Flow::Break => return Ok(Flow::Normal),
                            Flow::Stop => return Ok(Flow::Stop),
                            flow @ Flow::Return(_) => return Ok(flow),
                        }
                    }
                }
                Ok(Flow::Normal)
            }
            StatementKind::Return(value) => {
                let value = match value {
                    Some(expression) => self.evaluate(expression)?,
                    None => Value::Null,
                };
                if self.stopped {
                    Ok(Flow::Stop)
                } else {
                    Ok(Flow::Return(value))
                }
            }
            StatementKind::Break => Ok(Flow::Break),
        }
    }

    fn before_statement(&mut self, statement: &Statement) -> Result<bool, Diagnostic> {
        if !self.io.is_debugging() && !self.step_next {
            return Ok(false);
        }
        let point = ExecutionPoint {
            location: SourceLocation {
                line: statement.line,
                column: statement.column,
            },
            variables: self.visible_variables(),
        };
        let action = self.io.on_statement(&point).map_err(|message| {
            Diagnostic::new(
                DiagnosticKind::Runtime,
                format!("falha no controle de execução: {message}"),
                statement.line,
                statement.column,
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
        Ok(false)
    }

    fn visible_variables(&self) -> Vec<VariableSnapshot> {
        let mut visible = std::collections::BTreeMap::new();
        let function_scope = self.function_scope_indices.last().copied().unwrap_or(0);
        for (index, scope) in self.scopes.iter().enumerate() {
            if index != 0 && index < function_scope {
                continue;
            }
            for (name, binding) in scope {
                visible.insert(name.clone(), binding.value.borrow().to_text());
            }
        }
        visible
            .into_iter()
            .map(|(name, value)| VariableSnapshot { name, value })
            .collect()
    }

    fn execute_statements(&mut self, statements: &[Statement]) -> Result<Flow, Diagnostic> {
        for statement in statements {
            match self.execute_statement(statement)? {
                Flow::Normal => {}
                flow => return Ok(flow),
            }
        }
        Ok(Flow::Normal)
    }

    fn declare_variable(
        &mut self,
        variable: &Variable,
        statement: &Statement,
    ) -> Result<(), Diagnostic> {
        if variable.is_constant && variable.initializer.is_none() {
            return Err(runtime_error(
                statement,
                format!(
                    "a constante '{}' precisa de um valor inicial",
                    variable.name
                ),
            ));
        }
        if self
            .scopes
            .last()
            .is_some_and(|scope| scope.contains_key(&variable.name))
        {
            return Err(runtime_error(
                statement,
                format!(
                    "a variável '{}' já foi declarada neste escopo",
                    variable.name
                ),
            ));
        }
        let value = if variable.dimensions.is_empty() {
            let value = Value::default_for(variable.ty);
            match &variable.initializer {
                Some(expression) => self
                    .evaluate(expression)?
                    .coerce_to(variable.ty, expression)?,
                None => value,
            }
        } else {
            if variable.dimensions.len() > MAX_ARRAY_DIMENSIONS {
                return Err(runtime_error(
                    statement,
                    format!("vetores podem ter no máximo {MAX_ARRAY_DIMENSIONS} dimensões"),
                ));
            }
            let declared_dimensions = self.evaluate_array_dimensions(&variable.dimensions)?;
            let dimensions = match &variable.initializer {
                Some(initializer) => {
                    self.resolve_array_dimensions(&declared_dimensions, initializer)?
                }
                None if declared_dimensions.contains(&0)
                    && variable.dimensions.iter().any(Option::is_none) =>
                {
                    return Err(runtime_error(
                        statement,
                        "o tamanho do vetor precisa ser informado ou inferido por valores iniciais",
                    ));
                }
                None => declared_dimensions,
            };
            let mut elements_at_level = 1_usize;
            let mut element_count = 0_usize;
            for dimension in &dimensions {
                elements_at_level = elements_at_level
                    .checked_mul(*dimension)
                    .ok_or_else(|| runtime_error(statement, "tamanho do vetor fora do limite"))?;
                element_count = element_count
                    .checked_add(elements_at_level)
                    .ok_or_else(|| runtime_error(statement, "tamanho do vetor fora do limite"))?;
            }
            let total_elements = self
                .array_elements_allocated
                .checked_add(element_count)
                .ok_or_else(|| runtime_error(statement, "limite de elementos excedido"))?;
            if total_elements > self.array_element_limit {
                return Err(runtime_error(
                    statement,
                    format!(
                        "programa excede o limite de elementos de vetores ({})",
                        self.array_element_limit
                    ),
                ));
            }
            self.array_elements_allocated = total_elements;
            match &variable.initializer {
                Some(initializer) => {
                    self.initialize_array(variable.ty, &dimensions, initializer)?
                }
                None => make_array(variable.ty, &dimensions),
            }
        };
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                variable.name.clone(),
                Binding {
                    ty: variable.ty,
                    dimensions: variable.dimensions.len(),
                    is_constant: variable.is_constant,
                    value: Rc::new(RefCell::new(value)),
                },
            );
        }
        Ok(())
    }

    fn evaluate_array_dimensions(
        &mut self,
        declared: &[Option<Expression>],
    ) -> Result<Vec<usize>, Diagnostic> {
        declared
            .iter()
            .map(|dimension| {
                let Some(expression) = dimension else {
                    return Ok(0);
                };
                let size = self.evaluate(expression)?.as_integer(expression)?;
                usize::try_from(size)
                    .map_err(|_| runtime_error(expression, "tamanho de vetor negativo"))
            })
            .collect()
    }

    fn resolve_array_dimensions(
        &self,
        declared: &[usize],
        initializer: &Expression,
    ) -> Result<Vec<usize>, Diagnostic> {
        let mut dimensions = declared.to_vec();
        let mut current = Some(initializer);
        let dimension_count = dimensions.len();
        for (index, dimension) in dimensions.iter_mut().enumerate() {
            if let Some(expression) = current {
                let ExpressionKind::ArrayLiteral(items) = &expression.kind else {
                    return Err(runtime_error(
                        expression,
                        "esperada uma lista de valores entre chaves para iniciar o vetor",
                    ));
                };
                if *dimension == 0 {
                    *dimension = items.len();
                }
                if items.len() != *dimension {
                    return Err(runtime_error(
                        expression,
                        format!(
                            "o vetor foi declarado com {} valor(es), mas recebeu {}",
                            *dimension,
                            items.len()
                        ),
                    ));
                }
                current = items.first();
            }
            if index + 1 == dimension_count {
                break;
            }
        }
        Ok(dimensions)
    }

    fn initialize_array(
        &mut self,
        ty: Type,
        dimensions: &[usize],
        initializer: &Expression,
    ) -> Result<Value, Diagnostic> {
        let ExpressionKind::ArrayLiteral(items) = &initializer.kind else {
            return Err(runtime_error(
                initializer,
                "esperada uma lista de valores entre chaves para iniciar o vetor",
            ));
        };
        let length = dimensions[0];
        if items.len() != length {
            return Err(runtime_error(
                initializer,
                format!(
                    "o vetor foi declarado com {length} valor(es), mas recebeu {}",
                    items.len()
                ),
            ));
        }

        let mut values = Vec::with_capacity(length);
        for index in 0..length {
            let value = if let Some(item) = items.get(index) {
                if dimensions.len() == 1 {
                    self.evaluate(item)?.coerce_to(ty, item)?
                } else {
                    self.initialize_array(ty, &dimensions[1..], item)?
                }
            } else if dimensions.len() == 1 {
                Value::default_for(ty)
            } else {
                make_array(ty, &dimensions[1..])
            };
            values.push(Rc::new(RefCell::new(value)));
        }
        Ok(Value::Array(Rc::new(RefCell::new(values))))
    }

    fn evaluate(&mut self, expression: &Expression) -> Result<Value, Diagnostic> {
        let value = self.evaluate_inner(expression)?;
        self.validate_string_size(&value, expression)?;
        Ok(value)
    }

    fn evaluate_inner(&mut self, expression: &Expression) -> Result<Value, Diagnostic> {
        match &expression.kind {
            ExpressionKind::Integer(value) => Ok(Value::Integer(i64::from(*value as i32))),
            ExpressionKind::Real(value) => Ok(Value::Real(*value)),
            ExpressionKind::Logical(value) => Ok(Value::Logical(*value)),
            ExpressionKind::Character(value) => Ok(Value::Character(*value)),
            ExpressionKind::String(value) => Ok(Value::String(value.clone())),
            ExpressionKind::ArrayLiteral(_) => Err(runtime_error(
                expression,
                "valores entre chaves só podem iniciar vetores e matrizes",
            )),
            ExpressionKind::Variable(name) => self
                .lookup(name)
                .map(|binding| binding.value.borrow().clone())
                .ok_or_else(|| {
                    runtime_error(expression, format!("variável '{name}' não declarada"))
                }),
            ExpressionKind::Unary { operator, operand } => {
                let value = self.evaluate(operand)?;
                match operator {
                    UnaryOperator::Positive => match value {
                        value @ (Value::Integer(_) | Value::Real(_)) => Ok(value),
                        value => Err(runtime_error(
                            expression,
                            format!(
                                "não é possível aplicar sinal positivo a {}",
                                value.type_name()
                            ),
                        )),
                    },
                    UnaryOperator::Negate => match value {
                        Value::Integer(value) => {
                            Ok(Value::Integer((value as i32).wrapping_neg() as i64))
                        }
                        Value::Real(value) => Ok(Value::Real(-value)),
                        value => Err(runtime_error(
                            expression,
                            format!("não é possível negar {}", value.type_name()),
                        )),
                    },
                    UnaryOperator::Not => Ok(Value::Logical(!value.as_logical(expression)?)),
                    UnaryOperator::BitNot => value
                        .as_integer(expression)
                        .map(|value| Value::Integer(!(value as i32) as i64)),
                }
            }
            ExpressionKind::Binary {
                left,
                operator,
                right,
            } => {
                let left_value = self.evaluate(left)?;
                if *operator == BinaryOperator::And && !left_value.as_logical(left)? {
                    return Ok(Value::Logical(false));
                }
                if *operator == BinaryOperator::Or && left_value.as_logical(left)? {
                    return Ok(Value::Logical(true));
                }
                let right_value = self.evaluate(right)?;
                evaluate_binary(*operator, left_value, right_value, expression)
            }
            ExpressionKind::Assign {
                target,
                operator,
                value,
            } => {
                let target = self.prepare_target(target)?;
                let left = if *operator == AssignmentOperator::Assign {
                    None
                } else {
                    Some(target.value.borrow().clone())
                };
                let right = self.evaluate(value)?;
                let result = if *operator == AssignmentOperator::Assign {
                    right
                } else {
                    let binary = match operator {
                        AssignmentOperator::Add => BinaryOperator::Add,
                        AssignmentOperator::Subtract => BinaryOperator::Subtract,
                        AssignmentOperator::Multiply => BinaryOperator::Multiply,
                        AssignmentOperator::Divide => BinaryOperator::Divide,
                        AssignmentOperator::Assign => unreachable!(),
                    };
                    evaluate_binary(
                        binary,
                        left.expect("compound assignment value"),
                        right,
                        expression,
                    )?
                };
                self.store_target(&target, result.clone(), expression)?;
                Ok(result)
            }
            ExpressionKind::Call { callee, arguments } => {
                let name = callable_name(callee).ok_or_else(|| {
                    runtime_error(expression, "a expressão chamada não é uma função")
                })?;
                if name == "leia" {
                    self.read_into(arguments, expression)?;
                    return Ok(Value::Null);
                }
                if name == "escreva" || name == "escreva_linha" {
                    let mut output = String::new();
                    for argument in arguments {
                        let text = self.evaluate(argument)?.to_text();
                        let next_size = self
                            .output_bytes
                            .checked_add(output.len())
                            .and_then(|size| size.checked_add(text.len()))
                            .ok_or_else(|| {
                                runtime_error(expression, "limite de saída do programa excedido")
                            })?;
                        if next_size > self.output_limit {
                            return Err(runtime_error(
                                expression,
                                format!(
                                    "limite de saída do programa excedido ({} bytes)",
                                    self.output_limit
                                ),
                            ));
                        }
                        output.push_str(&text);
                    }
                    if name == "escreva_linha" {
                        if self
                            .output_bytes
                            .saturating_add(output.len())
                            .saturating_add(1)
                            > self.output_limit
                        {
                            return Err(runtime_error(
                                expression,
                                format!(
                                    "limite de saída do programa excedido ({} bytes)",
                                    self.output_limit
                                ),
                            ));
                        }
                        output.push('\n');
                    }
                    self.write_output(&output, expression)?;
                    return Ok(Value::Null);
                }
                if name == "limpa" {
                    if !arguments.is_empty() {
                        return Err(runtime_error(expression, "'limpa' não recebe argumentos"));
                    }
                    self.io.clear_output();
                    return Ok(Value::Null);
                }
                if name == "sorteia" {
                    let mut values = Vec::with_capacity(arguments.len());
                    for argument in arguments {
                        values.push(self.evaluate(argument)?);
                    }
                    return self
                        .call_util_builtin("sorteia", &values, expression)?
                        .ok_or_else(|| {
                            runtime_error(expression, "função 'sorteia' não encontrada")
                        });
                }
                if self.functions.contains_key(&name) {
                    return self.call_function(
                        &name,
                        arguments,
                        expression.line,
                        expression.column,
                    );
                }
                if let Some((module, function_name)) = name.rsplit_once('.') {
                    let library =
                        self.resolve_library(module)
                            .map(str::to_owned)
                            .ok_or_else(|| {
                                runtime_error(
                                    expression,
                                    format!("biblioteca '{module}' não incluída"),
                                )
                            })?;
                    let mut values = Vec::with_capacity(arguments.len());
                    for argument in arguments {
                        values.push(self.evaluate(argument)?);
                    }
                    if library == "Matematica" {
                        if let Some(result) = call_math_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Texto" {
                        if let Some(result) = call_text_builtin(
                            function_name,
                            &values,
                            expression,
                            self.string_limit,
                        )? {
                            return Ok(result);
                        }
                    } else if library == "Tipos" {
                        if let Some(result) =
                            call_types_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Calendario" {
                        if let Some(result) =
                            self.call_calendar_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Arquivos" {
                        if let Some(result) =
                            self.call_files_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Objetos" {
                        if let Some(result) =
                            self.call_objects_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Internet" {
                        if let Some(result) =
                            self.call_internet_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "ServicosWeb" {
                        if let Some(result) =
                            self.call_web_services_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Mouse" {
                        if let Some(result) =
                            self.call_mouse_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Teclado" {
                        if let Some(result) =
                            self.call_keyboard_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Graficos" {
                        if let Some(result) =
                            self.call_graphics_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Sons" {
                        if let Some(result) =
                            self.call_audio_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    } else if library == "Util" {
                        if let Some(result) =
                            self.call_util_builtin(function_name, &values, expression)?
                        {
                            return Ok(result);
                        }
                    }
                    return Err(runtime_error(
                        expression,
                        format!(
                            "função '{function_name}' não encontrada na biblioteca '{library}'"
                        ),
                    ));
                }
                let mut values = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    values.push(self.evaluate(argument)?);
                }
                if let Some(result) = call_math_builtin(&name, &values, expression)? {
                    return Ok(result);
                }
                self.call_function(&name, arguments, expression.line, expression.column)
            }
            ExpressionKind::Index { target, index } => {
                let array = self.evaluate(target)?;
                let index = self.evaluate(index)?.as_integer(expression)?;
                let Value::Array(values) = array else {
                    return Err(runtime_error(expression, "o valor não é um vetor"));
                };
                let values = values.borrow();
                let index = usize::try_from(index)
                    .map_err(|_| runtime_error(expression, "índice de vetor negativo"))?;
                values
                    .get(index)
                    .map(|value| value.borrow().clone())
                    .ok_or_else(|| runtime_error(expression, "índice de vetor fora do limite"))
            }
            ExpressionKind::Member { target, name } => {
                let ExpressionKind::Variable(module) = &target.kind else {
                    return Err(runtime_error(expression, "biblioteca desconhecida"));
                };
                let library = self.resolve_library(module).ok_or_else(|| {
                    runtime_error(expression, format!("biblioteca '{module}' não incluída"))
                })?;
                if library == "Matematica" && name == "PI" {
                    Ok(Value::Real(PI))
                } else if library == "Calendario" {
                    let value = match name.as_str() {
                        "DIA_DOMINGO" => 1,
                        "DIA_SEGUNDA_FEIRA" => 2,
                        "DIA_TERCA_FEIRA" => 3,
                        "DIA_QUARTA_FEIRA" => 4,
                        "DIA_QUINTA_FEIRA" => 5,
                        "DIA_SEXTA_FEIRA" => 6,
                        "DIA_SABADO" => 7,
                        "MES_JANEIRO" => 1,
                        "MES_FEVEREIRO" => 2,
                        "MES_MARCO" => 3,
                        "MES_ABRIL" => 4,
                        "MES_MAIO" => 5,
                        "MES_JUNHO" => 6,
                        "MES_JULHO" => 7,
                        "MES_AGOSTO" => 8,
                        "MES_SETEMBRO" => 9,
                        "MES_OUTUBRO" => 10,
                        "MES_NOVEMBRO" => 11,
                        "MES_DEZEMBRO" => 12,
                        _ => {
                            return Err(runtime_error(
                                expression,
                                format!("constante '{}.{}' não encontrada", module, name),
                            ));
                        }
                    };
                    Ok(Value::Integer(value))
                } else if library == "Arquivos" {
                    match name.as_str() {
                        "MODO_LEITURA" => Ok(Value::Integer(0)),
                        "MODO_ESCRITA" => Ok(Value::Integer(1)),
                        "MODO_ACRESCENTAR" => Ok(Value::Integer(2)),
                        _ => Err(runtime_error(
                            expression,
                            format!("constante '{}.{}' não encontrada", module, name),
                        )),
                    }
                } else if library == "Objetos" {
                    match name.as_str() {
                        "TIPO_INTEIRO" => Ok(Value::Integer(1)),
                        "TIPO_CADEIA" => Ok(Value::Integer(2)),
                        "TIPO_CARACTER" => Ok(Value::Integer(3)),
                        "TIPO_REAL" => Ok(Value::Integer(4)),
                        "TIPO_LOGICO" => Ok(Value::Integer(5)),
                        "TIPO_OBJETO" => Ok(Value::Integer(6)),
                        "TIPO_VETOR" => Ok(Value::Integer(7)),
                        _ => Err(runtime_error(
                            expression,
                            format!("constante '{}.{}' não encontrada", module, name),
                        )),
                    }
                } else if library == "ServicosWeb" {
                    match name.as_str() {
                        "OBTER" => Ok(Value::Integer(1)),
                        "PUBLICAR" => Ok(Value::Integer(2)),
                        "ATUALIZAR" => Ok(Value::Integer(3)),
                        "EXCLUIR" => Ok(Value::Integer(4)),
                        _ => Err(runtime_error(
                            expression,
                            format!("constante '{}.{}' não encontrada", module, name),
                        )),
                    }
                } else if library == "Mouse" {
                    match name.as_str() {
                        "BOTAO_ESQUERDO" => Ok(Value::Integer(0)),
                        "BOTAO_DIREITO" => Ok(Value::Integer(1)),
                        "BOTAO_MEIO" => Ok(Value::Integer(2)),
                        _ => Err(runtime_error(
                            expression,
                            format!("constante '{}.{}' não encontrada", module, name),
                        )),
                    }
                } else if library == "Teclado" {
                    crate::keyboard::constant_code(name)
                        .map(Value::Integer)
                        .ok_or_else(|| {
                            runtime_error(
                                expression,
                                format!("constante '{}.{}' não encontrada", module, name),
                            )
                        })
                } else if library == "Graficos" {
                    crate::graphics_constants::constant_value(name)
                        .map(Value::Integer)
                        .ok_or_else(|| {
                            runtime_error(
                                expression,
                                format!("constante '{}.{}' não encontrada", module, name),
                            )
                        })
                } else {
                    Err(runtime_error(
                        expression,
                        format!("constante '{}.{}' não encontrada", module, name),
                    ))
                }
            }
            ExpressionKind::Postfix { target, operator }
            | ExpressionKind::Prefix { target, operator } => {
                let is_prefix = matches!(&expression.kind, ExpressionKind::Prefix { .. });
                let target = self.prepare_target(target)?;
                let old = target.value.borrow().clone();
                let amount = match operator {
                    PostfixOperator::Increment => 1,
                    PostfixOperator::Decrement => -1,
                };
                let new = match old.clone() {
                    Value::Integer(value) => {
                        Value::Integer((value as i32).wrapping_add(amount as i32) as i64)
                    }
                    Value::Real(value) => Value::Real(value + amount as f64),
                    value => {
                        return Err(runtime_error(
                            expression,
                            format!(
                                "não é possível alterar {} com incremento",
                                value.type_name()
                            ),
                        ));
                    }
                };
                self.store_target(&target, new.clone(), expression)?;
                if is_prefix { Ok(new) } else { Ok(old) }
            }
        }
    }

    fn read_into(&mut self, arguments: &[Expression], call: &Expression) -> Result<(), Diagnostic> {
        if arguments.is_empty() {
            return Err(runtime_error(
                call,
                "'leia' precisa receber ao menos uma variável",
            ));
        }
        for source_target in arguments {
            let target = self.prepare_target(source_target)?;
            if target.is_constant {
                return Err(runtime_error(
                    source_target,
                    "não é possível alterar uma constante com 'leia'",
                ));
            }
            if target.dimensions > 0 {
                return Err(runtime_error(
                    source_target,
                    "'leia' precisa receber uma variável simples",
                ));
            }
            let ty = target.ty;
            let text = self.io.read_line().map_err(|message| {
                runtime_error(
                    source_target,
                    format!("não foi possível ler a entrada: {message}"),
                )
            })?;
            let value = parse_input(&text, ty, source_target)?;
            self.store_target(&target, value, source_target)?;
        }
        Ok(())
    }

    fn prepare_target(&mut self, target: &Expression) -> Result<PreparedTarget, Diagnostic> {
        match &target.kind {
            ExpressionKind::Variable(name) => self
                .lookup(name)
                .map(|binding| PreparedTarget {
                    value: binding.value.clone(),
                    ty: binding.ty,
                    dimensions: binding.dimensions,
                    is_constant: binding.is_constant,
                })
                .ok_or_else(|| runtime_error(target, format!("variável '{name}' não declarada"))),
            ExpressionKind::Index { target, index } => {
                let parent = self.prepare_target(target)?;
                let array = parent.value.borrow().clone();
                let Value::Array(values) = array else {
                    return Err(runtime_error(target, "o valor não é um vetor"));
                };
                let index = self.evaluate(index)?.as_integer(target)?;
                let index = usize::try_from(index)
                    .map_err(|_| runtime_error(target, "índice de vetor negativo"))?;
                values
                    .borrow()
                    .get(index)
                    .map(|value| PreparedTarget {
                        value: value.clone(),
                        ty: parent.ty,
                        dimensions: parent.dimensions.saturating_sub(1),
                        is_constant: parent.is_constant,
                    })
                    .ok_or_else(|| runtime_error(target, "índice de vetor fora do limite"))
            }
            _ => Err(runtime_error(
                target,
                "esperado uma variável para atribuição",
            )),
        }
    }

    fn target_type(&self, target: &Expression) -> Result<Type, Diagnostic> {
        match &target.kind {
            ExpressionKind::Variable(name) => self
                .lookup(name)
                .map(|binding| binding.ty)
                .ok_or_else(|| runtime_error(target, format!("variável '{name}' não declarada"))),
            ExpressionKind::Index { target, .. } => self.target_type(target),
            _ => Err(runtime_error(
                target,
                "esperado uma variável para atribuição",
            )),
        }
    }

    fn store_target(
        &self,
        target: &PreparedTarget,
        value: Value,
        context: &Expression,
    ) -> Result<(), Diagnostic> {
        if target.is_constant {
            return Err(runtime_error(
                context,
                "não é possível alterar uma constante",
            ));
        }
        if target.dimensions > 0 {
            return Err(runtime_error(
                context,
                "não é possível substituir um vetor inteiro",
            ));
        }
        let value = value.coerce_to(target.ty, context)?;
        self.validate_string_size(&value, context)?;
        *target.value.borrow_mut() = value;
        Ok(())
    }

    fn call_function(
        &mut self,
        name: &str,
        arguments: &[Expression],
        line: usize,
        column: usize,
    ) -> Result<Value, Diagnostic> {
        let (function, prepared_arguments) =
            self.resolve_function(name, arguments, line, column)?;
        if self.call_depth >= MAX_CALL_DEPTH {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                "limite de chamadas recursivas excedido",
                line,
                column,
            ));
        }

        let mut parameter_bindings = Vec::with_capacity(arguments.len());
        for (parameter, argument) in function.parameters.iter().zip(prepared_arguments) {
            let value = match argument {
                PreparedArgument::Value {
                    value, dimensions, ..
                } if dimensions > 0 => Rc::new(RefCell::new(value)),
                PreparedArgument::Value { value, .. } => Rc::new(RefCell::new(value.coerce_to(
                    parameter.ty,
                    &Expression {
                        kind: ExpressionKind::Variable(parameter.name.clone()),
                        line,
                        column,
                    },
                )?)),
                PreparedArgument::Reference { value, .. } => value,
            };
            parameter_bindings.push((
                parameter.name.clone(),
                Binding {
                    ty: parameter.ty,
                    dimensions: parameter.dimensions,
                    is_constant: false,
                    value,
                },
            ));
        }

        self.call_depth += 1;
        self.scopes.push(HashMap::new());
        self.function_scope_indices.push(self.scopes.len() - 1);
        let setup = (|| {
            let function_scope = self.scopes.last_mut().expect("function scope");
            for (name, binding) in parameter_bindings {
                function_scope.insert(name, binding);
            }
            let flow = self.execute_statement(&function.body)?;
            let value = match flow {
                Flow::Return(value) => value,
                Flow::Normal => Value::Null,
                Flow::Stop => {
                    self.stopped = true;
                    return Ok(Value::Null);
                }
                Flow::Break => {
                    return Err(Diagnostic::new(
                        DiagnosticKind::Runtime,
                        "'pare' não pode aparecer fora de um laço ou 'escolha'",
                        function.line,
                        function.column,
                    ));
                }
            };
            match function.return_type {
                Some(ty) => value.coerce_to(
                    ty,
                    &Expression {
                        kind: ExpressionKind::Variable(function.name.clone()),
                        line: function.line,
                        column: function.column,
                    },
                ),
                None if matches!(value, Value::Null) => Ok(value),
                None => Err(Diagnostic::new(
                    DiagnosticKind::Runtime,
                    format!(
                        "a função vazia '{}' não pode retornar um valor",
                        function.name
                    ),
                    function.line,
                    function.column,
                )),
            }
        })();
        self.function_scope_indices.pop();
        self.scopes.pop();
        self.call_depth -= 1;
        setup
    }

    fn resolve_function(
        &mut self,
        name: &str,
        arguments: &[Expression],
        line: usize,
        column: usize,
    ) -> Result<(Function, Vec<PreparedArgument>), Diagnostic> {
        let Some(functions) = self.functions.get(name).cloned() else {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!("função '{name}' não encontrada"),
                line,
                column,
            ));
        };
        let candidates: Vec<_> = functions
            .into_iter()
            .filter(|function| function.parameters.len() == arguments.len())
            .collect();
        if candidates.is_empty() {
            let expected = self.functions[name]
                .iter()
                .map(|function| function.parameters.len().to_string())
                .collect::<Vec<_>>()
                .join(" ou ");
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!(
                    "a função '{name}' espera {expected} argumento(s), recebeu {}",
                    arguments.len()
                ),
                line,
                column,
            ));
        }

        let mut prepared_arguments = Vec::with_capacity(arguments.len());
        for (index, argument) in arguments.iter().enumerate() {
            let by_reference = candidates[0].parameters[index].by_reference;
            if candidates
                .iter()
                .any(|function| function.parameters[index].by_reference != by_reference)
            {
                return Err(runtime_error(
                    argument,
                    format!("a função '{name}' tem assinaturas ambíguas por valor e referência"),
                ));
            }
            if by_reference {
                let (ty, dimensions, value) = self.reference_target(argument)?;
                prepared_arguments.push(PreparedArgument::Reference {
                    value,
                    ty,
                    dimensions,
                });
            } else {
                let dimensions = self.target_dimensions(argument)?;
                let value = self.evaluate(argument)?;
                let ty = if dimensions > 0 {
                    self.target_type(argument)?
                } else {
                    value_type(&value).ok_or_else(|| {
                        runtime_error(
                            argument,
                            "uma função sem retorno não pode ser usada como valor",
                        )
                    })?
                };
                prepared_arguments.push(PreparedArgument::Value {
                    value,
                    ty,
                    dimensions,
                });
            }
        }

        let mut matches = Vec::new();
        for function in candidates {
            let mut score = 0_u32;
            let mut compatible = true;
            for (parameter, argument) in function.parameters.iter().zip(&prepared_arguments) {
                let argument_score = match argument {
                    PreparedArgument::Reference { ty, dimensions, .. }
                        if parameter.by_reference
                            && *ty == parameter.ty
                            && *dimensions == parameter.dimensions =>
                    {
                        Some(0)
                    }
                    PreparedArgument::Value {
                        value,
                        ty,
                        dimensions,
                    } if !parameter.by_reference && *dimensions == parameter.dimensions => {
                        if *dimensions > 0 && !matches!(value, Value::Array(_)) {
                            None
                        } else if *ty == parameter.ty {
                            Some(0)
                        } else if *ty == Type::Integer && parameter.ty == Type::Real {
                            Some(1)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                if let Some(argument_score) = argument_score {
                    score += argument_score;
                } else {
                    compatible = false;
                    break;
                }
            }
            if compatible {
                matches.push((score, function));
            }
        }
        matches.sort_by_key(|(score, _)| *score);
        let Some((best_score, function)) = matches.first().cloned() else {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!("nenhuma assinatura da função '{name}' aceita estes argumentos"),
                line,
                column,
            ));
        };
        if matches
            .get(1)
            .is_some_and(|(score, _)| *score == best_score)
        {
            return Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                format!("chamada ambígua para a função '{name}'"),
                line,
                column,
            ));
        }
        Ok((function, prepared_arguments))
    }

    fn reference_target(
        &mut self,
        target: &Expression,
    ) -> Result<(Type, usize, Rc<RefCell<Value>>), Diagnostic> {
        let target_ref = self.prepare_target(target)?;
        if target_ref.is_constant {
            return Err(runtime_error(
                target,
                "não é possível passar uma constante por referência",
            ));
        }
        Ok((target_ref.ty, target_ref.dimensions, target_ref.value))
    }

    fn target_dimensions(&self, target: &Expression) -> Result<usize, Diagnostic> {
        match &target.kind {
            ExpressionKind::Variable(name) => self
                .lookup(name)
                .map(|binding| binding.dimensions)
                .ok_or_else(|| runtime_error(target, format!("variável '{name}' não declarada"))),
            ExpressionKind::Index { target, .. } => {
                Ok(self.target_dimensions(target)?.saturating_sub(1))
            }
            _ => Ok(0),
        }
    }

    fn lookup(&self, name: &str) -> Option<&Binding> {
        let function_scope = self.function_scope_indices.last().copied().unwrap_or(0);
        self.scopes[function_scope..]
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .or_else(|| self.scopes.first().and_then(|scope| scope.get(name)))
    }

    fn resolve_library(&self, module: &str) -> Option<&str> {
        self.libraries
            .iter()
            .find(|library| library.name == module || library.alias.as_deref() == Some(module))
            .map(|library| library.name.as_str())
    }

    fn call_util_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = name.to_owned();
        let arity = match function_name.as_str() {
            "numero_elementos" | "numero_linhas" | "numero_colunas" | "aguarde" => 1,
            "sorteia" => 2,
            "tempo_decorrido" | "obter_diretorio_usuario" => 0,
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'Util.{name}' espera {arity} argumento(s)"),
            ));
        }

        let integer_argument = |index: usize| arguments[index].as_integer(expression);
        let value = match function_name.as_str() {
            "numero_elementos" => {
                let Value::Array(values) = &arguments[0] else {
                    return Err(runtime_error(expression, "esperado um vetor"));
                };
                Value::Integer(values.borrow().len() as i32 as i64)
            }
            "numero_linhas" => {
                let Value::Array(rows) = &arguments[0] else {
                    return Err(runtime_error(expression, "esperada uma matriz"));
                };
                if let Some(first_row) = rows.borrow().first()
                    && !matches!(&*first_row.borrow(), Value::Array(_))
                {
                    return Err(runtime_error(expression, "esperada uma matriz"));
                }
                Value::Integer(rows.borrow().len() as i32 as i64)
            }
            "numero_colunas" => {
                let Value::Array(rows) = &arguments[0] else {
                    return Err(runtime_error(expression, "esperada uma matriz"));
                };
                let rows = rows.borrow();
                let first_row = rows
                    .first()
                    .ok_or_else(|| runtime_error(expression, "matriz sem linhas"))?
                    .borrow();
                let Value::Array(columns) = &*first_row else {
                    return Err(runtime_error(expression, "esperada uma matriz"));
                };
                let columns = columns.borrow().len();
                Value::Integer(columns as i32 as i64)
            }
            "sorteia" => {
                let minimum = integer_argument(0)?;
                let maximum = integer_argument(1)?;
                if minimum > maximum {
                    return Err(runtime_error(
                        expression,
                        "o limite inferior não pode ser maior que o limite superior",
                    ));
                }
                if minimum == maximum {
                    return Err(runtime_error(
                        expression,
                        "os limites inferior e superior não podem ser iguais",
                    ));
                }
                let minimum = minimum as i32;
                let bound = (maximum as i32).wrapping_add(1).wrapping_sub(minimum);
                if bound <= 0 {
                    return Err(runtime_error(
                        expression,
                        "limite aleatório fora do intervalo",
                    ));
                }
                Value::Integer(i64::from(
                    minimum.wrapping_add(self.next_random_bounded(bound)),
                ))
            }
            "aguarde" => {
                let delay = u64::try_from(integer_argument(0)?)
                    .map_err(|_| runtime_error(expression, "o intervalo de espera é negativo"))?;
                let remaining_wait = self.wait_limit_millis.saturating_sub(self.wait_millis_used);
                if delay > remaining_wait {
                    return Err(runtime_error(
                        expression,
                        format!(
                            "intervalo excede o limite de espera restante ({remaining_wait} ms)"
                        ),
                    ));
                }
                self.wait_millis_used += delay;
                self.io
                    .wait(Duration::from_millis(delay))
                    .map_err(|message| runtime_error(expression, message))?;
                Value::Null
            }
            "tempo_decorrido" => {
                let elapsed = SystemTime::now()
                    .duration_since(self.util_start_time)
                    .unwrap_or_default()
                    .as_millis() as i32;
                Value::Integer(i64::from(elapsed))
            }
            "obter_diretorio_usuario" => {
                let path = std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(std::path::PathBuf::from)
                    .ok_or_else(|| {
                        runtime_error(expression, "diretório do usuário indisponível")
                    })?;
                let path = if path.is_absolute() {
                    path
                } else {
                    std::env::current_dir()
                        .map_err(|error| runtime_error(expression, error.to_string()))?
                        .join(path)
                };
                Value::String(path.to_string_lossy().into_owned())
            }
            _ => unreachable!("validated utility function"),
        };
        Ok(Some(value))
    }

    fn call_calendar_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = name.to_owned();
        let arity = match function_name.as_str() {
            "dia_mes_atual" | "dia_semana_atual" | "mes_atual" | "ano_atual" | "minuto_atual"
            | "segundo_atual" | "milisegundo_atual" => 0,
            "hora_atual" => 1,
            "dia_semana_completo" | "dia_semana_curto" | "dia_semana_abreviado" => 3,
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'Calendario.{name}' espera {arity} argumento(s)"),
            ));
        }

        let value = match function_name.as_str() {
            "dia_semana_completo" | "dia_semana_curto" | "dia_semana_abreviado" => {
                let day = arguments[0].as_integer(expression)?;
                let day = usize::try_from(day)
                    .ok()
                    .and_then(|day| day.checked_sub(1))
                    .filter(|day| *day < 7)
                    .ok_or_else(|| {
                        runtime_error(
                            expression,
                            format!("'{day}' não corresponde a um dia da semana válido"),
                        )
                    })?;
                let names: [&str; 7] = match function_name.as_str() {
                    "dia_semana_completo" => [
                        "Domingo",
                        "Segunda-Feira",
                        "Terça-Feira",
                        "Quarta-Feira",
                        "Quinta-Feira",
                        "Sexta-Feira",
                        "Sabado",
                    ],
                    "dia_semana_curto" => [
                        "Domingo", "Segunda", "Terça", "Quarta", "Quinta", "Sexta", "Sabado",
                    ],
                    _ => ["Dom", "Seg", "Ter", "Qua", "Qui", "Sex", "Sab"],
                };
                let uppercase = arguments[1].as_logical(expression)?;
                let lowercase = arguments[2].as_logical(expression)?;
                let name = names[day];
                Value::String(if uppercase {
                    name.to_uppercase()
                } else if lowercase {
                    name.to_lowercase()
                } else {
                    name.to_owned()
                })
            }
            _ => {
                let now = self
                    .io
                    .calendar_now()
                    .map_err(|message| runtime_error(expression, message))?;
                let field = match function_name.as_str() {
                    "dia_mes_atual" => i64::from(now.day),
                    "dia_semana_atual" => i64::from(now.weekday),
                    "mes_atual" => i64::from(now.month),
                    "ano_atual" => i64::from(now.year),
                    "hora_atual" => {
                        let twelve_hour = arguments[0].as_logical(expression)?;
                        i64::from(if twelve_hour { now.hour % 12 } else { now.hour })
                    }
                    "minuto_atual" => i64::from(now.minute),
                    "segundo_atual" => i64::from(now.second),
                    "milisegundo_atual" => i64::from(now.millisecond),
                    _ => unreachable!("validated calendar function"),
                };
                Value::Integer(field)
            }
        };
        Ok(Some(value))
    }

    fn call_files_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = name.to_owned();
        let arity = match function_name.as_str() {
            "abrir_arquivo" | "escrever_linha" => 2,
            "fechar_arquivo" | "fim_arquivo" | "ler_linha" | "arquivo_existe"
            | "apagar_arquivo" | "criar_pasta" => 1,
            "substituir_texto" => 4,
            "listar_pastas" | "listar_arquivos" => 2,
            "selecionar_arquivo" => 2,
            "listar_arquivos_por_tipo" => 3,
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'Arquivos.{name}' espera {arity} argumento(s)"),
            ));
        }
        let text_argument = |index: usize| -> Result<String, Diagnostic> {
            match &arguments[index] {
                Value::String(value) => Ok(value.clone()),
                Value::Character(value) => Ok(value.to_string()),
                value => Err(runtime_error(
                    expression,
                    format!("esperado um texto, recebido {}", value.type_name()),
                )),
            }
        };
        let handle_argument = |index: usize| arguments[index].as_integer(expression);
        let host_error = |message: String| {
            runtime_error(expression, format!("falha em 'Arquivos.{name}': {message}"))
        };

        let value = match function_name.as_str() {
            "abrir_arquivo" => {
                let path = text_argument(0)?;
                let mode = match arguments[1].as_integer(expression)? {
                    0 => crate::FileAccessMode::Read,
                    1 => crate::FileAccessMode::Write,
                    2 => crate::FileAccessMode::Append,
                    _ => return Err(runtime_error(expression, "modo de acesso inválido")),
                };
                Value::Integer(
                    self.io
                        .portugol_file_open(&path, mode)
                        .map_err(host_error)? as i32 as i64,
                )
            }
            "fechar_arquivo" => {
                self.io
                    .portugol_file_close(handle_argument(0)?)
                    .map_err(host_error)?;
                Value::Null
            }
            "fim_arquivo" => Value::Logical(
                self.io
                    .portugol_file_is_eof(handle_argument(0)?)
                    .map_err(host_error)?,
            ),
            "ler_linha" => Value::String(
                self.io
                    .portugol_file_read_line(handle_argument(0)?)
                    .map_err(host_error)?,
            ),
            "escrever_linha" => {
                let line = text_argument(0)?;
                self.io
                    .portugol_file_write_line(handle_argument(1)?, &line)
                    .map_err(host_error)?;
                Value::Null
            }
            "substituir_texto" => {
                let path = text_argument(0)?;
                let search = text_argument(1)?;
                let replacement = text_argument(2)?;
                let only_first = arguments[3].as_logical(expression)?;
                self.io
                    .portugol_file_replace_text(&path, &search, &replacement, only_first)
                    .map_err(host_error)?;
                Value::Null
            }
            "arquivo_existe" => Value::Logical(
                self.io
                    .portugol_file_exists(&text_argument(0)?)
                    .map_err(host_error)?,
            ),
            "apagar_arquivo" => {
                self.io
                    .portugol_file_delete(&text_argument(0)?)
                    .map_err(host_error)?;
                Value::Null
            }
            "selecionar_arquivo" => {
                let extensions = string_array(&arguments[0], expression)?;
                let allow_all = arguments[1].as_logical(expression)?;
                Value::String(
                    self.io
                        .portugol_file_select(&extensions, allow_all)
                        .map_err(host_error)?
                        .unwrap_or_default(),
                )
            }
            "listar_pastas" => {
                let path = text_argument(0)?;
                let directories = self
                    .io
                    .portugol_list_directories(&path)
                    .map_err(host_error)?;
                set_string_array(&arguments[1], &directories, self.string_limit, expression)?;
                Value::Null
            }
            "listar_arquivos" => {
                let path = text_argument(0)?;
                let files = self
                    .io
                    .portugol_list_files(&path, None)
                    .map_err(host_error)?;
                set_string_array(&arguments[1], &files, self.string_limit, expression)?;
                Value::Null
            }
            "listar_arquivos_por_tipo" => {
                let path = text_argument(0)?;
                let extensions = string_array(&arguments[2], expression)?;
                let files = self
                    .io
                    .portugol_list_files(&path, Some(&extensions))
                    .map_err(host_error)?;
                set_string_array(&arguments[1], &files, self.string_limit, expression)?;
                Value::Null
            }
            "criar_pasta" => {
                self.io
                    .portugol_create_directory(&text_argument(0)?)
                    .map_err(host_error)?;
                Value::Null
            }
            _ => unreachable!("validated file library function"),
        };
        Ok(Some(value))
    }

    fn call_objects_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = name.to_owned();
        let arity = match function_name.as_str() {
            "criar_objeto" | "liberar" => 0,
            "criar_objeto_via_json" | "criar_objeto_via_xml" | "liberar_objeto" | "obter_json" => 1,
            "atribuir_propriedade" => 3,
            "obter_propriedade_tipo_inteiro"
            | "obter_propriedade_tipo_real"
            | "obter_propriedade_tipo_logico"
            | "obter_propriedade_tipo_caracter"
            | "obter_propriedade_tipo_cadeia"
            | "obter_propriedade_tipo_objeto"
            | "obter_tamanho_vetor_propriedade"
            | "contem_propriedade"
            | "tipo_propriedade" => 2,
            "obter_propriedade_tipo_objeto_em_vetor"
            | "obter_propriedade_tipo_caracter_em_vetor"
            | "obter_propriedade_tipo_logico_em_vetor"
            | "obter_propriedade_tipo_real_em_vetor"
            | "obter_propriedade_tipo_inteiro_em_vetor"
            | "obter_propriedade_tipo_cadeia_em_vetor" => 3,
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'Objetos.{name}' espera {arity} argumento(s)"),
            ));
        }
        let text_argument = |index: usize| -> Result<String, Diagnostic> {
            match &arguments[index] {
                Value::String(value) => Ok(value.clone()),
                Value::Character(value) => Ok(value.to_string()),
                value => Err(runtime_error(
                    expression,
                    format!("esperado um texto, recebido {}", value.type_name()),
                )),
            }
        };
        let object_handle = |index: usize| arguments[index].as_integer(expression);
        let object_error = |message: String| {
            runtime_error(expression, format!("falha em 'Objetos.{name}': {message}"))
        };

        let value = match function_name.as_str() {
            "criar_objeto" => Value::Integer(self.objects.create()),
            "criar_objeto_via_json" => {
                let json = text_argument(0)?;
                Value::Integer(
                    self.objects
                        .create_json(&json, self.array_element_limit.max(1)),
                )
            }
            "criar_objeto_via_xml" => {
                let xml = text_argument(0)?;
                Value::Integer(
                    self.objects
                        .create_xml(&xml, self.array_element_limit.max(1)),
                )
            }
            "atribuir_propriedade" => {
                let handle = object_handle(0)?;
                let name = text_argument(1)?;
                let value = property_value_from_runtime(&arguments[2]);
                self.objects
                    .set(handle, &name, value)
                    .map_err(object_error)?;
                Value::Null
            }
            "obter_propriedade_tipo_inteiro" => {
                let property = self
                    .objects
                    .get(object_handle(0)?, &text_argument(1)?)
                    .map_err(object_error)?;
                match property {
                    PropertyValue::Integer(value) => Value::Integer(value),
                    _ => return Err(object_error("tipo da propriedade não é inteiro".to_owned())),
                }
            }
            "obter_propriedade_tipo_real" => {
                let property = self
                    .objects
                    .get(object_handle(0)?, &text_argument(1)?)
                    .map_err(object_error)?;
                match property {
                    PropertyValue::Real(value) => Value::Real(value),
                    _ => return Err(object_error("tipo da propriedade não é real".to_owned())),
                }
            }
            "obter_propriedade_tipo_logico" => {
                let property = self
                    .objects
                    .get(object_handle(0)?, &text_argument(1)?)
                    .map_err(object_error)?;
                match property {
                    PropertyValue::Logical(value) => Value::Logical(value),
                    _ => return Err(object_error("tipo da propriedade não é lógico".to_owned())),
                }
            }
            "obter_propriedade_tipo_caracter" => {
                let property = self
                    .objects
                    .get(object_handle(0)?, &text_argument(1)?)
                    .map_err(object_error)?;
                match property {
                    PropertyValue::Character(value) => Value::Character(value),
                    PropertyValue::String(value) => value
                        .chars()
                        .next()
                        .map(Value::Character)
                        .ok_or_else(|| object_error("a cadeia está vazia".to_owned()))?,
                    _ => {
                        return Err(object_error(
                            "tipo da propriedade não pode ser convertido em caracter".to_owned(),
                        ));
                    }
                }
            }
            "obter_propriedade_tipo_cadeia" => {
                let property = self
                    .objects
                    .get(object_handle(0)?, &text_argument(1)?)
                    .map_err(object_error)?;
                match property {
                    PropertyValue::String(value) => Value::String(value),
                    _ => return Err(object_error("tipo da propriedade não é cadeia".to_owned())),
                }
            }
            "obter_propriedade_tipo_objeto" => {
                let property = self
                    .objects
                    .get(object_handle(0)?, &text_argument(1)?)
                    .map_err(object_error)?;
                Value::Integer(self.objects.nested_object(property).map_err(object_error)?)
            }
            "obter_propriedade_tipo_objeto_em_vetor" => {
                let property = self
                    .objects
                    .get_from_array(
                        object_handle(0)?,
                        &text_argument(1)?,
                        usize::try_from(object_handle(2)?)
                            .map_err(|_| runtime_error(expression, "índice inválido"))?,
                    )
                    .map_err(object_error)?;
                Value::Integer(self.objects.nested_object(property).map_err(object_error)?)
            }
            "obter_propriedade_tipo_caracter_em_vetor"
            | "obter_propriedade_tipo_logico_em_vetor"
            | "obter_propriedade_tipo_real_em_vetor"
            | "obter_propriedade_tipo_inteiro_em_vetor"
            | "obter_propriedade_tipo_cadeia_em_vetor" => {
                let handle = object_handle(0)?;
                let property_name = text_argument(1)?;
                let index = usize::try_from(object_handle(2)?)
                    .map_err(|_| runtime_error(expression, "índice inválido"))?;
                let property = self
                    .objects
                    .get_from_array(handle, &property_name, index)
                    .map_err(object_error)?;
                match (function_name.as_str(), property) {
                    ("obter_propriedade_tipo_caracter_em_vetor", PropertyValue::Character(c)) => {
                        Value::Character(c)
                    }
                    ("obter_propriedade_tipo_caracter_em_vetor", PropertyValue::String(value)) => {
                        value
                            .chars()
                            .next()
                            .map(Value::Character)
                            .ok_or_else(|| object_error("a cadeia está vazia".to_owned()))?
                    }
                    ("obter_propriedade_tipo_logico_em_vetor", PropertyValue::Logical(value)) => {
                        Value::Logical(value)
                    }
                    ("obter_propriedade_tipo_real_em_vetor", PropertyValue::Real(value)) => {
                        Value::Real(value)
                    }
                    ("obter_propriedade_tipo_inteiro_em_vetor", PropertyValue::Integer(value)) => {
                        Value::Integer(value)
                    }
                    ("obter_propriedade_tipo_cadeia_em_vetor", PropertyValue::String(value)) => {
                        Value::String(value)
                    }
                    _ => {
                        return Err(object_error(
                            "tipo do elemento do vetor não corresponde à função".to_owned(),
                        ));
                    }
                }
            }
            "obter_tamanho_vetor_propriedade" => Value::Integer(
                self.objects
                    .array_len(object_handle(0)?, &text_argument(1)?)
                    .map_err(object_error)? as i64,
            ),
            "liberar_objeto" => {
                self.objects
                    .release(object_handle(0)?)
                    .map_err(object_error)?;
                Value::Null
            }
            "liberar" => {
                self.objects.release_all();
                Value::Null
            }
            "obter_json" => {
                Value::String(self.objects.json(object_handle(0)?).map_err(object_error)?)
            }
            "contem_propriedade" => Value::Logical(
                self.objects
                    .contains(object_handle(0)?, &text_argument(1)?)
                    .map_err(object_error)?,
            ),
            "tipo_propriedade" => Value::Integer(
                self.objects
                    .type_code(object_handle(0)?, &text_argument(1)?)
                    .map_err(object_error)?,
            ),
            _ => unreachable!("validated object library function"),
        };
        Ok(Some(value))
    }

    fn call_internet_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = name.to_owned();
        let arity = match function_name.as_str() {
            "definir_tempo_limite" | "obter_texto" => 1,
            "baixar_imagem" => 2,
            "endereco_disponivel" => 1,
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'Internet.{name}' espera {arity} argumento(s)"),
            ));
        }
        let text_argument = |index: usize| -> Result<String, Diagnostic> {
            match &arguments[index] {
                Value::String(value) => Ok(value.clone()),
                Value::Character(value) => Ok(value.to_string()),
                value => Err(runtime_error(
                    expression,
                    format!("esperado um texto, recebido {}", value.type_name()),
                )),
            }
        };

        let value = match function_name.as_str() {
            "definir_tempo_limite" => {
                let timeout = u64::try_from(arguments[0].as_integer(expression)?)
                    .ok()
                    .filter(|timeout| (1..=30_000).contains(timeout))
                    .ok_or_else(|| {
                        runtime_error(
                            expression,
                            "o tempo limite deve estar entre 1 e 30000 milissegundos",
                        )
                    })?;
                self.internet_timeout_millis = timeout;
                Value::Null
            }
            "obter_texto" => {
                let url = text_argument(0)?;
                let response = self.send_http_request(
                    "GET",
                    &url,
                    Vec::new(),
                    Vec::new(),
                    self.internet_timeout_millis,
                    expression,
                )?;
                if !(200..300).contains(&response.status) {
                    return Err(runtime_error(
                        expression,
                        format!(
                            "não foi possível obter o conteúdo de '{url}' (HTTP {})",
                            response.status
                        ),
                    ));
                }
                let text = decode_http_text(&response.headers, &response.body)
                    .map_err(|message| runtime_error(expression, message))?;
                if text.is_empty() {
                    return Err(runtime_error(
                        expression,
                        format!("o endereço '{url}' não tem conteúdo"),
                    ));
                }
                if text.len() > self.string_limit {
                    return Err(runtime_error(
                        expression,
                        "a resposta excede o limite de texto",
                    ));
                }
                Value::String(text)
            }
            "baixar_imagem" => {
                let url = text_argument(0)?;
                let path = text_argument(1)?;
                let head = self.send_http_request(
                    "HEAD",
                    &url,
                    Vec::new(),
                    Vec::new(),
                    self.internet_timeout_millis,
                    expression,
                )?;
                if !(200..300).contains(&head.status) {
                    return Err(runtime_error(
                        expression,
                        format!(
                            "não foi possível obter a imagem de '{url}' (HTTP {})",
                            head.status
                        ),
                    ));
                }
                let content_type = head
                    .headers
                    .iter()
                    .find(|(key, _)| key.eq_ignore_ascii_case("content-type"))
                    .map(|(_, value)| {
                        value
                            .split(';')
                            .next()
                            .unwrap_or(value)
                            .trim()
                            .to_ascii_lowercase()
                    })
                    .unwrap_or_default();
                let extension = match content_type.as_str() {
                    "image/jpeg" | "image/jpg" => "jpg",
                    "image/png" => "png",
                    _ => return Err(runtime_error(expression, "formato de imagem não suportado")),
                };
                let response = self.send_http_request(
                    "GET",
                    &url,
                    Vec::new(),
                    Vec::new(),
                    self.internet_timeout_millis,
                    expression,
                )?;
                if !(200..300).contains(&response.status) {
                    return Err(runtime_error(
                        expression,
                        format!(
                            "não foi possível obter a imagem de '{url}' (HTTP {})",
                            response.status
                        ),
                    ));
                }
                let valid_signature = match extension {
                    "jpg" => response.body.starts_with(&[0xff, 0xd8, 0xff]),
                    "png" => response.body.starts_with(b"\x89PNG\r\n\x1a\n"),
                    _ => false,
                };
                if !valid_signature {
                    return Err(runtime_error(
                        expression,
                        "a resposta não corresponde ao formato de imagem anunciado",
                    ));
                }
                let target = format!("{path}.{extension}");
                self.io
                    .portugol_file_write_image(
                        &target,
                        &response.body,
                        if extension == "jpg" { "jpeg" } else { "png" },
                    )
                    .map_err(|message| runtime_error(expression, message))?;
                let handle = self
                    .call_graphics_builtin(
                        "carregar_imagem",
                        &[Value::String(target.clone())],
                        expression,
                    )?
                    .and_then(|value| match value {
                        Value::Integer(handle) => Some(handle),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        runtime_error(expression, "não foi possível decodificar a imagem")
                    })?;
                let save = self.call_graphics_builtin(
                    "salvar_imagem",
                    &[Value::Integer(handle), Value::String(target)],
                    expression,
                );
                let release = self.call_graphics_builtin(
                    "liberar_imagem",
                    &[Value::Integer(handle)],
                    expression,
                );
                save?;
                release?;
                Value::String(extension.to_owned())
            }
            "endereco_disponivel" => {
                let url = text_argument(0)?;
                match self.send_http_request(
                    "HEAD",
                    &url,
                    Vec::new(),
                    Vec::new(),
                    self.internet_timeout_millis,
                    expression,
                ) {
                    Ok(response) => {
                        let empty = response
                            .headers
                            .iter()
                            .find(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                            .and_then(|(_, value)| value.parse::<u64>().ok())
                            == Some(0);
                        Value::Logical(response.status != 404 && !empty)
                    }
                    Err(_) => Value::Logical(false),
                }
            }
            _ => unreachable!("validated Internet function"),
        };
        Ok(Some(value))
    }

    fn call_web_services_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = name.to_owned();
        let arity = match function_name.as_str() {
            "obter_dados" | "excluir_dados" | "abrir_conexao" | "adicionar_parametros" => 1,
            "publicar_dados" | "atualizar_dados" | "adicionar_cabecalho" => 2,
            "fazer_requisicao" => 1,
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'ServicosWeb.{name}' espera {arity} argumento(s)"),
            ));
        }
        let text_argument = |index: usize| -> Result<String, Diagnostic> {
            match &arguments[index] {
                Value::String(value) => Ok(value.clone()),
                Value::Character(value) => Ok(value.to_string()),
                value => Err(runtime_error(
                    expression,
                    format!("esperado um texto, recebido {}", value.type_name()),
                )),
            }
        };
        let value = match function_name.as_str() {
            "obter_dados" | "excluir_dados" | "publicar_dados" | "atualizar_dados" => {
                let method = match function_name.as_str() {
                    "obter_dados" => "GET",
                    "excluir_dados" => "DELETE",
                    "publicar_dados" => "POST",
                    _ => "PUT",
                };
                let body = if matches!(method, "POST" | "PUT") {
                    text_argument(1)?.into_bytes()
                } else {
                    Vec::new()
                };
                let response = self.send_http_request(
                    method,
                    &text_argument(0)?,
                    json_http_headers(),
                    body,
                    self.internet_timeout_millis,
                    expression,
                )?;
                Value::String(web_service_response(response, expression)?)
            }
            "abrir_conexao" => {
                self.custom_http_request = Some(CustomHttpRequest {
                    url: text_argument(0)?,
                    headers: json_http_headers(),
                    body: Vec::new(),
                });
                Value::Null
            }
            "adicionar_cabecalho" => {
                let request = self.custom_http_request.as_mut().ok_or_else(|| {
                    runtime_error(expression, "abra uma conexão antes de adicionar cabeçalhos")
                })?;
                request.headers.push((text_argument(0)?, text_argument(1)?));
                Value::Null
            }
            "adicionar_parametros" => {
                let request = self.custom_http_request.as_mut().ok_or_else(|| {
                    runtime_error(expression, "abra uma conexão antes de adicionar parâmetros")
                })?;
                request.body = text_argument(0)?.into_bytes();
                Value::Null
            }
            "fazer_requisicao" => {
                let method = match arguments[0].as_integer(expression)? {
                    1 => "GET",
                    2 => "POST",
                    3 => "PUT",
                    4 => "DELETE",
                    _ => return Ok(Some(Value::String(String::new()))),
                };
                let request = self.custom_http_request.clone().ok_or_else(|| {
                    runtime_error(expression, "abra uma conexão antes de fazer a requisição")
                })?;
                let response = self.send_http_request(
                    method,
                    &request.url,
                    request.headers.clone(),
                    if matches!(method, "POST" | "PUT") {
                        request.body.clone()
                    } else {
                        Vec::new()
                    },
                    self.internet_timeout_millis,
                    expression,
                )?;
                Value::String(web_service_response(response, expression)?)
            }
            _ => unreachable!("validated ServicosWeb function"),
        };
        Ok(Some(value))
    }

    fn call_mouse_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = name.to_owned();
        let arity = match function_name.as_str() {
            "botao_pressionado" => 1,
            "algum_botao_pressionado"
            | "ler_botao"
            | "posicao_x"
            | "posicao_y"
            | "ocultar_cursor"
            | "exibir_cursor" => 0,
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'Mouse.{name}' espera {arity} argumento(s)"),
            ));
        }
        let value = match function_name.as_str() {
            "botao_pressionado" => {
                let button = arguments[0].as_integer(expression)?;
                if !(0..=2).contains(&button) {
                    return Err(runtime_error(
                        expression,
                        format!("o código '{button}' não é um botão do mouse válido"),
                    ));
                }
                Value::Logical(
                    self.io
                        .portugol_mouse_button_pressed(button)
                        .map_err(|message| runtime_error(expression, message))?,
                )
            }
            "algum_botao_pressionado" => Value::Logical(
                self.io
                    .portugol_any_mouse_button_pressed()
                    .map_err(|message| runtime_error(expression, message))?,
            ),
            "ler_botao" => Value::Integer(
                self.io
                    .portugol_read_mouse_button()
                    .map_err(|message| runtime_error(expression, message))? as i32
                    as i64,
            ),
            "posicao_x" | "posicao_y" => {
                let (x, y) = self
                    .io
                    .portugol_mouse_position()
                    .map_err(|message| runtime_error(expression, message))?;
                Value::Integer((if function_name == "posicao_x" { x } else { y }) as i32 as i64)
            }
            "ocultar_cursor" | "exibir_cursor" => {
                self.io
                    .portugol_mouse_set_cursor_visible(function_name == "exibir_cursor")
                    .map_err(|message| runtime_error(expression, message))?;
                Value::Null
            }
            _ => unreachable!("validated Mouse function"),
        };
        Ok(Some(value))
    }

    fn value_to_host(
        value: &Value,
        expression: &Expression,
        depth: usize,
    ) -> Result<HostValue, Diagnostic> {
        if depth > MAX_ARRAY_DIMENSIONS {
            return Err(runtime_error(
                expression,
                "o valor contém arrays aninhados em excesso",
            ));
        }
        match value {
            Value::Integer(value) => Ok(HostValue::Integer(*value)),
            Value::Real(value) => Ok(HostValue::Real(*value)),
            Value::Logical(value) => Ok(HostValue::Logical(*value)),
            Value::Character(value) => Ok(HostValue::Character(*value)),
            Value::String(value) => Ok(HostValue::String(value.clone())),
            Value::Array(values) => values
                .borrow()
                .iter()
                .map(|value| Self::value_to_host(&value.borrow(), expression, depth + 1))
                .collect::<Result<Vec<_>, _>>()
                .map(HostValue::Array),
            Value::Null => Ok(HostValue::Null),
        }
    }

    fn call_graphics_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = if name == "obter_RGB" {
            "obter_rgb"
        } else {
            name
        };
        let (arity, returns_integer) = match name {
            "iniciar_modo_grafico" => (1, false),
            "fechar_janela" => (0, false),
            "minimizar_janela" => (0, false),
            "restaurar_janela" => (0, false),
            "ocultar_borda_janela" => (0, false),
            "exibir_borda_janela" => (0, false),
            "encerrar_modo_grafico" => (0, false),
            "definir_dimensoes_janela" => (2, false),
            "definir_titulo_janela" => (1, false),
            "limpar" => (0, false),
            "renderizar" => (0, false),
            "renderizar_imagem" => (2, true),
            "desenhar_retangulo" => (6, false),
            "desenhar_poligono" => (2, false),
            "desenhar_elipse" => (5, false),
            "desenhar_ponto" => (2, false),
            "desenhar_linha" => (4, false),
            "carregar_imagem" => (1, true),
            "proximo_frame_gif" => (1, false),
            "transformar_imagem" => (5, true),
            "redimensionar_imagem" => (4, true),
            "obter_cor_pixel" => (3, true),
            "obter_RGB" => (2, true),
            "transformar_porcao_imagem" => (9, true),
            "desenhar_imagem" => (3, false),
            "salvar_imagem" => (2, false),
            "desenhar_quadro_atual_gif" => (3, false),
            "desenhar_porcao_imagem" => (7, false),
            "obter_intervalo_gif" => (1, true),
            "obter_numero_quadros_gif" => (1, true),
            "obter_numero_quadro_atual_gif" => (1, true),
            "obter_quadro_gif" => (2, true),
            "definir_quadro_gif" => (2, false),
            "liberar_imagem" => (1, false),
            "desenhar_texto" => (3, false),
            "definir_cor" => (1, false),
            "definir_gradiente" => (3, false),
            "definir_fonte_texto" => (1, false),
            "definir_tamanho_texto" => (1, false),
            "definir_estilo_texto" => (3, false),
            "largura_texto" => (1, true),
            "altura_texto" => (1, true),
            "largura_imagem" => (1, true),
            "altura_imagem" => (1, true),
            "criar_cor" => (3, true),
            "carregar_fonte" => (1, false),
            "definir_icone_janela" => (1, false),
            "definir_opacidade" => (1, false),
            "definir_rotacao" => (1, false),
            "largura_janela" => (0, true),
            "altura_janela" => (0, true),
            "largura_tela" => (0, true),
            "altura_tela" => (0, true),
            "entrar_modo_tela_cheia" => (0, false),
            "sair_modo_tela_cheia" => (0, false),
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'Graficos.{name}' espera {arity} argumento(s)"),
            ));
        }
        let host_arguments = arguments
            .iter()
            .map(|value| Self::value_to_host(value, expression, 0))
            .collect::<Result<Vec<_>, _>>()?;
        let result = self
            .io
            .portugol_graphics(&function_name, &host_arguments)
            .map_err(|message| runtime_error(expression, message))?;
        let value = match (returns_integer, result) {
            (true, HostValue::Integer(value)) => Value::Integer(value as i32 as i64),
            (false, HostValue::Null) => Value::Null,
            (true, _) => {
                return Err(runtime_error(
                    expression,
                    format!("a função 'Graficos.{name}' não retornou um inteiro"),
                ));
            }
            (false, _) => {
                return Err(runtime_error(
                    expression,
                    format!("a função 'Graficos.{name}' não retornou vazio"),
                ));
            }
        };
        Ok(Some(value))
    }

    fn call_audio_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = name.to_owned();
        let (arity, returns_integer) = match function_name.as_str() {
            "carregar_som" => (1, true),
            "liberar_som" | "pausar_som" | "interromper_som" | "definir_volume" => (1, false),
            "reproduzir_som" => (2, true),
            "obter_tamanho_musica" | "obter_posicao_atual_musica" | "obter_volume_reproducao" => {
                (1, true)
            }
            "obter_volume" => (0, true),
            "definir_posicao_atual_musica" | "definir_volume_reproducao" => (2, false),
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'Sons.{name}' espera {arity} argumento(s)"),
            ));
        }
        let host_arguments = arguments
            .iter()
            .map(|value| Self::value_to_host(value, expression, 0))
            .collect::<Result<Vec<_>, _>>()?;
        let result = self
            .io
            .portugol_audio(&function_name, &host_arguments)
            .map_err(|message| runtime_error(expression, message))?;
        let value = match (returns_integer, result) {
            (true, HostValue::Integer(value)) => Value::Integer(value as i32 as i64),
            (false, HostValue::Null) => Value::Null,
            (true, _) => {
                return Err(runtime_error(
                    expression,
                    format!("a função 'Sons.{name}' não retornou um inteiro"),
                ));
            }
            (false, _) => {
                return Err(runtime_error(
                    expression,
                    format!("a função 'Sons.{name}' não retornou vazio"),
                ));
            }
        };
        Ok(Some(value))
    }

    fn call_keyboard_builtin(
        &mut self,
        name: &str,
        arguments: &[Value],
        expression: &Expression,
    ) -> Result<Option<Value>, Diagnostic> {
        let function_name = name.to_owned();
        let arity = match function_name.as_str() {
            "tecla_pressionada" | "caracter_tecla" => 1,
            "alguma_tecla_pressionada" | "ler_tecla" => 0,
            _ => return Ok(None),
        };
        if arguments.len() != arity {
            return Err(runtime_error(
                expression,
                format!("a função 'Teclado.{name}' espera {arity} argumento(s)"),
            ));
        }
        let value = match function_name.as_str() {
            "tecla_pressionada" => {
                let key = arguments[0].as_integer(expression)?;
                Value::Logical(
                    self.io
                        .portugol_key_pressed(key)
                        .map_err(|message| runtime_error(expression, message))?,
                )
            }
            "alguma_tecla_pressionada" => Value::Logical(
                self.io
                    .portugol_any_key_pressed()
                    .map_err(|message| runtime_error(expression, message))?,
            ),
            "ler_tecla" => Value::Integer(
                self.io
                    .portugol_read_key()
                    .map_err(|message| runtime_error(expression, message))? as i32
                    as i64,
            ),
            "caracter_tecla" => {
                let key = arguments[0].as_integer(expression)? as u16;
                Value::Character(char::from_u32(u32::from(key)).ok_or_else(|| {
                    runtime_error(expression, "surrogate UTF-16 não pode ser representado")
                })?)
            }
            _ => unreachable!("validated Teclado function"),
        };
        Ok(Some(value))
    }

    fn send_http_request(
        &mut self,
        method: &str,
        url: &str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
        timeout_millis: u64,
        expression: &Expression,
    ) -> Result<crate::HttpResponse, Diagnostic> {
        if body.len() > MAX_HTTP_RESPONSE_BYTES {
            return Err(runtime_error(
                expression,
                "o corpo da requisição excede 10 MiB",
            ));
        }
        let response = self
            .io
            .portugol_http_request(HttpRequest {
                method: method.to_owned(),
                url: url.to_owned(),
                headers,
                body,
                timeout_millis,
            })
            .map_err(|message| runtime_error(expression, message))?;
        if response.body.len() > MAX_HTTP_RESPONSE_BYTES {
            return Err(runtime_error(expression, "a resposta HTTP excede 10 MiB"));
        }
        Ok(response)
    }

    fn next_random(&mut self, bits: u32) -> u32 {
        self.random_state = self
            .random_state
            .wrapping_mul(0x5DEECE66D)
            .wrapping_add(0xB)
            & ((1_u64 << 48) - 1);
        (self.random_state >> (48 - bits)) as u32
    }

    fn next_random_bounded(&mut self, bound: i32) -> i32 {
        if (bound as u32).is_power_of_two() {
            return ((i64::from(bound) * i64::from(self.next_random(31))) >> 31) as i32;
        }
        loop {
            let bits = self.next_random(31) as i32;
            let value = bits % bound;
            if bits.wrapping_sub(value).wrapping_add(bound - 1) >= 0 {
                return value;
            }
        }
    }

    fn tick(&mut self, line: usize, column: usize) -> Result<(), Diagnostic> {
        if self.io.should_stop() {
            self.stopped = true;
            return Ok(());
        }
        self.steps = self.steps.saturating_add(1);
        if self.steps > self.step_limit {
            Err(Diagnostic::new(
                DiagnosticKind::Runtime,
                "limite de instruções excedido; a execução foi interrompida",
                line,
                column,
            ))
        } else {
            Ok(())
        }
    }

    fn write_output(&mut self, text: &str, expression: &Expression) -> Result<(), Diagnostic> {
        let next_size = self
            .output_bytes
            .checked_add(text.len())
            .ok_or_else(|| runtime_error(expression, "limite de saída do programa excedido"))?;
        if next_size > self.output_limit {
            return Err(runtime_error(
                expression,
                format!(
                    "limite de saída do programa excedido ({} bytes)",
                    self.output_limit
                ),
            ));
        }
        self.output_bytes = next_size;
        self.io.write(text);
        Ok(())
    }

    fn validate_string_size(
        &self,
        value: &Value,
        expression: &Expression,
    ) -> Result<(), Diagnostic> {
        if let Value::String(text) = value
            && text.len() > self.string_limit
        {
            return Err(runtime_error(
                expression,
                format!("texto excede o limite de {} bytes", self.string_limit),
            ));
        }
        Ok(())
    }
}

fn make_array(ty: Type, dimensions: &[usize]) -> Value {
    let length = dimensions.first().copied().unwrap_or_default();
    let values = (0..length)
        .map(|_| {
            let value = if dimensions.len() > 1 {
                make_array(ty, &dimensions[1..])
            } else {
                Value::default_for(ty)
            };
            Rc::new(RefCell::new(value))
        })
        .collect();
    Value::Array(Rc::new(RefCell::new(values)))
}

fn evaluate_binary(
    operator: BinaryOperator,
    left: Value,
    right: Value,
    expression: &Expression,
) -> Result<Value, Diagnostic> {
    use BinaryOperator as Op;
    match operator {
        Op::Add => match (&left, &right) {
            (Value::Logical(left), Value::Logical(right)) => Ok(Value::Logical(*left || *right)),
            (Value::String(_), _) | (_, Value::String(_)) => Ok(Value::String(format!(
                "{}{}",
                left.to_text(),
                right.to_text()
            ))),
            (Value::Character(_), Value::Character(_)) => Ok(Value::String(format!(
                "{}{}",
                left.to_text(),
                right.to_text()
            ))),
            _ => numeric_binary(left, right, expression, i32::wrapping_add, |a, b| a + b),
        },
        Op::Subtract => numeric_binary(left, right, expression, i32::wrapping_sub, |a, b| a - b),
        Op::Multiply => numeric_binary(left, right, expression, i32::wrapping_mul, |a, b| a * b),
        Op::Divide => {
            if let (Value::Integer(left), Value::Integer(right)) = (&left, &right) {
                if *right == 0 {
                    return Err(runtime_error(expression, "divisão por zero"));
                }
                return Ok(Value::Integer(
                    (*left as i32).wrapping_div(*right as i32) as i64
                ));
            }
            let right_number = right.as_number(expression)?;
            let result = left.as_number(expression)? / right_number;
            Ok(Value::Real(result))
        }
        Op::Remainder => {
            let divisor = right.as_integer(expression)?;
            if divisor == 0 {
                return Err(runtime_error(expression, "resto de divisão por zero"));
            }
            Ok(Value::Integer(
                (left.as_integer(expression)? as i32).wrapping_rem(divisor as i32) as i64,
            ))
        }
        Op::Equal => Ok(Value::Logical(values_equal(&left, &right))),
        Op::NotEqual => Ok(Value::Logical(!values_equal(&left, &right))),
        Op::BitAnd => Ok(Value::Integer(
            ((left.as_integer(expression)? as i32) & (right.as_integer(expression)? as i32)) as i64,
        )),
        Op::BitOr => Ok(Value::Integer(
            ((left.as_integer(expression)? as i32) | (right.as_integer(expression)? as i32)) as i64,
        )),
        Op::BitXor => Ok(Value::Integer(
            ((left.as_integer(expression)? as i32) ^ (right.as_integer(expression)? as i32)) as i64,
        )),
        Op::ShiftLeft | Op::ShiftRight => {
            let left = left.as_integer(expression)?;
            let amount = right.as_integer(expression)?;
            let amount = amount as u32 & (i32::BITS - 1);
            Ok(Value::Integer(if operator == Op::ShiftLeft {
                (left as i32).wrapping_shl(amount) as i64
            } else {
                ((left as i32) >> amount) as i64
            }))
        }
        Op::Less | Op::LessEqual | Op::Greater | Op::GreaterEqual => {
            let comparison = match (&left, &right) {
                (Value::String(left), Value::String(right)) => left.cmp(right),
                (Value::Character(left), Value::Character(right)) => left.cmp(right),
                _ => {
                    let left = left.as_number(expression)?;
                    let right = right.as_number(expression)?;
                    left.partial_cmp(&right).ok_or_else(|| {
                        runtime_error(expression, "não é possível comparar estes valores")
                    })?
                }
            };
            Ok(Value::Logical(match operator {
                Op::Less => comparison == Ordering::Less,
                Op::LessEqual => comparison != Ordering::Greater,
                Op::Greater => comparison == Ordering::Greater,
                Op::GreaterEqual => comparison != Ordering::Less,
                _ => unreachable!(),
            }))
        }
        Op::And | Op::Or => {
            let left = left.as_logical(expression)?;
            let right = right.as_logical(expression)?;
            Ok(Value::Logical(if operator == Op::And {
                left && right
            } else {
                left || right
            }))
        }
    }
}

fn numeric_binary(
    left: Value,
    right: Value,
    expression: &Expression,
    integer_operation: impl FnOnce(i32, i32) -> i32,
    real_operation: impl FnOnce(f64, f64) -> f64,
) -> Result<Value, Diagnostic> {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(integer_operation(
            left as i32,
            right as i32,
        ) as i64)),
        (left, right) => {
            let result = real_operation(left.as_number(expression)?, right.as_number(expression)?);
            Ok(Value::Real(result))
        }
    }
}

fn values_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Integer(left), Value::Integer(right)) => left == right,
        (Value::Real(left), Value::Real(right)) => left == right,
        (Value::Integer(left), Value::Real(right)) => *left as f64 == *right,
        (Value::Real(left), Value::Integer(right)) => *left == *right as f64,
        (Value::Logical(left), Value::Logical(right)) => left == right,
        (Value::Character(left), Value::Character(right)) => left == right,
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Null, Value::Null) => true,
        _ => false,
    }
}

fn callable_name(expression: &Expression) -> Option<String> {
    match &expression.kind {
        ExpressionKind::Variable(name) => Some(name.clone()),
        ExpressionKind::Member { target, name } => {
            let ExpressionKind::Variable(module) = &target.kind else {
                return None;
            };
            Some(format!("{module}.{name}"))
        }
        _ => None,
    }
}

fn json_http_headers() -> Vec<(String, String)> {
    vec![
        ("Accept".to_owned(), "application/json".to_owned()),
        ("Content-Type".to_owned(), "application/json".to_owned()),
    ]
}

fn web_service_response(
    response: crate::HttpResponse,
    expression: &Expression,
) -> Result<String, Diagnostic> {
    if response.status != 200 {
        return Ok(format!(
            "Requisição falhou: código HTTP: {}",
            response.status
        ));
    }
    let text = String::from_utf8(response.body)
        .map_err(|_| runtime_error(expression, "a resposta não contém texto UTF-8 válido"))?;
    Ok(text.lines().collect())
}

fn call_math_builtin(
    name: &str,
    arguments: &[Value],
    expression: &Expression,
) -> Result<Option<Value>, Diagnostic> {
    let function_name = name
        .rsplit_once('.')
        .map(|(_, function)| function)
        .unwrap_or(name)
        .to_ascii_lowercase();
    let arity = match function_name.as_str() {
        "potencia" | "raiz" | "arredondar" | "logaritmo" | "maior_numero" | "menor_numero" => 2,
        "seno" | "cosseno" | "tangente" | "valor_absoluto" => 1,
        _ => return Ok(None),
    };
    if arguments.len() != arity {
        return Err(runtime_error(
            expression,
            format!("a função '{name}' espera {arity} argumento(s)"),
        ));
    }
    let first = arguments[0].as_number(expression)?;
    let second = if arity == 2 {
        arguments[1].as_number(expression)?
    } else {
        0.0
    };
    let value = match function_name.as_str() {
        "potencia" => first.powf(second),
        "raiz" => first.powf(1.0 / second),
        "arredondar" => {
            let places = arguments[1].as_integer(expression)?.max(0);
            let places = i32::try_from(places).unwrap_or(i32::MAX);
            let factor = 10_f64.powi(places);
            java_double_to_long((first * factor + 0.5).floor()) as f64 / factor
        }
        "logaritmo" => {
            if first <= 0.0 || second <= 0.0 || second == 1.0 {
                return Err(runtime_error(
                    expression,
                    "argumentos inválidos para logaritmo",
                ));
            }
            first.log(second)
        }
        "seno" => first.sin(),
        "cosseno" => first.cos(),
        "tangente" => first.tan(),
        "valor_absoluto" => first.abs(),
        "maior_numero" if first.is_nan() || second.is_nan() => f64::NAN,
        "maior_numero" => first.max(second),
        "menor_numero" if first.is_nan() || second.is_nan() => f64::NAN,
        "menor_numero" => first.min(second),
        _ => unreachable!(),
    };
    Ok(Some(Value::Real(value)))
}

fn call_text_builtin(
    name: &str,
    arguments: &[Value],
    expression: &Expression,
    max_string_bytes: usize,
) -> Result<Option<Value>, Diagnostic> {
    let function_name = name.to_owned();
    let arity = match function_name.as_str() {
        "numero_caracteres" | "caixa_alta" | "caixa_baixa" => 1,
        "substituir" | "preencher_a_esquerda" | "posicao_texto" | "extrair_subtexto" => 3,
        "obter_caracter" => 2,
        _ => return Ok(None),
    };
    if arguments.len() != arity {
        return Err(runtime_error(
            expression,
            format!("a função 'Texto.{name}' espera {arity} argumento(s)"),
        ));
    }

    let string_argument = |index: usize| -> Result<String, Diagnostic> {
        match &arguments[index] {
            Value::String(value) => Ok(value.clone()),
            Value::Character(value) => Ok(value.to_string()),
            value => Err(runtime_error(
                expression,
                format!("esperado um texto, recebido {}", value.type_name()),
            )),
        }
    };
    let integer_argument =
        |index: usize| -> Result<i64, Diagnostic> { arguments[index].as_integer(expression) };

    let value = match function_name.as_str() {
        "numero_caracteres" => {
            let count = i64::try_from(string_argument(0)?.encode_utf16().count())
                .map_err(|_| runtime_error(expression, "texto excede o limite de caracteres"))?;
            Value::Integer(count)
        }
        "caixa_alta" => Value::String(transform_case(
            &string_argument(0)?,
            true,
            max_string_bytes,
            expression,
        )?),
        "caixa_baixa" => Value::String(transform_case(
            &string_argument(0)?,
            false,
            max_string_bytes,
            expression,
        )?),
        "substituir" => {
            let source = string_argument(0)?;
            let search = string_argument(1)?;
            let replacement = string_argument(2)?;
            Value::String(replace_all_bounded(
                &source,
                &search,
                &replacement,
                max_string_bytes,
                expression,
            )?)
        }
        "preencher_a_esquerda" => {
            let character = match &arguments[0] {
                Value::Character(value) => *value,
                value => {
                    return Err(runtime_error(
                        expression,
                        format!("esperado um caracter, recebido {}", value.type_name()),
                    ));
                }
            };
            let source = string_argument(2)?;
            let width = integer_argument(1)?;
            let source_units = source.encode_utf16().count();
            if width <= source_units as i64 {
                return Ok(Some(Value::String(source)));
            }
            let width = usize::try_from(width)
                .map_err(|_| runtime_error(expression, "tamanho de texto fora do limite"))?;
            let padding = width.saturating_sub(source_units);
            let padding_bytes = padding
                .checked_mul(character.len_utf8())
                .ok_or_else(|| runtime_error(expression, "texto excede o limite permitido"))?;
            let total_bytes = padding_bytes
                .checked_add(source.len())
                .ok_or_else(|| runtime_error(expression, "texto excede o limite permitido"))?;
            if total_bytes > max_string_bytes {
                return Err(runtime_error(
                    expression,
                    format!("texto excede o limite de {max_string_bytes} bytes"),
                ));
            }
            let mut padded = String::with_capacity(total_bytes);
            for _ in 0..padding {
                padded.push(character);
            }
            padded.push_str(&source);
            Value::String(padded)
        }
        "obter_caracter" => {
            let source = string_argument(0)?;
            let index = usize::try_from(integer_argument(1)?)
                .map_err(|_| runtime_error(expression, "índice de texto negativo"))?;
            let unit = source
                .encode_utf16()
                .nth(index)
                .ok_or_else(|| runtime_error(expression, "índice de texto fora do limite"))?;
            let character = char::from_u32(u32::from(unit)).ok_or_else(|| {
                runtime_error(
                    expression,
                    "um surrogate UTF-16 isolado não pode ser representado",
                )
            })?;
            Value::Character(character)
        }
        "posicao_texto" => {
            let search = string_argument(0)?;
            let source = string_argument(1)?;
            let start = integer_argument(2)?;
            let position = utf16_index_of(&source, &search, start)
                .map(|position| position as i32 as i64)
                .unwrap_or(-1);
            Value::Integer(position)
        }
        "extrair_subtexto" => {
            let source = string_argument(0)?;
            let start = usize::try_from(integer_argument(1)?)
                .map_err(|_| runtime_error(expression, "posição inicial negativa"))?;
            let end = usize::try_from(integer_argument(2)?)
                .map_err(|_| runtime_error(expression, "posição final negativa"))?;
            let units: Vec<u16> = source.encode_utf16().collect();
            let character_count = units.len();
            if start > end || end > character_count {
                return Err(runtime_error(
                    expression,
                    "intervalo de texto fora do limite",
                ));
            }
            let substring = String::from_utf16(&units[start..end])
                .map_err(|_| runtime_error(expression, "o intervalo divide um caractere UTF-16"))?;
            Value::String(substring)
        }
        _ => unreachable!("validated text library function"),
    };
    Ok(Some(value))
}

fn transform_case(
    source: &str,
    uppercase: bool,
    max_string_bytes: usize,
    expression: &Expression,
) -> Result<String, Diagnostic> {
    let mut transformed = String::with_capacity(source.len().min(max_string_bytes));
    let characters: Vec<char> = source.chars().collect();
    for (index, character) in characters.iter().copied().enumerate() {
        if uppercase {
            for mapped in character.to_uppercase() {
                push_bounded_char(&mut transformed, mapped, max_string_bytes, expression)?;
            }
        } else if character == 'Σ'
            && has_cased_character_before(&characters, index)
            && !has_cased_character_after(&characters, index)
        {
            push_bounded_char(&mut transformed, 'ς', max_string_bytes, expression)?;
        } else {
            for mapped in character.to_lowercase() {
                push_bounded_char(&mut transformed, mapped, max_string_bytes, expression)?;
            }
        }
    }
    Ok(transformed)
}

fn has_cased_character_before(characters: &[char], index: usize) -> bool {
    characters[..index]
        .iter()
        .rev()
        .find(|character| !is_case_ignorable(**character))
        .is_some_and(|character| character.is_lowercase() || character.is_uppercase())
}

fn has_cased_character_after(characters: &[char], index: usize) -> bool {
    characters[index + 1..]
        .iter()
        .find(|character| !is_case_ignorable(**character))
        .is_some_and(|character| character.is_lowercase() || character.is_uppercase())
}

fn is_case_ignorable(character: char) -> bool {
    // shortcut: final-sigma context uses a curated Unicode range set; use full Unicode case data if parity requires every code point.
    if character.is_alphanumeric() || character.is_whitespace() {
        return false;
    }
    character.is_ascii_punctuation()
        || matches!(
            character,
            '\u{00ad}'
                | '\u{034f}'
                | '\u{061c}'
                | '\u{180e}'
                | '\u{200b}'..='\u{200f}'
                | '\u{202a}'..='\u{202e}'
                | '\u{2060}'..='\u{206f}'
                | '\u{feff}'
                | '\u{0300}'..='\u{036f}'
                | '\u{0483}'..='\u{0489}'
                | '\u{0591}'..='\u{05bd}'
                | '\u{05bf}'
                | '\u{05c1}'..='\u{05c2}'
                | '\u{0610}'..='\u{061a}'
                | '\u{064b}'..='\u{065f}'
                | '\u{0670}'
                | '\u{06d6}'..='\u{06ed}'
                | '\u{0900}'..='\u{0903}'
                | '\u{093a}'..='\u{094f}'
                | '\u{1ab0}'..='\u{1aff}'
                | '\u{1dc0}'..='\u{1dff}'
                | '\u{20d0}'..='\u{20ff}'
                | '\u{2e00}'..='\u{2e7f}'
                | '\u{3001}'..='\u{303f}'
                | '\u{fe10}'..='\u{fe1f}'
                | '\u{fe30}'..='\u{fe4f}'
                | '\u{fe00}'..='\u{fe0f}'
                | '\u{ff01}'..='\u{ff0f}'
                | '\u{ff1a}'..='\u{ff20}'
                | '\u{ff3b}'..='\u{ff40}'
                | '\u{ff5b}'..='\u{ff65}'
                | '\u{1f3fb}'..='\u{1f3ff}'
        )
}

fn utf16_index_of(source: &str, search: &str, start: i64) -> Option<usize> {
    let source: Vec<u16> = source.encode_utf16().collect();
    let search: Vec<u16> = search.encode_utf16().collect();
    let start = usize::try_from(start.max(0)).ok()?.min(source.len());
    if search.is_empty() {
        return Some(start);
    }
    source[start..]
        .windows(search.len())
        .position(|window| window == search)
        .map(|offset| start + offset)
}

fn push_bounded_char(
    output: &mut String,
    character: char,
    max_string_bytes: usize,
    expression: &Expression,
) -> Result<(), Diagnostic> {
    let next_size = output
        .len()
        .checked_add(character.len_utf8())
        .ok_or_else(|| runtime_error(expression, "texto excede o limite permitido"))?;
    if next_size > max_string_bytes {
        return Err(runtime_error(
            expression,
            format!("texto excede o limite de {max_string_bytes} bytes"),
        ));
    }
    output.push(character);
    Ok(())
}

fn replace_all_bounded(
    source: &str,
    search: &str,
    replacement: &str,
    max_string_bytes: usize,
    expression: &Expression,
) -> Result<String, Diagnostic> {
    if search.is_empty() {
        if replacement.is_empty() {
            if source.len() > max_string_bytes {
                return Err(runtime_error(
                    expression,
                    format!("texto excede o limite de {max_string_bytes} bytes"),
                ));
            }
            return Ok(source.to_owned());
        }
        let source_units = source.encode_utf16().count();
        if source_units != source.chars().count() {
            return Err(runtime_error(
                expression,
                "a substituição separaria unidades surrogate UTF-16",
            ));
        }
        let occurrences = source_units
            .checked_add(1)
            .ok_or_else(|| runtime_error(expression, "texto excede o limite permitido"))?;
        let output_bytes = source
            .len()
            .checked_add(
                replacement
                    .len()
                    .checked_mul(occurrences)
                    .ok_or_else(|| runtime_error(expression, "texto excede o limite permitido"))?,
            )
            .ok_or_else(|| runtime_error(expression, "texto excede o limite permitido"))?;
        if output_bytes > max_string_bytes {
            return Err(runtime_error(
                expression,
                format!("texto excede o limite de {max_string_bytes} bytes"),
            ));
        }
        let mut output = String::with_capacity(output_bytes);
        output.push_str(replacement);
        for character in source.chars() {
            output.push(character);
            output.push_str(replacement);
        }
        return Ok(output);
    }
    let occurrences = source.match_indices(search).count();
    let removed_bytes = search
        .len()
        .checked_mul(occurrences)
        .ok_or_else(|| runtime_error(expression, "texto excede o limite permitido"))?;
    let added_bytes = replacement
        .len()
        .checked_mul(occurrences)
        .ok_or_else(|| runtime_error(expression, "texto excede o limite permitido"))?;
    let output_bytes = source
        .len()
        .checked_sub(removed_bytes)
        .and_then(|remaining| remaining.checked_add(added_bytes))
        .ok_or_else(|| runtime_error(expression, "texto excede o limite permitido"))?;
    if output_bytes > max_string_bytes {
        return Err(runtime_error(
            expression,
            format!("texto excede o limite de {max_string_bytes} bytes"),
        ));
    }
    Ok(source.replace(search, replacement))
}

fn call_types_builtin(
    name: &str,
    arguments: &[Value],
    expression: &Expression,
) -> Result<Option<Value>, Diagnostic> {
    let function_name = name.to_owned();
    let arity = match function_name.as_str() {
        "cadeia_e_real"
        | "cadeia_e_logico"
        | "cadeia_e_caracter"
        | "cadeia_para_caracter"
        | "cadeia_para_real"
        | "cadeia_para_logico"
        | "inteiro_e_caracter"
        | "inteiro_para_caracter"
        | "inteiro_para_logico"
        | "inteiro_para_real"
        | "caracter_e_inteiro"
        | "caracter_e_logico"
        | "caracter_para_cadeia"
        | "caracter_para_inteiro"
        | "caracter_para_logico"
        | "logico_para_cadeia"
        | "logico_para_inteiro"
        | "logico_para_caracter"
        | "real_para_cadeia"
        | "real_para_inteiro" => 1,
        "cadeia_e_inteiro" | "cadeia_para_inteiro" | "inteiro_para_cadeia" => 2,
        _ => return Ok(None),
    };
    if arguments.len() != arity {
        return Err(runtime_error(
            expression,
            format!("a função 'Tipos.{name}' espera {arity} argumento(s)"),
        ));
    }

    let text_argument = |index: usize| -> Result<String, Diagnostic> {
        match &arguments[index] {
            Value::String(value) => Ok(value.clone()),
            Value::Character(value) => Ok(value.to_string()),
            value => Err(runtime_error(
                expression,
                format!("esperado um texto, recebido {}", value.type_name()),
            )),
        }
    };
    let character_argument = |index: usize| -> Result<char, Diagnostic> {
        match arguments.get(index) {
            Some(Value::Character(value)) => Ok(*value),
            Some(value) => Err(runtime_error(
                expression,
                format!("esperado um caracter, recebido {}", value.type_name()),
            )),
            None => Err(runtime_error(expression, "argumento ausente")),
        }
    };
    let integer_argument =
        |index: usize| -> Result<i64, Diagnostic> { arguments[index].as_integer(expression) };
    let real_argument =
        |index: usize| -> Result<f64, Diagnostic> { arguments[index].as_number(expression) };
    let logical_argument =
        |index: usize| -> Result<bool, Diagnostic> { arguments[index].as_logical(expression) };
    let radix_argument = |index: usize| -> Result<u32, Diagnostic> {
        match integer_argument(index)? {
            2 => Ok(2),
            10 => Ok(10),
            16 => Ok(16),
            _ => Err(runtime_error(expression, "a base deve ser 2, 10 ou 16")),
        }
    };

    let value = match function_name.as_str() {
        "cadeia_e_inteiro" => {
            let text = text_argument(0)?;
            let radix = radix_argument(1)?;
            Value::Logical(is_integer_text(&text, radix))
        }
        "cadeia_e_real" => Value::Logical(is_real_text(&text_argument(0)?)),
        "cadeia_e_logico" => {
            let text = text_argument(0)?;
            Value::Logical(
                text.starts_with("verdadeiro")
                    || strip_final_line_terminator(&text).ends_with("falso"),
            )
        }
        "cadeia_e_caracter" => Value::Logical(text_argument(0)?.encode_utf16().count() == 1),
        "cadeia_para_caracter" => {
            let text = text_argument(0)?;
            if text.encode_utf16().count() != 1 {
                return Err(runtime_error(
                    expression,
                    "a cadeia deve conter um caractere",
                ));
            }
            let mut characters = text.chars();
            let character = characters
                .next()
                .filter(|_| characters.next().is_none())
                .ok_or_else(|| runtime_error(expression, "surrogate UTF-16 não representável"))?;
            Value::Character(character)
        }
        "cadeia_para_inteiro" => {
            let text = text_argument(0)?;
            let radix = radix_argument(1)?;
            Value::Integer(parse_radix_integer(&text, radix).map_err(|_| {
                runtime_error(expression, format!("'{text}' não é um inteiro válido"))
            })?)
        }
        "cadeia_para_real" => {
            let text = text_argument(0)?;
            Value::Real(parse_java_double(&text).ok_or_else(|| {
                runtime_error(expression, format!("'{text}' não é um número real válido"))
            })?)
        }
        "cadeia_para_logico" => match text_argument(0)?.as_str() {
            "verdadeiro" => Value::Logical(true),
            "falso" => Value::Logical(false),
            text => {
                return Err(runtime_error(
                    expression,
                    format!("'{text}' não é um valor lógico válido"),
                ));
            }
        },
        "inteiro_e_caracter" => Value::Logical((0..=9).contains(&integer_argument(0)?)),
        "inteiro_para_cadeia" => {
            let value = integer_argument(0)?;
            let radix = radix_argument(1)?;
            let value = i32::try_from(value)
                .map_err(|_| runtime_error(expression, "número fora do limite de inteiro"))?;
            Value::String(integer_to_radix_string(value, radix))
        }
        "inteiro_para_caracter" => {
            let value = integer_argument(0)?;
            let value = u32::try_from(value)
                .ok()
                .filter(|value| *value <= 9)
                .and_then(|value| char::from_digit(value, 10))
                .ok_or_else(|| runtime_error(expression, "o inteiro não representa um dígito"))?;
            Value::Character(value)
        }
        "inteiro_para_logico" => Value::Logical(integer_argument(0)? > 0),
        "inteiro_para_real" => Value::Real(integer_argument(0)? as f64),
        "caracter_e_inteiro" => {
            Value::Logical(is_integer_text(&character_argument(0)?.to_string(), 10))
        }
        "caracter_e_logico" => Value::Logical(matches!(
            character_argument(0)?.to_ascii_lowercase(),
            's' | 'n'
        )),
        "caracter_para_cadeia" => Value::String(character_argument(0)?.to_string()),
        "caracter_para_inteiro" => Value::Integer(
            character_argument(0)?
                .to_digit(10)
                .map(i64::from)
                .ok_or_else(|| runtime_error(expression, "o caracter não é um dígito"))?,
        ),
        "caracter_para_logico" => match character_argument(0)?.to_ascii_lowercase() {
            's' => Value::Logical(true),
            'n' => Value::Logical(false),
            _ => {
                return Err(runtime_error(
                    expression,
                    "use 'S' ou 'N' para o valor lógico",
                ));
            }
        },
        "logico_para_cadeia" => Value::String(if logical_argument(0)? {
            "verdadeiro".to_owned()
        } else {
            "falso".to_owned()
        }),
        "logico_para_inteiro" => Value::Integer(i64::from(logical_argument(0)?)),
        "logico_para_caracter" => Value::Character(if logical_argument(0)? { 'S' } else { 'N' }),
        "real_para_cadeia" => Value::String(java_double_to_string(real_argument(0)?)),
        "real_para_inteiro" => Value::Integer(i64::from(java_double_to_int(real_argument(0)?))),
        _ => unreachable!("validated types library function"),
    };
    Ok(Some(value))
}

fn integer_to_radix_string(value: i32, radix: u32) -> String {
    match radix {
        2 => format!("{:032b}", value as u32),
        10 => value.to_string(),
        16 => format!("{:08X}", value as u32),
        _ => unreachable!("validated radix"),
    }
}

fn is_integer_text(text: &str, radix: u32) -> bool {
    let text = strip_final_line_terminator(text);
    let digits = match radix {
        2 => text
            .strip_prefix("0b")
            .or_else(|| text.strip_prefix("0B"))
            .unwrap_or(text),
        10 => text.strip_prefix('-').unwrap_or(text),
        16 => text
            .strip_prefix("0x")
            .or_else(|| text.strip_prefix("0X"))
            .unwrap_or(text),
        _ => return false,
    };
    !digits.is_empty()
        && match radix {
            2 => digits.bytes().all(|byte| matches!(byte, b'0' | b'1')),
            10 => digits.bytes().all(|byte| byte.is_ascii_digit()),
            16 => digits.bytes().all(|byte| byte.is_ascii_hexdigit()),
            _ => false,
        }
}

fn is_real_text(text: &str) -> bool {
    let text = strip_final_line_terminator(text);
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let Some((whole, fraction)) = unsigned.split_once('.') else {
        return false;
    };
    !whole.is_empty()
        && !fraction.is_empty()
        && whole.bytes().all(|byte| byte.is_ascii_digit())
        && fraction.bytes().all(|byte| byte.is_ascii_digit())
}

fn strip_final_line_terminator(text: &str) -> &str {
    if let Some(text) = text.strip_suffix("\r\n") {
        text
    } else {
        match text.chars().next_back() {
            Some(character)
                if matches!(
                    character,
                    '\n' | '\r' | '\u{0085}' | '\u{2028}' | '\u{2029}'
                ) =>
            {
                &text[..text.len() - character.len_utf8()]
            }
            _ => text,
        }
    }
}

fn parse_radix_integer(text: &str, radix: u32) -> Result<i64, ()> {
    let (negative, unsigned) = text.strip_prefix('-').map_or_else(
        || (false, text.strip_prefix('+').unwrap_or(text)),
        |unsigned| (true, unsigned),
    );
    let prefixes: &[&str] = match radix {
        2 => &["0B", "0b"],
        10 => &[],
        16 => &["0X", "0x"],
        _ => return Err(()),
    };
    let mut digits = unsigned.to_owned();
    for prefix in prefixes {
        if let Some(start) = digits.find(prefix) {
            digits.replace_range(start..start + prefix.len(), "");
        }
    }
    if digits.is_empty() {
        return Err(());
    }
    let mut value = 0_i128;
    for character in digits.chars() {
        let digit = java_digit(character, radix).ok_or(())?;
        if digit >= radix {
            return Err(());
        }
        value = value
            .checked_mul(i128::from(radix))
            .and_then(|value| value.checked_add(i128::from(digit)))
            .ok_or(())?;
    }
    let value = if negative { -value } else { value };
    let value = i64::try_from(value).map_err(|_| ())?;
    Ok(i64::from(value as i32))
}

fn java_digit(character: char, radix: u32) -> Option<u32> {
    if let Some(digit) = character.to_digit(radix) {
        return Some(digit);
    }
    let codepoint = u32::from(character);
    // shortcut: covers Unicode decimal digit blocks known to the reference JDK; update with JDK data if new blocks matter.
    const DECIMAL_ZEROES: &[u32] = &[
        0x0660, 0x06f0, 0x07c0, 0x0966, 0x09e6, 0x0a66, 0x0ae6, 0x0b66, 0x0be6, 0x0c66, 0x0ce6,
        0x0d66, 0x0de6, 0x0e50, 0x0ed0, 0x0f20, 0x1040, 0x1090, 0x17e0, 0x1810, 0x1946, 0x19d0,
        0x1a80, 0x1a90, 0x1b50, 0x1bb0, 0x1c40, 0x1c50, 0xa620, 0xa8d0, 0xa900, 0xa9d0, 0xa9f0,
        0xaa50, 0xabf0, 0xff10, 0x104a0, 0x10d30, 0x11066, 0x110f0, 0x11136, 0x111d0, 0x112f0,
        0x11450, 0x114d0, 0x11650, 0x116c0, 0x11730, 0x118e0, 0x11950, 0x11c50, 0x11d50, 0x11da0,
        0x11f50, 0x16a60, 0x16ac0, 0x16b50, 0x1d7ce, 0x1e140, 0x1e2f0, 0x1e4f0, 0x1e5f1, 0x1e950,
        0x1fbf0,
    ];
    if let Some(zero) = DECIMAL_ZEROES
        .iter()
        .find(|&&zero| codepoint >= zero && codepoint < zero + 10)
    {
        return Some(codepoint - *zero);
    }
    let fullwidth_letter = match codepoint {
        0xff21..=0xff3a => codepoint - 0xff21 + 10,
        0xff41..=0xff5a => codepoint - 0xff41 + 10,
        _ => return None,
    };
    (fullwidth_letter < radix).then_some(fullwidth_letter)
}

fn parse_input(text: &str, ty: Type, expression: &Expression) -> Result<Value, Diagnostic> {
    match ty {
        Type::Void => Err(runtime_error(
            expression,
            "o tipo vazio não pode receber entrada",
        )),
        Type::Integer => text
            .trim()
            .parse::<i32>()
            .map(|value| Value::Integer(i64::from(value)))
            .map_err(|_| {
                runtime_error(
                    expression,
                    format!("'{text}' não é um número inteiro válido"),
                )
            }),
        Type::Real => {
            let normalized = text.trim().replace(',', ".");
            parse_java_double(&normalized)
                .map(Value::Real)
                .ok_or_else(|| {
                    runtime_error(expression, format!("'{text}' não é um número real válido"))
                })
        }
        Type::Logical => match text.trim().to_ascii_lowercase().as_str() {
            "verdadeiro" | "true" | "1" => Ok(Value::Logical(true)),
            "falso" | "false" | "0" => Ok(Value::Logical(false)),
            _ => Err(runtime_error(
                expression,
                format!("'{text}' não é um valor lógico válido"),
            )),
        },
        Type::Character => {
            if text.encode_utf16().count() == 1 {
                Ok(Value::Character(
                    text.chars().next().expect("one UTF-16 unit"),
                ))
            } else {
                Err(runtime_error(
                    expression,
                    "a entrada deve conter exatamente um caractere",
                ))
            }
        }
        Type::String => Ok(Value::String(text.to_owned())),
    }
}

fn parse_java_double(source: &str) -> Option<f64> {
    let text = source.trim_matches(|character: char| character <= '\u{20}');
    let (negative, unsigned) = text
        .strip_prefix('-')
        .map_or((false, text.strip_prefix('+').unwrap_or(text)), |text| {
            (true, text)
        });
    if unsigned == "NaN" {
        return Some(f64::NAN);
    }
    if unsigned == "Infinity" {
        return Some(if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }

    let without_suffix = if unsigned
        .chars()
        .last()
        .is_some_and(|character| matches!(character, 'f' | 'F' | 'd' | 'D'))
    {
        &unsigned[..unsigned.len() - 1]
    } else {
        unsigned
    };
    let sign = if negative { -1.0 } else { 1.0 };
    let hex = without_suffix
        .strip_prefix("0x")
        .or_else(|| without_suffix.strip_prefix("0X"));
    if let Some(hex) = hex {
        let exponent_index = hex.find(|character| character == 'p' || character == 'P')?;
        let (significand, exponent) = hex.split_at(exponent_index);
        let exponent = parse_saturated_exponent(&exponent[1..])?;
        let mut mantissa = 0.0;
        let mut fractional_digits = 0_i64;
        let mut discarded_digits = 0_i64;
        let mut after_decimal = false;
        let mut saw_digit = false;
        let mut saw_significant_digit = false;
        let mut kept_significant_digits = 0;
        for character in significand.chars() {
            if character == '.' && !after_decimal {
                after_decimal = true;
                continue;
            }
            let digit = character.to_digit(16)?;
            saw_digit = true;
            if after_decimal {
                fractional_digits += 1;
            }
            if saw_significant_digit || digit != 0 {
                saw_significant_digit = true;
                if kept_significant_digits < 16 {
                    mantissa = mantissa * 16.0 + f64::from(digit);
                    kept_significant_digits += 1;
                } else {
                    discarded_digits += 1;
                }
            }
        }
        if !saw_digit {
            return None;
        }
        if mantissa == 0.0 {
            return Some(sign * 0.0);
        }
        let scale = i64::from(exponent).saturating_add(
            discarded_digits
                .saturating_sub(fractional_digits)
                .saturating_mul(4),
        );
        let mantissa_exponent = mantissa.log2().floor() as i64;
        let normalized_mantissa = mantissa / 2_f64.powi(mantissa_exponent as i32);
        let exponent = scale.saturating_add(mantissa_exponent);
        let magnitude = if exponent > 1023 {
            f64::INFINITY
        } else if exponent < -1075 {
            0.0
        } else if exponent == -1075 {
            if normalized_mantissa > 1.0 {
                f64::from_bits(1)
            } else {
                0.0
            }
        } else {
            normalized_mantissa * 2_f64.powi(exponent as i32)
        };
        return Some(sign * magnitude);
    }
    if !is_java_decimal_literal(without_suffix) {
        return None;
    }
    let value = without_suffix.parse::<f64>().ok()?;
    Some(if negative { -value } else { value })
}

fn parse_saturated_exponent(source: &str) -> Option<i32> {
    let (negative, digits) = source.strip_prefix('-').map_or(
        (false, source.strip_prefix('+').unwrap_or(source)),
        |digits| (true, digits),
    );
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some(
        source
            .parse()
            .unwrap_or(if negative { i32::MIN } else { i32::MAX }),
    )
}

fn is_java_decimal_literal(text: &str) -> bool {
    let (significand, exponent) = text.find(['e', 'E']).map_or((text, None), |index| {
        (&text[..index], Some(&text[index + 1..]))
    });
    if let Some(exponent) = exponent {
        let digits = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
    }
    let mut parts = significand.split('.');
    let whole = parts.next().unwrap_or_default();
    let fraction = parts.next();
    if parts.next().is_some() {
        return false;
    }
    let digits = whole.bytes().chain(fraction.unwrap_or_default().bytes());
    let mut count = 0;
    for digit in digits {
        if !digit.is_ascii_digit() {
            return false;
        }
        count += 1;
    }
    count > 0
}

fn java_double_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value == f64::INFINITY {
        return "Infinity".to_owned();
    }
    if value == f64::NEG_INFINITY {
        return "-Infinity".to_owned();
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .to_owned();
    }

    let magnitude = value.abs();
    let mut text = if magnitude >= 1e7 || magnitude < 1e-3 {
        format!("{value:e}")
    } else {
        value.to_string()
    };
    if let Some((mantissa, exponent)) = text.split_once('e') {
        let mantissa = if mantissa.contains('.') {
            mantissa.to_owned()
        } else {
            format!("{mantissa}.0")
        };
        let exponent = exponent.parse::<i32>().unwrap_or_default();
        text = format!("{mantissa}E{exponent}");
    } else if !text.contains('.') {
        text.push_str(".0");
    }
    text
}

fn java_double_to_int(value: f64) -> i32 {
    if value.is_nan() {
        0
    } else if value >= i32::MAX as f64 {
        i32::MAX
    } else if value <= i32::MIN as f64 {
        i32::MIN
    } else {
        value.trunc() as i32
    }
}

fn java_double_to_long(value: f64) -> i64 {
    if value.is_nan() {
        0
    } else if value >= i64::MAX as f64 {
        i64::MAX
    } else if value <= i64::MIN as f64 {
        i64::MIN
    } else {
        value as i64
    }
}

fn decode_http_text(headers: &[(String, String)], body: &[u8]) -> Result<String, String> {
    let charset = headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case("content-type"))
        .and_then(|(_, content_type)| {
            content_type.split(';').skip(1).find_map(|parameter| {
                let (name, value) = parameter.trim().split_once('=')?;
                name.trim().eq_ignore_ascii_case("charset").then(|| {
                    value
                        .trim()
                        .trim_matches(|character| character == '"' || character == '\'')
                        .to_ascii_lowercase()
                })
            })
        })
        .unwrap_or_else(|| "iso-8859-1".to_owned());
    match charset.as_str() {
        "utf-8" | "utf8" => Ok(String::from_utf8_lossy(body).into_owned()),
        "iso-8859-1" | "iso8859-1" | "latin1" | "latin-1" => {
            Ok(body.iter().map(|byte| char::from(*byte)).collect())
        }
        "windows-1252" | "cp1252" => Ok(body
            .iter()
            .map(|byte| match *byte {
                0x80 => '€',
                0x82 => '‚',
                0x83 => 'ƒ',
                0x84 => '„',
                0x85 => '…',
                0x86 => '†',
                0x87 => '‡',
                0x88 => 'ˆ',
                0x89 => '‰',
                0x8a => 'Š',
                0x8b => '‹',
                0x8c => 'Œ',
                0x8e => 'Ž',
                0x91 => '‘',
                0x92 => '’',
                0x93 => '“',
                0x94 => '”',
                0x95 => '•',
                0x96 => '–',
                0x97 => '—',
                0x98 => '˜',
                0x99 => '™',
                0x9a => 'š',
                0x9b => '›',
                0x9c => 'œ',
                0x9e => 'ž',
                0x9f => 'Ÿ',
                byte => char::from(byte),
            })
            .collect()),
        "us-ascii" | "ascii" => Ok(body
            .iter()
            .map(|byte| {
                if byte.is_ascii() {
                    char::from(*byte)
                } else {
                    '\u{fffd}'
                }
            })
            .collect()),
        "utf-16le" | "utf16le" => Ok(String::from_utf16_lossy(
            &body
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect::<Vec<_>>(),
        )),
        "utf-16be" | "utf16be" => Ok(String::from_utf16_lossy(
            &body
                .chunks_exact(2)
                .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                .collect::<Vec<_>>(),
        )),
        _ => Err(format!("charset HTTP não suportado: {charset}")),
    }
}

fn runtime_error<T: HasLocation>(located: &T, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticKind::Runtime,
        message,
        located.line(),
        located.column(),
    )
}

trait HasLocation {
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

impl<T: HasLocation> HasLocation for Box<T> {
    fn line(&self) -> usize {
        self.as_ref().line()
    }
    fn column(&self) -> usize {
        self.as_ref().column()
    }
}

fn type_name(ty: Type) -> &'static str {
    match ty {
        Type::Integer => "inteiro",
        Type::Real => "real",
        Type::Logical => "logico",
        Type::Character => "caracter",
        Type::String => "cadeia",
        Type::Void => "vazio",
    }
}

fn value_type(value: &Value) -> Option<Type> {
    match value {
        Value::Integer(_) => Some(Type::Integer),
        Value::Real(_) => Some(Type::Real),
        Value::Logical(_) => Some(Type::Logical),
        Value::Character(_) => Some(Type::Character),
        Value::String(_) => Some(Type::String),
        Value::Array(_) | Value::Null => None,
    }
}

fn string_array(value: &Value, expression: &Expression) -> Result<Vec<String>, Diagnostic> {
    let Value::Array(values) = value else {
        return Err(runtime_error(expression, "esperado um vetor de cadeia"));
    };
    values
        .borrow()
        .iter()
        .map(|value| match &*value.borrow() {
            Value::String(value) => Ok(value.clone()),
            other => Err(runtime_error(
                expression,
                format!(
                    "esperado um vetor de cadeia, recebido vetor de {}",
                    other.type_name()
                ),
            )),
        })
        .collect()
}

fn set_string_array(
    target: &Value,
    entries: &[String],
    max_string_bytes: usize,
    expression: &Expression,
) -> Result<(), Diagnostic> {
    let Value::Array(values) = target else {
        return Err(runtime_error(expression, "esperado um vetor de cadeia"));
    };
    if entries.iter().any(|entry| entry.len() > max_string_bytes) {
        return Err(runtime_error(
            expression,
            format!("texto excede o limite de {max_string_bytes} bytes"),
        ));
    }
    let values = values.borrow_mut();
    if entries.len() > values.len() {
        return Err(runtime_error(
            expression,
            format!(
                "vetor destino comporta {} elementos, mas são necessários {}",
                values.len(),
                entries.len()
            ),
        ));
    }
    for (slot, entry) in values.iter().zip(entries) {
        *slot.borrow_mut() = Value::String(entry.clone());
    }
    for slot in values.iter().skip(entries.len()) {
        *slot.borrow_mut() = Value::String(String::new());
    }
    Ok(())
}

fn property_value_from_runtime(value: &Value) -> PropertyValue {
    match value {
        Value::Integer(value) => PropertyValue::Integer(*value),
        Value::Real(value) => PropertyValue::Real(*value),
        Value::Logical(value) => PropertyValue::Logical(*value),
        Value::Character(value) => PropertyValue::Character(*value),
        Value::String(value) => PropertyValue::String(value.clone()),
        Value::Array(values) => PropertyValue::Array(
            values
                .borrow()
                .iter()
                .map(|value| property_value_from_runtime(&value.borrow()))
                .collect(),
        ),
        Value::Null => PropertyValue::Null,
    }
}

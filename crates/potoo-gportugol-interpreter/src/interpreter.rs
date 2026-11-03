use std::collections::HashMap;

use crate::{
    ControlAction, DataType, Diagnostic, DiagnosticKind, ExecutionEvent, ExecutionOptions,
    InputPrompt, PauseReason, RuntimeIo, SourceSpan, VariableSnapshot,
    ast::{
        BinaryOp, Block, Expr, ExprKind, FunctionDecl, LValue, Literal, Program, Statement,
        UnaryOp, VariableDecl,
    },
    lexer::{Token, lex},
    parser::{parse_tokens_with_options, parse_with_options},
    source::SourceLocation,
    type_checker::check,
    types::{is_numeric, promoted_numeric_type},
    value::Value,
};

pub fn execute(source: &str, io: &mut dyn RuntimeIo) -> Result<Option<i64>, Vec<Diagnostic>> {
    execute_with_options(source, io, ExecutionOptions::default())
}

/// Executes a main G-Portugol file followed by function-only source files.
///
/// GPT 1.4.0 composes these files as one program; the first file must contain
/// the algorithm and globals, and additional files may contribute functions.
pub fn execute_with_sources(
    main_source: &str,
    additional_sources: &[&str],
    io: &mut dyn RuntimeIo,
) -> Result<Option<i64>, Vec<Diagnostic>> {
    execute_with_sources_and_options(
        main_source,
        additional_sources,
        io,
        ExecutionOptions::default(),
    )
}

/// Executes composed G-Portugol source files with caller-configured limits.
pub fn execute_with_sources_and_options(
    main_source: &str,
    additional_sources: &[&str],
    io: &mut dyn RuntimeIo,
    options: ExecutionOptions,
) -> Result<Option<i64>, Vec<Diagnostic>> {
    let sources = std::iter::once(main_source)
        .chain(additional_sources.iter().copied())
        .collect::<Vec<_>>();
    let (tokens, ranges) = lex_sources(&sources)?;
    let program = parse_tokens_with_options(tokens, options).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| label_source_diagnostic(diagnostic, &ranges))
            .collect::<Vec<_>>()
    })?;
    execute_program(program, io, options).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| label_source_diagnostic(diagnostic, &ranges))
            .collect::<Vec<_>>()
    })
}

pub fn execute_with_options(
    source: &str,
    io: &mut dyn RuntimeIo,
    options: ExecutionOptions,
) -> Result<Option<i64>, Vec<Diagnostic>> {
    let program = parse_with_options(source, options)?;
    execute_program(program, io, options)
}

fn execute_program(
    program: Program,
    io: &mut dyn RuntimeIo,
    options: ExecutionOptions,
) -> Result<Option<i64>, Vec<Diagnostic>> {
    check(&program)?;
    let mut interpreter = Interpreter::new(io, options, &program.functions);
    match interpreter.run(&program.main, &program.globals) {
        Ok(exit_code) => Ok(exit_code),
        Err(_) if interpreter.stopped => Ok(None),
        Err(error) => Err(vec![error]),
    }
}

fn lex_sources(sources: &[&str]) -> Result<(Vec<Token>, Vec<SourceRange>), Vec<Diagnostic>> {
    let mut tokens = Vec::new();
    let mut ranges = Vec::with_capacity(sources.len());
    let mut base = 0_usize;
    for (index, source) in sources.iter().enumerate() {
        let range = SourceRange {
            start: base,
            end: base.saturating_add(source.len()),
            label: format!("source {}", index + 1),
        };
        ranges.push(range);
        let mut source_tokens = match lex(source) {
            Ok(tokens) => tokens,
            Err(diagnostics) => {
                return Err(diagnostics
                    .into_iter()
                    .map(|mut diagnostic| {
                        diagnostic.span.start.byte_offset =
                            diagnostic.span.start.byte_offset.saturating_add(base);
                        diagnostic.span.end.byte_offset =
                            diagnostic.span.end.byte_offset.saturating_add(base);
                        label_source_diagnostic(diagnostic, &ranges)
                    })
                    .collect());
            }
        };
        if index + 1 < sources.len() {
            source_tokens.pop();
        }
        for mut token in source_tokens {
            token.span.start.byte_offset = token.span.start.byte_offset.saturating_add(base);
            token.span.end.byte_offset = token.span.end.byte_offset.saturating_add(base);
            tokens.push(token);
        }
        base = ranges[index].end.saturating_add(1);
    }
    Ok((tokens, ranges))
}

fn label_source_diagnostic(mut diagnostic: Diagnostic, ranges: &[SourceRange]) -> Diagnostic {
    if let Some(range) = ranges
        .iter()
        .find(|range| (range.start..=range.end).contains(&diagnostic.span.start.byte_offset))
    {
        diagnostic.message = format!("{}: {}", range.label, diagnostic.message);
        diagnostic.span.start.byte_offset = diagnostic
            .span
            .start
            .byte_offset
            .saturating_sub(range.start);
        diagnostic.span.end.byte_offset =
            diagnostic.span.end.byte_offset.saturating_sub(range.start);
    }
    diagnostic
}

#[derive(Clone)]
struct Variable {
    data_type: DataType,
    dimensions: Vec<usize>,
    value: Value,
}

struct ResolvedLValue {
    name: String,
    data_type: DataType,
    offset: Option<usize>,
    span: SourceSpan,
}

struct SourceRange {
    start: usize,
    end: usize,
    label: String,
}

enum Flow {
    Continue,
    Return(Option<Value>),
}

struct Interpreter<'a> {
    io: &'a mut dyn RuntimeIo,
    options: ExecutionOptions,
    steps: u64,
    output_bytes: usize,
    variables: HashMap<String, Variable>,
    scopes: Vec<HashMap<String, Variable>>,
    functions: HashMap<String, FunctionDecl>,
    call_depth: usize,
    return_types: Vec<Option<DataType>>,
    return_value: Option<Value>,
    allocated_array_elements: usize,
    step_next: bool,
    stopped: bool,
}

impl<'a> Interpreter<'a> {
    fn new(
        io: &'a mut dyn RuntimeIo,
        options: ExecutionOptions,
        functions: &[FunctionDecl],
    ) -> Self {
        Self {
            io,
            options,
            steps: 0,
            output_bytes: 0,
            variables: HashMap::new(),
            scopes: Vec::new(),
            functions: functions
                .iter()
                .map(|function| (function.name.clone(), function.clone()))
                .collect(),
            call_depth: 0,
            return_types: Vec::new(),
            return_value: None,
            allocated_array_elements: 0,
            step_next: false,
            stopped: false,
        }
    }

    fn run(
        &mut self,
        main: &Block,
        declarations: &[VariableDecl],
    ) -> Result<Option<i64>, Diagnostic> {
        self.declare_globals(declarations)?;
        self.return_types.push(Some(DataType::Integer));
        match self.block(main)? {
            Flow::Continue | Flow::Return(None) => Ok(None),
            Flow::Return(Some(value)) => value.integer().map(Some).ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticKind::Runtime,
                    "the main block must return an integer value",
                    main.span,
                )
            }),
        }
    }

    fn declare_globals(&mut self, declarations: &[VariableDecl]) -> Result<(), Diagnostic> {
        for declaration in declarations {
            for name in &declaration.names {
                let value = self.zero_value(
                    declaration.data_type,
                    &declaration.dimensions,
                    declaration.span,
                )?;
                self.variables.insert(
                    name.name.clone(),
                    Variable {
                        data_type: declaration.data_type,
                        dimensions: declaration.dimensions.clone(),
                        value: value.clone(),
                    },
                );
            }
        }
        Ok(())
    }

    fn zero_value(
        &mut self,
        data_type: DataType,
        dimensions: &[usize],
        span: SourceSpan,
    ) -> Result<Value, Diagnostic> {
        if dimensions.is_empty() {
            return Ok(Value::zero(data_type));
        }
        let count = dimensions
            .iter()
            .try_fold(1_usize, |total, dimension| total.checked_mul(*dimension))
            .ok_or_else(|| self.runtime_error("matrix size is too large", span))?;
        let allocated = self
            .allocated_array_elements
            .checked_add(count)
            .ok_or_else(|| self.runtime_error("array element limit exceeded", span))?;
        if allocated > self.options.max_array_elements {
            return Err(self.runtime_error("array element limit exceeded", span));
        }
        self.allocated_array_elements = allocated;
        Ok(Value::Array {
            element: data_type,
            dimensions: dimensions.to_vec(),
            values: vec![Value::zero(data_type); count],
        })
    }

    fn block(&mut self, block: &Block) -> Result<Flow, Diagnostic> {
        for statement in &block.statements {
            match self.statement(statement)? {
                Flow::Continue => {}
                Flow::Return(value) => return Ok(Flow::Return(value)),
            }
        }
        Ok(Flow::Continue)
    }

    fn statement(&mut self, statement: &Statement) -> Result<Flow, Diagnostic> {
        if let Statement::Sequence { statements, span } = statement {
            if statements.is_empty() {
                // Empty loop bodies still need execution budget and cancellation checks.
                self.tick(*span)?;
                return Ok(Flow::Continue);
            }
            for statement in statements {
                if let flow @ Flow::Return(_) = self.statement(statement)? {
                    return Ok(flow);
                }
            }
            return Ok(Flow::Continue);
        }
        self.tick(statement.span())?;
        self.before_statement(statement)?;
        match statement {
            Statement::Sequence { .. } => unreachable!("sequence is evaluated above"),
            Statement::Assign { target, value, .. } => {
                let target = self.resolve_lvalue(target)?;
                let result = self.expression(value, Some(target.data_type))?;
                self.assign_resolved(&target, result.convert(target.data_type, value.span)?)?;
            }
            Statement::Call { call, .. } => {
                if let ExprKind::Call {
                    name,
                    name_span,
                    args,
                } = &call.kind
                {
                    self.invoke(name, *name_span, args, None, call.span)?;
                }
            }
            Statement::Return { value, span } => {
                let return_type = self.return_types.last().copied().flatten();
                let value = value
                    .as_ref()
                    .map(|value| {
                        let data_type = return_type.ok_or_else(|| {
                            self.runtime_error("this function does not return a value", *span)
                        })?;
                        let result = self.expression(value, Some(data_type))?;
                        result.convert(data_type, value.span)
                    })
                    .transpose()?;
                if let Some(value) = &value {
                    self.return_value = Some(value.clone());
                }
                return Ok(Flow::Return(value));
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let condition = self.expression(condition, Some(DataType::Logical))?;
                let branch = if condition.truthy() {
                    Some(then_branch.as_ref())
                } else {
                    else_branch.as_deref()
                };
                if let Some(branch) = branch {
                    return self.statement(branch);
                }
            }
            Statement::While {
                condition, body, ..
            } => loop {
                if !self
                    .expression(condition, Some(DataType::Logical))?
                    .truthy()
                {
                    break;
                }
                if let Flow::Return(value) = self.statement(body)? {
                    return Ok(Flow::Return(value));
                }
            },
            Statement::Repeat {
                body, condition, ..
            } => loop {
                if let Flow::Return(value) = self.statement(body)? {
                    return Ok(Flow::Return(value));
                }
                if self
                    .expression(condition, Some(DataType::Logical))?
                    .truthy()
                {
                    break;
                }
            },
            Statement::For {
                target,
                start,
                end,
                step,
                body,
                ..
            } => return self.for_loop(target, start, end, *step, body),
        }
        Ok(Flow::Continue)
    }

    fn for_loop(
        &mut self,
        target: &LValue,
        start: &Expr,
        end: &Expr,
        step: i64,
        body: &Statement,
    ) -> Result<Flow, Diagnostic> {
        let location = self.resolve_lvalue(target)?;
        let target_type = location.data_type;
        let start_value = self
            .expression(start, Some(target_type))?
            .convert(target_type, start.span)?;
        self.assign_resolved(&location, start_value)?;
        loop {
            let end_value = self
                .expression(end, Some(target_type))?
                .convert(target_type, end.span)?;
            let current = self.read_resolved(&location)?;
            let current_number = current.real().ok_or_else(|| {
                self.runtime_error("for-loop bounds must be numeric", target.span)
            })?;
            let end_number = end_value.real().ok_or_else(|| {
                self.runtime_error("for-loop bounds must be numeric", target.span)
            })?;
            if (step > 0 && current_number > end_number)
                || (step < 0 && current_number < end_number)
            {
                self.assign_resolved(&location, end_value)?;
                break;
            }
            if let Flow::Return(value) = self.statement(body)? {
                return Ok(Flow::Return(value));
            }
            let actual = self.read_resolved(&location)?;
            let next = self.add_step(actual, step, target_type, target.span)?;
            self.assign_resolved(&location, next)?;
        }
        Ok(Flow::Continue)
    }

    fn add_step(
        &self,
        value: Value,
        step: i64,
        data_type: DataType,
        span: SourceSpan,
    ) -> Result<Value, Diagnostic> {
        if data_type == DataType::Real {
            Ok(Value::Real(value.real().unwrap_or(0.0) + step as f64))
        } else {
            let next = i64::from(
                (value
                    .integer()
                    .ok_or_else(|| self.runtime_error("invalid counter", span))?
                    as i32)
                    .wrapping_add(step as i32),
            );
            Value::Integer(next).convert(data_type, span)
        }
    }

    fn print(&mut self, args: &[Expr], span: SourceSpan) -> Result<(), Diagnostic> {
        let mut output = String::new();
        for argument in args {
            let value = self.expression(argument, None)?;
            let text = value.output_text().ok_or_else(|| {
                self.runtime_error("cannot print an entire matrix", argument.span)
            })?;
            let pending_bytes = output
                .len()
                .checked_add(text.len())
                .and_then(|bytes| bytes.checked_add(1))
                .and_then(|bytes| self.output_bytes.checked_add(bytes))
                .ok_or_else(|| self.runtime_error("output limit exceeded", span))?;
            if pending_bytes > self.options.max_output_bytes {
                return Err(self.runtime_error("output limit exceeded", span));
            }
            output.push_str(&text);
        }
        output.push('\n');
        self.write(&output, span)
    }

    fn invoke(
        &mut self,
        name: &str,
        name_span: SourceSpan,
        args: &[Expr],
        expected: Option<DataType>,
        call_span: SourceSpan,
    ) -> Result<Option<Value>, Diagnostic> {
        match name {
            "imprima" => {
                self.print(args, call_span)?;
                Ok(None)
            }
            "leia" => self.read_input(args, name_span.start, expected).map(Some),
            "absoluto" | "arredonda" | "potencia" | "raiz_quadrada" => {
                self.standard_function(name, args, call_span).map(Some)
            }
            _ => self.call_function(name, name_span, args, call_span),
        }
    }

    fn standard_function(
        &mut self,
        name: &str,
        args: &[Expr],
        span: SourceSpan,
    ) -> Result<Value, Diagnostic> {
        let expected = if name == "potencia" { 2 } else { 1 };
        if args.len() != expected {
            return Err(self.runtime_error(
                format!(
                    "function '{name}' expects {expected} arguments, got {}",
                    args.len()
                ),
                span,
            ));
        }
        let mut values = args
            .iter()
            .map(|argument| {
                self.expression(argument, Some(DataType::Real))?
                    .convert(DataType::Real, argument.span)
            })
            .collect::<Result<Vec<_>, _>>()?;
        if name == "arredonda" {
            let value = self.return_value.clone().map_or_else(
                || Value::zero(DataType::Real),
                |value| value.retag(DataType::Real),
            );
            self.return_value = Some(value.clone());
            return Ok(value);
        }
        let first = values.remove(0).real().unwrap_or_default();
        let result = match name {
            "absoluto" if first == 0.0 => first,
            "absoluto" => first.abs(),
            "potencia" => {
                let exponent = values[0].real().unwrap_or_default();
                if exponent == 0.0 {
                    1.0
                } else if exponent == 1.0 {
                    first
                } else if exponent == -1.0 {
                    round_gpt_real(1.0 / first)
                } else {
                    let count = values[0].integer().unwrap_or_default() as i32;
                    let mut result = 1.0;
                    for _ in 0..count {
                        result = round_gpt_real(result * first);
                    }
                    result
                }
            }
            "raiz_quadrada" => {
                let n = first as i32;
                let mut x = 0.0;
                let mut xn = 0.0;
                let mut exact_root = None;
                for i in 0..=n {
                    let value = round_gpt_real(f64::from(i.wrapping_mul(i)) - first);
                    if value == 0.0 {
                        exact_root = Some(i);
                        break;
                    }
                    if value > 0.0 {
                        xn = round_gpt_real(f64::from(i.wrapping_add(i.wrapping_sub(1))) / 2.0);
                        break;
                    }
                }
                if let Some(exact_root) = exact_root {
                    f64::from(exact_root)
                } else {
                    let mut iterations = 0;
                    while iterations < 100 && x != xn {
                        iterations += 1;
                        x = xn;
                        let square = round_gpt_real(x * x);
                        let difference = round_gpt_real(square - f64::from(n));
                        let denominator = round_gpt_real(2.0 * x);
                        let quotient = round_gpt_real(difference / denominator);
                        xn = round_gpt_real(x - quotient);
                    }
                    xn
                }
            }
            _ => unreachable!(),
        };
        let value = Value::Real(result);
        self.return_value = Some(value.clone());
        Ok(value)
    }

    fn call_function(
        &mut self,
        name: &str,
        name_span: SourceSpan,
        args: &[Expr],
        call_span: SourceSpan,
    ) -> Result<Option<Value>, Diagnostic> {
        let function = self.functions.get(name).cloned().ok_or_else(|| {
            self.runtime_error(format!("function '{name}' is not declared"), name_span)
        })?;
        if self.call_depth >= self.options.max_call_depth {
            return Err(self.runtime_error("function call depth limit exceeded", call_span));
        }

        let parameters = function
            .parameters
            .iter()
            .flat_map(|declaration| {
                declaration
                    .names
                    .iter()
                    .map(move |name| (declaration, name))
            })
            .collect::<Vec<_>>();
        let mut values = Vec::with_capacity(args.len());
        for (argument, (parameter, _)) in args.iter().zip(&parameters) {
            let value = self.expression(argument, Some(parameter.data_type))?;
            let value = match value {
                value if parameter.dimensions.is_empty() => {
                    value.convert(parameter.data_type, argument.span)?
                }
                Value::Array {
                    dimensions, values, ..
                } => Value::Array {
                    element: parameter.data_type,
                    dimensions,
                    values,
                },
                value => value,
            };
            values.push(value);
        }

        self.call_depth += 1;
        let result = self.run_function(&function, &parameters, values);
        self.call_depth -= 1;
        result
    }

    fn run_function(
        &mut self,
        function: &FunctionDecl,
        parameters: &[(&VariableDecl, &crate::ast::Identifier)],
        values: Vec<Value>,
    ) -> Result<Option<Value>, Diagnostic> {
        let allocation_base = self.allocated_array_elements;
        let frame = (|| {
            let mut scope = HashMap::new();
            for ((declaration, name), value) in parameters.iter().zip(values) {
                self.reserve_value(&value, name.span)?;
                scope.insert(
                    name.name.clone(),
                    Variable {
                        data_type: declaration.data_type,
                        dimensions: declaration.dimensions.clone(),
                        value,
                    },
                );
            }
            for declaration in &function.locals {
                for name in &declaration.names {
                    let value = self.zero_value(
                        declaration.data_type,
                        &declaration.dimensions,
                        declaration.span,
                    )?;
                    scope.insert(
                        name.name.clone(),
                        Variable {
                            data_type: declaration.data_type,
                            dimensions: declaration.dimensions.clone(),
                            value: value.clone(),
                        },
                    );
                }
            }
            Ok::<_, Diagnostic>(scope)
        })();
        let scope = match frame {
            Ok(scope) => scope,
            Err(error) => {
                self.allocated_array_elements = allocation_base;
                return Err(error);
            }
        };

        self.scopes.push(scope);
        self.return_types.push(function.return_type);
        let execution = self.block(&function.body);
        self.return_types.pop();
        self.scopes.pop();
        self.allocated_array_elements = allocation_base;
        match execution? {
            Flow::Return(Some(value)) => function
                .return_type
                .map(|data_type| value.convert(data_type, function.span).map(Some))
                .unwrap_or_else(|| {
                    Err(self.runtime_error(
                        "function without a return type returned a value",
                        function.span,
                    ))
                }),
            Flow::Return(None) | Flow::Continue => {
                let Some(data_type) = function.return_type else {
                    return Ok(None);
                };
                let value = self
                    .return_value
                    .clone()
                    .map_or_else(|| Value::zero(data_type), |value| value.retag(data_type));
                self.return_value = Some(value.clone());
                Ok(Some(value))
            }
        }
    }

    fn reserve_value(&mut self, value: &Value, span: SourceSpan) -> Result<(), Diagnostic> {
        let count = match value {
            Value::Array { values, .. } => values.len(),
            _ => return Ok(()),
        };
        let allocated = self
            .allocated_array_elements
            .checked_add(count)
            .ok_or_else(|| self.runtime_error("array element limit exceeded", span))?;
        if allocated > self.options.max_array_elements {
            return Err(self.runtime_error("array element limit exceeded", span));
        }
        self.allocated_array_elements = allocated;
        Ok(())
    }

    fn expression(
        &mut self,
        expression: &Expr,
        expected: Option<DataType>,
    ) -> Result<Value, Diagnostic> {
        match &expression.kind {
            ExprKind::Literal(literal) => {
                let value = match literal {
                    Literal::Integer(value) => Value::Integer(*value),
                    Literal::Real(value) => Value::Real(*value),
                    Literal::Character(value) => Value::Character(*value),
                    Literal::String(value) => Value::Literal(value.clone()),
                    Literal::Logical(value) => Value::Logical(*value),
                };
                self.check_string(&value, expression.span)?;
                Ok(value)
            }
            ExprKind::Variable(lvalue) => self.read_lvalue(lvalue),
            ExprKind::Call {
                name,
                name_span,
                args,
            } => self
                .invoke(name, *name_span, args, expected, expression.span)?
                .ok_or_else(|| {
                    self.runtime_error(
                        format!("function '{name}' does not return a value"),
                        expression.span,
                    )
                }),
            ExprKind::Unary { op, value } => {
                let value = self.expression(value, None)?;
                self.unary(*op, value, expression.span)
            }
            ExprKind::Binary { left, op, right } => {
                let left_value = self.expression(left, None)?;
                let right_value = self.expression(right, None)?;
                self.binary(*op, left_value, right_value, expression.span)
            }
        }
    }

    fn read_input(
        &mut self,
        args: &[Expr],
        source: SourceLocation,
        expected: Option<DataType>,
    ) -> Result<Value, Diagnostic> {
        if !args.is_empty() {
            return Err(self.runtime_error(
                "leia does not accept arguments",
                SourceSpan {
                    start: source,
                    end: source,
                },
            ));
        }
        let data_type = expected.unwrap_or(DataType::Literal);
        let prompt = InputPrompt { data_type, source };
        let input = self.io.read_line(&prompt).map_err(|message| {
            self.runtime_error(
                format!("input failed: {message}"),
                SourceSpan {
                    start: source,
                    end: source,
                },
            )
        })?;
        let input = input.trim_end_matches(['\r', '\n']);
        let value = match data_type {
            DataType::Integer => Value::Integer(parse_integer_prefix(&input)),
            DataType::Real => Value::Real(parse_real_prefix(&input)),
            DataType::Character => {
                Value::CharacterByte(input.as_bytes().first().map_or(0, |byte| *byte as i8))
            }
            DataType::Literal => Value::Literal(input.to_owned()),
            DataType::Logical => Value::Logical(parse_logical(&input)),
        };
        self.check_string(
            &value,
            SourceSpan {
                start: source,
                end: source,
            },
        )?;
        Ok(value)
    }

    fn unary(
        &self,
        operator: UnaryOp,
        value: Value,
        span: SourceSpan,
    ) -> Result<Value, Diagnostic> {
        match operator {
            UnaryOp::Positive => match value {
                Value::Integer(value) => Ok(Value::Integer(i64::from(value as i32))),
                value => Ok(value),
            },
            UnaryOp::Negative => match value {
                Value::Integer(value) => {
                    Ok(Value::Integer(i64::from((value as i32).wrapping_neg())))
                }
                Value::Real(value) => Ok(Value::Real(-value)),
                Value::Character(value) => Ok(Value::CharacterByte((-(value as i64)) as i8)),
                Value::CharacterByte(value) => Ok(Value::CharacterByte(value.wrapping_neg())),
                Value::Logical(value) => Ok(Value::LogicalNumber(if value { -1 } else { 0 })),
                Value::LogicalNumber(value) => Ok(Value::LogicalNumber(value.wrapping_neg())),
                _ => Err(self.runtime_error("invalid unary minus operand", span)),
            },
            UnaryOp::Not => Ok(Value::Logical(match value {
                Value::Literal(value) => parse_real_prefix(&value) == 0.0,
                Value::NullLiteral => true,
                _ => !value.truthy(),
            })),
            UnaryOp::BitwiseNot => {
                let data_type = value.data_type();
                let value = value
                    .integer()
                    .ok_or_else(|| self.runtime_error("invalid bitwise not operand", span))?
                    as i32;
                Ok(match data_type {
                    Some(DataType::Character) => Value::CharacterByte((!value) as i8),
                    Some(DataType::Logical) => Value::LogicalNumber(!value),
                    _ => Value::Integer(i64::from(!value)),
                })
            }
        }
    }

    fn binary(
        &self,
        operator: BinaryOp,
        left: Value,
        right: Value,
        span: SourceSpan,
    ) -> Result<Value, Diagnostic> {
        let left_type = left
            .data_type()
            .ok_or_else(|| self.runtime_error("invalid matrix operand", span))?;
        let right_type = right
            .data_type()
            .ok_or_else(|| self.runtime_error("invalid matrix operand", span))?;
        let left_real = left.real();
        let right_real = right.real();
        let numeric = is_numeric(left_type) && is_numeric(right_type);
        let literal_pair = matches!(
            (&left, &right),
            (
                Value::Literal(_) | Value::NullLiteral,
                Value::Literal(_) | Value::NullLiteral
            )
        );
        match operator {
            BinaryOp::Or => Ok(Value::Logical(left.truthy() || right.truthy())),
            BinaryOp::And => Ok(Value::Logical(left.truthy() && right.truthy())),
            BinaryOp::BitwiseOr | BinaryOp::BitwiseXor | BinaryOp::BitwiseAnd => {
                let left = left.integer().ok_or_else(|| {
                    self.runtime_error("bitwise operators require integers", span)
                })?;
                let right = right.integer().ok_or_else(|| {
                    self.runtime_error("bitwise operators require integers", span)
                })?;
                Ok(Value::Integer(i64::from(match operator {
                    BinaryOp::BitwiseOr => (left as i32) | (right as i32),
                    BinaryOp::BitwiseXor => (left as i32) ^ (right as i32),
                    _ => (left as i32) & (right as i32),
                })))
            }
            BinaryOp::Equal | BinaryOp::NotEqual if numeric || literal_pair => {
                let equal = if literal_pair {
                    literal_equal(&left, &right)
                } else if left_type == DataType::Real || right_type == DataType::Real {
                    left_real == right_real
                } else {
                    left.integer() == right.integer()
                };
                Ok(Value::Logical(if operator == BinaryOp::Equal {
                    equal
                } else {
                    !equal
                }))
            }
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
                if numeric || literal_pair =>
            {
                let result = if literal_pair {
                    let ordering = literal_length(&left).cmp(&literal_length(&right));
                    match operator {
                        BinaryOp::Less => ordering.is_lt(),
                        BinaryOp::LessEqual => !ordering.is_gt(),
                        BinaryOp::Greater => ordering.is_gt(),
                        _ => !ordering.is_lt(),
                    }
                } else if left_type == DataType::Real || right_type == DataType::Real {
                    let (left, right) = (left_real.unwrap_or(0.0), right_real.unwrap_or(0.0));
                    match operator {
                        BinaryOp::Less => left < right,
                        BinaryOp::LessEqual => left <= right,
                        BinaryOp::Greater => left > right,
                        _ => left >= right,
                    }
                } else {
                    let ordering = left.integer().cmp(&right.integer());
                    match operator {
                        BinaryOp::Less => ordering.is_lt(),
                        BinaryOp::LessEqual => !ordering.is_gt(),
                        BinaryOp::Greater => ordering.is_gt(),
                        _ => !ordering.is_lt(),
                    }
                };
                Ok(Value::Logical(result))
            }
            BinaryOp::Remainder
                if numeric && left_type != DataType::Real && right_type != DataType::Real =>
            {
                let left = left
                    .integer()
                    .ok_or_else(|| self.runtime_error("invalid numeric operand", span))?
                    as i32;
                let right = right
                    .integer()
                    .ok_or_else(|| self.runtime_error("invalid numeric operand", span))?
                    as i32;
                if right == 0 {
                    return Err(self.runtime_error("division by zero", span));
                }
                Ok(Value::Integer(i64::from(left.wrapping_rem(right))))
            }
            BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide
                if numeric =>
            {
                let result_type = promoted_numeric_type(left_type, right_type);
                if result_type == DataType::Real {
                    let left = left_real.unwrap_or(0.0);
                    let right = right_real.unwrap_or(0.0);
                    if operator == BinaryOp::Divide && right == 0.0 {
                        return Err(self.runtime_error("division by zero", span));
                    }
                    Ok(Value::Real(round_gpt_real(match operator {
                        BinaryOp::Add => left + right,
                        BinaryOp::Subtract => left - right,
                        BinaryOp::Multiply => left * right,
                        BinaryOp::Divide => left / right,
                        _ => unreachable!(),
                    })))
                } else {
                    let left = left
                        .integer()
                        .ok_or_else(|| self.runtime_error("invalid numeric operand", span))?;
                    let right = right
                        .integer()
                        .ok_or_else(|| self.runtime_error("invalid numeric operand", span))?;
                    if operator == BinaryOp::Divide && right == 0 {
                        return Err(self.runtime_error("division by zero", span));
                    }
                    let (left, right) = (left as i32, right as i32);
                    let result = match operator {
                        BinaryOp::Add => left.wrapping_add(right),
                        BinaryOp::Subtract => left.wrapping_sub(right),
                        BinaryOp::Multiply => left.wrapping_mul(right),
                        BinaryOp::Divide => left.wrapping_div(right),
                        _ => unreachable!(),
                    };
                    Value::Integer(i64::from(result)).convert(result_type, span)
                }
            }
            _ => Err(self.runtime_error("invalid operands for operator", span)),
        }
    }

    fn resolve_lvalue(&mut self, target: &LValue) -> Result<ResolvedLValue, Diagnostic> {
        let Some(variable) = self.variable(&target.name) else {
            return Err(self.runtime_error(
                format!("variable '{}' is not declared", target.name),
                target.name_span,
            ));
        };
        let (data_type, dimensions) = (variable.data_type, variable.dimensions.clone());
        let offset = if dimensions.is_empty() {
            if !target.indexes.is_empty() {
                return Err(self.runtime_error("scalar variable cannot be indexed", target.span));
            }
            None
        } else {
            if dimensions.len() != target.indexes.len() {
                return Err(self.runtime_error(
                    "array assignment requires every dimension to be indexed",
                    target.span,
                ));
            }
            Some(self.array_offset(target, &dimensions)?)
        };
        Ok(ResolvedLValue {
            name: target.name.clone(),
            data_type,
            offset,
            span: target.span,
        })
    }

    fn read_lvalue(&mut self, lvalue: &LValue) -> Result<Value, Diagnostic> {
        let Some(dimensions) = self
            .variable(&lvalue.name)
            .map(|variable| variable.dimensions.clone())
        else {
            return Err(self.runtime_error(
                format!("variable '{}' is not declared", lvalue.name),
                lvalue.name_span,
            ));
        };
        if lvalue.indexes.is_empty() {
            return Ok(self
                .variable(&lvalue.name)
                .expect("variable was found above")
                .value
                .clone());
        }
        let prefix_offset = self.array_offset(lvalue, &dimensions)?;
        let variable = self
            .variable(&lvalue.name)
            .expect("variable was found above");
        let Value::Array {
            element,
            dimensions: _,
            values,
        } = &variable.value
        else {
            return Err(self.runtime_error("scalar variable cannot be indexed", lvalue.span));
        };
        if lvalue.indexes.len() == dimensions.len() {
            let value = values
                .get(prefix_offset)
                .cloned()
                .ok_or_else(|| self.runtime_error("matrix index is out of bounds", lvalue.span))?;
            return Ok(if value.data_type() == Some(*element) {
                value
            } else {
                Value::DeclaredType {
                    data_type: *element,
                    value: Box::new(value),
                }
            });
        }
        let remaining = &dimensions[lvalue.indexes.len()..];
        let count = array_size(remaining)
            .ok_or_else(|| self.runtime_error("matrix size is too large", lvalue.span))?;
        let start = prefix_offset
            .checked_mul(count)
            .ok_or_else(|| self.runtime_error("matrix size is too large", lvalue.span))?;
        let end = start
            .checked_add(count)
            .ok_or_else(|| self.runtime_error("matrix size is too large", lvalue.span))?;
        let slice = values
            .get(start..end)
            .ok_or_else(|| self.runtime_error("matrix index is out of bounds", lvalue.span))?;
        Ok(Value::Array {
            element: *element,
            dimensions: remaining.to_vec(),
            values: slice.to_vec(),
        })
    }

    fn array_offset(&mut self, lvalue: &LValue, dimensions: &[usize]) -> Result<usize, Diagnostic> {
        if lvalue.indexes.len() > dimensions.len() {
            return Err(self.runtime_error("too many indexes for matrix", lvalue.span));
        }
        let mut offset = 0_usize;
        for (dimension_index, (index, size)) in lvalue.indexes.iter().zip(dimensions).enumerate() {
            let value = self.expression(index, Some(DataType::Integer))?;
            let index_value = value
                .integer()
                .ok_or_else(|| self.runtime_error("matrix indexes must be integers", index.span))?;
            let index_value = usize::try_from(index_value)
                .ok()
                .filter(|index| index < size);
            let Some(index_value) = index_value else {
                return Err(self.runtime_error(
                    format!(
                        "matrix index is out of bounds for dimension {}",
                        dimension_index + 1
                    ),
                    index.span,
                ));
            };
            offset = offset
                .checked_mul(*size)
                .and_then(|value| value.checked_add(index_value))
                .ok_or_else(|| self.runtime_error("matrix index is too large", index.span))?;
        }
        Ok(offset)
    }

    fn read_resolved(&self, target: &ResolvedLValue) -> Result<Value, Diagnostic> {
        let variable = self.variable(&target.name).ok_or_else(|| {
            self.runtime_error(
                format!("variable '{}' is not declared", target.name),
                target.span,
            )
        })?;
        match target.offset {
            None => Ok(variable.value.clone()),
            Some(offset) => match &variable.value {
                Value::Array { values, .. } => values.get(offset).cloned().ok_or_else(|| {
                    self.runtime_error("matrix index is out of bounds", target.span)
                }),
                _ => Err(self.runtime_error("invalid matrix storage", target.span)),
            },
        }
    }

    fn assign_resolved(&mut self, target: &ResolvedLValue, value: Value) -> Result<(), Diagnostic> {
        let Some(variable) = self.variable_mut(&target.name) else {
            return Err(self.runtime_error(
                format!("variable '{}' is not declared", target.name),
                target.span,
            ));
        };
        match (&mut variable.value, target.offset) {
            (value_slot, None) => *value_slot = value,
            (Value::Array { values, .. }, Some(offset)) => {
                let slot = values.get_mut(offset).ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticKind::Runtime,
                        "matrix index is out of bounds",
                        target.span,
                    )
                })?;
                *slot = value;
            }
            (_, Some(_)) => {
                return Err(Diagnostic::new(
                    DiagnosticKind::Runtime,
                    "invalid matrix storage",
                    target.span,
                ));
            }
        }
        Ok(())
    }

    fn variable(&self, name: &str) -> Option<&Variable> {
        self.scopes
            .last()
            .and_then(|scope| scope.get(name))
            .or_else(|| self.variables.get(name))
    }

    fn variable_mut(&mut self, name: &str) -> Option<&mut Variable> {
        if self
            .scopes
            .last()
            .is_some_and(|scope| scope.contains_key(name))
        {
            let scope = self.scopes.last_mut().expect("scope was found above");
            scope.get_mut(name)
        } else {
            self.variables.get_mut(name)
        }
    }

    fn tick(&mut self, span: SourceSpan) -> Result<(), Diagnostic> {
        if self.io.should_stop() {
            return Err(self.runtime_error("execution cancelled", span));
        }
        self.steps = self.steps.saturating_add(1);
        if self.steps > self.options.max_steps {
            return Err(self.runtime_error("execution step limit exceeded", span));
        }
        Ok(())
    }

    fn before_statement(&mut self, statement: &Statement) -> Result<(), Diagnostic> {
        if !self.io.is_debugging() && !self.step_next {
            return Ok(());
        }
        let event = ExecutionEvent {
            location: statement.span().start,
            variables: self.visible_variables(),
            pause_reason: Some(if self.step_next {
                PauseReason::Step
            } else {
                PauseReason::Debugger
            }),
        };
        match self.io.on_execution_event(&event).map_err(|message| {
            self.runtime_error(
                format!("debugger control failed: {message}"),
                statement.span(),
            )
        })? {
            ControlAction::Continue => self.step_next = false,
            ControlAction::Step => self.step_next = true,
            ControlAction::Stop => {
                self.stopped = true;
                return Err(self.runtime_error("execution stopped by debugger", statement.span()));
            }
        }
        Ok(())
    }

    fn visible_variables(&self) -> Vec<VariableSnapshot> {
        let mut variables = std::collections::BTreeMap::new();
        for (name, variable) in &self.variables {
            variables.insert(name.clone(), snapshot_value(&variable.value));
        }
        if let Some(scope) = self.scopes.last() {
            for (name, variable) in scope {
                variables.insert(name.clone(), snapshot_value(&variable.value));
            }
        }
        variables
            .into_iter()
            .map(|(name, value)| VariableSnapshot { name, value })
            .collect()
    }

    fn write(&mut self, text: &str, span: SourceSpan) -> Result<(), Diagnostic> {
        let next_total = self
            .output_bytes
            .checked_add(text.len())
            .ok_or_else(|| self.runtime_error("output limit exceeded", span))?;
        if next_total > self.options.max_output_bytes {
            return Err(self.runtime_error("output limit exceeded", span));
        }
        self.io.write(text);
        self.output_bytes = next_total;
        Ok(())
    }

    fn check_string(&self, value: &Value, span: SourceSpan) -> Result<(), Diagnostic> {
        if matches!(value, Value::Literal(text) if text.len() > self.options.max_string_bytes) {
            return Err(self.runtime_error("string limit exceeded", span));
        }
        Ok(())
    }

    fn runtime_error(&self, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
        Diagnostic::new(DiagnosticKind::Runtime, message, span)
    }
}

fn parse_integer_prefix(input: &str) -> i64 {
    let token = numeric_prefix(input, false);
    token
        .parse::<i64>()
        .map(|value| i64::from(value as i32))
        .unwrap_or(0)
}

fn round_gpt_real(value: f64) -> f64 {
    if value == 0.0 || !value.is_finite() {
        return value;
    }

    // GPT 1.4.0's interpreted arithmetic retains six significant decimal digits.
    format!("{value:.5e}").parse().unwrap_or(value)
}

fn parse_real_prefix(input: &str) -> f64 {
    let token = numeric_prefix(input, true);
    token.parse().unwrap_or(0.0)
}

fn numeric_prefix(input: &str, decimal: bool) -> &str {
    let input = input.trim_start();
    let bytes = input.as_bytes();
    let mut cursor = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        cursor += 1;
    }
    let mut digits = 0;
    while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
        cursor += 1;
        digits += 1;
    }
    if decimal && bytes.get(cursor) == Some(&b'.') {
        cursor += 1;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return "";
    }
    let mut end = cursor;
    if decimal && matches!(bytes.get(cursor), Some(b'e' | b'E')) {
        let exponent_start = cursor;
        cursor += 1;
        if matches!(bytes.get(cursor), Some(b'+' | b'-')) {
            cursor += 1;
        }
        let exponent_digits = cursor;
        while bytes.get(cursor).is_some_and(u8::is_ascii_digit) {
            cursor += 1;
        }
        if cursor > exponent_digits {
            end = cursor;
        } else {
            end = exponent_start;
        }
    }
    &input[..end]
}

fn parse_logical(input: &str) -> bool {
    !matches!(input, "" | "falso" | "0")
}

fn literal_length(value: &Value) -> usize {
    match value {
        Value::Literal(text) => text.len(),
        _ => 0,
    }
}

fn literal_equal(left: &Value, right: &Value) -> bool {
    let left = match left {
        Value::Literal(value) => value.as_str(),
        Value::NullLiteral => "",
        _ => return false,
    };
    let right = match right {
        Value::Literal(value) => value.as_str(),
        Value::NullLiteral => "",
        _ => return false,
    };
    left == right
}

fn array_size(dimensions: &[usize]) -> Option<usize> {
    dimensions
        .iter()
        .try_fold(1_usize, |total, dimension| total.checked_mul(*dimension))
}

fn snapshot_value(value: &Value) -> String {
    match value {
        Value::Array { dimensions, .. } => format!(
            "[{}]",
            dimensions
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(" × ")
        ),
        _ => value.output_text().unwrap_or_default(),
    }
}

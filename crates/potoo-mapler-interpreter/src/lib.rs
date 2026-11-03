//! Rust interpreter for the documented Portugol Mapler dialect.

pub mod ast;
mod checker;
mod interpreter;
mod lexer;
mod parser;

use std::fmt;

/// Kind of problem reported while parsing, checking, or executing a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    Lexical,
    Syntax,
    Type,
    Runtime,
}

/// Source-located Mapler diagnostic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

impl Diagnostic {
    pub(crate) fn new(
        kind: DiagnosticKind,
        message: impl Into<String>,
        line: usize,
        column: usize,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            line,
            column,
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for Diagnostic {}

/// Type requested by a Mapler `ler` statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Integer,
    Real,
    Logical,
    Chain,
    Character,
}

/// Metadata for a variable read by the running program.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputPrompt {
    pub variable_name: String,
    pub data_type: DataType,
}

/// One-based position of a statement being executed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

/// A variable and its displayed value at an execution point.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableSnapshot {
    pub name: String,
    pub value: String,
}

/// Reason the interpreter paused for the host debugger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseReason {
    Debug,
    Step,
}

/// Current statement location and visible variables for the host debugger.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionEvent {
    pub location: SourceLocation,
    pub variables: Vec<VariableSnapshot>,
    pub pause_reason: Option<PauseReason>,
}

/// Host action after an execution event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    Continue,
    Step,
    Stop,
}

/// Host-provided input and output services for the interpreter.
pub trait RuntimeIo {
    fn write(&mut self, text: &str);
    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String>;

    /// Allows the host to interrupt execution, including a running loop.
    fn should_stop(&mut self) -> bool {
        false
    }

    /// Returns whether execution started in debugger mode.
    fn is_debugging(&self) -> bool {
        false
    }

    /// Reports the current statement and waits for a debugger action.
    fn on_execution_event(&mut self, _event: &ExecutionEvent) -> Result<ControlAction, String> {
        Ok(ControlAction::Continue)
    }
}

/// Limits resources used by parsing and execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionOptions {
    pub max_steps: u64,
    pub max_output_bytes: usize,
    pub max_string_bytes: usize,
    pub max_variable_slots: usize,
    pub max_call_depth: usize,
    pub max_parse_depth: usize,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_output_bytes: 1024 * 1024,
            max_string_bytes: 1024 * 1024,
            max_variable_slots: 500,
            max_call_depth: 32,
            max_parse_depth: 128,
        }
    }
}

/// Parses one complete Mapler source file without executing it.
pub fn parse(source: &str) -> Result<ast::Program, Vec<Diagnostic>> {
    parse_with_options(source, ExecutionOptions::default())
}

/// Parses one complete Mapler source file with an explicit nesting limit.
pub fn parse_with_options(
    source: &str,
    options: ExecutionOptions,
) -> Result<ast::Program, Vec<Diagnostic>> {
    parser::parse(source, options.max_parse_depth, options.max_string_bytes)
}

/// Checks declarations, variable use, and documented expression types.
pub fn check(program: &ast::Program) -> Result<(), Vec<Diagnostic>> {
    checker::check(program)
}

/// Parses, checks, and executes one complete Mapler source file.
pub fn execute(source: &str, io: &mut dyn RuntimeIo) -> Result<(), Vec<Diagnostic>> {
    execute_with_options(source, io, ExecutionOptions::default())
}

/// Parses, checks, and executes source with explicit resource limits.
pub fn execute_with_options(
    source: &str,
    io: &mut dyn RuntimeIo,
    options: ExecutionOptions,
) -> Result<(), Vec<Diagnostic>> {
    let program = parse_with_options(source, options)?;
    check(&program)?;
    let statements = program.statements.clone();
    let interpreter =
        interpreter::Interpreter::new(program, io, options).map_err(|error| vec![error])?;
    interpreter.execute(&statements)
}

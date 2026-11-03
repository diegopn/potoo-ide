#![doc = include_str!("../README.md")]

pub mod ast;
mod builtins;
mod environment;
mod format;
mod interpreter;
mod lexer;
mod parser;
mod value;

use std::fmt;
use std::time::Duration;

/// Kind of problem reported while reading, parsing, or executing a program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    Lexical,
    Syntax,
    Runtime,
}

/// Source-located diagnostic returned by the interpreter.
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

/// VisuAlg scalar type requested by a `leia` statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Integer,
    Real,
    Character,
    Logical,
}

/// Metadata for one interactive `leia` prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputPrompt {
    pub variable_name: String,
    pub data_type: DataType,
}

/// One-based source position reported by execution and debugger events.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

/// A visible variable and its current display value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableSnapshot {
    pub name: String,
    pub value: String,
}

/// Why execution stopped before a statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseReason {
    Step,
    Pause,
    Debug,
}

/// Current execution position and visible values supplied to the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionEvent {
    pub location: SourceLocation,
    pub variables: Vec<VariableSnapshot>,
    pub pause_reason: Option<PauseReason>,
}

/// Action selected by the host while execution is paused or being stepped.
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

    /// Clears the host output area when a host maps `limpatela` to that area.
    fn clear_output(&mut self) {}

    /// Clears the simulated console without requiring the host to erase output history.
    fn clear_screen(&mut self) {
        self.clear_output();
    }

    /// Allows a host to interrupt a CPU-bound loop while the program is running.
    fn should_stop(&mut self) -> bool {
        false
    }

    /// Reads an optional sequence of lines from a program data file.
    fn read_data_file(&mut self, _path: &str) -> Result<Option<Vec<String>>, String> {
        Err("o host não oferece suporte a arquivos de dados".to_owned())
    }

    /// Creates an empty data file when the program first uses it.
    fn create_data_file(&mut self, _path: &str) -> Result<(), String> {
        Err("o host não oferece suporte a arquivos de dados".to_owned())
    }

    /// Appends one input value to a data file being recorded.
    fn append_data_file_line(&mut self, _path: &str, _line: &str) -> Result<(), String> {
        Err("o host não oferece suporte a arquivos de dados".to_owned())
    }

    /// Waits for the requested duration. Hosts should make this stoppable.
    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        std::thread::sleep(duration);
        Ok(())
    }

    /// Returns whether the host started this run in debugger mode.
    fn is_debugging(&self) -> bool {
        false
    }

    /// Reports source position, values, and optional pause reason to the host.
    fn on_execution_event(&mut self, _event: &ExecutionEvent) -> Result<ControlAction, String> {
        Ok(ControlAction::Continue)
    }

    /// Notifies the host of the current source line during timed execution.
    fn on_line_changed(&mut self, _line: usize) {}
}

/// Limits the resources used by one program execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionOptions {
    pub max_steps: u64,
    pub max_output_bytes: usize,
    pub max_string_bytes: usize,
    pub max_variable_slots: usize,
    pub max_file_bytes: usize,
    pub max_wait_millis: u64,
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
            max_file_bytes: 1024 * 1024,
            max_wait_millis: 30_000,
            max_call_depth: 32,
            max_parse_depth: 128,
        }
    }
}

/// Parses one complete VisuAlg source file into its syntax tree without executing it.
pub fn parse(source: &str) -> Result<ast::Program, Vec<Diagnostic>> {
    parser::parse(source, ExecutionOptions::default().max_parse_depth)
}

/// Parses and executes one complete VisuAlg source file.
pub fn execute(source: &str, io: &mut dyn RuntimeIo) -> Result<(), Vec<Diagnostic>> {
    execute_with_options(source, io, ExecutionOptions::default())
}

/// Parses and executes a source file with explicit resource limits.
pub fn execute_with_options(
    source: &str,
    io: &mut dyn RuntimeIo,
    options: ExecutionOptions,
) -> Result<(), Vec<Diagnostic>> {
    let program = parser::parse(source, options.max_parse_depth)?;
    interpreter::Interpreter::new(program, io, options).execute()
}

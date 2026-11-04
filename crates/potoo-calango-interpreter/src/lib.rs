#![doc = include_str!("../README.md")]

pub mod ast;
mod interpreter;
mod lexer;
mod parser;
mod value;

pub use interpreter::{execute, execute_with_options};
pub use parser::{parse, parse_with_options};
pub use value::Value;

use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    Lexical,
    Syntax,
    Type,
    Runtime,
}

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

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for Diagnostic {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Integer,
    Real,
    Logical,
    Character,
    Text,
}

/// A value requested by a Calango input statement.
///
/// Character prompts come from `leiaCaracter`; hosts should return as soon as
/// one character is entered instead of waiting for a line terminator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputPrompt {
    pub variable_name: Option<String>,
    pub data_type: DataType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableSnapshot {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseReason {
    Debugger,
    Step,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionEvent {
    pub location: SourceLocation,
    pub variables: Vec<VariableSnapshot>,
    pub pause_reason: Option<PauseReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    Continue,
    Step,
    Stop,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionOptions {
    pub max_steps: u64,
    pub max_output_bytes: usize,
    pub max_string_bytes: usize,
    pub max_variable_slots: usize,
    pub max_array_length: usize,
    pub max_call_depth: usize,
    pub max_parse_depth: usize,
    pub max_input_wait_millis: u64,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_output_bytes: 1024 * 1024,
            max_string_bytes: 1024 * 1024,
            max_variable_slots: 500,
            max_array_length: 100_000,
            max_call_depth: 128,
            max_parse_depth: 128,
            max_input_wait_millis: 30_000,
        }
    }
}

pub trait RuntimeIo {
    fn write(&mut self, text: &str);
    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String>;

    fn read_line_with_timeout(
        &mut self,
        prompt: &InputPrompt,
        _timeout: Duration,
    ) -> Result<String, String> {
        self.read_line(prompt)
    }

    fn should_stop(&mut self) -> bool {
        false
    }

    fn clear_output(&mut self) {}

    fn clear_screen(&mut self) {}

    fn is_debugging(&self) -> bool {
        false
    }

    fn on_execution_event(&mut self, _event: &ExecutionEvent) -> Result<ControlAction, String> {
        Ok(ControlAction::Continue)
    }

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        std::thread::sleep(duration);
        Ok(())
    }
}

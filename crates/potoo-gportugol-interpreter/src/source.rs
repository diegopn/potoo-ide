use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Integer,
    Real,
    Character,
    Literal,
    Logical,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputPrompt {
    pub data_type: DataType,
    pub source: SourceLocation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseReason {
    /// The host is running an interactive debugger session.
    Debugger,
    /// The host requested one statement to execute before pausing again.
    Step,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    /// Resume execution and report the next statement when debugging remains active.
    Continue,
    /// Execute one statement and report the following statement.
    Step,
    /// End the current program execution without a runtime error.
    Stop,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

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
    pub span: SourceSpan,
}

impl Diagnostic {
    pub(crate) fn new(kind: DiagnosticKind, message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
        }
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}:{}: {}",
            self.span.start.line, self.span.start.column, self.message
        )
    }
}

impl std::error::Error for Diagnostic {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableSnapshot {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionEvent {
    /// Source location of the statement about to execute.
    pub location: SourceLocation,
    /// Values currently visible in global and local scopes.
    pub variables: Vec<VariableSnapshot>,
    /// Why execution is pausing before this statement.
    pub pause_reason: Option<PauseReason>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionOptions {
    pub max_steps: u64,
    pub max_output_bytes: usize,
    pub max_string_bytes: usize,
    pub max_array_elements: usize,
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
            max_array_elements: 100_000,
            max_call_depth: 32,
            max_parse_depth: 128,
            max_input_wait_millis: 30_000,
        }
    }
}

pub trait RuntimeIo {
    fn write(&mut self, text: &str);
    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String>;

    /// Reads input with a host-enforced deadline when the host can interrupt its input source.
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

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        std::thread::sleep(duration);
        Ok(())
    }

    /// Enables statement callbacks while the host is debugging this program.
    fn is_debugging(&self) -> bool {
        false
    }

    /// Lets the host continue, step, or stop before a statement executes.
    fn on_execution_event(&mut self, _event: &ExecutionEvent) -> Result<ControlAction, String> {
        Ok(ControlAction::Continue)
    }
}

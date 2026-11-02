use std::time::Duration;

/// Type requested by a Potigol input expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataType {
    Integer,
    Real,
    Text,
    Character,
    Logical,
}

/// Information the host can display when the program requests input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputPrompt {
    pub data_type: DataType,
    pub source: SourceLocation,
}

/// Why an execution event pauses for host control.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseReason {
    Debugger,
    Step,
}

/// Action the host selects while execution is paused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    Continue,
    Step,
    Stop,
}

/// One-based source location reported by the interpreter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
    pub byte_offset: usize,
}

/// Half-open source range. The end position is exclusive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceSpan {
    pub start: SourceLocation,
    pub end: SourceLocation,
}

/// Kind of lexical, syntax, type, or runtime diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    Lexical,
    Syntax,
    Type,
    Runtime,
}

/// A source-located issue reported by the Potigol interpreter.
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

/// A visible variable value sent to a debugger host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableSnapshot {
    pub name: String,
    pub value: String,
}

/// Current statement location and visible variables during execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionEvent {
    pub location: SourceLocation,
    pub variables: Vec<VariableSnapshot>,
    pub pause_reason: Option<PauseReason>,
}

/// Resource limits for one Potigol execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionOptions {
    pub max_steps: u64,
    pub max_output_bytes: usize,
    pub max_string_bytes: usize,
    pub max_collection_elements: usize,
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
            max_collection_elements: 100_000,
            max_wait_millis: 30_000,
            max_call_depth: 128,
            max_parse_depth: 128,
        }
    }
}

/// Input, output, cancellation, waiting, and debugger services provided by a host.
pub trait RuntimeIo {
    fn write(&mut self, text: &str);
    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String>;

    fn is_eof(&self) -> bool {
        false
    }

    fn should_stop(&mut self) -> bool {
        false
    }

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        std::thread::sleep(duration);
        Ok(())
    }

    fn is_debugging(&self) -> bool {
        false
    }

    fn on_execution_event(&mut self, _event: &ExecutionEvent) -> Result<ControlAction, String> {
        Ok(ControlAction::Continue)
    }
}

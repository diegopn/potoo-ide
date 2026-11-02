#![doc = include_str!("../README.md")]

mod lexer;
mod source;

pub use lexer::{Token, TokenKind, lex};
pub use source::{
    ControlAction, DataType, Diagnostic, DiagnosticKind, ExecutionEvent, ExecutionOptions,
    InputPrompt, PauseReason, RuntimeIo, SourceLocation, SourceSpan, VariableSnapshot,
};

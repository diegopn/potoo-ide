#![doc = include_str!("../README.md")]

pub mod ast;
mod interpreter;
mod lexer;
mod parser;
mod source;
mod type_checker;
pub mod types;
mod value;

pub use interpreter::{
    execute, execute_with_options, execute_with_sources, execute_with_sources_and_options,
};
pub use lexer::{Token, TokenKind, lex};
pub use parser::{parse, parse_with_options};
pub use source::{
    ControlAction, DataType, Diagnostic, DiagnosticKind, ExecutionEvent, ExecutionOptions,
    InputPrompt, PauseReason, RuntimeIo, SourceLocation, SourceSpan, VariableSnapshot,
};
pub use type_checker::check;
pub use types::Type;

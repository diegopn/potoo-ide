#![doc = include_str!("../README.md")]

pub mod ast;
mod lexer;
mod parser;
mod source;
mod type_checker;
pub mod types;

pub use lexer::{Token, TokenKind, lex};
pub use parser::{parse, parse_with_options};
pub use source::{
    ControlAction, DataType, Diagnostic, DiagnosticKind, ExecutionEvent, ExecutionOptions,
    InputPrompt, PauseReason, RuntimeIo, SourceLocation, SourceSpan, VariableSnapshot,
};
pub use type_checker::check;
pub use types::{ClassInfo, FieldInfo, FunctionSignature, TypeInfo, ValueType};

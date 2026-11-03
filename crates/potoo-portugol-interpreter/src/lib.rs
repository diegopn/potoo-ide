//! Reusable Portugol language engine for the Potoo IDE and other Rust apps.

pub mod ast;
mod graphics_constants;
mod interpreter;
mod keyboard;
mod lexer;
mod objects;
mod parser;
mod semantic;

use std::fmt;
use std::time::Duration;

/// Kind of problem reported while analyzing or executing a Portugol program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticKind {
    Lexical,
    Syntax,
    Type,
    Runtime,
}

/// Source location for an error reported by the engine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub line: usize,
    pub column: usize,
}

/// One-based location of a currently executing statement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

/// A visible variable value reported to a debugger host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariableSnapshot {
    pub name: String,
    pub value: String,
}

/// Values visible at a statement boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionPoint {
    pub location: SourceLocation,
    pub variables: Vec<VariableSnapshot>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlAction {
    Continue,
    Step,
    Stop,
}

/// Local date and time supplied by the runtime host for the `Calendario` library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalendarDateTime {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// Sunday is 1, matching the `Calendario` library in Portugol 2.0.
    pub weekday: u32,
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
    pub millisecond: u32,
}

/// An HTTP request delegated by a standard library function to the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub timeout_millis: u64,
}

/// The status, headers, and bounded response body returned by the host.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// A scalar or array value passed between a language library and its host.
#[derive(Clone, Debug, PartialEq)]
pub enum HostValue {
    Null,
    Integer(i64),
    Real(f64),
    Logical(bool),
    Character(char),
    String(String),
    Array(Vec<HostValue>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileAccessMode {
    Read,
    Write,
    Append,
}

impl Diagnostic {
    fn new(kind: DiagnosticKind, message: impl Into<String>, line: usize, column: usize) -> Self {
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

/// Host-provided input and output for an executing program.
pub trait RuntimeIo {
    fn write(&mut self, text: &str);
    fn read_line(&mut self) -> Result<String, String>;

    /// Clears the host output area when the program calls `limpa()`.
    fn clear_output(&mut self) {}

    /// Allows a host to interrupt a CPU-bound loop while the program is running.
    fn should_stop(&mut self) -> bool {
        false
    }

    /// Waits for a requested duration. Hosts can override this to make waits interruptible.
    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        std::thread::sleep(duration);
        Ok(())
    }

    /// Returns local wall-clock fields for the `Calendario` library.
    fn calendar_now(&mut self) -> Result<CalendarDateTime, String> {
        Err("o host não fornece data e hora locais".to_owned())
    }

    /// Opens a project file for the `Arquivos` library and returns its opaque handle.
    fn portugol_file_open(&mut self, _path: &str, _mode: FileAccessMode) -> Result<i64, String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    fn portugol_file_close(&mut self, _handle: i64) -> Result<(), String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    fn portugol_file_is_eof(&mut self, _handle: i64) -> Result<bool, String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    fn portugol_file_read_line(&mut self, _handle: i64) -> Result<String, String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    fn portugol_file_write_line(&mut self, _handle: i64, _line: &str) -> Result<(), String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    /// Replaces text using Portugol 2.0 regular-expression search and replacement syntax.
    fn portugol_file_replace_text(
        &mut self,
        _path: &str,
        _search: &str,
        _replacement: &str,
        _only_first: bool,
    ) -> Result<(), String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    /// Sends an HTTP request for the `Internet` and `ServicosWeb` libraries.
    fn portugol_http_request(&mut self, _request: HttpRequest) -> Result<HttpResponse, String> {
        Err("o host não oferece suporte às bibliotecas Internet e ServicosWeb".to_owned())
    }

    /// Writes downloaded data to a project-relative path for `Internet.baixar_imagem`.
    fn portugol_file_write_bytes(&mut self, _path: &str, _bytes: &[u8]) -> Result<(), String> {
        Err("o host não oferece suporte ao download de arquivos".to_owned())
    }

    /// Tests a Java-compatible key code while a Portugol graphics window is focused.
    fn portugol_key_pressed(&mut self, _key: i64) -> Result<bool, String> {
        Err("o host não oferece suporte à biblioteca Teclado".to_owned())
    }

    /// Returns whether any key is held while a Portugol graphics window is focused.
    fn portugol_any_key_pressed(&mut self) -> Result<bool, String> {
        Err("o host não oferece suporte à biblioteca Teclado".to_owned())
    }

    /// Waits for a key press in the Portugol graphics window and returns its Java-compatible code.
    fn portugol_read_key(&mut self) -> Result<i64, String> {
        Err("o host não oferece suporte à biblioteca Teclado".to_owned())
    }

    /// Tests a Portugol mouse button (`0` left, `1` right, `2` middle).
    fn portugol_mouse_button_pressed(&mut self, _button: i64) -> Result<bool, String> {
        Err("o host não oferece suporte à biblioteca Mouse".to_owned())
    }

    /// Returns whether any mouse button is held in the Portugol graphics window.
    fn portugol_any_mouse_button_pressed(&mut self) -> Result<bool, String> {
        Err("o host não oferece suporte à biblioteca Mouse".to_owned())
    }

    /// Waits for a mouse click in the Portugol graphics window and returns its button code.
    fn portugol_read_mouse_button(&mut self) -> Result<i64, String> {
        Err("o host não oferece suporte à biblioteca Mouse".to_owned())
    }

    /// Returns the pointer coordinates relative to the Portugol graphics window.
    fn portugol_mouse_position(&mut self) -> Result<(i64, i64), String> {
        Err("o host não oferece suporte à biblioteca Mouse".to_owned())
    }

    /// Hides or shows the pointer over the Portugol graphics window.
    fn portugol_mouse_set_cursor_visible(&mut self, _visible: bool) -> Result<(), String> {
        Err("o host não oferece suporte à biblioteca Mouse".to_owned())
    }

    /// Runs an operation from the `Graficos` library in the host's graphics window.
    fn portugol_graphics(
        &mut self,
        _function: &str,
        _arguments: &[HostValue],
    ) -> Result<HostValue, String> {
        Err("o host não oferece suporte à biblioteca Graficos".to_owned())
    }

    /// Runs an operation from the `Sons` library in the host's audio service.
    fn portugol_audio(
        &mut self,
        _function: &str,
        _arguments: &[HostValue],
    ) -> Result<HostValue, String> {
        Err("o host não oferece suporte à biblioteca Sons".to_owned())
    }

    fn portugol_file_exists(&mut self, _path: &str) -> Result<bool, String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    fn portugol_file_delete(&mut self, _path: &str) -> Result<(), String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    fn portugol_file_select(
        &mut self,
        _extensions: &[String],
        _allow_all_files: bool,
    ) -> Result<Option<String>, String> {
        Err("o host não oferece suporte à seleção de arquivos".to_owned())
    }

    fn portugol_list_directories(&mut self, _path: &str) -> Result<Vec<String>, String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    fn portugol_list_files(
        &mut self,
        _path: &str,
        _extensions: Option<&[String]>,
    ) -> Result<Vec<String>, String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    fn portugol_create_directory(&mut self, _path: &str) -> Result<(), String> {
        Err("o host não oferece suporte à biblioteca Arquivos".to_owned())
    }

    /// Enables statement callbacks when a host starts a debugging session.
    fn is_debugging(&self) -> bool {
        false
    }

    /// Lets a host continue, step, or stop at an executed statement boundary.
    fn on_statement(&mut self, _point: &ExecutionPoint) -> Result<ControlAction, String> {
        Ok(ControlAction::Continue)
    }
}

/// Limits the amount of work a single execution may perform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecutionOptions {
    /// Maximum number of executed statements and loop iterations.
    pub max_steps: u64,
    /// Maximum total output sent to the host, in bytes.
    pub max_output_bytes: usize,
    /// Maximum size of one string value, in bytes.
    pub max_string_bytes: usize,
    /// Maximum number of allocated array elements per program.
    pub max_array_elements: usize,
    /// Maximum total time spent in `Util.aguarde` during one run, in milliseconds.
    pub max_wait_millis: u64,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            max_steps: 1_000_000,
            max_output_bytes: 1024 * 1024,
            max_string_bytes: 1024 * 1024,
            max_array_elements: 100_000,
            max_wait_millis: 30_000,
        }
    }
}

/// Parses and executes one complete Portugol source file.
pub fn execute(source: &str, io: &mut dyn RuntimeIo) -> Result<(), Vec<Diagnostic>> {
    execute_with_options(source, io, ExecutionOptions::default())
}

/// Parses one complete Portugol source file into its syntax tree without executing it.
pub fn parse(source: &str) -> Result<ast::Program, Vec<Diagnostic>> {
    let tokens = lexer::scan(source)?;
    parser::Parser::new(tokens).parse()
}

/// Parses and executes a source file with explicit execution limits.
pub fn execute_with_options(
    source: &str,
    io: &mut dyn RuntimeIo,
    options: ExecutionOptions,
) -> Result<(), Vec<Diagnostic>> {
    let program = parse(source)?;
    semantic::analyze(&program)?;
    interpreter::Interpreter::new(program, io, options).execute()
}

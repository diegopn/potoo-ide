use std::sync::mpsc::{Receiver, Sender};

use crate::features::media::audio::{AudioCommand, AudioResponse};
use potoo_portugol_interpreter as portugol;

pub(crate) const MAX_FILE_BYTES: u64 = 1024 * 1024;
pub(crate) const STOP_SENTINEL: &str = "__potoo_execution_stopped__";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiagnosticKind {
    Lexical,
    Syntax,
    Type,
    Runtime,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExecutionDiagnostic {
    pub(crate) kind: DiagnosticKind,
    pub(crate) message: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InputPrompt {
    pub(crate) variable_name: Option<String>,
    pub(crate) data_type: Option<String>,
    pub(crate) immediate_character: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DebugPauseReason {
    Step,
    Pause,
    Debug,
    StaticErrorNotReached,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DebuggerSnapshot {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) variables: Vec<(String, String)>,
    pub(crate) pause_reason: Option<DebugPauseReason>,
}

#[derive(Debug)]
pub(crate) enum HostRequest {
    KeyboardKeyPressed(i64),
    KeyboardAnyKeyPressed,
    KeyboardReadKey,
    MouseButtonPressed(i64),
    MouseAnyButtonPressed,
    MouseReadButton,
    MousePosition,
    MouseSetCursorVisible(bool),
    GraphicsWindowIsOpen,
    JerimumPreviewStart,
    Graphics {
        function: String,
        arguments: Vec<portugol::HostValue>,
    },
}

#[derive(Debug)]
pub(crate) enum ExecutionEvent {
    Output(String),
    ClearOutput,
    InputRequested(InputPrompt),
    FileSelectionRequested {
        extensions: Vec<String>,
        allow_all_files: bool,
    },
    HostRequest {
        request: HostRequest,
        response: Sender<Result<portugol::HostValue, String>>,
    },
    AudioRequest {
        command: AudioCommand,
        response: Sender<Result<AudioResponse, String>>,
    },
    Debugger(DebuggerSnapshot),
    LineChanged(usize),
    Finished {
        result: Result<(), Vec<ExecutionDiagnostic>>,
        stopped: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ControlAction {
    Continue,
    Step,
    Stop,
}

pub(crate) struct RunningProgram {
    pub(crate) events: Receiver<ExecutionEvent>,
    pub(crate) input: Sender<String>,
    pub(crate) control: Sender<ControlAction>,
    pub(crate) cancellable: gtk::gio::Cancellable,
}

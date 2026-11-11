mod io;
mod protocol;
mod worker;

#[cfg(test)]
mod tests;

#[cfg(test)]
use potoo_calango_interpreter as calango;
#[cfg(test)]
use potoo_portugol_interpreter as portugol;
#[cfg(test)]
use potoo_potigol_interpreter as potigol;
#[cfg(test)]
use potoo_visualg_interpreter as visualg;

pub(crate) use io::WorkerIo;
pub(crate) use protocol::{
    ControlAction, DebugPauseReason, DebuggerSnapshot, DiagnosticKind, ExecutionDiagnostic,
    ExecutionEvent, HostRequest, InputPrompt, MAX_FILE_BYTES, RunningProgram, STOP_SENTINEL,
};
pub(crate) use worker::start;

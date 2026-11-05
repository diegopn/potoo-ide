use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use crate::languages::model::Language;

use super::{
    ControlAction, DebugPauseReason, DiagnosticKind, ExecutionDiagnostic, ExecutionEvent,
    RunningProgram, STOP_SENTINEL, WorkerIo,
};

const MAX_EVENTS_PER_UI_TICK: usize = 128;

pub(crate) fn start(
    source: String,
    language: Language,
    document_path: Option<PathBuf>,
    project_root: Option<PathBuf>,
    debugging: bool,
) -> RunningProgram {
    let (event_sender, events) = mpsc::sync_channel(MAX_EVENTS_PER_UI_TICK * 2);
    let (input, input_receiver) = mpsc::channel();
    let (control, control_receiver) = mpsc::channel();
    let cancellable = gtk::gio::Cancellable::new();
    let worker_cancellable = cancellable.clone();
    let root = project_root.or_else(|| {
        document_path
            .as_deref()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
    });

    std::thread::spawn(move || {
        let mut io = WorkerIo {
            events: event_sender.clone(),
            input: input_receiver,
            control: control_receiver,
            pending_action: None,
            debugging,
            project_root: root,
            cancellable: worker_cancellable,
            output_bytes: 0,
            output_pending: String::new(),
            output_truncated: false,
            stopped: false,
            last_visualg_line: None,
            open_portugol_files: HashMap::new(),
        };
        let result = execute_source(language, &source, &mut io);
        let result = match result {
            Err(diagnostics)
                if debugging
                    && diagnostics
                        .iter()
                        .any(|diagnostic| diagnostic.kind != DiagnosticKind::Runtime) =>
            {
                let location = diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.kind != DiagnosticKind::Runtime)
                    .map_or((1, 1), |diagnostic| {
                        (diagnostic.line.max(1), diagnostic.column.max(1))
                    });
                let prefix = crate::features::debugging::executable_prefix(
                    language, &source, location.0, location.1,
                );
                let prefix_has_static_errors = crate::features::diagnostics::analyze(
                    0,
                    "<debugger-prefix>",
                    None,
                    language,
                    &prefix,
                )
                .iter()
                .any(|diagnostic| {
                    diagnostic.severity == crate::features::diagnostics::DiagnosticSeverity::Error
                        && diagnostic.category
                            != crate::features::diagnostics::DiagnosticCategory::Runtime
                });
                if prefix.is_empty() || prefix_has_static_errors {
                    Err(diagnostics)
                } else {
                    match execute_source(language, &prefix, &mut io) {
                        Ok(()) => {
                            let pause = WorkerIo::execution_snapshot(
                                location.0,
                                location.1,
                                [],
                                Some(DebugPauseReason::Debug),
                            );
                            match io.await_action(pause) {
                                Ok(ControlAction::Stop) => Ok(()),
                                Ok(_) => Err(diagnostics),
                                Err(message) => Err(vec![ExecutionDiagnostic {
                                    kind: DiagnosticKind::Runtime,
                                    message,
                                    line: location.0,
                                    column: location.1,
                                }]),
                            }
                        }
                        Err(prefix_diagnostics)
                            if prefix_diagnostics.iter().any(|diagnostic| {
                                diagnostic.kind == DiagnosticKind::Runtime
                                    && diagnostic.line < location.0
                            }) =>
                        {
                            Err(prefix_diagnostics)
                        }
                        Err(_) => Err(diagnostics),
                    }
                }
            }
            result => result,
        };
        io.flush_output();
        let stopped = io.stopped
            || result.as_ref().is_err_and(|diagnostics| {
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message == STOP_SENTINEL)
            });
        let result = if stopped { Ok(()) } else { result };
        let _ = event_sender.send(ExecutionEvent::Finished { result, stopped });
    });

    RunningProgram {
        events,
        input,
        control,
        cancellable,
    }
}

fn execute_source(
    language: Language,
    source: &str,
    io: &mut WorkerIo,
) -> Result<(), Vec<ExecutionDiagnostic>> {
    (language.profile().execution)(source, io)
}

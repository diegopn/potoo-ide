use std::fs::OpenOptions;
use std::io::Write;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use crate::features::execution::{
    ControlAction, DebugPauseReason, DiagnosticKind, ExecutionDiagnostic, ExecutionEvent,
    InputPrompt, MAX_FILE_BYTES, STOP_SENTINEL, WorkerIo,
};
use potoo_visualg_interpreter as visualg;

impl visualg::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &visualg::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(InputPrompt {
            variable_name: Some(prompt.variable_name.clone()),
            data_type: Some(
                match prompt.data_type {
                    visualg::DataType::Integer => "inteiro",
                    visualg::DataType::Real => "real",
                    visualg::DataType::Character => "caractere",
                    visualg::DataType::Logical => "logico",
                }
                .to_owned(),
            ),
            immediate_character: false,
        })
    }

    fn clear_output(&mut self) {
        self.output_pending.clear();
        let _ = self.events.send(ExecutionEvent::ClearOutput);
    }

    fn clear_screen(&mut self) {}

    fn on_line_changed(&mut self, line: usize) {
        if self.last_visualg_line != Some(line) {
            self.last_visualg_line = Some(line);
            let _ = self.events.send(ExecutionEvent::LineChanged(line));
        }
    }

    fn should_stop(&mut self) -> bool {
        if self.poll_stop() {
            self.stopped = true;
            true
        } else {
            false
        }
    }

    fn read_data_file(&mut self, path: &str) -> Result<Option<Vec<String>>, String> {
        WorkerIo::read_data_file(self, path)
    }

    fn create_data_file(&mut self, path: &str) -> Result<(), String> {
        let path = self.safe_data_path(path)?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map(|_| ())
            .or_else(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(format!("não foi possível criar o arquivo: {error}"))
                }
            })
    }

    fn append_data_file_line(&mut self, path: &str, line: &str) -> Result<(), String> {
        let path = self.safe_data_path(path)?;
        let mut file = OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(|error| format!("não foi possível gravar o arquivo: {error}"))?;
        let current_len = file
            .metadata()
            .map_err(|error| format!("não foi possível verificar o arquivo: {error}"))?
            .len();
        let appended_len = u64::try_from(line.len())
            .ok()
            .and_then(|length| length.checked_add(1))
            .ok_or_else(|| "arquivo de dados excede o limite de 1 MiB".to_owned())?;
        if current_len
            .checked_add(appended_len)
            .is_none_or(|length| length > MAX_FILE_BYTES)
        {
            return Err("arquivo de dados excede o limite de 1 MiB".to_owned());
        }
        writeln!(file, "{line}")
            .map_err(|error| format!("não foi possível gravar o arquivo: {error}"))
    }

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        let started = std::time::Instant::now();
        loop {
            let remaining = duration.saturating_sub(started.elapsed());
            if remaining.is_zero() {
                return Ok(());
            }
            match self
                .control
                .recv_timeout(remaining.min(Duration::from_millis(40)))
            {
                Ok(ControlAction::Stop) => {
                    self.stopped = true;
                    return Err(STOP_SENTINEL.to_owned());
                }
                Ok(action) => {
                    self.pending_action = Some(action);
                    return Ok(());
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.stopped = true;
                    return Err(STOP_SENTINEL.to_owned());
                }
            }
        }
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(
        &mut self,
        event: &visualg::ExecutionEvent,
    ) -> Result<visualg::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            visualg::PauseReason::Step => DebugPauseReason::Step,
            visualg::PauseReason::Pause => DebugPauseReason::Pause,
            visualg::PauseReason::Debug => DebugPauseReason::Debug,
        });
        let action = self.await_action(Self::execution_snapshot(
            event.location.line,
            event.location.column,
            event
                .variables
                .iter()
                .map(|variable| (variable.name.clone(), variable.value.clone())),
            reason,
        ))?;
        if action == ControlAction::Continue {
            self.debugging = false;
        }
        Ok(match action {
            ControlAction::Continue => visualg::ControlAction::Continue,
            ControlAction::Step => visualg::ControlAction::Step,
            ControlAction::Stop => visualg::ControlAction::Stop,
        })
    }
}

fn convert_visualg_diagnostics(diagnostics: Vec<visualg::Diagnostic>) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                visualg::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                visualg::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                visualg::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
        })
        .collect()
}

pub(crate) fn execute(source: &str, io: &mut WorkerIo) -> Result<(), Vec<ExecutionDiagnostic>> {
    visualg::execute(source, io).map_err(convert_visualg_diagnostics)
}

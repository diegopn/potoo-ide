use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;

use crate::features::execution::{
    ControlAction, DebugPauseReason, DiagnosticKind, ExecutionDiagnostic, HostRequest, InputPrompt,
    STOP_SENTINEL, WorkerIo,
};
use potoo_portugol_interpreter as portugol;
use potoo_potigol_interpreter as potigol;

impl potigol::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &potigol::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(InputPrompt {
            variable_name: None,
            data_type: Some(
                match prompt.data_type {
                    potigol::DataType::Integer => "inteiro",
                    potigol::DataType::Real => "real",
                    potigol::DataType::Text => "texto",
                    potigol::DataType::Character => "caractere",
                    potigol::DataType::Logical => "lógico",
                }
                .to_owned(),
            ),
            immediate_character: false,
        })
    }

    fn should_stop(&mut self) -> bool {
        if self.poll_stop() {
            self.stopped = true;
            true
        } else {
            false
        }
    }

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        let started = std::time::Instant::now();
        loop {
            if self.should_stop() {
                return Err(STOP_SENTINEL.to_owned());
            }
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
                Ok(action) => self.pending_action = Some(action),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.stopped = true;
                    return Err(STOP_SENTINEL.to_owned());
                }
            }
        }
    }

    fn jerimum_request(
        &mut self,
        request: potigol::JerimumRequest,
    ) -> Result<potigol::JerimumValue, String> {
        fn to_host(value: potigol::JerimumValue) -> portugol::HostValue {
            match value {
                potigol::JerimumValue::Null => portugol::HostValue::Null,
                potigol::JerimumValue::Integer(value) => portugol::HostValue::Integer(value),
                potigol::JerimumValue::Real(value) => portugol::HostValue::Real(value),
                potigol::JerimumValue::Logical(value) => portugol::HostValue::Logical(value),
                potigol::JerimumValue::String(value) => portugol::HostValue::String(value),
                potigol::JerimumValue::Array(values) => {
                    portugol::HostValue::Array(values.into_iter().map(to_host).collect())
                }
            }
        }
        fn from_host(value: portugol::HostValue) -> potigol::JerimumValue {
            match value {
                portugol::HostValue::Null => potigol::JerimumValue::Null,
                portugol::HostValue::Integer(value) => potigol::JerimumValue::Integer(value),
                portugol::HostValue::Real(value) => potigol::JerimumValue::Real(value),
                portugol::HostValue::Logical(value) => potigol::JerimumValue::Logical(value),
                portugol::HostValue::Character(value) => {
                    potigol::JerimumValue::String(value.to_string())
                }
                portugol::HostValue::String(value) => potigol::JerimumValue::String(value),
                portugol::HostValue::Array(values) => {
                    potigol::JerimumValue::Array(values.into_iter().map(from_host).collect())
                }
            }
        }

        match request {
            potigol::JerimumRequest::Graphics {
                function,
                arguments,
            } => {
                let request = if function == "iniciar_modo_grafico" {
                    HostRequest::JerimumPreviewStart
                } else {
                    HostRequest::Graphics {
                        function,
                        arguments: arguments.into_iter().map(to_host).collect(),
                    }
                };
                self.request_host(request).map(from_host)
            }
            potigol::JerimumRequest::Audio {
                function,
                arguments,
            } => {
                let command =
                    crate::languages::potigol::jerimum::audio::command(&function, &arguments)?;
                self.request_audio_host(command)
                    .map(crate::languages::potigol::jerimum::audio::response)
            }
            potigol::JerimumRequest::WindowIsOpen => match self
                .request_host(HostRequest::GraphicsWindowIsOpen)?
            {
                portugol::HostValue::Logical(value) => Ok(potigol::JerimumValue::Logical(value)),
                _ => Err("resposta inválida do host para o estado da janela".to_owned()),
            },
            potigol::JerimumRequest::KeyPressed { key } => {
                match self.request_host(HostRequest::KeyboardKeyPressed(key))? {
                    portugol::HostValue::Logical(value) => {
                        Ok(potigol::JerimumValue::Logical(value))
                    }
                    _ => Err("resposta inválida do host para o teclado".to_owned()),
                }
            }
            potigol::JerimumRequest::MouseButtonPressed { button } => {
                match self.request_host(HostRequest::MouseButtonPressed(button))? {
                    portugol::HostValue::Logical(value) => {
                        Ok(potigol::JerimumValue::Logical(value))
                    }
                    _ => Err("resposta inválida do host para o mouse".to_owned()),
                }
            }
            potigol::JerimumRequest::MousePosition => {
                self.request_host(HostRequest::MousePosition).map(from_host)
            }
            potigol::JerimumRequest::ReadFileLines { path } => {
                let lines = self
                    .read_data_file(&path)?
                    .ok_or_else(|| "arquivo de dados indisponível".to_owned())?;
                Ok(potigol::JerimumValue::Array(
                    lines
                        .into_iter()
                        .map(potigol::JerimumValue::String)
                        .collect(),
                ))
            }
        }
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(
        &mut self,
        event: &potigol::ExecutionEvent,
    ) -> Result<potigol::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            potigol::PauseReason::Debugger => DebugPauseReason::Debug,
            potigol::PauseReason::Step => DebugPauseReason::Step,
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
        Ok(match action {
            ControlAction::Continue => potigol::ControlAction::Continue,
            ControlAction::Step => potigol::ControlAction::Step,
            ControlAction::Stop => potigol::ControlAction::Stop,
        })
    }
}

fn convert_potigol_diagnostics(diagnostics: Vec<potigol::Diagnostic>) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                potigol::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                potigol::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                potigol::DiagnosticKind::Type => DiagnosticKind::Type,
                potigol::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.span.start.line,
            column: diagnostic.span.start.column,
        })
        .collect()
}

pub(crate) fn execute(source: &str, io: &mut WorkerIo) -> Result<(), Vec<ExecutionDiagnostic>> {
    potigol::execute(source, io).map_err(convert_potigol_diagnostics)
}

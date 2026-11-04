use crate::features::execution::{
    ControlAction, DebugPauseReason, DiagnosticKind, ExecutionDiagnostic, InputPrompt, WorkerIo,
};
use potoo_mapler_interpreter as mapler;

impl mapler::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &mapler::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(InputPrompt {
            variable_name: Some(prompt.variable_name.clone()),
            data_type: Some(
                match prompt.data_type {
                    mapler::DataType::Integer => "inteiro",
                    mapler::DataType::Real => "real",
                    mapler::DataType::Logical => "lógico",
                    mapler::DataType::Chain => "cadeia",
                    mapler::DataType::Character => "caractere",
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

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(
        &mut self,
        event: &mapler::ExecutionEvent,
    ) -> Result<mapler::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            mapler::PauseReason::Debug => DebugPauseReason::Debug,
            mapler::PauseReason::Step => DebugPauseReason::Step,
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
            ControlAction::Continue => mapler::ControlAction::Continue,
            ControlAction::Step => mapler::ControlAction::Step,
            ControlAction::Stop => mapler::ControlAction::Stop,
        })
    }
}

fn convert_mapler_diagnostics(diagnostics: Vec<mapler::Diagnostic>) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                mapler::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                mapler::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                mapler::DiagnosticKind::Type => DiagnosticKind::Type,
                mapler::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
        })
        .collect()
}

pub(crate) fn execute(source: &str, io: &mut WorkerIo) -> Result<(), Vec<ExecutionDiagnostic>> {
    mapler::execute(source, io).map_err(convert_mapler_diagnostics)
}

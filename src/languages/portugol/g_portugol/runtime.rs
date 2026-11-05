use std::time::Duration;

use crate::features::execution::{
    ControlAction, DebugPauseReason, DiagnosticKind, ExecutionDiagnostic, InputPrompt, WorkerIo,
};
use potoo_gportugol_interpreter as gportugol;

impl gportugol::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &gportugol::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(gportugol_input_prompt(prompt))
    }

    fn read_line_with_timeout(
        &mut self,
        prompt: &gportugol::InputPrompt,
        timeout: Duration,
    ) -> Result<String, String> {
        self.send_input_prompt_with_timeout(gportugol_input_prompt(prompt), Some(timeout))
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
        event: &gportugol::ExecutionEvent,
    ) -> Result<gportugol::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            gportugol::PauseReason::Debugger => DebugPauseReason::Debug,
            gportugol::PauseReason::Step => DebugPauseReason::Step,
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
            ControlAction::Continue => gportugol::ControlAction::Continue,
            ControlAction::Step => gportugol::ControlAction::Step,
            ControlAction::Stop => gportugol::ControlAction::Stop,
        })
    }
}

fn gportugol_input_prompt(prompt: &gportugol::InputPrompt) -> InputPrompt {
    InputPrompt {
        variable_name: None,
        data_type: Some(
            match prompt.data_type {
                gportugol::DataType::Integer => "inteiro",
                gportugol::DataType::Real => "real",
                gportugol::DataType::Character => "caractere",
                gportugol::DataType::Literal => "literal",
                gportugol::DataType::Logical => "lógico",
            }
            .to_owned(),
        ),
        immediate_character: false,
    }
}

fn convert_gportugol_diagnostics(
    diagnostics: Vec<gportugol::Diagnostic>,
) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                gportugol::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                gportugol::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                gportugol::DiagnosticKind::Type => DiagnosticKind::Type,
                gportugol::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.span.start.line,
            column: diagnostic.span.start.column,
        })
        .collect()
}

pub(crate) fn execute(source: &str, io: &mut WorkerIo) -> Result<(), Vec<ExecutionDiagnostic>> {
    gportugol::execute(source, io)
        .map(|_| ())
        .map_err(convert_gportugol_diagnostics)
}

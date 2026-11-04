use std::time::Duration;

use crate::features::execution::{
    ControlAction, DebugPauseReason, DiagnosticKind, ExecutionDiagnostic, ExecutionEvent,
    InputPrompt, WorkerIo,
};
use potoo_calango_interpreter as calango;

impl calango::RuntimeIo for WorkerIo {
    fn write(&mut self, text: &str) {
        self.write_limited(text);
    }

    fn read_line(&mut self, prompt: &calango::InputPrompt) -> Result<String, String> {
        self.send_input_prompt(calango_input_prompt(prompt))
    }

    fn read_line_with_timeout(
        &mut self,
        prompt: &calango::InputPrompt,
        timeout: Duration,
    ) -> Result<String, String> {
        self.send_input_prompt_with_timeout(calango_input_prompt(prompt), Some(timeout))
    }

    fn should_stop(&mut self) -> bool {
        if self.poll_stop() {
            self.stopped = true;
            true
        } else {
            false
        }
    }

    fn clear_output(&mut self) {
        self.output_pending.clear();
        let _ = self.events.send(ExecutionEvent::ClearOutput);
    }

    fn clear_screen(&mut self) {
        self.clear_output();
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(
        &mut self,
        event: &calango::ExecutionEvent,
    ) -> Result<calango::ControlAction, String> {
        let reason = event.pause_reason.map(|reason| match reason {
            calango::PauseReason::Debugger => DebugPauseReason::Debug,
            calango::PauseReason::Step => DebugPauseReason::Step,
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
            ControlAction::Continue => calango::ControlAction::Continue,
            ControlAction::Step => calango::ControlAction::Step,
            ControlAction::Stop => calango::ControlAction::Stop,
        })
    }
}

pub(crate) fn calango_input_prompt(prompt: &calango::InputPrompt) -> InputPrompt {
    InputPrompt {
        variable_name: prompt.variable_name.clone(),
        data_type: Some(
            match prompt.data_type {
                calango::DataType::Integer => "inteiro",
                calango::DataType::Real => "real",
                calango::DataType::Logical => "lógico",
                calango::DataType::Character => "caractere",
                calango::DataType::Text => "texto",
            }
            .to_owned(),
        ),
        immediate_character: prompt.data_type == calango::DataType::Character,
    }
}

fn convert_calango_diagnostics(diagnostics: Vec<calango::Diagnostic>) -> Vec<ExecutionDiagnostic> {
    diagnostics
        .into_iter()
        .map(|diagnostic| ExecutionDiagnostic {
            kind: match diagnostic.kind {
                calango::DiagnosticKind::Lexical => DiagnosticKind::Lexical,
                calango::DiagnosticKind::Syntax => DiagnosticKind::Syntax,
                calango::DiagnosticKind::Type => DiagnosticKind::Type,
                calango::DiagnosticKind::Runtime => DiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
        })
        .collect()
}

pub(crate) fn execute(source: &str, io: &mut WorkerIo) -> Result<(), Vec<ExecutionDiagnostic>> {
    calango::execute(source, io).map_err(convert_calango_diagnostics)
}

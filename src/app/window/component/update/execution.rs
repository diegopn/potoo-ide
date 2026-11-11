use super::*;

impl MainWindow {
    pub(super) fn handle_execution(
        &mut self,
        message: MainWindowMsg,
        sender: ComponentSender<Self>,
    ) {
        match message {
            MainWindowMsg::Run => {
                if self.active_execution_id.is_some() {
                    self.send_execution_control(ControlAction::Stop);
                    self.output_status = self.text("Interrompendo execução…").to_string();
                } else {
                    self.start_execution(sender.clone(), false);
                }
            }
            MainWindowMsg::StartDebug => {
                if self.active_execution_id.is_some() {
                    self.output_status = self
                        .text("Interrompa a execução atual antes de iniciar a depuração.")
                        .to_string();
                } else {
                    self.start_execution(sender.clone(), true);
                }
            }
            MainWindowMsg::StepDebug => {
                self.send_execution_control(ControlAction::Step);
                self.debugger_snapshot = None;
            }
            MainWindowMsg::StopDebug => {
                self.send_execution_control(ControlAction::Stop);
                self.debugger_snapshot = None;
            }
            MainWindowMsg::ExecutionEvent(execution_id, event) => {
                self.handle_execution_event(execution_id, event, sender.clone());
            }
            MainWindowMsg::ExecutionLost(execution_id) => {
                self.handle_execution_lost(execution_id);
            }
            MainWindowMsg::FileSelectionResolved(execution_id, selection) => {
                if execution_reply_is_current(execution_id, self.active_execution_id) {
                    if self.file_selection_cancellable.as_ref().is_some_and(
                        |(picker_execution_id, _)| *picker_execution_id == execution_id,
                    ) {
                        self.file_selection_cancellable = None;
                    }
                    if self.input_requested {
                        let input = selection.unwrap_or_else(|error| {
                            self.output_status = error;
                            String::new()
                        });
                        if self
                            .execution_input
                            .as_ref()
                            .is_some_and(|input_sender| input_sender.send(input).is_ok())
                        {
                            self.input_requested = false;
                            self.input_prompt = None;
                        } else {
                            self.output_status = self
                                .text("Não foi possível enviar a entrada ao programa.")
                                .to_string();
                        }
                    }
                }
            }
            MainWindowMsg::SubmitProgramInput(input) => {
                if self
                    .input_prompt
                    .as_ref()
                    .is_none_or(|prompt| !prompt.immediate_character || !input.is_empty())
                {
                    self.submit_program_input(input);
                }
            }
            MainWindowMsg::ProgramInputChanged(input) => {
                if !input.is_empty()
                    && self
                        .input_prompt
                        .as_ref()
                        .is_some_and(|prompt| prompt.immediate_character)
                {
                    self.submit_program_input(input);
                }
            }
            MainWindowMsg::OpenGamePreviewWindow => {
                if let Err(error) = self
                    .graphics_host
                    .open_jerimum_preview_window(self.text("Prévia do jogo"))
                {
                    self.output_status = error;
                }
            }
            _ => unreachable!("update message routed to the wrong domain"),
        }
    }
}

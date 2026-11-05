use super::*;

impl MainWindow {
    pub(super) fn handle_execution_event(
        &mut self,
        execution_id: u64,
        event: ExecutionEvent,
        sender: ComponentSender<Self>,
    ) {
        if execution_reply_is_current(execution_id, self.active_execution_id) {
            match event {
                ExecutionEvent::Output(text) => self.output_text.push_str(&text),
                ExecutionEvent::ClearOutput => self.output_text.clear(),
                ExecutionEvent::InputRequested(prompt) => {
                    if let (Some(name), Some(data_type)) =
                        (&prompt.variable_name, &prompt.data_type)
                    {
                        self.output_status =
                            format!("{} {name} ({data_type})", self.text("Entrada solicitada:"));
                    }
                    self.input_prompt = Some(prompt);
                    self.input_requested = true;
                }
                ExecutionEvent::FileSelectionRequested {
                    extensions,
                    allow_all_files,
                } => {
                    self.input_prompt = None;
                    self.input_requested = true;
                    if self.execution_input.is_none() {
                        self.input_requested = false;
                        self.output_status = self
                            .text("Não foi possível enviar a entrada ao programa.")
                            .to_string();
                        return;
                    }
                    let cancellable = gtk::gio::Cancellable::new();
                    self.file_selection_cancellable = Some((execution_id, cancellable.clone()));
                    show_portugol_file_selection_dialog(
                        &self.window,
                        self.ui_language,
                        execution_id,
                        self.execution_project_root.clone(),
                        extensions,
                        allow_all_files,
                        cancellable,
                        sender.clone(),
                    );
                }
                ExecutionEvent::AudioRequest { command, response } => {
                    let root = self.execution_project_root.clone();
                    let _ = response.send(self.audio_host.handle(root.as_deref(), command));
                }
                ExecutionEvent::HostRequest { request, response } => {
                    let result = match request {
                        execution::HostRequest::GraphicsWindowIsOpen => {
                            Ok(Some(potoo_portugol_interpreter::HostValue::Logical(
                                self.graphics_host.is_open(),
                            )))
                        }
                        execution::HostRequest::JerimumPreviewStart => {
                            self.graphics_host.start_jerimum_preview().map(|value| {
                                self.right_panel_section = RightPanelSection::GamePreview;
                                self.right_panel_collapsed = false;
                                self.game_preview_active = true;
                                Some(value)
                            })
                        }
                        execution::HostRequest::Graphics {
                            function,
                            arguments,
                        } => {
                            let root = self.execution_project_root.clone();
                            self.graphics_host
                                .graphics(root.as_deref(), &function, &arguments)
                                .map(Some)
                        }
                        request => self
                            .graphics_host
                            .handle_input_request(request, response.clone()),
                    };
                    match result {
                        Ok(Some(value)) => {
                            let _ = response.send(Ok(value));
                        }
                        Ok(None) => {}
                        Err(error) => {
                            let _ = response.send(Err(error));
                        }
                    }
                }
                ExecutionEvent::LineChanged(line) => self.execution_line = Some(line),
                ExecutionEvent::Debugger(snapshot) => {
                    self.debugging_active = true;
                    self.debugger_document_id = self.execution_document_id;
                    self.right_panel_section = RightPanelSection::Debugger;
                    self.right_panel_collapsed = false;
                    self.output_status =
                        format!("{} {}", self.text("Pausado na linha"), snapshot.line);
                    if self.execution_document_id == self.active_document_id {
                        self.debugger_learning_hint = self
                            .learning_hint
                            .as_ref()
                            .filter(|hint| hint.line == snapshot.line)
                            .cloned();
                        if self.learning_cursor_line != snapshot.line
                            || (self.debugger_learning_hint.is_none() && !self.learning_loading)
                        {
                            self.learning_cursor_line = snapshot.line;
                            self.schedule_learning_refresh(
                                sender.clone(),
                                Duration::from_millis(40),
                            );
                        }
                    } else {
                        self.debugger_learning_hint = None;
                    }
                    self.debugger_snapshot = Some(snapshot);
                }
                ExecutionEvent::Finished { result, stopped } => {
                    self.cancel_file_selection(execution_id);
                    self.audio_host.stop_all();
                    self.graphics_host.finish();
                    self.game_preview_active = false;
                    let finished_document_id = self.execution_document_id;
                    let debugger_run = self.debugger_document_id == finished_document_id;
                    if debugger_run {
                        self.debugger_snapshot = None;
                        self.debugger_learning_hint = None;
                    }
                    let source_matches_execution = finished_document_id
                        .and_then(|id| self.documents.iter().find(|document| document.id == id))
                        .is_some_and(|document| {
                            self.execution_source_fingerprint
                                == Some(Self::source_fingerprint(&document.contents))
                        });
                    self.active_execution_id = None;
                    self.debugging_active = false;
                    self.execution_document_id = None;
                    self.execution_line = None;
                    self.execution_project_root = None;
                    self.debugger_document_id = None;
                    self.execution_source_fingerprint = None;
                    self.execution_input = None;
                    self.execution_control = None;
                    self.execution_cancellable = None;
                    self.input_requested = false;
                    self.input_prompt = None;
                    match result {
                        Ok(()) => {
                            if source_matches_execution
                                && let Some(document) = finished_document_id.and_then(|id| {
                                    self.documents.iter_mut().find(|document| document.id == id)
                                })
                            {
                                document.runtime_diagnostics.clear();
                            }
                            self.execution_diagnostics.clear();
                            self.execution_diagnostics_document_id = None;
                            self.output_status = if stopped {
                                self.text("Execução interrompida.").to_string()
                            } else {
                                self.text("Execução concluída.").to_string()
                            };
                        }
                        Err(diagnostics) => {
                            let diagnostic_source = if source_matches_execution {
                                finished_document_id.and_then(|id| {
                                    self.documents
                                        .iter()
                                        .find(|document| document.id == id)
                                        .map(|document| {
                                            (document.language, document.contents.clone())
                                        })
                                })
                            } else {
                                None
                            };
                            let diagnostic_text = format_diagnostics(
                                &diagnostics,
                                diagnostic_source
                                    .as_ref()
                                    .map(|(language, source)| (*language, source.as_str())),
                            );
                            self.problem_text.clone_from(&diagnostic_text);
                            self.debugger_error = debugger_run;
                            self.output_status =
                                self.text("O programa terminou com um erro.").to_string();
                            if source_matches_execution {
                                if let Some(document) = finished_document_id.and_then(|id| {
                                    self.documents.iter_mut().find(|document| document.id == id)
                                }) {
                                    document.runtime_diagnostics =
                                        crate::features::diagnostics::from_execution(
                                            document.id,
                                            &document.name,
                                            document.path.as_deref(),
                                            document.language,
                                            &document.contents,
                                            &diagnostics,
                                        );
                                }
                                self.execution_diagnostics = diagnostics;
                                self.execution_diagnostics_document_id = finished_document_id;
                                self.schedule_learning_refresh(
                                    sender.clone(),
                                    Duration::from_millis(20),
                                );
                            } else {
                                self.execution_diagnostics.clear();
                                self.execution_diagnostics_document_id = None;
                            }
                        }
                    }
                }
            }
        }
    }

    pub(super) fn submit_program_input(&mut self, input: String) {
        if self.input_requested {
            if self
                .execution_input
                .as_ref()
                .is_some_and(|sender| sender.send(input).is_ok())
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

    pub(super) fn handle_execution_lost(&mut self, execution_id: u64) {
        if execution_reply_is_current(execution_id, self.active_execution_id) {
            self.cancel_file_selection(execution_id);
            self.audio_host.stop_all();
            self.graphics_host.finish();
            self.game_preview_active = false;
            self.active_execution_id = None;
            self.debugging_active = false;
            self.execution_document_id = None;
            self.execution_line = None;
            self.execution_project_root = None;
            self.debugger_document_id = None;
            self.execution_source_fingerprint = None;
            self.execution_input = None;
            self.execution_control = None;
            self.execution_cancellable = None;
            self.input_requested = false;
            self.input_prompt = None;
            self.debugger_snapshot = None;
            self.debugger_learning_hint = None;
            self.problem_text = self
                .text("O interpretador foi encerrado antes de concluir o programa.")
                .to_string();
            self.output_status = self
                .text("Não foi possível concluir a execução.")
                .to_string();
        }
    }
}

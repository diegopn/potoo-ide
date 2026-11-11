use super::*;

impl MainWindow {
    pub(super) fn handle_analysis(
        &mut self,
        message: MainWindowMsg,
        sender: ComponentSender<Self>,
    ) {
        match message {
            MainWindowMsg::CursorMoved(line) => {
                let line = line.max(1);
                if self.learning_cursor_line != line {
                    self.learning_cursor_line = line;
                    self.schedule_learning_refresh(sender.clone(), Duration::from_millis(100));
                }
            }
            MainWindowMsg::LearningDebounceElapsed(generation) => {
                if generation == self.learning_generation {
                    if let Some(document) = self.active_document().cloned() {
                        let line = self.learning_cursor_line;
                        let revision = self.learning_revision;
                        let runtime_diagnostics =
                            if self.execution_diagnostics_document_id == Some(document.id) {
                                self.execution_diagnostics.clone()
                            } else {
                                Vec::new()
                            };
                        self.learning_worker.submit(LearningAnalysisRequest {
                            generation,
                            document_id: document.id,
                            revision,
                            line,
                            contents: document.contents,
                            language: document.language,
                            runtime_diagnostics,
                            editor_diagnostics: document.diagnostics.unwrap_or_default(),
                        });
                    } else {
                        self.learning_loading = false;
                    }
                }
            }
            MainWindowMsg::LearningAnalysisReady {
                generation,
                document_id,
                revision,
                line,
                language,
                hint,
            } => {
                let current = CurrentLearningAnalysis {
                    generation: self.learning_generation,
                    active_document_id: self.active_document_id,
                    learning_document_id: self.learning_document_id,
                    revision: self.learning_revision,
                    line: self.learning_cursor_line,
                    language: self.learning_language,
                };
                let result = LearningAnalysisIdentity {
                    generation,
                    document_id,
                    revision,
                    line,
                    language,
                };
                if current.accepts(result) {
                    if self.debugger_document_id == Some(document_id)
                        && self.active_document_id == Some(document_id)
                        && self
                            .debugger_snapshot
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.line == line)
                    {
                        self.debugger_learning_hint = Some(hint.clone());
                    }
                    self.learning_hint = Some(hint);
                    self.learning_loading = false;
                }
            }
            MainWindowMsg::DiagnosticsDebounceElapsed(identity) => {
                if self.diagnostics_pending == Some(identity)
                    && identity.generation == self.diagnostics_generation
                    && let Some(document) = self.active_document().cloned()
                    && diagnostics::is_current_analysis(
                        identity,
                        self.diagnostics_generation,
                        document.id,
                        document.revision,
                        document.language,
                    )
                {
                    self.diagnostics_worker.submit(DiagnosticsAnalysisRequest {
                        identity,
                        source_name: document.name,
                        source_path: document.path,
                        contents: document.contents,
                    });
                }
            }
            MainWindowMsg::DiagnosticsAnalysisReady {
                identity,
                diagnostics,
            } => {
                let active_document_id = self.active_document_id;
                let generation = self.diagnostics_generation;
                let accepted = if let Some(document) = self
                    .documents
                    .iter_mut()
                    .find(|document| document.id == identity.document_id)
                {
                    if active_document_id == Some(identity.document_id)
                        && diagnostics::is_current_analysis(
                            identity,
                            generation,
                            document.id,
                            document.revision,
                            document.language,
                        )
                    {
                        document.diagnostics = Some(diagnostics);
                        self.diagnostics_pending = None;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };
                if accepted {
                    self.schedule_learning_refresh(sender.clone(), Duration::from_millis(40));
                }
            }
            MainWindowMsg::NavigateToDiagnostic(diagnostic) => {
                if self
                    .documents
                    .iter()
                    .any(|document| document.id == diagnostic.document_id)
                {
                    self.active_document_id = Some(diagnostic.document_id);
                    self.navigation_serial = self.navigation_serial.wrapping_add(1).max(1);
                    self.navigation_target = Some(DiagnosticNavigation {
                        serial: self.navigation_serial,
                        document_id: diagnostic.document_id,
                        offset: diagnostic.offset,
                    });
                }
            }
            _ => unreachable!("update message routed to the wrong domain"),
        }
    }
}

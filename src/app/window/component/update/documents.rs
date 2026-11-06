use super::*;

impl MainWindow {
    pub(super) fn handle_documents(
        &mut self,
        message: MainWindowMsg,
        sender: ComponentSender<Self>,
    ) {
        match message {
            MainWindowMsg::DocumentReadCompleted {
                project_generation,
                path,
                language_override,
                standalone,
                result,
            } => self.finish_document_read(
                project_generation,
                path,
                language_override,
                standalone,
                result,
                sender.clone(),
            ),
            MainWindowMsg::OpenFile(path) => self.open_document(path, None, sender.clone()),
            MainWindowMsg::OpenMaplerText(path) => {
                self.open_document(path, Some(Language::Mapler), sender.clone())
            }
            MainWindowMsg::ActivateDocument(id) => {
                if self.documents.iter().any(|doc| doc.id == id) {
                    self.active_document_id = Some(id);
                    if self.debugger_document_id == Some(id)
                        && let Some(snapshot) = &self.debugger_snapshot
                    {
                        self.learning_cursor_line = snapshot.line;
                    }
                }
            }
            MainWindowMsg::CloseDocument(id) => {
                if self
                    .documents
                    .iter()
                    .find(|document| document.id == id)
                    .is_some_and(SourceDocument::is_dirty)
                {
                    self.pending_close_document_id = Some(id);
                } else {
                    self.close_document(id);
                }
            }
            MainWindowMsg::CloseDocumentDecision(id, discard_changes) => {
                if self.pending_close_document_id == Some(id) {
                    self.pending_close_document_id = None;
                    if discard_changes {
                        self.close_document(id);
                    }
                }
            }
            MainWindowMsg::SaveAndCloseDocument(id) => {
                if self.pending_close_document_id == Some(id) {
                    self.pending_close_document_id = None;
                    if let Some(path) = self
                        .documents
                        .iter()
                        .find(|document| document.id == id)
                        .and_then(|document| document.path.clone())
                    {
                        self.save_document(id, path, sender.clone());
                        if self
                            .documents
                            .iter()
                            .find(|document| document.id == id)
                            .is_some_and(|document| !document.is_dirty())
                        {
                            self.close_document(id);
                        }
                    } else {
                        self.pending_close_after_save_id = Some(id);
                        self.pending_save_as_id = Some(id);
                    }
                }
            }
            MainWindowMsg::SaveCurrent => self.request_save(sender.clone()),
            MainWindowMsg::SaveAsSelected(id, path) => {
                let language_save = self.pending_language_save_as_id == Some(id);
                self.pending_save_as_id = None;
                let saved = self.save_document(id, path, sender.clone());
                if language_save {
                    self.pending_language_save_as_id = None;
                    if saved {
                        self.finish_language_change(sender.clone());
                    } else {
                        self.abort_language_change(false);
                    }
                }
                if self.pending_close_after_save_id == Some(id) {
                    self.pending_close_after_save_id = None;
                    if self
                        .documents
                        .iter()
                        .find(|document| document.id == id)
                        .is_some_and(|document| !document.is_dirty())
                    {
                        self.close_document(id);
                    }
                }
            }
            MainWindowMsg::SaveAsCanceled(id) => {
                let language_save = self.pending_language_save_as_id == Some(id);
                if self.pending_save_as_id == Some(id) {
                    self.pending_save_as_id = None;
                }
                if language_save {
                    self.abort_language_change(true);
                }
                if self.pending_close_after_save_id == Some(id) {
                    self.pending_close_after_save_id = None;
                }
            }
            MainWindowMsg::RequestClose => {
                if self.documents.iter().any(SourceDocument::is_dirty) {
                    self.pending_close_app = true;
                } else {
                    self.close_app_ready = true;
                    self.persist_session(true, false);
                }
            }
            MainWindowMsg::CloseAppDecision(discard_changes) => {
                self.pending_close_app = false;
                self.close_app_ready = discard_changes;
                if discard_changes {
                    self.persist_session(true, true);
                }
            }
            MainWindowMsg::SourceChanged(contents) => {
                let edited_document_id = self.active_document_id;
                if let Some(document) = self
                    .documents
                    .iter_mut()
                    .find(|document| Some(document.id) == edited_document_id)
                {
                    document.contents = contents;
                    document.revision = document.revision.wrapping_add(1);
                    document.diagnostics = None;
                    document.runtime_diagnostics.clear();
                }
                self.learning_revision = self.learning_revision.wrapping_add(1);
                if self.execution_diagnostics_document_id == edited_document_id {
                    self.execution_diagnostics.clear();
                    self.execution_diagnostics_document_id = None;
                }
                self.schedule_learning_refresh(sender.clone(), Duration::from_millis(280));
                self.schedule_diagnostics_refresh(sender.clone(), Duration::from_millis(280));
                self.recovery_generation += 1;
                let generation = self.recovery_generation;
                let sender_for_recovery = sender.clone();
                gtk::glib::timeout_add_local_once(Duration::from_millis(800), move || {
                    sender_for_recovery.input(MainWindowMsg::PersistRecovery(generation));
                });
            }
            MainWindowMsg::PersistRecovery(generation) => {
                if generation == self.recovery_generation {
                    self.persist_session(false, false);
                }
            }
            _ => unreachable!("update message routed to the wrong domain"),
        }
    }
}

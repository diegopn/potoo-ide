use super::super::*;
use super::super::{analysis::*, dialogs::*, persistence::*};

impl MainWindow {
    pub(super) fn update_component(
        &mut self,
        message: MainWindowMsg,
        sender: ComponentSender<MainWindow>,
    ) {
        let persist_after = matches!(
            &message,
            MainWindowMsg::ProjectSelected(_)
                | MainWindowMsg::RecentSelected(_)
                | MainWindowMsg::ProjectSwitchDecision(_)
                | MainWindowMsg::ProjectScanCompleted { .. }
                | MainWindowMsg::DocumentReadCompleted { .. }
                | MainWindowMsg::CreateProject(_, _, _)
                | MainWindowMsg::CreateFile(_, _, _)
                | MainWindowMsg::CreateFolder(_, _)
                | MainWindowMsg::RenameProjectItemConfirmed(_, _)
                | MainWindowMsg::DeleteProjectItemConfirmed(_)
                | MainWindowMsg::OpenFile(_)
                | MainWindowMsg::OpenMaplerText(_)
                | MainWindowMsg::ActivateDocument(_)
                | MainWindowMsg::CloseDocument(_)
                | MainWindowMsg::CloseDocumentDecision(_, _)
                | MainWindowMsg::SaveAndCloseDocument(_)
                | MainWindowMsg::SaveCurrent
                | MainWindowMsg::SaveAsSelected(_, _)
        );
        match message {
            MainWindowMsg::InterfaceLanguageSelected(language) => {
                if self.ui_language != language
                    && self.preferred_ui_language != language
                    && self.pending_ui_language.is_none()
                    && self.pending_save_as_id.is_none()
                    && self.pending_close_after_save_id.is_none()
                {
                    self.pending_ui_language = Some(language);
                    self.reopen_settings_after_language_restart = self.settings_visible;
                    self.settings_visible = false;
                    self.finish_language_change(sender.clone());
                }
            }
            MainWindowMsg::ProjectSelected(path) => {
                if self.documents.iter().any(SourceDocument::is_dirty) {
                    self.pending_project = Some(path);
                    self.pending_project_scan = None;
                    self.approved_project_switch = None;
                } else {
                    self.load_project(path, false, sender.clone());
                }
            }
            MainWindowMsg::RecentSelected(path) => {
                if let Some(path) = normalize_recent_path(path.clone()) {
                    if path.is_dir() {
                        if self.documents.iter().any(SourceDocument::is_dirty) {
                            self.pending_project = Some(path);
                            self.pending_project_scan = None;
                            self.approved_project_switch = None;
                        } else {
                            self.load_project(path, false, sender.clone());
                        }
                    } else {
                        self.open_document(path, None, sender.clone());
                    }
                } else {
                    self.output_status = match self.forget_recent(&path) {
                        Ok(()) => self
                            .text("O arquivo ou a pasta recente não está mais disponível.")
                            .to_string(),
                        Err(error) => format!(
                            "{}: {error}",
                            self.text("Não foi possível atualizar programas recentes")
                        ),
                    };
                }
            }
            MainWindowMsg::RecentRemoved(path) => {
                if let Err(error) = self.forget_recent(&path) {
                    self.output_status = format!(
                        "{}: {error}",
                        self.text("Não foi possível atualizar programas recentes")
                    );
                }
            }
            MainWindowMsg::ProjectSwitchDecision(discard_changes) => {
                if discard_changes {
                    if let Some(path) = self.pending_project.take() {
                        if let Some((scanned_path, files)) = self.pending_project_scan.take()
                            && scanned_path == path
                        {
                            let refresh_after = std::mem::take(&mut self.project_refresh_needed);
                            self.approved_project_switch = None;
                            self.apply_loaded_project(path.clone(), files, sender.clone());
                            if refresh_after {
                                self.refresh_project_scan(path, sender.clone());
                            }
                        } else {
                            self.load_project(path, true, sender.clone());
                        }
                    }
                } else {
                    self.pending_project = None;
                    self.pending_project_scan = None;
                    self.approved_project_switch = None;
                    self.project_refresh_needed = false;
                }
            }
            MainWindowMsg::ProjectScanCompleted {
                project_generation,
                scan_generation,
                root,
                load_project,
                result,
            } => self.finish_project_scan(
                project_generation,
                scan_generation,
                root,
                load_project,
                result,
                sender.clone(),
            ),
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
            MainWindowMsg::CreateProject(parent, name, template) => {
                self.create_project(parent, name, template, sender.clone());
            }
            MainWindowMsg::CreateFile(name, language, relative_folder) => {
                self.create_project_file(name, language, relative_folder, sender.clone())
            }
            MainWindowMsg::CreateFolder(name, relative_folder) => {
                self.create_project_folder(name, relative_folder, sender.clone())
            }
            MainWindowMsg::RenameProjectItem(path) => match self.validate_project_item(&path) {
                Ok(is_directory) => show_rename_project_item_dialog(
                    &self.window,
                    path,
                    is_directory,
                    self.ui_language,
                    self.dark,
                    sender.clone(),
                ),
                Err(error) => self.output_status = error,
            },
            MainWindowMsg::RenameProjectItemConfirmed(path, name) => {
                self.rename_project_item(path, name, sender.clone());
            }
            MainWindowMsg::DeleteProjectItem(path) => match self.validate_project_item(&path) {
                Ok(is_directory) => {
                    let has_unsaved_changes = self.documents.iter().any(|document| {
                        document.is_dirty()
                            && document.path.as_deref().is_some_and(|document_path| {
                                document_path == path
                                    || (is_directory && document_path.starts_with(&path))
                            })
                    });
                    let sender_for_delete = sender.clone();
                    let path_for_delete = path.clone();
                    show_potoo_confirmation(
                        &self.window,
                        self.ui_language,
                        self.dark,
                        "Excluir item do projeto",
                        "Deseja excluir este item do projeto?",
                        if has_unsaved_changes {
                            "Há alterações não salvas em arquivos deste item; elas serão perdidas."
                        } else {
                            "O item e todo o seu conteúdo serão excluídos permanentemente."
                        },
                        vec!["Cancelar", "Excluir"],
                        Some(1),
                        Some(1),
                        move |response| {
                            if response == 1 {
                                sender_for_delete.input(MainWindowMsg::DeleteProjectItemConfirmed(
                                    path_for_delete.clone(),
                                ));
                            }
                        },
                    );
                }
                Err(error) => self.output_status = error,
            },
            MainWindowMsg::DeleteProjectItemConfirmed(path) => {
                self.delete_project_item(path, sender.clone());
            }
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
            MainWindowMsg::FileOperationFailed(message) => self.output_status = message,
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
            MainWindowMsg::ThemeSelected(theme) => {
                self.theme = theme;
                self.dark = theme_is_dark(theme);
                self.save_preferences();
            }
            MainWindowMsg::SystemThemeChanged(dark) => {
                if self.theme == ThemePreference::System {
                    self.dark = dark;
                }
            }
            MainWindowMsg::FontSizeSelected(font_size) => {
                self.font_size = font_size;
                self.save_preferences();
            }
            MainWindowMsg::LineNumbersChanged(visible) => {
                self.line_numbers_visible = visible;
                self.save_preferences();
            }
            MainWindowMsg::WordWrapChanged(enabled) => {
                self.word_wrap = enabled;
                self.save_preferences();
            }
            MainWindowMsg::IndentWidthSelected(width) => {
                self.indent_width = if width == 2 { 2 } else { 4 };
                self.save_preferences();
            }
            MainWindowMsg::OpenSettings => self.settings_visible = true,
            MainWindowMsg::CloseSettings => self.settings_visible = false,
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
            MainWindowMsg::CursorMoved(line) => {
                let line = line.max(1);
                if self.learning_cursor_line != line {
                    self.learning_cursor_line = line;
                    self.schedule_learning_refresh(sender.clone(), Duration::from_millis(100));
                }
            }
            MainWindowMsg::SelectRightPanel(section) => {
                if self.right_panel_collapsed {
                    self.right_panel_section = section;
                    self.right_panel_collapsed = false;
                } else if self.right_panel_section == section {
                    self.right_panel_collapsed = true;
                } else {
                    self.right_panel_section = section;
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
            MainWindowMsg::PersistRecovery(generation) => {
                if generation == self.recovery_generation {
                    self.persist_session(false, false);
                }
            }
        }

        let active_language = self.active_document().map(|document| document.language);
        if self.learning_document_id != self.active_document_id
            || self.learning_language != active_language
        {
            self.learning_revision = self.learning_revision.wrapping_add(1);
            self.schedule_learning_refresh(sender.clone(), Duration::from_millis(80));
        }
        self.ensure_diagnostics_for_active_document(sender.clone());

        if persist_after {
            self.persist_session(false, false);
        }
    }
}

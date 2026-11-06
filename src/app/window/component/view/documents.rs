use super::*;

impl MainWindow {
    pub(super) fn sync_completion_provider_view(
        &self,
        widgets: &mut MainUi,
        view: &ViewContext<'_>,
    ) {
        let ui = widgets;
        let active_language = view.active_language;
        ui.completion_provider.set_language(
            active_language
                .filter(|language| crate::languages::completion::is_available(*language)),
        );
        ui.completion_provider.set_ui_language(self.ui_language);
    }
    pub(super) fn sync_active_document_view(
        &self,
        widgets: &mut MainUi,
        view: &ViewContext<'_>,
    ) -> bool {
        let ui = widgets;
        let active_document = view.active_document;
        let active_language = view.active_language;
        let active_document_id = active_document.map(|document| document.id);
        let active_document_changed = ui.displayed_active_document_id != active_document_id;
        if active_document_changed {
            ui.code_view.completion().hide();
            ui.code_buffer.set_text(
                active_document
                    .map(|document| document.contents.as_str())
                    .unwrap_or_default(),
            );
            ui.displayed_active_document_id = active_document_id;
        }
        if active_document_changed || ui.displayed_execution_line != self.execution_line {
            if self.execution_document_id == self.active_document_id
                && let Some(line) = self.execution_line
                && let Some(iter) = ui.code_buffer.iter_at_line(line.saturating_sub(1) as i32)
            {
                ui.code_buffer.place_cursor(&iter);
            }
            ui.displayed_execution_line = self.execution_line;
        }
        if ui.displayed_language != active_language {
            ui.status_language
                .set_text(active_language.map(Language::label).unwrap_or(""));
            ui.status_language.set_visible(active_language.is_some());
            ui.displayed_language = active_language;
        }
        ui.default_new_language.set(self.new_file_language);
        active_document_changed
    }
    pub(super) fn sync_document_tabs_view(
        &self,
        widgets: &mut MainUi,
        sender: ComponentSender<MainWindow>,
    ) {
        let ui = widgets;
        let tab_signature = self
            .documents
            .iter()
            .map(|document| {
                (
                    document.id,
                    document.name.clone(),
                    document.is_dirty(),
                    Some(document.id) == self.active_document_id,
                )
            })
            .collect::<Vec<_>>();
        if ui.displayed_tabs != tab_signature {
            content_panel::rebuild_editor_tabs(
                &ui.editor_tabs,
                &self.documents,
                self.active_document_id,
                self.ui_language,
                sender.clone(),
            );
            ui.displayed_tabs = tab_signature;
        }

        if ui.displayed_pending_project != self.pending_project {
            ui.displayed_pending_project
                .clone_from(&self.pending_project);
            if self.pending_project.is_some() {
                show_project_switch_confirmation(
                    &ui.window,
                    self.ui_language,
                    ui.dark_theme.get(),
                    sender.clone(),
                );
            }
        }

        if ui.displayed_pending_close_document_id != self.pending_close_document_id {
            ui.displayed_pending_close_document_id = self.pending_close_document_id;
            if let Some(id) = self.pending_close_document_id {
                show_close_document_confirmation(
                    &ui.window,
                    id,
                    self.ui_language,
                    ui.dark_theme.get(),
                    sender.clone(),
                );
            }
        }

        if ui.displayed_pending_close_app != self.pending_close_app {
            ui.displayed_pending_close_app = self.pending_close_app;
            if self.pending_close_app {
                show_close_app_confirmation(
                    &ui.window,
                    self.ui_language,
                    ui.dark_theme.get(),
                    sender.clone(),
                );
            }
        }

        if ui.displayed_pending_save_as_id != self.pending_save_as_id {
            ui.displayed_pending_save_as_id = self.pending_save_as_id;
            if let Some(id) = self.pending_save_as_id {
                if let Some(document) = self.documents.iter().find(|doc| doc.id == id) {
                    show_save_dialog(
                        &ui.window,
                        document,
                        self.project_root.as_deref(),
                        self.ui_language,
                        sender.clone(),
                    );
                }
            }
        }
    }
    pub(super) fn sync_close_app_view(&self, widgets: &mut MainUi) {
        let ui = widgets;
        if self.close_app_ready && !ui.displayed_close_app_ready {
            ui.displayed_close_app_ready = true;
            self.graphics_host.close_preview_window();
            ui.close_authorized.set(true);
            ui.window.close();
        }
    }
    pub(super) fn sync_diagnostics_list_view(
        &self,
        widgets: &mut MainUi,
        sender: ComponentSender<MainWindow>,
    ) -> (Vec<EditorDiagnostic>, bool) {
        let ui = widgets;
        let editor_diagnostics = self.all_diagnostics();
        let diagnostics_changed = ui.displayed_diagnostics != editor_diagnostics;
        if diagnostics_changed {
            content_panel::show_diagnostics(
                &ui.problems_list,
                &editor_diagnostics,
                self.ui_language,
                sender.clone(),
            );
            ui.displayed_diagnostics.clone_from(&editor_diagnostics);
        }
        if ui.displayed_execution_diagnostics_document_id != self.execution_diagnostics_document_id
        {
            if self.execution_diagnostics_document_id.is_some()
                && self.execution_diagnostics_document_id == self.active_document_id
            {
                ui.problems_tab.emit_clicked();
            }
            ui.displayed_execution_diagnostics_document_id = self.execution_diagnostics_document_id;
        }
        (editor_diagnostics, diagnostics_changed)
    }
    pub(super) fn sync_editor_marks_view(
        &self,
        widgets: &mut MainUi,
        view: &ViewContext<'_>,
        active_document_changed: bool,
        editor_diagnostics: &[EditorDiagnostic],
        diagnostics_changed: bool,
    ) {
        let ui = widgets;
        let active_document = view.active_document;
        let active_language = view.active_language;
        if let Some(target) = self.navigation_target
            && target.serial != ui.displayed_navigation_serial
            && self.active_document_id == Some(target.document_id)
        {
            let mut iter = ui.code_buffer.iter_at_offset(
                target
                    .offset
                    .min(ui.code_buffer.char_count().max(0) as usize) as i32,
            );
            ui.code_buffer.place_cursor(&iter);
            ui.code_view.scroll_to_iter(&mut iter, 0.2, false, 0.0, 0.0);
            ui.code_view.grab_focus();
            ui.displayed_navigation_serial = target.serial;
        }

        content_panel::highlight_buffer(
            &ui.code_buffer,
            active_language.unwrap_or(self.new_file_language),
            &ui.syntax_tags,
        );
        if active_document_changed || diagnostics_changed {
            let active_diagnostics = editor_diagnostics
                .iter()
                .filter(|diagnostic| Some(diagnostic.document_id) == self.active_document_id)
                .cloned()
                .collect::<Vec<_>>();
            content_panel::apply_diagnostics(
                &ui.code_view,
                &ui.code_buffer,
                &active_diagnostics,
                &ui.syntax_tags,
                self.dark,
            );
        }
        update_cursor_position(&ui.code_buffer, &ui.status_position, self.ui_language);
        ui.status_position.set_visible(active_document.is_some());
    }
}

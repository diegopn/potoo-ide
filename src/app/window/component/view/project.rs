use super::*;

impl MainWindow {
    pub(super) fn sync_new_item_context_view(&self, widgets: &mut MainUi) {
        let ui = widgets;
        ui.project_root_for_new_items
            .replace(self.project_root.clone());
        ui.project_files_for_new_items
            .replace(self.project_files.clone());
    }
    pub(super) fn sync_workspace_view(
        &self,
        widgets: &mut MainUi,
        view: &ViewContext<'_>,
        sender: ComponentSender<MainWindow>,
    ) {
        let ui = widgets;
        let active_document = view.active_document;
        if ui.displayed_start_page != self.start_page_visible {
            if ui.displayed_start_page && !self.start_page_visible {
                ui.window.maximize();
            }
            ui.main_stack
                .set_visible_child_name(if self.start_page_visible {
                    "start"
                } else {
                    "workspace"
                });
            ui.run_button.set_visible(!self.start_page_visible);
            ui.project_title.set_text(if self.start_page_visible {
                tr(self.ui_language, "Início")
            } else {
                &self.project_name
            });
            ui.displayed_start_page = self.start_page_visible;
        }

        if ui.displayed_project_name != self.project_name {
            if !self.start_page_visible {
                ui.project_title.set_text(&self.project_name);
            }
            ui.project_label.set_text(&self.project_name);
            ui.displayed_project_name.clone_from(&self.project_name);
        }

        if ui.displayed_recent_paths != self.recent_paths {
            let recent_preview = &self.recent_paths[..self.recent_paths.len().min(2)];
            ui.start_page
                .recent_section
                .set_visible(!self.recent_paths.is_empty());
            start_page::rebuild_recent_items(
                &ui.start_page.recent_list,
                recent_preview,
                self.ui_language,
                sender.clone(),
            );
            start_page::rebuild_recent_items(
                &ui.start_page.all_recent_list,
                &self.recent_paths,
                self.ui_language,
                sender.clone(),
            );
            ui.start_page
                .recent_more_button
                .set_visible(self.recent_paths.len() > 2);
            ui.displayed_recent_paths.clone_from(&self.recent_paths);
        }

        let active_path = active_document.and_then(|document| document.path.as_ref());
        let project_file_signature = self
            .project_files
            .iter()
            .map(|file| (file.path.clone(), active_path == Some(&file.path)))
            .collect::<Vec<_>>();
        let project_open = self.project_root.is_some();
        if ui.displayed_project_files != project_file_signature
            || ui.displayed_project_open != project_open
        {
            ui.expanded_folders.borrow_mut().retain(|expanded| {
                self.project_files
                    .iter()
                    .any(|item| item.is_directory && item.path == *expanded)
            });
            sidebar::rebuild_file_list(
                &ui.file_list,
                &self.project_files,
                active_path,
                project_open,
                self.project_root.as_deref(),
                &ui.empty_state,
                self.ui_language,
                &ui.expanded_folders,
                sender.clone(),
            );
            ui.displayed_project_files = project_file_signature;
            ui.displayed_project_open = project_open;
            ui.project_row.set_visible(project_open);
            ui.new_file_button.set_sensitive(project_open);
        }
    }
}

use super::*;

impl MainWindow {
    pub(super) fn handle_projects(
        &mut self,
        message: MainWindowMsg,
        sender: ComponentSender<Self>,
    ) {
        match message {
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
            MainWindowMsg::FileOperationFailed(message) => self.output_status = message,
            _ => unreachable!("update message routed to the wrong domain"),
        }
    }
}

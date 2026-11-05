use super::*;
use super::{persistence::*, project::*};

impl MainWindow {
    pub(super) fn active_document(&self) -> Option<&SourceDocument> {
        self.documents
            .iter()
            .find(|document| Some(document.id) == self.active_document_id)
    }

    pub(super) fn source_fingerprint(source: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        hasher.finish()
    }

    pub(super) fn schedule_learning_refresh(
        &mut self,
        sender: ComponentSender<Self>,
        delay: Duration,
    ) {
        let generation =
            invalidate_learning_hint(&mut self.learning_generation, &mut self.learning_hint);
        self.learning_worker.invalidate(generation);
        self.learning_document_id = self.active_document_id;
        self.learning_language = self.active_document().map(|document| document.language);
        self.learning_loading = self.learning_document_id.is_some();
        if self.learning_document_id.is_some() {
            gtk::glib::timeout_add_local_once(delay, move || {
                sender.input(MainWindowMsg::LearningDebounceElapsed(generation));
            });
        }
    }

    pub(super) fn schedule_diagnostics_refresh(
        &mut self,
        sender: ComponentSender<Self>,
        delay: Duration,
    ) {
        self.diagnostics_generation = self.diagnostics_generation.wrapping_add(1).max(1);
        let generation = self.diagnostics_generation;
        self.diagnostics_worker.invalidate(generation);
        self.diagnostics_pending = None;
        let Some(document) = self.active_document() else {
            return;
        };
        let identity = AnalysisIdentity {
            generation,
            document_id: document.id,
            revision: document.revision,
            language: document.language,
        };
        self.diagnostics_pending = Some(identity);
        gtk::glib::timeout_add_local_once(delay, move || {
            sender.input(MainWindowMsg::DiagnosticsDebounceElapsed(identity));
        });
    }

    pub(super) fn ensure_diagnostics_for_active_document(&mut self, sender: ComponentSender<Self>) {
        if self.diagnostics_active_document_id != self.active_document_id {
            self.diagnostics_active_document_id = self.active_document_id;
            self.diagnostics_generation = self.diagnostics_generation.wrapping_add(1).max(1);
            self.diagnostics_worker
                .invalidate(self.diagnostics_generation);
            self.diagnostics_pending = None;
        }
        let should_analyze = self
            .active_document()
            .is_some_and(|document| document.diagnostics.is_none());
        if should_analyze
            && !self.diagnostics_pending.is_some_and(|identity| {
                Some(identity.document_id) == self.active_document_id
                    && self.active_document().is_some_and(|document| {
                        identity.revision == document.revision
                            && identity.language == document.language
                    })
            })
        {
            self.schedule_diagnostics_refresh(sender, Duration::from_millis(40));
        }
    }

    pub(super) fn all_diagnostics(&self) -> Vec<EditorDiagnostic> {
        let mut diagnostics = Vec::new();
        for document in &self.documents {
            let static_diagnostics = document.diagnostics.as_deref().unwrap_or_default();
            diagnostics.extend(static_diagnostics.iter().cloned());
            diagnostics.extend(
                document
                    .runtime_diagnostics
                    .iter()
                    .filter(|runtime| {
                        !static_diagnostics.iter().any(|static_diagnostic| {
                            static_diagnostic.category == runtime.category
                                && static_diagnostic.message == runtime.message
                                && static_diagnostic.line == runtime.line
                                && static_diagnostic.column == runtime.column
                        })
                    })
                    .cloned(),
            );
        }
        diagnostics
    }

    pub(super) fn cancel_file_selection(&mut self, execution_id: u64) {
        if self
            .file_selection_cancellable
            .as_ref()
            .is_some_and(|(picker_execution_id, _)| *picker_execution_id == execution_id)
            && let Some((_, cancellable)) = self.file_selection_cancellable.take()
        {
            cancellable.cancel();
        }
    }

    pub(super) fn start_execution(&mut self, sender: ComponentSender<Self>, debugging: bool) {
        if self.active_execution_id.is_some() {
            return;
        }
        let Some(document) = self.active_document().cloned() else {
            self.output_status = self.text("Abra um programa antes de executar.").to_string();
            return;
        };

        self.output_text.clear();
        self.problem_text.clear();
        self.input_requested = false;
        self.input_prompt = None;
        self.debugger_snapshot = None;
        self.debugger_error = false;
        self.debugger_learning_hint = None;
        self.debugging_active = debugging;
        self.execution_line = None;
        if let Some(current) = self
            .documents
            .iter_mut()
            .find(|current| current.id == document.id)
        {
            current.runtime_diagnostics.clear();
        }
        self.execution_diagnostics.clear();
        self.execution_diagnostics_document_id = None;
        self.schedule_learning_refresh(sender.clone(), Duration::from_millis(20));
        self.execution_source_fingerprint = Some(Self::source_fingerprint(&document.contents));
        self.output_status = self
            .text(if debugging {
                "Depuração iniciada…"
            } else {
                "Executando o programa…"
            })
            .to_string();
        let execution_id = self.next_execution_id;
        self.next_execution_id = self.next_execution_id.wrapping_add(1).max(1);
        self.active_execution_id = Some(execution_id);
        self.execution_document_id = Some(document.id);
        self.debugger_document_id = debugging.then_some(document.id);
        let execution_root = self.project_root.clone().or_else(|| {
            document
                .path
                .as_deref()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
        });
        self.execution_project_root = execution_root.clone();

        let running = execution::start(
            document.contents,
            document.language,
            document.path.clone(),
            execution_root,
            debugging,
        );
        self.execution_input = Some(running.input);
        self.execution_control = Some(running.control);
        self.execution_cancellable = Some(running.cancellable);
        let events = running.events;
        gtk::glib::timeout_add_local(Duration::from_millis(16), move || {
            for _ in 0..MAX_EXECUTION_EVENTS_PER_TICK {
                match events.try_recv() {
                    Ok(event) => {
                        let finished = matches!(&event, ExecutionEvent::Finished { .. });
                        sender.input(MainWindowMsg::ExecutionEvent(execution_id, event));
                        if finished {
                            return gtk::glib::ControlFlow::Break;
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        return gtk::glib::ControlFlow::Continue;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        sender.input(MainWindowMsg::ExecutionLost(execution_id));
                        return gtk::glib::ControlFlow::Break;
                    }
                }
            }
            gtk::glib::ControlFlow::Continue
        });
    }

    pub(super) fn send_execution_control(&self, action: ControlAction) {
        if action == ControlAction::Stop
            && let Some((execution_id, cancellable)) = &self.file_selection_cancellable
            && self.active_execution_id == Some(*execution_id)
        {
            cancellable.cancel();
        }
        if action == ControlAction::Stop
            && let Some(cancellable) = &self.execution_cancellable
        {
            cancellable.cancel();
        }
        if let Some(control) = &self.execution_control {
            let _ = control.send(action);
        }
    }

    pub(super) fn create_document(
        &mut self,
        path: Option<PathBuf>,
        name: String,
        language: Language,
        contents: String,
        saved_contents: String,
    ) -> u64 {
        let id = self.next_document_id;
        self.next_document_id += 1;
        self.documents.push(SourceDocument {
            id,
            path,
            name,
            language,
            contents,
            saved_contents,
            revision: 0,
            diagnostics: None,
            runtime_diagnostics: Vec::new(),
        });
        self.active_document_id = Some(id);
        id
    }

    pub(super) fn open_document(
        &mut self,
        path: PathBuf,
        language_override: Option<Language>,
        sender: ComponentSender<Self>,
    ) {
        if language_override.is_none()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
        {
            let sender_for_confirmation = sender.clone();
            let path_for_confirmation = path.clone();
            show_potoo_confirmation(
                &self.window,
                self.ui_language,
                self.dark,
                "Abrir arquivo Mapler",
                "Deseja abrir este arquivo .txt como Mapler?",
                "A IDE original do Mapler usa .txt. O Potoo usa .mapler; arquivos .txt só são abertos como Mapler após esta confirmação.",
                vec!["Cancelar", "Abrir como Mapler"],
                Some(1),
                None,
                move |response| {
                    if response == 1 {
                        sender_for_confirmation
                            .input(MainWindowMsg::OpenMaplerText(path_for_confirmation.clone()));
                    }
                },
            );
            return;
        }
        let standalone = self
            .project_root
            .as_ref()
            .map_or(true, |root| !path.starts_with(root));
        if let Some(document) = self
            .documents
            .iter()
            .find(|doc| doc.path.as_ref() == Some(&path))
        {
            self.active_document_id = Some(document.id);
            self.start_page_visible = false;
            return;
        }
        if language_override.is_none() && Language::from_path(&path).is_none() {
            self.output_status = self
                .text("O editor abre arquivos .por, .alg, .poti, .gpt, .clg e .mapler; arquivos .txt exigem confirmação como Mapler.")
                .to_string();
            return;
        }
        let project_generation = self.project_generation;
        self.output_status = self.text("Abrindo arquivo…").to_string();
        std::thread::spawn(move || {
            let result = fs::read_to_string(&path).map_err(|error| error.to_string());
            sender.input(MainWindowMsg::DocumentReadCompleted {
                project_generation,
                path,
                language_override,
                standalone,
                result,
            });
        });
    }

    pub(super) fn schedule_project_scan(
        &mut self,
        root: PathBuf,
        load_project: bool,
        sender: ComponentSender<Self>,
    ) {
        if !load_project && self.project_load_pending {
            return;
        }
        self.project_scan_generation = self.project_scan_generation.wrapping_add(1).max(1);
        let scan_generation = self.project_scan_generation;
        let project_generation = self.project_generation;
        if load_project {
            self.project_load_pending = true;
        }
        std::thread::spawn(move || {
            let result = scan_project_files(&root).map_err(|error| error.to_string());
            sender.input(MainWindowMsg::ProjectScanCompleted {
                project_generation,
                scan_generation,
                root,
                load_project,
                result,
            });
        });
    }

    pub(super) fn refresh_project_scan(&mut self, root: PathBuf, sender: ComponentSender<Self>) {
        if self.project_load_pending {
            self.project_refresh_needed = true;
            return;
        }
        self.schedule_project_scan(root, false, sender);
    }

    pub(super) fn load_project(
        &mut self,
        root: PathBuf,
        discard_changes_confirmed: bool,
        sender: ComponentSender<Self>,
    ) {
        self.project_generation = self.project_generation.wrapping_add(1).max(1);
        self.pending_project_scan = None;
        self.approved_project_switch = discard_changes_confirmed.then(|| root.clone());
        self.project_refresh_needed = false;
        self.output_status = self.text("Abrindo pasta…").to_string();
        self.schedule_project_scan(root, true, sender);
    }

    pub(super) fn finish_document_read(
        &mut self,
        project_generation: u64,
        path: PathBuf,
        language_override: Option<Language>,
        standalone: bool,
        result: Result<String, String>,
        sender: ComponentSender<Self>,
    ) {
        if project_generation != self.project_generation {
            return;
        }
        if let Some(document) = self
            .documents
            .iter()
            .find(|document| document.path.as_ref() == Some(&path))
        {
            self.active_document_id = Some(document.id);
            self.start_page_visible = false;
            return;
        }
        let contents = match result {
            Ok(contents) => contents,
            Err(error) => {
                self.output_status =
                    format!("{}: {error}", self.text("Não foi possível abrir o arquivo"));
                return;
            }
        };
        if self.project_root.is_none()
            && let Some(root) = path.parent().map(Path::to_path_buf)
        {
            self.project_name = root
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| root.display().to_string());
            self.project_root = Some(root.clone());
            self.schedule_project_scan(root, false, sender);
        }
        let Some(language) = language_override.or_else(|| Language::from_path(&path)) else {
            return;
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.display().to_string());
        self.create_document(
            Some(path.clone()),
            name,
            language,
            contents.clone(),
            contents,
        );
        self.start_page_visible = false;
        if standalone {
            self.remember_recent(path);
        }
        self.output_status = self.text("Arquivo aberto").to_string();
    }

    pub(super) fn finish_project_scan(
        &mut self,
        project_generation: u64,
        scan_generation: u64,
        root: PathBuf,
        load_project: bool,
        result: Result<Vec<ProjectFile>, String>,
        sender: ComponentSender<Self>,
    ) {
        if !project_scan_is_current(
            self.project_generation,
            self.project_scan_generation,
            self.project_root.as_deref(),
            project_generation,
            scan_generation,
            &root,
            load_project,
        ) {
            return;
        }
        let refresh_after_load = if load_project {
            self.project_load_pending = false;
            std::mem::take(&mut self.project_refresh_needed)
        } else {
            false
        };
        let files = match result {
            Ok(files) => files,
            Err(error) => {
                self.pending_project_scan = None;
                if load_project {
                    self.approved_project_switch = None;
                }
                let message = if load_project {
                    "Não foi possível abrir a pasta"
                } else {
                    "Não foi possível atualizar os arquivos do projeto"
                };
                self.output_status = format!("{}: {error}", self.text(message));
                if refresh_after_load && let Some(current_root) = self.project_root.clone() {
                    self.refresh_project_scan(current_root, sender);
                }
                return;
            }
        };
        if !load_project {
            self.project_files = files;
            return;
        }
        if self.documents.iter().any(SourceDocument::is_dirty)
            && self.approved_project_switch.as_deref() != Some(root.as_path())
        {
            self.pending_project = Some(root.clone());
            self.pending_project_scan = Some((root, files));
            self.project_refresh_needed = refresh_after_load;
            return;
        }
        self.approved_project_switch = None;
        self.apply_loaded_project(root.clone(), files, sender.clone());
        if refresh_after_load {
            self.refresh_project_scan(root, sender);
        }
    }

    pub(super) fn apply_loaded_project(
        &mut self,
        root: PathBuf,
        files: Vec<ProjectFile>,
        sender: ComponentSender<Self>,
    ) {
        self.project_name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        self.remember_recent(root.clone());
        self.project_root = Some(root);
        self.start_page_visible = false;
        self.project_files = files;
        self.documents.clear();
        self.active_document_id = None;
        let root_file = self.project_root.as_ref().and_then(|root| {
            self.project_files
                .iter()
                .find(|file| {
                    !file.is_directory
                        && file.path.parent() == Some(root.as_path())
                        && Language::from_path(&file.path).is_some()
                })
                .map(|file| file.path.clone())
        });
        let first_source_file = root_file.or_else(|| {
            self.project_files
                .iter()
                .find(|file| !file.is_directory && Language::from_path(&file.path).is_some())
                .map(|file| file.path.clone())
        });
        if let Some(path) = first_source_file {
            self.open_document(path, None, sender);
        } else {
            self.output_status = self
                .text("Pasta aberta. Crie um arquivo .por, .alg, .poti, .gpt, .clg ou .mapler para começar.")
                .to_string();
        }
    }

    pub(super) fn save_document(
        &mut self,
        id: u64,
        path: PathBuf,
        sender: ComponentSender<Self>,
    ) -> bool {
        let Some(index) = self.documents.iter().position(|doc| doc.id == id) else {
            return false;
        };
        let (path, language) =
            match supported_save_path(path, self.documents[index].language, self.ui_language) {
                Ok(path) => path,
                Err(error) => {
                    self.output_status = error;
                    return false;
                }
            };
        if self.documents[index].language == Language::Mapler
            && self.documents[index].path.as_ref().is_some_and(|source| {
                source != &path
                    && source
                        .extension()
                        .and_then(|extension| extension.to_str())
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
                    && path.exists()
            })
        {
            self.output_status = self
                .text("Já existe um arquivo neste caminho; escolha outro nome em Salvar como.")
                .to_string();
            return false;
        }
        let standalone = self
            .project_root
            .as_ref()
            .map_or(true, |root| !path.starts_with(root));
        match atomic_write(&path, self.documents[index].contents.as_bytes()) {
            Ok(()) => {
                let language_changed = {
                    let document = &mut self.documents[index];
                    document.path = Some(path.clone());
                    let language_changed = document.language != language;
                    document.language = language;
                    document.name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    document.saved_contents.clone_from(&document.contents);
                    if language_changed {
                        document.revision = document.revision.wrapping_add(1);
                        document.diagnostics = None;
                        document.runtime_diagnostics.clear();
                    }
                    let source_name = document.name.clone();
                    let source_path = document.path.clone();
                    for diagnostic in document
                        .diagnostics
                        .iter_mut()
                        .flatten()
                        .chain(document.runtime_diagnostics.iter_mut())
                    {
                        diagnostic.source_name.clone_from(&source_name);
                        diagnostic.source_path.clone_from(&source_path);
                        diagnostic.language = language;
                    }
                    language_changed
                };
                if language_changed && self.execution_diagnostics_document_id == Some(id) {
                    self.execution_diagnostics.clear();
                    self.execution_diagnostics_document_id = None;
                }
                if self.project_root.is_none() {
                    self.project_root = path.parent().map(Path::to_path_buf);
                    if let Some(root) = &self.project_root {
                        self.project_name = root
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| root.display().to_string());
                    }
                }
                if let Some(root) = self.project_root.clone() {
                    self.refresh_project_scan(root, sender);
                }
                if standalone {
                    self.remember_recent(path.clone());
                }
                self.output_status = self.text("Arquivo salvo").to_string();
                true
            }
            Err(error) => {
                self.output_status = format!(
                    "{}: {error}",
                    self.text("Não foi possível salvar o arquivo")
                );
                false
            }
        }
    }

    pub(super) fn create_project_file(
        &mut self,
        name: String,
        template: ProjectTemplate,
        relative_folder: PathBuf,
        sender: ComponentSender<Self>,
    ) {
        let Some(root) = self.project_root.clone() else {
            self.output_status = self
                .text("Abra uma pasta de projeto antes de criar itens.")
                .to_string();
            return;
        };
        let folder = match resolve_project_directory(&root, &relative_folder, self.ui_language) {
            Ok(folder) => folder,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };
        let (path, language) =
            match new_source_path(&folder, &name, template.language(), self.ui_language) {
                Ok(source) => source,
                Err(error) => {
                    self.output_status = error;
                    return;
                }
            };
        let contents = if language == Language::Potigol {
            template.new_file_sample()
        } else {
            ""
        };

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut file) => {
                if let Err(error) = file.write_all(contents.as_bytes()) {
                    let _ = fs::remove_file(&path);
                    self.output_status =
                        format!("{}: {error}", self.text("Não foi possível criar o arquivo"));
                    return;
                }
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.create_document(
                    Some(path.clone()),
                    name,
                    language,
                    contents.to_owned(),
                    contents.to_owned(),
                );
                self.new_file_language = language;
                self.refresh_project_scan(root, sender);
                self.output_status = self.text("Novo arquivo criado").to_string();
                self.save_preferences();
            }
            Err(error) => {
                self.output_status = if error.kind() == std::io::ErrorKind::AlreadyExists {
                    self.text("Já existe um arquivo com esse nome.").to_string()
                } else {
                    format!("{}: {error}", self.text("Não foi possível criar o arquivo"))
                };
            }
        }
    }

    pub(super) fn create_project_folder(
        &mut self,
        name: String,
        relative_folder: PathBuf,
        sender: ComponentSender<Self>,
    ) {
        let Some(root) = self.project_root.clone() else {
            self.output_status = self
                .text("Abra uma pasta de projeto antes de criar itens.")
                .to_string();
            return;
        };
        let name = name.trim();
        if !is_valid_directory_name(name) {
            self.output_status = self.text("Digite um nome válido para a pasta.").to_string();
            return;
        }

        let folder = match resolve_project_directory(&root, &relative_folder, self.ui_language) {
            Ok(folder) => folder,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };

        match fs::create_dir(folder.join(name)) {
            Ok(()) => {
                self.refresh_project_scan(root, sender);
                self.output_status = self.text("Pasta criada").to_string();
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                self.output_status = self.text("Já existe uma pasta com esse nome.").to_string();
            }
            Err(error) => {
                self.output_status =
                    format!("{}: {error}", self.text("Não foi possível criar a pasta"));
            }
        }
    }

    pub(super) fn validate_project_item(&self, path: &Path) -> Result<bool, String> {
        let unavailable = || {
            self.text("O item selecionado não está mais disponível.")
                .to_string()
        };
        let Some(root) = self.project_root.as_deref() else {
            return Err(unavailable());
        };
        let Some(item) = self.project_files.iter().find(|item| item.path == path) else {
            return Err(unavailable());
        };
        let root = fs::canonicalize(root).map_err(|_| unavailable())?;
        let item_path = fs::canonicalize(path).map_err(|_| unavailable())?;
        if item_path == root
            || !item_path.starts_with(&root)
            || item_path.is_dir() != item.is_directory
        {
            return Err(unavailable());
        }
        Ok(item.is_directory)
    }

    pub(super) fn rename_project_item(
        &mut self,
        path: PathBuf,
        name: String,
        sender: ComponentSender<Self>,
    ) {
        let is_directory = match self.validate_project_item(&path) {
            Ok(is_directory) => is_directory,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };
        let name = match normalized_project_item_name(&path, &name, is_directory, self.ui_language)
        {
            Ok(name) => name,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };
        let Some(parent) = path.parent() else {
            self.output_status = self
                .text("O item selecionado não está mais disponível.")
                .to_string();
            return;
        };
        let new_path = parent.join(name);
        if new_path == path {
            return;
        }
        if new_path.exists() && fs::canonicalize(&new_path).ok() != fs::canonicalize(&path).ok() {
            self.output_status = self.text("Já existe um item com esse nome.").to_string();
            return;
        }

        if let Err(error) = fs::rename(&path, &new_path) {
            self.output_status =
                format!("{}: {error}", self.text("Não foi possível renomear o item"));
            return;
        }

        let mut clear_execution_diagnostics = false;
        for document in &mut self.documents {
            let Some(document_path) = document.path.clone() else {
                continue;
            };
            let Ok(relative_path) = document_path.strip_prefix(&path) else {
                continue;
            };
            if document_path == path {
                document.name = new_path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| new_path.display().to_string());
                if let Some(language) = Language::from_path(&new_path) {
                    if document.language != language {
                        document.revision = document.revision.wrapping_add(1);
                        document.diagnostics = None;
                        document.runtime_diagnostics.clear();
                        clear_execution_diagnostics |=
                            self.execution_diagnostics_document_id == Some(document.id);
                    }
                    document.language = language;
                }
            }
            document.path = Some(new_path.join(relative_path));
            let source_name = document.name.clone();
            let source_path = document.path.clone();
            let language = document.language;
            for diagnostic in document
                .diagnostics
                .iter_mut()
                .flatten()
                .chain(document.runtime_diagnostics.iter_mut())
            {
                diagnostic.source_name.clone_from(&source_name);
                diagnostic.source_path.clone_from(&source_path);
                diagnostic.language = language;
            }
        }
        if clear_execution_diagnostics {
            self.execution_diagnostics.clear();
            self.execution_diagnostics_document_id = None;
        }

        if let Some(root) = self.project_root.clone() {
            self.refresh_project_scan(root, sender);
        }
        self.output_status = self.text("Item renomeado.").to_string();
    }

    pub(super) fn delete_project_item(&mut self, path: PathBuf, sender: ComponentSender<Self>) {
        let is_directory = match self.validate_project_item(&path) {
            Ok(is_directory) => is_directory,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };
        let result = if is_directory {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        if let Err(error) = result {
            self.output_status =
                format!("{}: {error}", self.text("Não foi possível excluir o item"));
            return;
        }

        let removed_documents = self
            .documents
            .iter()
            .filter(|document| {
                document.path.as_deref().is_some_and(|document_path| {
                    document_path == path || (is_directory && document_path.starts_with(&path))
                })
            })
            .map(|document| document.id)
            .collect::<Vec<_>>();
        for id in removed_documents {
            self.close_document(id);
        }

        self.project_files
            .retain(|item| item.path != path && !item.path.starts_with(&path));
        if let Some(root) = self.project_root.clone() {
            self.refresh_project_scan(root, sender);
        }
        self.output_status = self.text("Item excluído.").to_string();
    }

    pub(super) fn create_project(
        &mut self,
        parent: PathBuf,
        name: String,
        template: ProjectTemplate,
        sender: ComponentSender<Self>,
    ) {
        let name = name.trim();
        if !is_valid_directory_name(name) {
            self.output_status = self
                .text("Digite um nome válido para o projeto.")
                .to_string();
            return;
        }

        let root = parent.join(name);
        if let Err(error) = fs::create_dir(&root) {
            self.output_status = if error.kind() == std::io::ErrorKind::AlreadyExists {
                self.text("Já existe um projeto com esse nome.").to_string()
            } else {
                format!("{}: {error}", self.text("Não foi possível criar o projeto"))
            };
            return;
        }

        let source_path = root.join(template.file_name());
        if let Err(error) = fs::write(&source_path, template.sample()) {
            let _ = fs::remove_file(&source_path);
            let cleanup_error = fs::remove_dir(&root).err();
            self.output_status = match cleanup_error {
                Some(cleanup_error) => format!(
                    "{}: {error} ({}: {cleanup_error})",
                    self.text("Não foi possível criar o projeto"),
                    self.text("Não foi possível remover a pasta incompleta")
                ),
                None => format!("{}: {error}", self.text("Não foi possível criar o projeto")),
            };
            return;
        }

        self.output_status = self.text("Projeto criado").to_string();
        sender.input(MainWindowMsg::ProjectSelected(root));
    }

    pub(super) fn close_document(&mut self, id: u64) {
        let Some(index) = self.documents.iter().position(|document| document.id == id) else {
            return;
        };
        self.documents.remove(index);
        if self.active_document_id == Some(id) {
            self.active_document_id = self
                .documents
                .get(index)
                .or_else(|| {
                    index
                        .checked_sub(1)
                        .and_then(|previous| self.documents.get(previous))
                })
                .map(|document| document.id);
        }
    }

    pub(super) fn request_save(&mut self, sender: ComponentSender<Self>) {
        match self
            .active_document()
            .map(|document| (document.id, document.path.clone()))
        {
            Some((id, Some(path))) => {
                self.save_document(id, path, sender);
            }
            Some((id, _)) => self.pending_save_as_id = Some(id),
            None => {
                self.output_status = self
                    .text("Crie ou abra um arquivo antes de salvar.")
                    .to_string();
            }
        }
    }
}

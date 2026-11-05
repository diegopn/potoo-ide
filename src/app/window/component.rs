use super::*;
use super::{analysis::*, dialogs::*, learning::*, persistence::*, settings::*, view::*};

pub struct MainUi {
    pub(super) window: gtk::ApplicationWindow,
    pub(super) content: gtk::Box,
    pub(super) main_stack: gtk::Stack,
    pub(super) dark_theme: Rc<Cell<bool>>,
    pub(super) default_new_language: Rc<Cell<Language>>,
    pub(super) project_root_for_new_items: Rc<RefCell<Option<PathBuf>>>,
    pub(super) project_files_for_new_items: Rc<RefCell<Vec<ProjectFile>>>,
    pub(super) settings: SettingsUi,
    pub(super) project_title: gtk::Label,
    pub(super) run_button: gtk::Button,
    pub(super) run_icon: gtk::Image,
    pub(super) run_label: gtk::Label,
    pub(super) start_page: start_page::StartPageUi,
    pub(super) project_row: gtk::Box,
    pub(super) project_label: gtk::Label,
    pub(super) file_list: gtk::Box,
    pub(super) new_file_button: gtk::MenuButton,
    pub(super) empty_state: gtk::Widget,
    pub(super) expanded_folders: sidebar::ExpandedFolders,
    pub(super) editor_tabs: gtk::Box,
    pub(super) status_language: gtk::Label,
    pub(super) status_details: gtk::Label,
    pub(super) status_position: gtk::Label,
    pub(super) code_buffer: gtk::TextBuffer,
    pub(super) code_view: sourceview5::View,
    pub(super) completion_provider: content_panel::CompletionProvider,
    pub(super) syntax_tags: SyntaxTags,
    pub(super) output_buffer: gtk::TextBuffer,
    pub(super) output_view: gtk::TextView,
    pub(super) problems_tab: gtk::Button,
    pub(super) problems_list: gtk::Box,
    pub(super) input_entry: gtk::Entry,
    pub(super) output_status: gtk::Label,
    pub(super) debugger_start: gtk::Button,
    pub(super) debugger_controls: gtk::Stack,
    pub(super) debugger_step: gtk::Button,
    pub(super) debugger_stop: gtk::Button,
    pub(super) debugger_position: gtk::Label,
    pub(super) debugger_variables: gtk::TextBuffer,
    pub(super) debugger_learning: content_panel::DebuggerLearningUi,
    pub(super) right_panel_revealer: gtk::Revealer,
    pub(super) right_panel_stack: gtk::Stack,
    pub(super) learning_tool_button: gtk::Button,
    pub(super) debugger_tool_button: gtk::Button,
    pub(super) game_preview_button: gtk::Button,
    pub(super) game_preview_stack: gtk::Stack,
    pub(super) game_preview_area: gtk::DrawingArea,
    pub(super) game_preview_open_button: gtk::Button,
    pub(super) learning_spinner: gtk::Spinner,
    pub(super) learning_analysis_stack: gtk::Stack,
    pub(super) learning_line_title: gtk::Label,
    pub(super) learning_explanation: gtk::Label,
    pub(super) learning_experiment: gtk::Label,
    pub(super) learning_concept_section: gtk::Box,
    pub(super) learning_experiment_section: gtk::Box,
    pub(super) learning_concept_title: gtk::Label,
    pub(super) learning_concept_text: gtk::Label,
    pub(super) learning_problem_list: gtk::Box,
    pub(super) learning_problem_empty: gtk::Label,
    pub(super) displayed_language: Option<Language>,
    pub(super) displayed_active_document_id: Option<u64>,
    pub(super) displayed_project_name: String,
    pub(super) displayed_project_files: Vec<(PathBuf, bool)>,
    pub(super) displayed_project_open: bool,
    pub(super) displayed_recent_paths: Vec<PathBuf>,
    pub(super) displayed_start_page: bool,
    pub(super) displayed_run_active: bool,
    pub(super) displayed_run_language: UiLanguage,
    pub(super) displayed_tabs: Vec<(u64, String, bool, bool)>,
    pub(super) displayed_pending_project: Option<PathBuf>,
    pub(super) displayed_pending_close_document_id: Option<u64>,
    pub(super) displayed_pending_close_app: bool,
    pub(super) displayed_pending_save_as_id: Option<u64>,
    pub(super) displayed_close_app_ready: bool,
    pub(super) close_authorized: Rc<Cell<bool>>,
    pub(super) displayed_theme: ThemePreference,
    pub(super) displayed_dark: bool,
    pub(super) displayed_font_size: u32,
    pub(super) displayed_line_numbers_visible: bool,
    pub(super) displayed_word_wrap: bool,
    pub(super) displayed_indent_width: u32,
    pub(super) displayed_preferred_ui_language: UiLanguage,
    pub(super) displayed_settings_visible: bool,
    pub(super) displayed_output_text: String,
    pub(super) displayed_diagnostics: Vec<EditorDiagnostic>,
    pub(super) displayed_execution_diagnostics_document_id: Option<u64>,
    pub(super) displayed_navigation_serial: u64,
    pub(super) displayed_input_requested: bool,
    pub(super) displayed_output_status: String,
    pub(super) displayed_debugger_snapshot: Option<DebuggerSnapshot>,
    pub(super) displayed_debugger_error: bool,
    pub(super) displayed_execution_line: Option<usize>,
    pub(super) displayed_debugger_learning_hint: Option<LearningHint>,
    pub(super) displayed_debugger_analysis_state: &'static str,
    pub(super) displayed_learning_hint: Option<LearningHint>,
    pub(super) displayed_learning_loading: bool,
    pub(super) displayed_right_panel_section: Option<RightPanelSection>,
    pub(super) displayed_right_panel_collapsed: bool,
    pub(super) displayed_game_preview_active: bool,
}

impl SimpleComponent for MainWindow {
    type Init = ();
    type Input = MainWindowMsg;
    type Output = ();
    type Root = gtk::ApplicationWindow;
    type Widgets = MainUi;

    fn init_root() -> Self::Root {
        gtk::ApplicationWindow::default()
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.set_title(Some("Potoo IDE"));
        root.set_default_size(980, 700);
        root.add_css_class("potoo-window");
        let preferences = load_preferences();
        let saved_session = load_session();
        let unreadable_session = session_file().exists() && saved_session.is_none();
        let recovering_session = saved_session
            .as_ref()
            .is_some_and(|session| !session.clean_exit);
        let restored_root = saved_session
            .as_ref()
            .filter(|_| recovering_session)
            .and_then(|session| {
                session
                    .project_root
                    .as_ref()
                    .filter(|path| path.is_dir())
                    .cloned()
                    .or_else(|| {
                        let document_parent = |document: &SessionDocument| {
                            document
                                .path
                                .as_deref()
                                .and_then(Path::parent)
                                .filter(|path| path.is_dir())
                                .map(Path::to_path_buf)
                        };
                        session
                            .documents
                            .get(session.active_index)
                            .and_then(document_parent)
                            .or_else(|| session.documents.iter().find_map(document_parent))
                    })
            });
        let mut recent_paths = load_recent_paths();
        if recent_paths.is_empty() {
            if let Some(session) = &saved_session {
                let migration_paths = session.project_root.iter().chain(
                    session
                        .documents
                        .iter()
                        .filter_map(|document| document.path.as_ref()),
                );
                for path in migration_paths {
                    if let Some(path) = normalize_recent_path(path.clone()) {
                        if !recent_paths.contains(&path) {
                            recent_paths.push(path);
                        }
                    }
                }
                recent_paths.truncate(MAX_RECENT_ITEMS);
                if !recent_paths.is_empty() {
                    let _ = persist_recent_paths(&recent_paths);
                }
            }
        }
        let (documents, active_document_id, recovered_changes) = if recovering_session {
            saved_session
                .map(restore_session)
                .unwrap_or_else(|| (Vec::new(), None, false))
        } else {
            (Vec::new(), None, false)
        };
        let start_page_visible =
            !recovering_session || (restored_root.is_none() && documents.is_empty());
        if !start_page_visible || recent_paths.len() >= 2 {
            root.maximize();
        }
        let project_files = Vec::new();
        let project_name = restored_root
            .as_deref()
            .and_then(Path::file_name)
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| tr(preferences.ui_language, "Meu primeiro programa").to_string());
        let initial_dark = theme_is_dark(preferences.theme);
        if let Some(settings) = gtk::Settings::default() {
            let sender_for_system_theme = sender.clone();
            settings.connect_notify_local(
                Some("gtk-interface-color-scheme"),
                move |settings, _| {
                    let dark = settings
                        .property::<gtk::InterfaceColorScheme>("gtk-interface-color-scheme")
                        == gtk::InterfaceColorScheme::Dark;
                    sender_for_system_theme.input(MainWindowMsg::SystemThemeChanged(dark));
                },
            );
        }
        let learning_worker = LearningAnalysisWorker::new(sender.clone());
        let diagnostics_worker = DiagnosticsAnalysisWorker::new(sender.clone());

        let mut model = Self {
            window: root.clone(),
            audio_host: crate::features::media::audio::AudioHost::default(),
            graphics_host: {
                let sender = sender.clone();
                crate::features::media::graphics::GraphicsHost::new(root.clone(), move || {
                    sender.input(MainWindowMsg::StopDebug);
                })
            },
            new_file_language: preferences.default_language,
            next_document_id: documents.len() as u64 + 1,
            documents,
            active_document_id,
            project_root: restored_root,
            project_name,
            project_files,
            project_generation: 0,
            project_scan_generation: 0,
            pending_project: None,
            pending_project_scan: None,
            approved_project_switch: None,
            project_load_pending: false,
            project_refresh_needed: false,
            pending_close_document_id: None,
            pending_close_after_save_id: None,
            pending_save_as_id: None,
            pending_ui_language: None,
            pending_language_save_as_id: None,
            reopen_settings_after_language_restart: false,
            pending_close_app: false,
            close_app_ready: false,
            ui_language: preferences.ui_language,
            preferred_ui_language: preferences.ui_language,
            theme: preferences.theme,
            dark: initial_dark,
            font_size: preferences.font_size,
            line_numbers_visible: preferences.line_numbers_visible,
            word_wrap: preferences.word_wrap,
            indent_width: preferences.indent_width,
            recovery_generation: 0,
            settings_visible: false,
            output_text: tr(
                preferences.ui_language,
                "A saída aparecerá aqui após a execução.",
            )
            .to_string(),
            output_status: if recovered_changes {
                tr(
                    preferences.ui_language,
                    "Alterações da sessão anterior recuperadas.",
                )
                .to_string()
            } else if unreadable_session {
                tr(
                    preferences.ui_language,
                    "Não foi possível ler a sessão anterior. Os arquivos originais continuam intactos.",
                )
                .to_string()
            } else {
                tr(preferences.ui_language, "Pronto para executar").to_string()
            },
            problem_text: tr(preferences.ui_language, "Nenhum problema detectado").to_string(),
            game_preview_active: false,
            next_execution_id: 1,
            active_execution_id: None,
            debugging_active: false,
            execution_document_id: None,
            execution_line: None,
            execution_project_root: None,
            debugger_document_id: None,
            execution_input: None,
            execution_control: None,
            execution_cancellable: None,
            file_selection_cancellable: None,
            input_requested: false,
            input_prompt: None,
            debugger_snapshot: None,
            debugger_error: false,
            debugger_learning_hint: None,
            execution_diagnostics: Vec::new(),
            execution_diagnostics_document_id: None,
            execution_source_fingerprint: None,
            diagnostics_generation: 0,
            diagnostics_worker,
            diagnostics_active_document_id: None,
            diagnostics_pending: None,
            navigation_serial: 0,
            navigation_target: None,
            learning_generation: 0,
            learning_revision: 0,
            learning_cursor_line: 1,
            learning_document_id: None,
            learning_language: None,
            learning_hint: None,
            learning_loading: false,
            right_panel_section: RightPanelSection::Learning,
            right_panel_collapsed: false,
            learning_worker,
            recent_paths,
            start_page_visible,
        };
        let project_files_for_new_items = Rc::new(RefCell::new(model.project_files.clone()));
        let mut widgets = build_window_content(
            root.clone(),
            model.project_root.clone(),
            model.start_page_visible,
            &model.recent_paths,
            project_files_for_new_items,
            sender.clone(),
            preferences,
            initial_dark,
        );
        model
            .graphics_host
            .set_jerimum_preview_area(widgets.game_preview_area.clone());
        root.set_child(Some(&widgets.content));
        if let Some(project_root) = model.project_root.clone() {
            model.schedule_project_scan(project_root, false, sender.clone());
        }
        content_panel::rebuild_editor_tabs(
            &widgets.editor_tabs,
            &model.documents,
            model.active_document_id,
            model.ui_language,
            sender.clone(),
        );
        sender.input(MainWindowMsg::CursorMoved(1));
        let recent_preview = &model.recent_paths[..model.recent_paths.len().min(2)];
        widgets
            .start_page
            .recent_section
            .set_visible(!model.recent_paths.is_empty());
        start_page::rebuild_recent_items(
            &widgets.start_page.recent_list,
            recent_preview,
            model.ui_language,
            sender.clone(),
        );
        start_page::rebuild_recent_items(
            &widgets.start_page.all_recent_list,
            &model.recent_paths,
            model.ui_language,
            sender.clone(),
        );
        widgets
            .start_page
            .recent_more_button
            .set_visible(model.recent_paths.len() > 2);
        model.ensure_diagnostics_for_active_document(sender.clone());
        widgets.displayed_tabs = model
            .documents
            .iter()
            .map(|document| {
                (
                    document.id,
                    document.name.clone(),
                    document.is_dirty(),
                    Some(document.id) == model.active_document_id,
                )
            })
            .collect();

        if !unreadable_session {
            if let Err(error) = persist_session(&model, false, false) {
                model.output_status = format!(
                    "{}: {error}",
                    tr(preferences.ui_language, "Não foi possível salvar a sessão")
                );
            }
        }

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
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

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        let ui = widgets;
        ui.project_root_for_new_items
            .replace(self.project_root.clone());
        ui.project_files_for_new_items
            .replace(self.project_files.clone());

        let active_document = self.active_document();
        let active_language = active_document.map(|document| document.language);
        ui.completion_provider.set_language(active_language);
        ui.completion_provider.set_ui_language(self.ui_language);
        let game_preview_available = active_language == Some(Language::Potigol);
        ui.game_preview_button.set_visible(game_preview_available);
        let right_panel_section = if !game_preview_available
            && self.right_panel_section == RightPanelSection::GamePreview
        {
            RightPanelSection::Learning
        } else {
            self.right_panel_section
        };

        if ui.displayed_right_panel_section != Some(right_panel_section)
            || ui.displayed_right_panel_collapsed != self.right_panel_collapsed
        {
            let collapsed = self.right_panel_collapsed;
            let section = right_panel_section;
            ui.right_panel_stack.set_visible_child_name(match section {
                RightPanelSection::Learning => "learning",
                RightPanelSection::Debugger => "debugger",
                RightPanelSection::GamePreview => "game-preview",
            });
            ui.right_panel_revealer.set_reveal_child(!collapsed);

            let learning_selected = section == RightPanelSection::Learning;
            let debugger_selected = section == RightPanelSection::Debugger;
            let game_preview_selected = section == RightPanelSection::GamePreview;
            ui.learning_tool_button
                .set_css_classes(if learning_selected {
                    &["potoo-tool-rail-button", "potoo-tool-rail-button-selected"]
                } else {
                    &["potoo-tool-rail-button"]
                });
            ui.debugger_tool_button
                .set_css_classes(if debugger_selected {
                    &["potoo-tool-rail-button", "potoo-tool-rail-button-selected"]
                } else {
                    &["potoo-tool-rail-button"]
                });
            ui.game_preview_button
                .set_css_classes(if game_preview_selected {
                    &["potoo-tool-rail-button", "potoo-tool-rail-button-selected"]
                } else {
                    &["potoo-tool-rail-button"]
                });
            ui.learning_tool_button.set_tooltip_text(Some(self.text(
                if learning_selected && !collapsed {
                    "Recolher Aprenda enquanto programa"
                } else {
                    "Abrir Aprenda enquanto programa"
                },
            )));
            ui.debugger_tool_button.set_tooltip_text(Some(self.text(
                if section == RightPanelSection::Debugger && !collapsed {
                    "Recolher depurador"
                } else {
                    "Abrir depurador"
                },
            )));
            ui.game_preview_button.set_tooltip_text(Some(self.text(
                if game_preview_selected && !collapsed {
                    "Recolher prévia do jogo"
                } else {
                    "Abrir prévia do jogo"
                },
            )));
            ui.displayed_right_panel_section = Some(section);
            ui.displayed_right_panel_collapsed = collapsed;
        }

        if ui.displayed_game_preview_active != self.game_preview_active {
            ui.game_preview_stack
                .set_visible_child_name(if self.game_preview_active {
                    "canvas"
                } else {
                    "idle"
                });
            ui.displayed_game_preview_active = self.game_preview_active;
        }
        ui.game_preview_open_button
            .set_sensitive(self.game_preview_active);

        let active_document_id = active_document.map(|document| document.id);
        let current_learning_hint = if self.learning_loading {
            ui.displayed_learning_hint.clone()
        } else {
            self.learning_hint.clone()
        };
        if ui.displayed_learning_hint != current_learning_hint {
            if let Some(hint) = &current_learning_hint {
                let topic_name = learning_topic_label(hint.topic, self.ui_language);
                ui.learning_line_title.set_text(&format!(
                    "{} {} · {topic_name}",
                    self.text("Linha"),
                    hint.line
                ));
                let (explanation, concept, experiment) = learning_copy(hint, self.ui_language);
                ui.learning_explanation.set_text(&explanation);
                ui.learning_experiment.set_text(&experiment);
                let show_learning_concept = hint.topic != LearningTopic::Unrecognized;
                ui.learning_concept_section
                    .set_visible(show_learning_concept);
                ui.learning_experiment_section
                    .set_visible(show_learning_concept && !experiment.is_empty());
                ui.learning_concept_title
                    .set_text(&format!("{} {topic_name}", self.text("Conceito:")));
                ui.learning_concept_text.set_text(&concept);
                content_panel::show_learning_problems(
                    &ui.learning_problem_list,
                    &ui.learning_problem_empty,
                    &hint.problems,
                    self.ui_language,
                    &ui.code_buffer,
                    ui.code_view.upcast_ref(),
                );
                ui.learning_problem_empty
                    .set_text(self.text("Nenhum problema conhecido nesta linha."));
                ui.learning_problem_empty.set_text(self.text(
                    if hint.topic == LearningTopic::Unrecognized {
                        "A análise não reconheceu esta construção; não foi possível verificar problemas."
                    } else {
                        "Nenhum problema conhecido nesta linha."
                    },
                ));
            } else {
                ui.learning_concept_section.set_visible(true);
                ui.learning_experiment_section.set_visible(true);
                ui.learning_line_title
                    .set_text(self.text("Linha 1 · programa"));
                ui.learning_explanation.set_text(
                    self.text(
                        "Selecione uma linha para ver uma explicação contextual do programa.",
                    ),
                );
                ui.learning_experiment.set_text(
                    self.text(
                        "Mova o cursor para uma condição, um laço, uma entrada ou uma saída.",
                    ),
                );
                ui.learning_concept_title
                    .set_text(self.text("Conceito: programa"));
                ui.learning_concept_text.set_text(
                    self.text(
                        "O painel acompanha a linha selecionada e o conteúdo atual do editor.",
                    ),
                );
                content_panel::show_learning_problems(
                    &ui.learning_problem_list,
                    &ui.learning_problem_empty,
                    &[],
                    self.ui_language,
                    &ui.code_buffer,
                    ui.code_view.upcast_ref(),
                );
            }
            ui.displayed_learning_hint
                .clone_from(&current_learning_hint);
        }
        if ui.displayed_learning_loading != self.learning_loading {
            ui.learning_spinner.set_spinning(self.learning_loading);
            ui.learning_spinner.set_visible(self.learning_loading);
            ui.learning_analysis_stack
                .set_visible_child_name(if self.learning_loading {
                    "loading"
                } else {
                    "content"
                });
            ui.displayed_learning_loading = self.learning_loading;
        }
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

        if ui.displayed_indent_width != self.indent_width {
            ui.code_view.set_indent_width(self.indent_width as i32);
            ui.code_view.set_tab_width(self.indent_width);
            ui.settings
                .indent_width_picker
                .set_selected(if self.indent_width == 2 { 0 } else { 1 });
            ui.status_details.set_text(&format!(
                "UTF-8  ·  {}: {}",
                tr(self.ui_language, "Espaços"),
                self.indent_width
            ));
            ui.displayed_indent_width = self.indent_width;
        }

        let displayed_preferred_ui_language = self
            .pending_ui_language
            .unwrap_or(self.preferred_ui_language);
        if ui.displayed_preferred_ui_language != displayed_preferred_ui_language {
            ui.settings
                .ui_language_picker
                .set_selected(displayed_preferred_ui_language.selected_index());
            ui.displayed_preferred_ui_language = displayed_preferred_ui_language;
        }

        if ui.displayed_dark != self.dark {
            ui.dark_theme.set(self.dark);
            if self.dark {
                ui.window.add_css_class("potoo-dark");
                ui.settings.window.add_css_class("potoo-dark");
            } else {
                ui.window.remove_css_class("potoo-dark");
                ui.settings.window.remove_css_class("potoo-dark");
            }
            content_panel::set_syntax_palette(&ui.syntax_tags, self.dark);
            content_panel::install_diagnostic_markers(&ui.code_view, self.dark);
            ui.displayed_dark = self.dark;
        }

        if ui.displayed_theme != self.theme {
            select_theme_button(&ui.settings.theme_buttons, self.theme.selected_index());
            ui.displayed_theme = self.theme;
        }
        if ui.displayed_font_size != self.font_size {
            set_font_size(&ui.code_view, ui.displayed_font_size, self.font_size);
            ui.settings
                .font_size_picker
                .set_selected(font_size_index(self.font_size));
            ui.displayed_font_size = self.font_size;
        }
        if ui.displayed_line_numbers_visible != self.line_numbers_visible {
            ui.code_view
                .set_show_line_numbers(self.line_numbers_visible);
            ui.settings
                .line_numbers_switch
                .set_active(self.line_numbers_visible);
            ui.displayed_line_numbers_visible = self.line_numbers_visible;
        }
        if ui.displayed_word_wrap != self.word_wrap {
            ui.code_view.set_wrap_mode(if self.word_wrap {
                gtk::WrapMode::WordChar
            } else {
                gtk::WrapMode::None
            });
            ui.settings.word_wrap_switch.set_active(self.word_wrap);
            ui.displayed_word_wrap = self.word_wrap;
        }
        if ui.displayed_settings_visible != self.settings_visible {
            if self.settings_visible {
                ui.settings.window.present();
            } else {
                ui.settings.window.set_visible(false);
            }
            ui.displayed_settings_visible = self.settings_visible;
        }

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

        if self.close_app_ready && !ui.displayed_close_app_ready {
            ui.displayed_close_app_ready = true;
            self.graphics_host.close_preview_window();
            ui.close_authorized.set(true);
            ui.window.close();
        }

        if ui.displayed_output_text != self.output_text {
            ui.output_buffer.set_text(&self.output_text);
            if self.output_text == self.text("A saída aparecerá aqui após a execução.") {
                ui.output_view
                    .add_css_class("potoo-output-placeholder-text");
            } else {
                ui.output_view
                    .remove_css_class("potoo-output-placeholder-text");
            }
            ui.displayed_output_text.clone_from(&self.output_text);
        }
        if ui.displayed_output_status != self.output_status {
            ui.output_status.set_text(&self.output_status);
            ui.displayed_output_status.clone_from(&self.output_status);
        }
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
        if ui.displayed_input_requested != self.input_requested {
            ui.input_entry.set_visible(self.input_requested);
            if self.input_requested {
                if let Some(prompt) = &self.input_prompt {
                    let placeholder = match (&prompt.variable_name, &prompt.data_type) {
                        (Some(name), Some(data_type)) => format!("{} ({data_type})", name),
                        _ => self.text("Digite a entrada do programa").to_owned(),
                    };
                    ui.input_entry.set_placeholder_text(Some(&placeholder));
                }
                ui.input_entry.grab_focus();
            } else {
                ui.input_entry.set_text("");
            }
            ui.displayed_input_requested = self.input_requested;
        }
        ui.debugger_controls
            .set_visible_child_name(if self.debugging_active {
                "controls"
            } else {
                "start"
            });
        ui.debugger_start.set_tooltip_text(Some(self.text(
            if self.active_execution_id.is_some() {
                "Interrompa a execução atual antes de iniciar a depuração."
            } else {
                "Iniciar depuração"
            },
        )));
        ui.debugger_start
            .set_sensitive(self.active_execution_id.is_none());
        ui.debugger_step.set_sensitive(
            self.debugging_active
                && self.active_execution_id.is_some()
                && self.debugger_snapshot.is_some(),
        );
        ui.debugger_stop.set_visible(self.debugging_active);
        ui.debugger_stop
            .set_sensitive(self.debugging_active && self.active_execution_id.is_some());
        let debugger_error = self.debugger_error
            && self.active_document_id.is_some()
            && self.execution_diagnostics_document_id == self.active_document_id;
        if ui.displayed_debugger_snapshot != self.debugger_snapshot
            || ui.displayed_debugger_error != debugger_error
        {
            if let Some(snapshot) = &self.debugger_snapshot {
                let reason = match snapshot.pause_reason {
                    Some(execution::DebugPauseReason::Step) => self.text("passo a passo"),
                    Some(execution::DebugPauseReason::Pause) => self.text("pausa"),
                    Some(execution::DebugPauseReason::Debug) => self.text("ponto de depuração"),
                    None => self.text("depuração"),
                };
                ui.debugger_position.set_text(&format!(
                    "{} {} · {reason}",
                    self.text("Linha"),
                    snapshot.line
                ));
                let variables = if snapshot.variables.is_empty() {
                    self.text("Nenhuma variável visível.").to_owned()
                } else {
                    snapshot
                        .variables
                        .iter()
                        .map(|(name, value)| format!("{name} = {value}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                ui.debugger_variables.set_text(&variables);
                if self.execution_document_id == self.active_document_id
                    && let Some(iter) = ui
                        .code_buffer
                        .iter_at_line(snapshot.line.saturating_sub(1) as i32)
                {
                    ui.code_buffer.place_cursor(&iter);
                }
            } else if debugger_error {
                ui.debugger_position
                    .set_text(self.text("Depuração encerrada com erro"));
                ui.debugger_variables.set_text(&self.problem_text);
            } else {
                ui.debugger_position
                    .set_text(self.text("Aguardando depuração"));
                ui.debugger_variables.set_text(
                    self.text(
                        "Os valores das variáveis aparecerão durante a execução passo a passo.",
                    ),
                );
            }
            ui.displayed_debugger_snapshot
                .clone_from(&self.debugger_snapshot);
            ui.displayed_debugger_error = debugger_error;
        }
        let current_debugger_hint = self
            .debugger_learning_hint
            .as_ref()
            .filter(|hint| {
                self.debugger_document_id == self.active_document_id
                    && self
                        .debugger_snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.line == hint.line)
            })
            .cloned();
        if ui.displayed_debugger_learning_hint != current_debugger_hint {
            ui.debugger_learning
                .content_scroll
                .vadjustment()
                .set_value(0.0);
            if let Some(hint) = &current_debugger_hint {
                let topic_name = learning_topic_label(hint.topic, self.ui_language);
                ui.debugger_learning.line_title.set_text(&format!(
                    "{} {} · {topic_name}",
                    self.text("Linha"),
                    hint.line
                ));
                let (explanation, concept, _) = learning_copy(hint, self.ui_language);
                ui.debugger_learning.explanation.set_text(&explanation);
                let show_learning_concept = hint.topic != LearningTopic::Unrecognized;
                ui.debugger_learning
                    .concept_section
                    .set_visible(show_learning_concept);
                ui.debugger_learning
                    .concept_title
                    .set_text(&format!("{} {topic_name}", self.text("Conceito:")));
                ui.debugger_learning.concept_text.set_text(&concept);
                content_panel::show_learning_problems(
                    &ui.debugger_learning.problem_list,
                    &ui.debugger_learning.problem_empty,
                    &hint.problems,
                    self.ui_language,
                    &ui.code_buffer,
                    ui.code_view.upcast_ref(),
                );
                ui.debugger_learning
                    .problem_empty
                    .set_text(self.text("Nenhum problema conhecido nesta linha."));
                ui.debugger_learning.problem_empty.set_text(self.text(
                    if hint.topic == LearningTopic::Unrecognized {
                        "A análise não reconheceu esta construção; não foi possível verificar problemas."
                    } else {
                        "Nenhum problema conhecido nesta linha."
                    },
                ));
            } else {
                ui.debugger_learning.concept_section.set_visible(false);
                ui.debugger_learning
                    .line_title
                    .set_text(self.text("Linha 1 · programa"));
                ui.debugger_learning.explanation.set_text("");
                ui.debugger_learning
                    .concept_title
                    .set_text(self.text("Conceito: programa"));
                ui.debugger_learning.concept_text.set_text("");
                content_panel::show_learning_problems(
                    &ui.debugger_learning.problem_list,
                    &ui.debugger_learning.problem_empty,
                    &[],
                    self.ui_language,
                    &ui.code_buffer,
                    ui.code_view.upcast_ref(),
                );
            }
            ui.displayed_debugger_learning_hint
                .clone_from(&current_debugger_hint);
        }
        let debugger_analysis_state = if !self.debugging_active {
            "idle"
        } else if self.debugger_snapshot.is_none()
            || self.debugger_document_id != self.active_document_id
        {
            "waiting"
        } else if current_debugger_hint.is_some() {
            "content"
        } else {
            "loading"
        };
        if ui.displayed_debugger_analysis_state != debugger_analysis_state {
            let loading = matches!(debugger_analysis_state, "waiting" | "loading");
            ui.debugger_learning.spinner.set_spinning(loading);
            ui.debugger_learning.spinner.set_visible(loading);
            if loading {
                ui.debugger_learning.loading_label.set_text(self.text(
                    if debugger_analysis_state == "waiting" {
                        "Executando o programa…"
                    } else {
                        "Analisando linha…"
                    },
                ));
            }
            let idle = debugger_analysis_state == "idle";
            ui.debugger_learning.analysis_stack.set_visible(!idle);
            if !idle {
                ui.debugger_learning.analysis_stack.set_visible_child_name(
                    if debugger_analysis_state == "content" {
                        "content"
                    } else {
                        "loading"
                    },
                );
            }
            ui.displayed_debugger_analysis_state = debugger_analysis_state;
        }
        let execution_active = self.active_execution_id.is_some();
        if ui.displayed_run_active != execution_active
            || ui.displayed_run_language != self.ui_language
        {
            if execution_active && !ui.displayed_run_active {
                ui.code_view.completion().hide();
            }
            let (icon_name, label) = if execution_active && !self.debugging_active {
                ("potoo-stop-symbolic", "Parar")
            } else {
                ("potoo-play-symbolic", "Executar")
            };
            if execution_active && !self.debugging_active {
                ui.run_button.add_css_class("potoo-stop-button");
            } else {
                ui.run_button.remove_css_class("potoo-stop-button");
            }
            ui.run_icon.set_icon_name(Some(icon_name));
            ui.run_label.set_text(self.text(label));
            ui.run_button.set_tooltip_text(Some(self.text(
                if execution_active && !self.debugging_active {
                    "Parar"
                } else {
                    "Executar o programa atual"
                },
            )));
            ui.displayed_run_active = execution_active;
            ui.displayed_run_language = self.ui_language;
        }
        ui.run_button.set_sensitive(!self.debugging_active);

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

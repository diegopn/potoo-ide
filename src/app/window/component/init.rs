use super::super::*;
use super::super::{analysis::*, persistence::*, view::*};

pub(super) fn init(
    _init: (),
    root: gtk::ApplicationWindow,
    sender: ComponentSender<MainWindow>,
) -> ComponentParts<MainWindow> {
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
        settings.connect_notify_local(Some("gtk-interface-color-scheme"), move |settings, _| {
            let dark = settings.property::<gtk::InterfaceColorScheme>("gtk-interface-color-scheme")
                == gtk::InterfaceColorScheme::Dark;
            sender_for_system_theme.input(MainWindowMsg::SystemThemeChanged(dark));
        });
    }
    let learning_worker = LearningAnalysisWorker::new(sender.clone());
    let diagnostics_worker = DiagnosticsAnalysisWorker::new(sender.clone());

    let mut model = MainWindow {
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

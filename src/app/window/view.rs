use super::component::MainUi;
use super::settings::*;
use super::*;

pub(super) fn input_cancel_controller(
    entry: &gtk::Entry,
    on_cancel: impl Fn() + 'static,
) -> gtk::EventControllerKey {
    let controller = gtk::EventControllerKey::new();
    let entry = entry.clone();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            entry.set_text("");
            on_cancel();
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    controller
}

pub(super) fn build_window_content(
    window: gtk::ApplicationWindow,
    initial_project_root: Option<PathBuf>,
    initial_start_page: bool,
    initial_recent_paths: &[PathBuf],
    project_files_for_new_items: Rc<RefCell<Vec<ProjectFile>>>,
    sender: ComponentSender<MainWindow>,
    preferences: Preferences,
    initial_dark: bool,
) -> MainUi {
    let ui_language = preferences.ui_language;
    let ui_language_state = Rc::new(Cell::new(ui_language));
    let close_authorized = Rc::new(Cell::new(false));
    let close_authorized_for_window = close_authorized.clone();
    let sender_for_close = sender.clone();
    window.connect_close_request(move |_| {
        if close_authorized_for_window.replace(false) {
            gtk::glib::Propagation::Proceed
        } else {
            sender_for_close.input(MainWindowMsg::RequestClose);
            gtk::glib::Propagation::Stop
        }
    });

    let header = header::build_header(&window, ui_language_state.clone());
    let project_title = header.project_title.clone();
    let run_button = header.run_button.clone();
    let run_icon = header.run_icon.clone();
    let run_label = header.run_label.clone();
    project_title.set_text(if initial_start_page {
        tr(ui_language, "Início")
    } else {
        tr(ui_language, "Meu primeiro programa")
    });
    run_button.set_visible(!initial_start_page);

    let start_page = start_page::build_start_page(ui_language);

    let dark_theme = Rc::new(Cell::new(initial_dark));
    let settings = build_settings_window(
        &window,
        sender.clone(),
        preferences,
        initial_dark,
        ui_language_state.clone(),
    );

    let file_sidebar = sidebar::build_file_sidebar(ui_language);
    let expanded_folders = file_sidebar.expanded_folders.clone();
    let sidebar_widget = file_sidebar.widget;
    let project_row = file_sidebar.project_row;
    let project_label = file_sidebar.project_label;
    let new_project_button = file_sidebar.new_project_button;
    let file_list = file_sidebar.file_list;
    let new_file_button = file_sidebar.new_file_button;
    let empty_state = file_sidebar.empty_state;
    let empty_open_project_button = file_sidebar.empty_open_project_button;
    let empty_open_file_button = file_sidebar.empty_open_file_button;
    let new_program_button = file_sidebar.new_program_button;
    let open_file_menu_button = file_sidebar.open_file_menu_button;
    let new_folder_button = file_sidebar.new_folder_button;
    let new_item_popover = file_sidebar.new_item_popover;
    let settings_button = file_sidebar.settings_button;
    let default_new_language = Rc::new(Cell::new(preferences.default_language));
    let project_root_for_new_items = Rc::new(RefCell::new(initial_project_root));

    let status_language = gtk::Label::new(Some("Portugol"));
    let status_details = gtk::Label::new(Some(&format!(
        "UTF-8  ·  {}: {}",
        tr(ui_language, "Espaços"),
        preferences.indent_width
    )));
    status_language.add_css_class("potoo-status-primary");
    status_details.add_css_class("potoo-muted");
    let status_position = gtk::Label::new(Some("Linha 1 · Coluna 1"));
    status_position.add_css_class("potoo-muted");
    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_bar.add_css_class("potoo-statusbar");
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_language);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_details);
    let status_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    status_spacer.set_hexpand(true);
    status_bar.append(&status_spacer);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_position);
    let workspace_host = gtk::Box::new(gtk::Orientation::Vertical, 0);
    workspace_host.set_hexpand(true);
    workspace_host.set_vexpand(true);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&workspace_host);
    content.append(&status_bar);

    let content_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content_root.add_css_class("potoo-root");
    if let Some(header_widget) = &header.widget {
        content_root.append(header_widget);
    }
    let main_stack = gtk::Stack::new();
    main_stack.add_css_class("potoo-main-stack");
    main_stack.set_hexpand(true);
    main_stack.set_vexpand(true);
    main_stack.add_named(&content, Some("workspace"));
    main_stack.add_named(&start_page.widget, Some("start"));
    main_stack.set_visible_child_name(if initial_start_page {
        "start"
    } else {
        "workspace"
    });
    content_root.append(&main_stack);
    if initial_dark {
        window.add_css_class("potoo-dark");
    }

    let sender_for_empty_open_project = sender.clone();
    let window_for_empty_open_project = window.clone();
    let language_for_empty_open_project = ui_language_state.clone();
    empty_open_project_button.connect_clicked(move |_| {
        show_open_project_dialog(
            &window_for_empty_open_project,
            language_for_empty_open_project.get(),
            sender_for_empty_open_project.clone(),
            None,
        );
    });

    let sender_for_empty_open_file = sender.clone();
    let window_for_empty_open_file = window.clone();
    let language_for_empty_open_file = ui_language_state.clone();
    empty_open_file_button.connect_clicked(move |_| {
        show_open_file_dialog(
            &window_for_empty_open_file,
            language_for_empty_open_file.get(),
            sender_for_empty_open_file.clone(),
        );
    });

    let sender_for_new_project = sender.clone();
    let window_for_new_project = window.clone();
    let default_new_language_for_project = default_new_language.clone();
    let dark_theme_for_project = dark_theme.clone();
    let language_for_new_project = ui_language_state.clone();
    new_project_button.connect_clicked(move |_| {
        show_new_project_dialog(
            &window_for_new_project,
            language_for_new_project.get(),
            ProjectTemplate::default_for_language(default_new_language_for_project.get()),
            true,
            dark_theme_for_project.get(),
            sender_for_new_project.clone(),
        );
    });

    let sender_for_start_portugol = sender.clone();
    let window_for_start_portugol = window.clone();
    let dark_for_start_portugol = dark_theme.clone();
    let language_for_start_portugol = ui_language_state.clone();
    start_page.portugol_button.connect_clicked(move |_| {
        show_new_project_dialog(
            &window_for_start_portugol,
            language_for_start_portugol.get(),
            ProjectTemplate::GPortugol,
            false,
            dark_for_start_portugol.get(),
            sender_for_start_portugol.clone(),
        );
    });

    let sender_for_start_potigol = sender.clone();
    let window_for_start_potigol = window.clone();
    let dark_for_start_potigol = dark_theme.clone();
    let language_for_start_potigol = ui_language_state.clone();
    start_page.potigol_button.connect_clicked(move |_| {
        show_new_project_dialog(
            &window_for_start_potigol,
            language_for_start_potigol.get(),
            ProjectTemplate::PotigolClassic,
            false,
            dark_for_start_potigol.get(),
            sender_for_start_potigol.clone(),
        );
    });

    let sender_for_start_calango = sender.clone();
    let window_for_start_calango = window.clone();
    let dark_for_start_calango = dark_theme.clone();
    let language_for_start_calango = ui_language_state.clone();
    start_page.calango_button.connect_clicked(move |_| {
        show_new_project_dialog(
            &window_for_start_calango,
            language_for_start_calango.get(),
            ProjectTemplate::Calango,
            false,
            dark_for_start_calango.get(),
            sender_for_start_calango.clone(),
        );
    });

    let sender_for_start_open = sender.clone();
    let window_for_start_open = window.clone();
    let language_for_start_open = ui_language_state.clone();
    start_page.open_folder_button.connect_clicked(move |_| {
        show_open_project_dialog(
            &window_for_start_open,
            language_for_start_open.get(),
            sender_for_start_open.clone(),
            None,
        );
    });

    let sender_for_start_open_file = sender.clone();
    let window_for_start_open_file = window.clone();
    let language_for_start_open_file = ui_language_state.clone();
    start_page.open_file_button.connect_clicked(move |_| {
        show_open_file_dialog(
            &window_for_start_open_file,
            language_for_start_open_file.get(),
            sender_for_start_open_file.clone(),
        );
    });

    let sender_for_new_file = sender.clone();
    let window_for_new_file = window.clone();
    let default_new_language_for_new_file = default_new_language.clone();
    let project_root_for_new_file = project_root_for_new_items.clone();
    let project_files_for_new_file = project_files_for_new_items.clone();
    let dark_theme_for_new_file = dark_theme.clone();
    let language_for_new_file = ui_language_state.clone();
    let popover_for_new_file = new_item_popover.clone();
    new_program_button.connect_clicked(move |_| {
        popover_for_new_file.popdown();
        let Some(project_root) = project_root_for_new_file.borrow().clone() else {
            return;
        };
        show_new_file_dialog(
            &window_for_new_file,
            default_new_language_for_new_file.get(),
            language_for_new_file.get(),
            project_directory_options(
                &project_root,
                language_for_new_file.get(),
                &project_files_for_new_file.borrow(),
            ),
            dark_theme_for_new_file.get(),
            sender_for_new_file.clone(),
        );
    });

    let sender_for_open_file_menu = sender.clone();
    let window_for_open_file_menu = window.clone();
    let language_for_open_file_menu = ui_language_state.clone();
    let popover_for_open_file_menu = new_item_popover.clone();
    open_file_menu_button.connect_clicked(move |_| {
        popover_for_open_file_menu.popdown();
        show_open_file_dialog(
            &window_for_open_file_menu,
            language_for_open_file_menu.get(),
            sender_for_open_file_menu.clone(),
        );
    });

    let sender_for_new_folder = sender.clone();
    let window_for_new_folder = window.clone();
    let project_root_for_new_folder = project_root_for_new_items.clone();
    let project_files_for_new_folder = project_files_for_new_items.clone();
    let dark_theme_for_new_folder = dark_theme.clone();
    let language_for_new_folder = ui_language_state.clone();
    let popover_for_new_folder = new_item_popover.clone();
    new_folder_button.connect_clicked(move |_| {
        popover_for_new_folder.popdown();
        let Some(project_root) = project_root_for_new_folder.borrow().clone() else {
            return;
        };
        show_new_folder_dialog(
            &window_for_new_folder,
            language_for_new_folder.get(),
            project_directory_options(
                &project_root,
                language_for_new_folder.get(),
                &project_files_for_new_folder.borrow(),
            ),
            dark_theme_for_new_folder.get(),
            sender_for_new_folder.clone(),
        );
    });

    let sender_for_settings = sender.clone();
    settings_button.connect_clicked(move |_| {
        sender_for_settings.input(MainWindowMsg::OpenSettings);
    });

    let key_controller = gtk::EventControllerKey::new();
    let sender_for_shortcuts = sender;
    let window_for_shortcuts = window.clone();
    let language_for_shortcuts = ui_language_state.clone();
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        let primary_modifier = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            || (cfg!(target_os = "macos") && modifiers.contains(gtk::gdk::ModifierType::META_MASK));
        if key == gtk::gdk::Key::s
            && primary_modifier
            && !modifiers.contains(gtk::gdk::ModifierType::SHIFT_MASK)
        {
            sender_for_shortcuts.input(MainWindowMsg::SaveCurrent);
            gtk::glib::Propagation::Stop
        } else if key == gtk::gdk::Key::o && primary_modifier {
            show_open_file_dialog(
                &window_for_shortcuts,
                language_for_shortcuts.get(),
                sender_for_shortcuts.clone(),
            );
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);

    MainUi {
        window,
        content: content_root,
        main_stack,
        dark_theme,
        default_new_language,
        project_root_for_new_items,
        project_files_for_new_items,
        settings,
        project_title,
        run_button,
        run_icon,
        run_label,
        start_page,
        project_row,
        project_label,
        file_list,
        new_file_button,
        empty_state,
        expanded_folders,
        sidebar_widget,
        workspace_host,
        workspace: None,
        status_language,
        status_details,
        status_position,
        displayed_language: Some(Language::Portugol),
        displayed_active_document_id: Some(0),
        displayed_project_name: String::new(),
        displayed_project_files: Vec::new(),
        displayed_project_open: false,
        displayed_recent_paths: initial_recent_paths.to_vec(),
        displayed_start_page: initial_start_page,
        displayed_run_active: false,
        displayed_run_language: ui_language,
        displayed_tabs: Vec::new(),
        displayed_pending_project: None,
        displayed_pending_close_document_id: None,
        displayed_pending_close_app: false,
        displayed_pending_save_as_id: None,
        displayed_close_app_ready: false,
        close_authorized,
        displayed_theme: preferences.theme,
        displayed_dark: initial_dark,
        displayed_font_size: preferences.font_size,
        displayed_line_numbers_visible: preferences.line_numbers_visible,
        displayed_word_wrap: preferences.word_wrap,
        displayed_indent_width: preferences.indent_width,
        displayed_preferred_ui_language: preferences.ui_language,
        displayed_settings_visible: false,
        displayed_output_text: tr(ui_language, "A saída aparecerá aqui após a execução.")
            .to_string(),
        displayed_diagnostics: Vec::new(),
        displayed_execution_diagnostics_document_id: None,
        displayed_navigation_serial: 0,
        displayed_input_requested: false,
        displayed_output_status: tr(ui_language, "Pronto para executar").to_string(),
        displayed_debugger_snapshot: None,
        displayed_debugger_error: false,
        displayed_execution_line: None,
        displayed_debugger_learning_hint: None,
        displayed_debugger_analysis_state: "idle",
        displayed_learning_hint: None,
        displayed_learning_loading: false,
        displayed_right_panel_section: None,
        displayed_right_panel_collapsed: true,
        displayed_game_preview_active: false,
    }
}

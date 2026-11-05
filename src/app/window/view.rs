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

    let source_buffer = sourceview5::Buffer::new(None::<&gtk::TextTagTable>);
    let code_buffer = source_buffer.clone().upcast::<gtk::TextBuffer>();
    code_buffer.set_text(Language::Portugol.sample());
    let syntax_tags = content_panel::make_syntax_tags(&code_buffer);
    content_panel::set_syntax_palette(&syntax_tags, initial_dark);
    content_panel::highlight_buffer(&code_buffer, Language::Portugol, &syntax_tags);

    let code_view = sourceview5::View::with_buffer(&source_buffer);
    code_view.set_show_line_numbers(preferences.line_numbers_visible);
    sourceview5::prelude::ViewExt::gutter(&code_view, gtk::TextWindowType::Left)
        .add_css_class("potoo-code-gutter");
    content_panel::install_diagnostic_markers(&code_view, initial_dark);
    code_view.set_monospace(true);
    code_view.set_wrap_mode(if preferences.word_wrap {
        gtk::WrapMode::WordChar
    } else {
        gtk::WrapMode::None
    });
    code_view.set_left_margin(12);
    code_view.set_right_margin(16);
    code_view.set_top_margin(14);
    code_view.set_bottom_margin(14);
    code_view.set_auto_indent(true);
    code_view.set_indent_width(preferences.indent_width as i32);
    code_view.set_tab_width(preferences.indent_width);
    code_view.set_insert_spaces_instead_of_tabs(true);
    code_view.set_indent_on_tab(true);
    code_view.add_css_class("potoo-code-editor");
    code_view.add_css_class(&format!("potoo-font-{}", preferences.font_size));
    let completion_provider = content_panel::CompletionProvider::new();
    completion_provider.set_ui_language(ui_language);
    let completion = code_view.completion();
    completion.set_page_size(8);
    completion.set_select_on_show(true);
    completion.set_show_icons(false);
    completion.set_remember_info_visibility(false);
    completion.add_provider(&completion_provider);

    let code_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&code_view)
        .build();

    let editor_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_row.add_css_class("potoo-editor-row");
    editor_row.append(&code_scroll);

    let editor_tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_tabs.set_hexpand(true);
    let save_tooltip = tr(
        ui_language,
        if cfg!(target_os = "macos") {
            "Salvar arquivo (⌘S)"
        } else {
            "Salvar arquivo (Ctrl+S)"
        },
    );
    let save_button = gtk::Button::new();
    let save_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    save_content.append(&icon("save"));
    save_content.append(&gtk::Label::new(Some(tr(ui_language, "Salvar"))));
    save_button.set_child(Some(&save_content));
    save_button.set_tooltip_text(Some(save_tooltip));
    save_button.add_css_class("potoo-save-button");

    let editor_actions = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_actions.add_css_class("potoo-editor-tab-actions");
    editor_actions.append(&save_button);
    let tab_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tab_bar.add_css_class("potoo-editor-tabbar");
    tab_bar.append(&editor_tabs);
    tab_bar.append(&editor_actions);

    let editor = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor.add_css_class("potoo-editor");
    editor.append(&tab_bar);
    editor.append(&editor_row);

    let output_ui: content_panel::OutputPanelUi =
        content_panel::build_output_panel(ui_language, sender.clone());
    let output_panel = output_ui.panel;
    let debugger_page = output_ui.debugger_page;
    let output_buffer = output_ui.output_buffer;
    let output_tab_output = output_ui.output_tab;
    let problems_tab = output_ui.problems_tab;
    let output_view = output_ui.output_view;
    let problems_list = output_ui.problems_list;
    let debugger_controls = output_ui.debugger_controls;
    let debugger_start = output_ui.debugger_start;
    let debugger_step = output_ui.debugger_step;
    let debugger_stop = output_ui.debugger_stop;
    let debugger_position = output_ui.debugger_position;
    let debugger_variables = output_ui.debugger_variables;
    let debugger_learning = output_ui.debugger_learning;
    let input_entry = gtk::Entry::new();
    input_entry.add_css_class("potoo-program-input");
    input_entry.set_placeholder_text(Some(tr(ui_language, "Digite um valor e pressione Enter")));
    input_entry.set_tooltip_text(Some(tr(
        ui_language,
        "Pressione Esc para cancelar a execução.",
    )));
    input_entry.set_margin_start(12);
    input_entry.set_margin_end(12);
    input_entry.set_margin_top(6);
    input_entry.set_margin_bottom(6);
    input_entry.set_visible(false);
    let output_status = gtk::Label::new(Some(tr(ui_language, "Pronto para executar")));
    output_status.set_xalign(0.0);
    output_status.add_css_class("potoo-muted");
    let output_status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    output_status_row.add_css_class("potoo-output-status-row");
    let output_ready_icon = icon("ready");
    output_ready_icon.set_margin_start(12);
    output_status.set_margin_end(12);
    output_status_row.append(&output_ready_icon);
    output_status_row.append(&output_status);

    let output_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
    output_area.add_css_class("potoo-output-area");
    output_area.append(&output_panel);
    output_area.append(&input_entry);
    output_area.append(&output_status_row);

    let editor_output = gtk::Paned::new(gtk::Orientation::Vertical);
    editor_output.set_wide_handle(false);
    editor_output.set_start_child(Some(&editor));
    editor_output.set_end_child(Some(&output_area));
    editor_output.set_resize_start_child(true);
    editor_output.set_resize_end_child(false);
    editor_output.set_shrink_start_child(true);
    editor_output.set_shrink_end_child(false);
    editor_output.set_position(570);
    let window_for_output = window.clone();
    editor_output.connect_map(move |paned| {
        let paned = paned.clone();
        let window = window_for_output.clone();
        gtk::glib::timeout_add_local(Duration::from_millis(32), move || {
            let height = paned.height();
            let window_height = window.height();
            if height <= 0 || window_height <= 0 || height < window_height.saturating_sub(150) {
                return gtk::glib::ControlFlow::Continue;
            }
            paned.set_position(height.saturating_sub(218).max(0));
            gtk::glib::ControlFlow::Break
        });
    });

    let learning_ui = content_panel::build_learning_panel(ui_language);
    let tools_ui = content_panel::build_tool_panel(
        learning_ui.panel.clone(),
        debugger_page,
        ui_language,
        sender.clone(),
    );
    let tools_panel = tools_ui.panel.clone();
    let editor_and_tools = gtk::Paned::new(gtk::Orientation::Horizontal);
    editor_and_tools.set_wide_handle(false);
    editor_and_tools.set_start_child(Some(&editor_output));
    editor_and_tools.set_end_child(Some(&tools_panel));
    editor_and_tools.set_resize_start_child(true);
    editor_and_tools.set_resize_end_child(false);
    editor_and_tools.set_shrink_start_child(false);
    editor_and_tools.set_shrink_end_child(false);
    editor_and_tools.set_position(820);
    editor_and_tools.connect_map(|paned| {
        let paned = paned.clone();
        gtk::glib::idle_add_local(move || {
            let width = paned.width();
            if width <= 0 {
                return gtk::glib::ControlFlow::Continue;
            }
            paned.set_position(width.saturating_sub(348).max(440));
            gtk::glib::ControlFlow::Break
        });
    });
    let paned_for_tools = editor_and_tools.clone();
    tools_ui
        .revealer
        .connect_child_revealed_notify(move |revealer| {
            let width = paned_for_tools.width();
            if width > 0 {
                let panel_width = if revealer.is_child_revealed() {
                    348
                } else {
                    42
                };
                paned_for_tools.set_position(width.saturating_sub(panel_width).max(440));
            }
        });

    let workspace = gtk::Paned::new(gtk::Orientation::Horizontal);
    workspace.set_wide_handle(false);
    workspace.set_start_child(Some(&sidebar_widget));
    workspace.set_end_child(Some(&editor_and_tools));
    workspace.set_resize_start_child(false);
    workspace.set_resize_end_child(true);
    workspace.set_shrink_start_child(false);
    workspace.set_shrink_end_child(false);
    workspace.set_position(242);
    workspace.set_vexpand(true);

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
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&workspace);
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

    let sender_for_save = sender.clone();
    save_button.connect_clicked(move |_| {
        sender_for_save.input(MainWindowMsg::SaveCurrent);
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

    let sender_for_run = sender.clone();
    let output_tab_for_run = output_tab_output.clone();
    run_button.connect_clicked(move |_| {
        output_tab_for_run.emit_clicked();
        sender_for_run.input(MainWindowMsg::Run);
    });

    let sender_for_program_input = sender.clone();
    input_entry.connect_activate(move |entry| {
        sender_for_program_input.input(MainWindowMsg::SubmitProgramInput(entry.text().to_string()));
        entry.set_text("");
    });

    let sender_for_character_input = sender.clone();
    input_entry.connect_changed(move |entry| {
        sender_for_character_input
            .input(MainWindowMsg::ProgramInputChanged(entry.text().to_string()));
    });

    let sender_for_input_cancel = sender.clone();
    input_entry.add_controller(input_cancel_controller(&input_entry, move || {
        sender_for_input_cancel.input(MainWindowMsg::StopDebug);
    }));

    let sender_for_source = sender.clone();
    code_buffer.connect_changed(move |buffer| {
        let contents = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        sender_for_source.input(MainWindowMsg::SourceChanged(contents.to_string()));
    });

    let sender_for_cursor = sender.clone();
    code_buffer.connect_mark_set(move |buffer, _, _| {
        let line = (buffer.iter_at_offset(buffer.cursor_position()).line() + 1) as usize;
        sender_for_cursor.input(MainWindowMsg::CursorMoved(line));
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
        editor_tabs,
        status_language,
        status_details,
        status_position,
        code_buffer,
        code_view,
        completion_provider,
        syntax_tags,
        output_buffer,
        output_view,
        problems_tab,
        problems_list,
        input_entry,
        output_status,
        debugger_start,
        debugger_controls,
        debugger_step,
        debugger_stop,
        debugger_position,
        debugger_variables,
        debugger_learning,
        right_panel_revealer: tools_ui.revealer,
        right_panel_stack: tools_ui.stack,
        learning_tool_button: tools_ui.learning_button,
        debugger_tool_button: tools_ui.debugger_button,
        game_preview_button: tools_ui.game_preview_button,
        game_preview_stack: tools_ui.game_preview_stack,
        game_preview_area: tools_ui.game_preview_area,
        game_preview_open_button: tools_ui.game_preview_open_button,
        learning_spinner: learning_ui.spinner,
        learning_analysis_stack: learning_ui.analysis_stack,
        learning_line_title: learning_ui.line_title,
        learning_explanation: learning_ui.explanation,
        learning_experiment: learning_ui.experiment,
        learning_concept_section: learning_ui.concept_section,
        learning_experiment_section: learning_ui.experiment_section,
        learning_concept_title: learning_ui.concept_title,
        learning_concept_text: learning_ui.concept_text,
        learning_problem_list: learning_ui.problem_list,
        learning_problem_empty: learning_ui.problem_empty,
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

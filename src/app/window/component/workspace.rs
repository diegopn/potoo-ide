use super::super::*;
use super::MainUi;

pub(in crate::app::window) struct WorkspaceUi {
    pub(super) root: gtk::Widget,
    pub(super) editor_tabs: gtk::Box,
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
    pub(super) tools_panel: gtk::Box,
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
}

pub(super) fn should_initialize_workspace(
    start_page_visible: bool,
    workspace_exists: bool,
) -> bool {
    !start_page_visible && !workspace_exists
}

fn initial_editor_source(contents: Option<&str>) -> &str {
    contents.unwrap_or_else(|| Language::Portugol.sample())
}

fn apply_editor_preferences(
    code_view: &sourceview5::View,
    font_size: u32,
    line_numbers_visible: bool,
    word_wrap: bool,
    indent_width: u32,
) {
    code_view.set_show_line_numbers(line_numbers_visible);
    code_view.set_wrap_mode(if word_wrap {
        gtk::WrapMode::WordChar
    } else {
        gtk::WrapMode::None
    });
    code_view.set_indent_width(indent_width as i32);
    code_view.set_tab_width(indent_width);
    code_view.add_css_class(&format!("potoo-font-{font_size}"));
}

pub(super) fn build_workspace_ui(
    model: &MainWindow,
    ui: &MainUi,
    sender: ComponentSender<MainWindow>,
) -> WorkspaceUi {
    let ui_language = model.ui_language;
    let active_document = model.active_document();
    let initial_language = active_document
        .map(|document| document.language)
        .unwrap_or(Language::Portugol);
    let source_buffer = sourceview5::Buffer::new(None::<&gtk::TextTagTable>);
    let code_buffer = source_buffer.clone().upcast::<gtk::TextBuffer>();
    code_buffer.set_text(initial_editor_source(
        active_document.map(|document| document.contents.as_str()),
    ));
    let syntax_tags = content_panel::make_syntax_tags(&code_buffer);
    content_panel::set_syntax_palette(&syntax_tags, model.dark);
    content_panel::highlight_buffer(&code_buffer, initial_language, &syntax_tags);

    let code_view = sourceview5::View::with_buffer(&source_buffer);
    apply_editor_preferences(
        &code_view,
        model.font_size,
        model.line_numbers_visible,
        model.word_wrap,
        model.indent_width,
    );
    sourceview5::prelude::ViewExt::gutter(&code_view, gtk::TextWindowType::Left)
        .add_css_class("potoo-code-gutter");
    content_panel::install_diagnostic_markers(&code_view, model.dark);
    code_view.set_monospace(true);
    code_view.set_left_margin(12);
    code_view.set_right_margin(16);
    code_view.set_top_margin(14);
    code_view.set_bottom_margin(14);
    code_view.set_auto_indent(true);
    code_view.set_insert_spaces_instead_of_tabs(true);
    code_view.set_indent_on_tab(true);
    code_view.add_css_class("potoo-code-editor");
    let completion_provider = content_panel::CompletionProvider::new();
    completion_provider.set_ui_language(ui_language);
    completion_provider.set_language(
        active_document
            .map(|document| document.language)
            .filter(|language| crate::languages::completion::is_available(*language)),
    );
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
    let window_for_output = ui.window.clone();
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
    workspace.set_start_child(Some(&ui.sidebar_widget));
    workspace.set_end_child(Some(&editor_and_tools));
    workspace.set_resize_start_child(false);
    workspace.set_resize_end_child(true);
    workspace.set_shrink_start_child(false);
    workspace.set_shrink_end_child(false);
    workspace.set_position(242);
    workspace.set_vexpand(true);

    let sender_for_save = sender.clone();
    save_button.connect_clicked(move |_| {
        sender_for_save.input(MainWindowMsg::SaveCurrent);
    });

    let sender_for_run = sender.clone();
    let output_tab_for_run = output_tab_output.clone();
    ui.run_button.connect_clicked(move |_| {
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
    input_entry.add_controller(super::super::view::input_cancel_controller(
        &input_entry,
        move || {
            sender_for_input_cancel.input(MainWindowMsg::StopDebug);
        },
    ));

    let sender_for_source = sender.clone();
    code_buffer.connect_changed(move |buffer| {
        let contents = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        sender_for_source.input(MainWindowMsg::SourceChanged(contents.to_string()));
    });

    let sender_for_cursor = sender;
    code_buffer.connect_mark_set(move |buffer, _, _| {
        let line = (buffer.iter_at_offset(buffer.cursor_position()).line() + 1) as usize;
        sender_for_cursor.input(MainWindowMsg::CursorMoved(line));
    });

    WorkspaceUi {
        root: workspace.upcast::<gtk::Widget>(),
        editor_tabs,
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
        tools_panel,
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
    }
}

pub(super) fn initialize_workspace_if_needed(
    model: &MainWindow,
    ui: &mut MainUi,
    sender: ComponentSender<MainWindow>,
) {
    if !should_initialize_workspace(model.start_page_visible, ui.workspace.is_some()) {
        return;
    }

    let workspace = build_workspace_ui(model, ui, sender);
    model
        .graphics_host
        .set_jerimum_preview_area(workspace.game_preview_area.clone());
    ui.workspace_host.append(&workspace.root);
    ui.workspace = Some(workspace);
    ui.displayed_active_document_id = model.active_document_id;
}

#[cfg(test)]
mod tests {
    use super::{apply_editor_preferences, initial_editor_source, should_initialize_workspace};
    use crate::languages::model::Language;

    #[test]
    fn workspace_creation_policy_defers_fresh_start() {
        assert!(!should_initialize_workspace(true, false));
        assert!(should_initialize_workspace(false, false));
    }

    #[test]
    fn workspace_creation_policy_builds_once_when_requested() {
        assert!(!should_initialize_workspace(false, true));
        assert!(!should_initialize_workspace(true, true));
    }

    #[test]
    fn empty_project_keeps_default_sample() {
        assert_eq!(initial_editor_source(None), Language::Portugol.sample());
    }

    #[test]
    fn opened_file_content_is_shown() {
        let source = "algoritmo\n  escreva(\"Olá\")\nfimalgoritmo";

        assert_eq!(initial_editor_source(Some(source)), source);
    }

    #[test]
    fn unclean_session_is_restored_on_first_workspace_render() {
        use crate::app::window::persistence::{SessionDocument, SessionSnapshot, restore_session};

        let source = "algoritmo recuperado\n  escreva(\"rascunho\")\nfimalgoritmo";
        let (documents, active_document_id, recovered_changes) = restore_session(SessionSnapshot {
            clean_exit: false,
            project_root: None,
            active_index: 0,
            documents: vec![SessionDocument {
                path: None,
                name: "recuperado.por".to_owned(),
                language: Language::Portugol,
                contents: source.to_owned(),
                saved_contents: "versão salva".to_owned(),
            }],
        });
        let active_document = documents
            .iter()
            .find(|document| Some(document.id) == active_document_id)
            .expect("the restored active document should exist");

        assert!(recovered_changes);
        assert_eq!(
            initial_editor_source(Some(&active_document.contents)),
            source
        );
    }

    #[test]
    #[ignore = "requires a GTK display; run with xvfb-run"]
    fn settings_changed_before_first_workspace_are_applied() {
        use gtk::prelude::*;

        gtk::init().expect("GTK should initialize under Xvfb");
        let buffer = sourceview5::Buffer::new(None::<&gtk::TextTagTable>);
        let view = sourceview5::View::with_buffer(&buffer);
        apply_editor_preferences(&view, 18, false, true, 2);

        assert_eq!(view.property::<i32>("indent-width"), 2);
        assert_eq!(view.property::<u32>("tab-width"), 2);
        assert!(!view.property::<bool>("show-line-numbers"));
        assert_eq!(view.wrap_mode(), gtk::WrapMode::WordChar);
        assert!(view.has_css_class("potoo-font-18"));
    }
}

mod diagnostics;
mod learning;
mod output;
mod syntax;

pub(crate) use diagnostics::{apply_diagnostics, install_diagnostic_markers, show_diagnostics};
#[cfg(test)]
pub(crate) use learning::localize_learning_problem;
pub(crate) use learning::{build_learning_panel, show_learning_problems};
pub(crate) use output::build_output_panel;
pub(crate) use syntax::{highlight_buffer, make_syntax_tags, set_syntax_palette};

use self::learning::{build_debugger_learning, connect_debugger_start};
use self::syntax::apply_tag_range;

use super::model::SourceDocument;
use crate::languages::model::Language;
use crate::{
    app::{MainWindow, MainWindowMsg, RightPanelSection, UiLanguage, tr},
    features::diagnostics::{DiagnosticCategory, DiagnosticSeverity, EditorDiagnostic},
    ui::helpers::{clear_box, icon},
};
use gtk::prelude::*;
use relm4::prelude::*;
use sourceview5::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct SyntaxTags {
    keyword: gtk::TextTag,
    string: gtk::TextTag,
    number: gtk::TextTag,
    comment: gtk::TextTag,
    current_line: gtk::TextTag,
    matching_delimiter: gtk::TextTag,
    diagnostic_error_line: gtk::TextTag,
    diagnostic_warning_line: gtk::TextTag,
    diagnostic_error_range: gtk::TextTag,
    diagnostic_warning_range: gtk::TextTag,
}

pub(crate) struct OutputPanelUi {
    pub(crate) panel: gtk::Box,
    pub(crate) debugger_page: gtk::Box,
    pub(crate) output_buffer: gtk::TextBuffer,
    pub(crate) output_tab: gtk::Button,
    pub(crate) problems_tab: gtk::Button,
    pub(crate) output_view: gtk::TextView,
    pub(crate) problems_list: gtk::Box,
    pub(crate) debugger_controls: gtk::Stack,
    pub(crate) debugger_start: gtk::Button,
    pub(crate) debugger_step: gtk::Button,
    pub(crate) debugger_stop: gtk::Button,
    pub(crate) debugger_position: gtk::Label,
    pub(crate) debugger_variables: gtk::TextBuffer,
    pub(crate) debugger_learning: DebuggerLearningUi,
}

pub(crate) struct DebuggerLearningUi {
    pub(crate) spinner: gtk::Spinner,
    pub(crate) loading_label: gtk::Label,
    pub(crate) analysis_stack: gtk::Stack,
    pub(crate) content_scroll: gtk::ScrolledWindow,
    pub(crate) line_title: gtk::Label,
    pub(crate) explanation: gtk::Label,
    pub(crate) concept_section: gtk::Box,
    pub(crate) concept_title: gtk::Label,
    pub(crate) concept_text: gtk::Label,
    pub(crate) experiment_section: gtk::Box,
    pub(crate) experiment: gtk::Label,
    pub(crate) problem_list: gtk::Box,
    pub(crate) problem_empty: gtk::Label,
}

pub(crate) struct LearningPanelUi {
    pub(crate) panel: gtk::Box,
    pub(crate) spinner: gtk::Spinner,
    pub(crate) analysis_stack: gtk::Stack,
    pub(crate) line_title: gtk::Label,
    pub(crate) explanation: gtk::Label,
    pub(crate) concept_section: gtk::Box,
    pub(crate) experiment: gtk::Label,
    pub(crate) experiment_section: gtk::Box,
    pub(crate) concept_title: gtk::Label,
    pub(crate) concept_text: gtk::Label,
    pub(crate) problem_list: gtk::Box,
    pub(crate) problem_empty: gtk::Label,
}

pub(crate) struct ToolPanelUi {
    pub(crate) panel: gtk::Box,
    pub(crate) revealer: gtk::Revealer,
    pub(crate) stack: gtk::Stack,
    pub(crate) learning_button: gtk::Button,
    pub(crate) debugger_button: gtk::Button,
    pub(crate) game_preview_button: gtk::Button,
    pub(crate) game_preview_stack: gtk::Stack,
    pub(crate) game_preview_area: gtk::DrawingArea,
    pub(crate) game_preview_open_button: gtk::Button,
}

pub(crate) fn rebuild_editor_tabs(
    editor_tabs: &gtk::Box,
    documents: &[SourceDocument],
    active_document_id: Option<u64>,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    clear_box(editor_tabs);
    for document in documents {
        let tab = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        tab.add_css_class("potoo-editor-tab-container");
        let button = gtk::Button::new();
        button.add_css_class("potoo-editor-tab");
        let is_active = Some(document.id) == active_document_id;
        if is_active {
            button.add_css_class("potoo-editor-tab-selected");
        }
        if document.is_dirty() {
            button.add_css_class("potoo-editor-tab-dirty");
        }
        button.set_tooltip_text(
            document
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .as_deref(),
        );
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        content.set_margin_start(12);
        content.set_margin_end(4);
        content.set_margin_top(8);
        content.set_margin_bottom(8);
        content.append(&icon("document"));
        let name = gtk::Label::new(Some(&document.name));
        name.set_max_width_chars(22);
        name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        content.append(&name);
        if document.is_dirty() {
            let modified = gtk::Label::new(Some("•"));
            modified.add_css_class("potoo-tab-modified");
            content.append(&modified);
        }
        button.set_child(Some(&content));
        button.set_tooltip_text(Some(&document.name));
        let id = document.id;
        let sender_for_tab = sender.clone();
        button.connect_clicked(move |_| {
            sender_for_tab.input(MainWindowMsg::ActivateDocument(id));
        });

        let close_button = gtk::Button::new();
        close_button.set_child(Some(&icon("close")));
        close_button.set_tooltip_text(Some(tr(language, "Fechar arquivo")));
        close_button.add_css_class("potoo-close-tab-button");
        if is_active {
            close_button.add_css_class("potoo-close-tab-button-selected");
        }
        let sender_for_close = sender.clone();
        close_button.connect_clicked(move |_| {
            sender_for_close.input(MainWindowMsg::CloseDocument(id));
        });
        tab.append(&button);
        tab.append(&close_button);
        editor_tabs.append(&tab);
    }
}

pub(crate) fn build_tool_panel(
    learning: gtk::Box,
    debugger: gtk::Box,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) -> ToolPanelUi {
    let panel = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    panel.add_css_class("potoo-tool-panel");
    panel.set_vexpand(true);

    let stack = gtk::Stack::new();
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(120);
    stack.add_named(&learning, Some("learning"));
    stack.add_named(&debugger, Some("debugger"));
    let preview_panel = gtk::Box::new(gtk::Orientation::Vertical, 12);
    preview_panel.add_css_class("potoo-game-preview-panel");
    preview_panel.set_vexpand(true);
    preview_panel.set_size_request(306, -1);
    let preview_title = gtk::Label::new(Some(tr(language, "Prévia do jogo")));
    preview_title.set_xalign(0.0);
    preview_title.add_css_class("potoo-learning-title");
    preview_title.set_margin_start(18);
    preview_title.set_margin_end(18);
    preview_title.set_margin_top(18);
    let game_preview_stack = gtk::Stack::new();
    game_preview_stack.set_vexpand(true);
    game_preview_stack.set_hexpand(true);
    let preview_empty = gtk::Label::new(Some(tr(
        language,
        "Execute um programa Potigol Jerimum para ver o jogo aqui.",
    )));
    preview_empty.set_wrap(true);
    preview_empty.set_xalign(0.0);
    preview_empty.set_yalign(0.0);
    preview_empty.set_hexpand(true);
    preview_empty.set_vexpand(true);
    preview_empty.set_halign(gtk::Align::Fill);
    preview_empty.set_valign(gtk::Align::Start);
    preview_empty.set_margin_start(18);
    preview_empty.set_margin_end(18);
    preview_empty.set_margin_top(18);
    preview_empty.set_margin_bottom(18);
    preview_empty.add_css_class("potoo-game-preview-empty");
    let game_preview_area = gtk::DrawingArea::new();
    game_preview_area.set_hexpand(true);
    game_preview_area.set_vexpand(true);
    game_preview_area.set_halign(gtk::Align::Fill);
    game_preview_area.set_valign(gtk::Align::Fill);
    game_preview_area.set_focusable(true);
    game_preview_area.add_css_class("potoo-game-preview-area");
    let game_preview_frame = gtk::AspectFrame::new(0.5, 0.0, 4.0 / 3.0, false);
    game_preview_frame.set_hexpand(true);
    game_preview_frame.set_vexpand(true);
    game_preview_frame.set_margin_start(18);
    game_preview_frame.set_margin_end(18);
    game_preview_frame.set_child(Some(&game_preview_area));
    game_preview_stack.add_named(&preview_empty, Some("idle"));
    game_preview_stack.add_named(&game_preview_frame, Some("canvas"));
    game_preview_stack.set_visible_child_name("idle");
    preview_panel.append(&preview_title);
    preview_panel.append(&game_preview_stack);
    let game_preview_open_button = gtk::Button::new();
    let open_window_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    open_window_content.set_halign(gtk::Align::Center);
    open_window_content.append(&icon("open-external"));
    open_window_content.append(&gtk::Label::new(Some(tr(
        language,
        "Abrir em janela separada",
    ))));
    game_preview_open_button.set_child(Some(&open_window_content));
    game_preview_open_button.set_tooltip_text(Some(tr(language, "Abrir em janela separada")));
    game_preview_open_button.add_css_class("potoo-debugger-start");
    game_preview_open_button.add_css_class("potoo-game-preview-open-window");
    game_preview_open_button.set_hexpand(true);
    game_preview_open_button.set_sensitive(false);
    let sender_for_game_window = sender.clone();
    game_preview_open_button.connect_clicked(move |_| {
        sender_for_game_window.input(MainWindowMsg::OpenGamePreviewWindow);
    });
    preview_panel.append(&game_preview_open_button);
    stack.add_named(&preview_panel, Some("game-preview"));
    stack.set_visible_child_name("learning");

    let revealer = gtk::Revealer::new();
    revealer.set_transition_type(gtk::RevealerTransitionType::SlideLeft);
    revealer.set_transition_duration(180);
    revealer.set_reveal_child(true);
    revealer.set_child(Some(&stack));
    panel.append(&revealer);

    let rail = gtk::Box::new(gtk::Orientation::Vertical, 4);
    rail.add_css_class("potoo-tool-rail");
    rail.set_size_request(40, -1);
    rail.set_valign(gtk::Align::Fill);

    let learning_button = gtk::Button::new();
    learning_button.set_child(Some(&icon("learning")));
    learning_button.set_tooltip_text(Some(tr(language, "Abrir Aprenda enquanto programa")));
    learning_button.add_css_class("potoo-tool-rail-button");
    learning_button.add_css_class("potoo-tool-rail-button-selected");
    let sender_for_learning = sender.clone();
    learning_button.connect_clicked(move |_| {
        sender_for_learning.input(MainWindowMsg::SelectRightPanel(RightPanelSection::Learning));
    });

    let debugger_button = gtk::Button::new();
    debugger_button.set_child(Some(&icon("debugger")));
    debugger_button.set_tooltip_text(Some(tr(language, "Abrir depurador")));
    debugger_button.add_css_class("potoo-tool-rail-button");
    let sender_for_debugger = sender.clone();
    debugger_button.connect_clicked(move |_| {
        sender_for_debugger.input(MainWindowMsg::SelectRightPanel(RightPanelSection::Debugger));
    });

    let game_preview_button = gtk::Button::new();
    game_preview_button.set_child(Some(&icon("game-preview")));
    game_preview_button.set_tooltip_text(Some(tr(language, "Abrir prévia do jogo")));
    game_preview_button.add_css_class("potoo-tool-rail-button");
    let sender_for_game_preview = sender.clone();
    game_preview_button.connect_clicked(move |_| {
        sender_for_game_preview.input(MainWindowMsg::SelectRightPanel(
            RightPanelSection::GamePreview,
        ));
    });

    rail.append(&learning_button);
    rail.append(&debugger_button);
    rail.append(&game_preview_button);
    panel.append(&rail);

    ToolPanelUi {
        panel,
        revealer,
        stack,
        learning_button,
        debugger_button,
        game_preview_button,
        game_preview_stack,
        game_preview_area,
        game_preview_open_button,
    }
}

#[cfg(test)]
#[path = "../../../../../tests/learning/problem_localization_tests.rs"]
mod problem_localization_tests;

#[cfg(test)]
mod debugger_ui_tests {
    use super::connect_debugger_start;
    use gtk::prelude::*;
    use std::{cell::Cell, rc::Rc};

    #[test]
    #[ignore = "requires a GTK display; run with xvfb-run"]
    fn starting_debugging_reveals_controls_before_execution_events() {
        gtk::init().expect("GTK should initialize under Xvfb");
        let stack = gtk::Stack::new();
        stack.add_named(&gtk::Label::new(Some("start")), Some("start"));
        stack.add_named(&gtk::Label::new(Some("controls")), Some("controls"));
        stack.set_visible_child_name("start");
        let start = gtk::Button::new();
        let sent = Rc::new(Cell::new(false));
        let sent_by_callback = sent.clone();

        connect_debugger_start(&start, &stack, move || sent_by_callback.set(true));
        start.emit_clicked();

        assert_eq!(stack.visible_child_name().as_deref(), Some("controls"));
        assert!(sent.get());
    }
}

#[cfg(test)]
mod gportugol_syntax_tests {
    use super::{highlight_buffer, make_syntax_tags};
    use crate::languages::model::Language;
    use gtk::prelude::*;

    fn has_tag_at(
        buffer: &gtk::TextBuffer,
        tag: &gtk::TextTag,
        source: &str,
        needle: &str,
    ) -> bool {
        let byte_offset = source.find(needle).expect("needle should exist in source");
        let char_offset = source[..byte_offset].chars().count();
        buffer.iter_at_offset(char_offset as i32).has_tag(tag)
    }

    #[test]
    #[ignore = "requires a GTK display; run with xvfb-run"]
    fn gportugol_highlights_keywords_and_comments() {
        gtk::init().expect("GTK should initialize under Xvfb");
        let source = "algoritmo exemplo;\ninício\n  imprima(\"oi\"); /* fim */ // comentário\n";
        let buffer = gtk::TextBuffer::new(None);
        buffer.set_text(source);
        let tags = make_syntax_tags(&buffer);
        highlight_buffer(&buffer, Language::GPortugol, &tags);

        let comments = buffer
            .tag_table()
            .lookup("comment")
            .expect("the comment syntax tag should be registered");
        assert!(has_tag_at(&buffer, &tags.keyword, source, "início"));
        assert!(has_tag_at(&buffer, &tags.keyword, source, "imprima"));
        assert!(has_tag_at(&buffer, &comments, source, "/*"));
        assert!(has_tag_at(&buffer, &comments, source, "//"));
        assert!(!has_tag_at(&buffer, &tags.keyword, source, "fim"));
    }
}

#[cfg(test)]
mod editor_diagnostic_presentation_tests {
    use super::{apply_diagnostics, install_diagnostic_markers, make_syntax_tags};
    use crate::{
        features::diagnostics::{DiagnosticCategory, DiagnosticSeverity, EditorDiagnostic},
        languages::model::Language,
    };
    use gtk::prelude::*;
    use sourceview5::prelude::*;

    #[test]
    #[ignore = "requires a GTK display; run with xvfb-run"]
    fn marks_the_exact_multibyte_span_and_clears_old_marks() {
        gtk::init().expect("GTK should initialize under Xvfb");
        let source_buffer = sourceview5::Buffer::new(None::<&gtk::TextTagTable>);
        let buffer = source_buffer.clone().upcast::<gtk::TextBuffer>();
        buffer.set_text("ação erro\n");
        let view = sourceview5::View::with_buffer(&source_buffer);
        let tags = make_syntax_tags(&buffer);
        install_diagnostic_markers(&view, false);
        let diagnostic = EditorDiagnostic {
            document_id: 1,
            source_name: "teste.gpt".to_owned(),
            source_path: None,
            language: Language::GPortugol,
            severity: DiagnosticSeverity::Error,
            category: DiagnosticCategory::Syntax,
            message: "erro de sintaxe".to_owned(),
            offset: 5,
            range: Some(5..9),
            line: 1,
            column: 6,
        };

        apply_diagnostics(&view, &buffer, &[diagnostic], &tags, false);
        assert!(
            buffer
                .iter_at_offset(6)
                .has_tag(&tags.diagnostic_error_range)
        );
        assert!(
            !buffer
                .iter_at_offset(4)
                .has_tag(&tags.diagnostic_error_range)
        );
        assert!(
            buffer
                .iter_at_offset(0)
                .has_tag(&tags.diagnostic_error_line)
        );
        assert_eq!(source_buffer.source_marks_at_line(0, None).len(), 1);

        apply_diagnostics(&view, &buffer, &[], &tags, false);
        assert!(
            !buffer
                .iter_at_offset(6)
                .has_tag(&tags.diagnostic_error_range)
        );
        assert!(
            !buffer
                .iter_at_offset(0)
                .has_tag(&tags.diagnostic_error_line)
        );
        assert!(source_buffer.source_marks_at_line(0, None).is_empty());
    }
}

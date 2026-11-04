use super::model::{Language, SourceDocument};
use crate::{
    app::{MainWindow, MainWindowMsg, RightPanelSection, UiLanguage, tr},
    components::{clear_box, icon},
    diagnostics::{DiagnosticCategory, DiagnosticSeverity, EditorDiagnostic},
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

pub(crate) fn build_learning_panel(language: UiLanguage) -> LearningPanelUi {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("potoo-learning-panel");
    panel.set_vexpand(true);
    panel.set_size_request(306, -1);
    let heading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    heading.set_margin_start(18);
    heading.set_margin_end(18);
    heading.set_margin_top(18);
    heading.set_margin_bottom(12);
    let title = gtk::Label::new(Some(tr(language, "Aprenda enquanto programa")));
    title.set_xalign(0.0);
    title.set_hexpand(true);
    title.add_css_class("potoo-learning-title");
    title.set_wrap(true);
    let spinner = gtk::Spinner::new();
    spinner.add_css_class("potoo-learning-spinner");
    spinner.set_visible(false);
    heading.append(&title);
    panel.append(&heading);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .build();
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.set_margin_top(6);
    content.set_margin_bottom(18);
    scroll.set_child(Some(&content));

    let line_title = gtk::Label::new(Some(tr(language, "Linha 1 · programa")));
    line_title.set_xalign(0.0);
    line_title.add_css_class("heading");
    line_title.add_css_class("potoo-learning-line-title");
    content.append(&line_title);

    let explanation = gtk::Label::new(Some(tr(
        language,
        "Selecione uma linha para ver uma explicação contextual do programa.",
    )));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.set_selectable(true);
    explanation.add_css_class("potoo-learning-description");
    content.append(&explanation);

    let concept_section = gtk::Box::new(gtk::Orientation::Vertical, 16);
    let concept = gtk::Box::new(gtk::Orientation::Vertical, 8);
    concept.add_css_class("potoo-card");
    concept.add_css_class("potoo-concept-card");
    concept.set_margin_start(4);
    concept.set_margin_end(4);
    concept.set_margin_top(8);
    concept.set_margin_bottom(8);
    let concept_title = gtk::Label::new(Some(tr(language, "Conceito: programa")));
    concept_title.set_xalign(0.0);
    concept_title.add_css_class("heading");
    let concept_text = gtk::Label::new(Some(tr(
        language,
        "O painel acompanha a linha selecionada e o conteúdo atual do editor.",
    )));
    concept_text.set_xalign(0.0);
    concept_text.set_wrap(true);
    concept_text.set_selectable(true);
    concept.append(&concept_title);
    concept.append(&concept_text);
    concept_section.append(&concept);
    concept_section.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&concept_section);

    let problems = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let problems_title = gtk::Label::new(Some(tr(language, "Problemas nesta linha")));
    problems_title.set_xalign(0.0);
    problems_title.add_css_class("heading");
    problems.append(&problems_title);

    let problem_list = gtk::Box::new(gtk::Orientation::Vertical, 8);
    problems.append(&problem_list);
    let problem_empty =
        gtk::Label::new(Some(tr(language, "Nenhum problema conhecido nesta linha.")));
    problem_empty.set_xalign(0.0);
    problem_empty.set_wrap(true);
    problem_empty.add_css_class("potoo-learning-problem-empty");
    problems.append(&problem_empty);
    content.append(&problems);

    let experiment_section = gtk::Box::new(gtk::Orientation::Vertical, 16);
    experiment_section.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let experiment_title = gtk::Label::new(Some(tr(language, "Experimente")));
    experiment_title.set_xalign(0.0);
    experiment_title.add_css_class("heading");
    experiment_section.append(&experiment_title);

    let experiment = gtk::Label::new(Some(tr(
        language,
        "Mova o cursor para uma condição, um laço, uma entrada ou uma saída.",
    )));
    experiment.set_xalign(0.0);
    experiment.set_wrap(true);
    experiment.set_selectable(true);
    experiment.add_css_class("potoo-card");
    experiment.add_css_class("potoo-experiment-card");
    experiment.set_margin_start(4);
    experiment.set_margin_end(4);
    experiment.set_margin_top(4);
    experiment.set_margin_bottom(4);
    experiment_section.append(&experiment);
    content.append(&experiment_section);

    let analysis_stack = gtk::Stack::new();
    analysis_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    analysis_stack.set_transition_duration(180);
    analysis_stack.set_vexpand(true);
    analysis_stack.add_named(&scroll, Some("content"));
    let loading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    loading.set_halign(gtk::Align::Center);
    loading.set_valign(gtk::Align::Center);
    let loading_message = gtk::Label::new(Some(tr(language, "Analisando a linha selecionada…")));
    loading_message.set_xalign(0.0);
    loading_message.set_wrap(true);
    loading_message.add_css_class("potoo-learning-loading");
    loading.append(&spinner);
    loading.append(&loading_message);
    analysis_stack.add_named(&loading, Some("loading"));
    panel.append(&analysis_stack);

    LearningPanelUi {
        panel,
        spinner,
        analysis_stack,
        line_title,
        explanation,
        experiment,
        concept_section,
        concept_title,
        concept_text,
        experiment_section,
        problem_list,
        problem_empty,
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

pub(crate) fn show_learning_problems(
    list: &gtk::Box,
    empty: &gtk::Label,
    problems: &[crate::learning::LearningProblem],
    language: UiLanguage,
    code_buffer: &gtk::TextBuffer,
    code_view: &gtk::TextView,
) {
    clear_box(list);
    empty.set_visible(problems.is_empty());
    for problem in problems {
        let (problem_message, problem_suggestion) = localize_learning_problem(problem, language);
        let card = gtk::Button::new();
        card.add_css_class("potoo-learning-problem-card");
        card.add_css_class(match problem.severity {
            crate::learning::LearningSeverity::Error => "potoo-learning-problem-error",
            crate::learning::LearningSeverity::Warning => "potoo-learning-problem-warning",
        });
        card.set_hexpand(true);
        card.set_focus_on_click(false);
        let contents = gtk::Box::new(gtk::Orientation::Vertical, 6);
        contents.set_halign(gtk::Align::Fill);
        let title = gtk::Label::new(Some(match problem.kind {
            crate::learning::LearningProblemKind::Syntax => tr(language, "Erro de sintaxe"),
            crate::learning::LearningProblemKind::Type => tr(language, "Erro de tipo"),
            crate::learning::LearningProblemKind::Runtime => {
                tr(language, "Erro durante a execução")
            }
            crate::learning::LearningProblemKind::Logic => tr(language, "Índice fora dos limites"),
            crate::learning::LearningProblemKind::PossibleLogic => {
                tr(language, "Possível problema lógico")
            }
        }));
        title.set_xalign(0.0);
        title.add_css_class("potoo-learning-problem-title");
        let location = gtk::Label::new(Some(&format!(
            "{} {} · {} {}",
            tr(language, "Linha"),
            problem.line,
            tr(language, "Coluna"),
            problem.column
        )));
        location.set_xalign(0.0);
        location.add_css_class("potoo-learning-problem-location");
        let message = gtk::Label::new(Some(&problem_message));
        message.set_xalign(0.0);
        message.set_wrap(true);
        message.set_selectable(true);
        message.add_css_class("potoo-learning-problem-message");
        contents.append(&title);
        contents.append(&location);
        contents.append(&message);
        if let Some(suggestion) = problem_suggestion.as_deref() {
            let suggestion = gtk::Label::new(Some(suggestion));
            suggestion.set_xalign(0.0);
            suggestion.set_wrap(true);
            suggestion.add_css_class("potoo-learning-problem-suggestion");
            contents.append(&suggestion);
        }
        card.set_child(Some(&contents));
        let code_buffer = code_buffer.clone();
        let code_view = code_view.clone();
        let line = problem.line.saturating_sub(1).min(i32::MAX as usize) as i32;
        let column = problem.column.saturating_sub(1).min(i32::MAX as usize) as i32;
        card.connect_clicked(move |_| {
            let mut iter = code_buffer
                .iter_at_line_offset(line, column)
                .unwrap_or_else(|| {
                    code_buffer
                        .iter_at_line(line)
                        .unwrap_or_else(|| code_buffer.start_iter())
                });
            code_buffer.place_cursor(&iter);
            code_view.scroll_to_iter(&mut iter, 0.15, false, 0.0, 0.0);
            code_view.grab_focus();
        });
        list.append(&card);
    }
}

fn localize_learning_problem(
    problem: &crate::learning::LearningProblem,
    language: UiLanguage,
) -> (String, Option<String>) {
    use crate::learning::{LearningProblemKind, LearningProblemText};

    match &problem.text {
        LearningProblemText::ConstantCondition { always_true } => (
            tr(
                language,
                if *always_true {
                    "A condição é sempre verdadeira; o ramo alternativo pode nunca ser executado."
                } else {
                    "A condição é sempre falsa; o bloco pode nunca ser executado."
                },
            )
            .to_owned(),
            Some(
                tr(
                    language,
                    "Confira se a condição deveria comparar uma variável.",
                )
                .to_owned(),
            ),
        ),
        LearningProblemText::ConstantIndex {
            index,
            name,
            lower_bound,
            upper_bound,
        } => {
            let message = tr(
                language,
                "O índice {value} está fora dos limites de `{name}` ({lower} a {upper}).",
            )
            .replace("{value}", &index.to_string())
            .replace("{name}", name)
            .replace("{lower}", &lower_bound.to_string())
            .replace("{upper}", &upper_bound.to_string());
            (
                message,
                Some(
                    tr(
                        language,
                        "Use um índice dentro da faixa declarada para esse vetor ou matriz.",
                    )
                    .to_owned(),
                ),
            )
        }
        LearningProblemText::InterpreterDiagnostic => {
            if language == UiLanguage::Portuguese {
                return (problem.message.clone(), problem.suggestion.clone());
            }
            if let Some(message) = translate_interpreter_diagnostic(&problem.message, language) {
                return (message, None);
            }

            let summary = match problem.kind {
                LearningProblemKind::Syntax => "Há um problema de sintaxe nesta linha.",
                LearningProblemKind::Type => {
                    "O interpretador encontrou uma incompatibilidade de tipos nesta linha."
                }
                LearningProblemKind::Runtime => "A execução encontrou um problema nesta linha.",
                LearningProblemKind::Logic => "Índice fora dos limites",
                LearningProblemKind::PossibleLogic => "Possível problema lógico",
            };
            let detail = tr(
                language,
                "Detalhe original do interpretador (português): {message}",
            )
            .replace("{message}", &problem.message);
            (tr(language, summary).to_owned(), Some(detail))
        }
    }
}

fn translate_interpreter_diagnostic(message: &str, language: UiLanguage) -> Option<String> {
    let translated = match message {
        "divisão por zero" => ("Division by zero", "División por cero"),
        "divisão inteira por zero" => ("Integer division by zero", "División entera por cero"),
        "resto de divisão por zero" => ("Remainder by zero", "Resto de una división por cero"),
        "estouro na operação inteira" => (
            "Integer operation overflow.",
            "Desbordamiento en una operación entera.",
        ),
        "estouro em operação com inteiros" => (
            "Integer operation overflow.",
            "Desbordamiento en una operación entera.",
        ),
        "estouro em divisão inteira" => (
            "Integer division overflow.",
            "Desbordamiento en una división entera.",
        ),
        "estouro ao negar inteiro" => (
            "Integer negation overflow.",
            "Desbordamiento al negar un entero.",
        ),
        "resultado real fora do limite" => (
            "Real result is out of range.",
            "El resultado real está fuera de rango.",
        ),
        "índice de vetor negativo" => (
            "Array index cannot be negative",
            "El índice no puede ser negativo",
        ),
        "número fora do limite de inteiro" => (
            "Number is outside the integer range",
            "El número está fuera del rango de enteros",
        ),
        "não é possível comparar estes valores" => (
            "These values cannot be compared",
            "No se pueden comparar estos valores",
        ),
        "índice fora dos limites" | "índice de vetor fora do limite" => (
            "Array index out of bounds",
            "Índice de vector fuera de los límites",
        ),
        "variável não declarada" => ("Variable is not declared", "La variable no está declarada"),
        "esperada uma expressão" => ("Expected an expression", "Se esperaba una expresión"),
        "esperado um identificador" => ("Expected an identifier", "Se esperaba un identificador"),
        "esperado o nome da função" => (
            "Expected the function name",
            "Se esperaba el nombre de la función",
        ),
        "esperado o nome do parâmetro" => (
            "Expected the parameter name",
            "Se esperaba el nombre del parámetro",
        ),
        "esperado o nome da variável" => (
            "Expected the variable name",
            "Se esperaba el nombre de la variable",
        ),
        "esperado o nome da biblioteca" => (
            "Expected the library name",
            "Se esperaba el nombre de la biblioteca",
        ),
        "esperado o nome do algoritmo entre aspas" => (
            "Expected the algorithm name in quotation marks",
            "Se esperaba el nombre del algoritmo entre comillas",
        ),
        "esperado o nome do arquivo entre aspas" => (
            "Expected the file name in quotation marks",
            "Se esperaba el nombre del archivo entre comillas",
        ),
        "esperado um apelido para a biblioteca" => (
            "Expected a library alias",
            "Se esperaba un alias para la biblioteca",
        ),
        "esperado o tipo da variável" => (
            "Expected the variable type",
            "Se esperaba el tipo de la variable",
        ),
        "tipo de variável desconhecido" => {
            ("Unknown variable type", "Tipo de variable desconocido")
        }
        "esperado um tipo: inteiro, real, caractere ou logico" => (
            "Expected a type: integer, real, character, or logical",
            "Se esperaba un tipo: entero, real, carácter o lógico",
        ),
        "tipo VisuAlg desconhecido" => ("Unknown VisuAlg type", "Tipo de VisuAlg desconocido"),
        "esperado um limite inteiro constante para o vetor" => (
            "Expected a constant integer bound for the array",
            "Se esperaba un límite entero constante para el vector",
        ),
        "o programa deve começar com 'programa'" => (
            "The program must start with 'programa'",
            "El programa debe comenzar con 'programa'",
        ),
        "esperado ':' ou bloco após o rótulo do caso" => (
            "Expected ':' or a block after the case label",
            "Se esperaba ':' o un bloque después de la etiqueta del caso",
        ),
        "esperado 'caso', 'outrocaso' ou 'fimescolha'" => (
            "Expected 'caso', 'outrocaso', or 'fimescolha'",
            "Se esperaba 'caso', 'outrocaso' o 'fimescolha'",
        ),
        "esperado 'var', 'arquivo', uma rotina ou 'inicio'" => (
            "Expected 'var', 'arquivo', a routine, or 'inicio'",
            "Se esperaba 'var', 'arquivo', una rutina o 'inicio'",
        ),
        "esperado o fim da linha" => (
            "Expected the end of the line",
            "Se esperaba el fin de la línea",
        ),
        "esperada uma nova linha ou ';' entre instruções" => (
            "Expected a new line or ';' between statements",
            "Se esperaba una nueva línea o ';' entre instrucciones",
        ),
        "há código depois do fim do programa" => (
            "There is code after the end of the program",
            "Hay código después del final del programa",
        ),
        "programa excede o limite de blocos aninhados" => (
            "The program exceeds the nested block limit",
            "El programa supera el límite de bloques anidados",
        ),
        "expressão excede o limite de aninhamento" => (
            "The expression exceeds the nesting limit",
            "La expresión supera el límite de anidamiento",
        ),
        "palavra reservada não pode ser usada como identificador" => (
            "A reserved word cannot be used as an identifier",
            "No se puede usar una palabra reservada como identificador",
        ),
        "vetores podem ter no máximo duas dimensões" => (
            "Arrays can have at most two dimensions",
            "Los vectores pueden tener como máximo dos dimensiones",
        ),
        "a função precisa declarar o tipo do resultado" => (
            "The function must declare its result type",
            "La función debe declarar el tipo de resultado",
        ),
        "um procedimento não declara tipo de retorno" => (
            "A procedure does not declare a return type",
            "Un procedimiento no declara tipo de retorno",
        ),
        "o valor não é um vetor" => ("The value is not an array", "El valor no es un vector"),
        "esperada uma matriz" => ("Expected a matrix", "Se esperaba una matriz"),
        "texto não terminado" => ("Unterminated string", "Cadena sin terminar"),
        "texto não terminado antes do fim da linha" => (
            "String not terminated before the end of the line",
            "Cadena sin terminar antes del final de la línea",
        ),
        "caractere não terminado" => ("Unterminated character literal", "Carácter sin terminar"),
        "literal de caracter deve conter um caractere" => (
            "A character literal must contain one character",
            "Un literal de carácter debe contener un carácter",
        ),
        "sequência de escape incompleta" => (
            "Incomplete escape sequence",
            "Secuencia de escape incompleta",
        ),
        "a declaração 'arquivo' só pode aparecer uma vez" => (
            "The 'arquivo' declaration can appear only once",
            "La declaración 'arquivo' solo puede aparecer una vez",
        ),
        "'outrocaso' deve ser o último bloco de 'escolha'" => (
            "'outrocaso' must be the last block in 'escolha'",
            "'outrocaso' debe ser el último bloque de 'escolha'",
        ),
        _ => {
            if let Some(name) = message
                .strip_prefix("variável '")
                .and_then(|rest| rest.strip_suffix("' não declarada"))
            {
                return Some(match language {
                    UiLanguage::English => format!("Variable `{name}` is not declared"),
                    UiLanguage::Spanish => format!("La variable `{name}` no está declarada"),
                    UiLanguage::Portuguese => message.to_owned(),
                });
            }
            if let Some(translated) = translate_expected_token(message, language) {
                return Some(translated);
            }
            return None;
        }
    };
    Some(match language {
        UiLanguage::English => translated.0.to_owned(),
        UiLanguage::Spanish => translated.1.to_owned(),
        UiLanguage::Portuguese => message.to_owned(),
    })
}

fn translate_expected_token(message: &str, language: UiLanguage) -> Option<String> {
    let expected = message.strip_prefix("esperado '")?;
    let (token, context) = expected.split_once('\'')?;
    let context_translation = match context {
        "" => ("", ""),
        " após 'programa'" => (" after 'programa'", " después de 'programa'"),
        " após o nome da função" => (
            " after the function name",
            " después del nombre de la función",
        ),
        " após a dimensão do parâmetro" => (
            " after the parameter dimension",
            " después de la dimensión del parámetro",
        ),
        " após os parâmetros" => (" after the parameters", " después de los parámetros"),
        " após 'inclua'" => (" after 'inclua'", " después de 'inclua'"),
        " após o bloco 'faca'" => (" after the 'faca' block", " después del bloque 'faca'"),
        " após 'para'" => (" after 'para'", " después de 'para'"),
        " após o início do 'para'" => (
            " after the start of 'para'",
            " después del inicio de 'para'",
        ),
        " após a condição do 'para'" => (
            " after the 'para' condition",
            " después de la condición de 'para'",
        ),
        " após o cabeçalho do 'para'" => (
            " after the 'para' header",
            " después del encabezado de 'para'",
        ),
        " após a expressão de 'escolha'" => (
            " after the 'escolha' expression",
            " después de la expresión de 'escolha'",
        ),
        " após o tamanho do vetor" => (" after the array size", " después del tamaño del vector"),
        " após a condição" => (" after the condition", " después de la condición"),
        " após os argumentos" => (" after the arguments", " después de los argumentos"),
        " após o índice" => (" after the index", " después del índice"),
        " após a expressão" => (" after the expression", " después de la expresión"),
        " após os valores" => (" after the values", " después de los valores"),
        " antes do fim do arquivo" => (
            " before the end of the file",
            " antes del final del archivo",
        ),
        " antes da lista de argumentos" => (
            " before the argument list",
            " antes de la lista de argumentos",
        ),
        " para fechar o programa" => (" to close the program", " para cerrar el programa"),
        " para fechar o bloco" => (" to close the block", " para cerrar el bloque"),
        " para fechar 'escolha'" => (" to close 'escolha'", " para cerrar 'escolha'"),
        " para fechar o caso" => (" to close the case", " para cerrar el caso"),
        _ => return None,
    };
    let (context, punctuation) = match language {
        UiLanguage::English => (context_translation.0, "."),
        UiLanguage::Spanish => (context_translation.1, "."),
        UiLanguage::Portuguese => return Some(message.to_owned()),
    };
    let subject = match language {
        UiLanguage::English => "Expected",
        UiLanguage::Spanish => "Se esperaba",
        UiLanguage::Portuguese => unreachable!(),
    };
    Some(format!("{subject} '{token}'{context}{punctuation}"))
}

pub(crate) fn build_output_panel(
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) -> OutputPanelUi {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("potoo-output-panel");
    panel.set_vexpand(true);

    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    tabs.add_css_class("potoo-output-tabs");
    let output_tab = gtk::Button::with_label(tr(language, "Saída"));
    let problems_tab = gtk::Button::with_label(tr(language, "Problemas"));
    let tab_buttons = [output_tab.clone(), problems_tab.clone()];
    output_tab.add_css_class("potoo-output-tab-selected");
    for button in &tab_buttons {
        button.add_css_class("potoo-output-tab");
        tabs.append(button);
    }

    let stack = gtk::Stack::new();
    stack.add_css_class("potoo-output-stack");
    stack.set_vexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::None);

    let output = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(14)
        .right_margin(14)
        .top_margin(12)
        .bottom_margin(12)
        .build();
    output.add_css_class("potoo-output-view");
    output.add_css_class("potoo-output-placeholder-text");
    let output_buffer = output.buffer();
    output_buffer.set_text(tr(language, "A saída aparecerá aqui após a execução."));
    let output_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&output)
        .build();
    stack.add_named(&output_scroll, Some("output"));

    let problems = gtk::Box::new(gtk::Orientation::Vertical, 6);
    problems.add_css_class("potoo-problems-list");
    problems.set_margin_start(12);
    problems.set_margin_end(12);
    problems.set_margin_top(10);
    problems.set_margin_bottom(12);
    let empty_problems = gtk::Label::new(Some(tr(language, "Nenhum problema detectado")));
    empty_problems.set_xalign(0.0);
    empty_problems.add_css_class("potoo-problems-empty");
    problems.append(&empty_problems);
    let problems_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&problems)
        .build();
    stack.add_named(&problems_scroll, Some("problems"));

    let debugger = gtk::Box::new(gtk::Orientation::Vertical, 8);
    debugger.add_css_class("potoo-debugger-page");
    debugger.set_size_request(278, -1);
    debugger.set_vexpand(true);
    debugger.set_margin_start(14);
    debugger.set_margin_end(14);
    debugger.set_margin_top(12);
    debugger.set_margin_bottom(12);
    let debugger_controls = gtk::Stack::new();
    debugger_controls.set_transition_type(gtk::StackTransitionType::Crossfade);
    debugger_controls.set_transition_duration(200);
    debugger_controls.set_interpolate_size(true);
    debugger_controls.set_hexpand(true);
    let debugger_start = gtk::Button::new();
    let start_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    start_content.set_halign(gtk::Align::Center);
    start_content.append(&icon("debugger"));
    start_content.append(&gtk::Label::new(Some(tr(language, "Iniciar depuração"))));
    debugger_start.set_child(Some(&start_content));
    debugger_start.set_tooltip_text(Some(tr(language, "Iniciar depuração")));
    debugger_start.add_css_class("potoo-debugger-start");
    debugger_start.set_hexpand(true);

    let controls = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    controls.set_halign(gtk::Align::Fill);
    controls.set_hexpand(true);
    controls.set_homogeneous(true);
    let debugger_step = gtk::Button::new();
    debugger_step.set_child(Some(&icon("step")));
    debugger_step.set_tooltip_text(Some(tr(language, "Avançar um passo")));
    debugger_step.update_property(&[gtk::accessible::Property::Label(tr(
        language,
        "Avançar um passo",
    ))]);
    let debugger_stop = gtk::Button::new();
    let stop_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    stop_content.set_halign(gtk::Align::Center);
    stop_content.append(&icon("stop"));
    stop_content.append(&gtk::Label::new(Some(tr(language, "Parar depuração"))));
    debugger_stop.set_child(Some(&stop_content));
    debugger_stop.set_tooltip_text(Some(tr(language, "Parar depuração")));
    debugger_stop.update_property(&[gtk::accessible::Property::Label(tr(
        language,
        "Parar depuração",
    ))]);
    debugger_stop.add_css_class("potoo-debugger-stop");
    debugger_stop.set_hexpand(true);
    debugger_stop.set_visible(false);
    debugger_step.add_css_class("potoo-debugger-action");
    debugger_step.add_css_class("potoo-debugger-step");
    debugger_step.set_hexpand(true);
    controls.append(&debugger_step);
    let sender_for_start = sender.clone();
    connect_debugger_start(&debugger_start, &debugger_controls, move || {
        sender_for_start.input(MainWindowMsg::StartDebug);
    });
    let sender_for_step = sender.clone();
    debugger_step.connect_clicked(move |_| sender_for_step.input(MainWindowMsg::StepDebug));
    debugger_stop.connect_clicked(move |_| sender.input(MainWindowMsg::StopDebug));
    debugger_controls.add_named(&debugger_start, Some("start"));
    debugger_controls.add_named(&controls, Some("controls"));
    debugger_controls.set_visible_child_name("start");
    debugger.append(&debugger_controls);
    let debugger_position = gtk::Label::new(Some(tr(language, "Aguardando depuração")));
    debugger_position.set_xalign(0.0);
    debugger_position.set_margin_top(16);
    debugger_position.add_css_class("potoo-debugger-position");
    debugger.append(&debugger_position);
    let debugger_variables = gtk::TextBuffer::new(None);
    debugger_variables.set_text(tr(
        language,
        "Os valores das variáveis aparecerão durante a execução passo a passo.",
    ));
    let variables_view = gtk::TextView::builder()
        .buffer(&debugger_variables)
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(8)
        .right_margin(8)
        .top_margin(6)
        .bottom_margin(6)
        .build();
    variables_view.add_css_class("potoo-debugger-variables");
    let variables_scroll = gtk::ScrolledWindow::builder()
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&variables_view)
        .build();
    variables_scroll.add_css_class("potoo-debugger-output");
    variables_scroll.set_min_content_height(96);
    variables_scroll.set_max_content_height(96);
    variables_scroll.set_propagate_natural_height(false);
    variables_scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    variables_scroll.set_vscrollbar_policy(gtk::PolicyType::Automatic);
    variables_scroll.set_margin_top(8);
    variables_scroll.set_vexpand(false);
    debugger.append(&variables_scroll);
    let debugger_learning = build_debugger_learning(language);
    debugger.append(&debugger_learning.analysis_stack);
    debugger.append(&debugger_stop);
    stack.set_visible_child_name("output");

    for (button, child_name) in tab_buttons.iter().zip(["output", "problems"]) {
        let button_for_switch = button.clone();
        let stack_for_switch = stack.clone();
        let buttons_for_selection = tab_buttons.clone();
        button.connect_clicked(move |_| {
            for candidate in &buttons_for_selection {
                candidate.remove_css_class("potoo-output-tab-selected");
            }
            button_for_switch.add_css_class("potoo-output-tab-selected");
            stack_for_switch.set_visible_child_name(child_name);
        });
    }

    panel.append(&tabs);
    panel.append(&stack);
    OutputPanelUi {
        panel,
        debugger_page: debugger,
        output_buffer,
        output_tab,
        output_view: output,
        problems_list: problems,
        debugger_controls,
        debugger_start,
        debugger_step,
        debugger_stop,
        debugger_position,
        debugger_variables,
        debugger_learning,
    }
}

pub(crate) fn show_diagnostics(
    container: &gtk::Box,
    diagnostics: &[EditorDiagnostic],
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    clear_box(container);
    if diagnostics.is_empty() {
        let empty = gtk::Label::new(Some(tr(language, "Nenhum problema detectado")));
        empty.set_xalign(0.0);
        empty.add_css_class("potoo-problems-empty");
        container.append(&empty);
        return;
    }

    for diagnostic in diagnostics {
        let severity = match diagnostic.severity {
            DiagnosticSeverity::Error => ("Erro", "diagnostic-error", "potoo-problem-error"),
            DiagnosticSeverity::Warning => ("Aviso", "diagnostic-warning", "potoo-problem-warning"),
            DiagnosticSeverity::Information => (
                "Informação",
                "diagnostic-information",
                "potoo-problem-information",
            ),
            DiagnosticSeverity::Hint => ("Dica", "diagnostic-hint", "potoo-problem-hint"),
        };
        let category = match diagnostic.category {
            DiagnosticCategory::Lexical => tr(language, "Erro léxico"),
            DiagnosticCategory::Syntax => tr(language, "Erro de sintaxe"),
            DiagnosticCategory::Type => tr(language, "Erro de tipo"),
            DiagnosticCategory::Runtime => tr(language, "Erro de execução"),
            DiagnosticCategory::StaticAnalysis => tr(language, "Análise estática"),
        };
        let file = diagnostic
            .source_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| diagnostic.source_name.clone());

        let button = gtk::Button::new();
        button.set_halign(gtk::Align::Fill);
        button.set_hexpand(true);
        button.add_css_class("potoo-problem-row");
        button.add_css_class(severity.2);
        button.set_tooltip_text(Some(&diagnostic.message));

        let content = gtk::Box::new(gtk::Orientation::Vertical, 5);
        content.set_margin_start(8);
        content.set_margin_end(8);
        content.set_margin_top(7);
        content.set_margin_bottom(7);
        let heading = gtk::Box::new(gtk::Orientation::Horizontal, 7);
        heading.set_hexpand(true);
        let severity_icon = icon(severity.1);
        severity_icon.set_pixel_size(16);
        heading.append(&severity_icon);
        let severity_label = gtk::Label::new(Some(tr(language, severity.0)));
        severity_label.add_css_class("potoo-problem-severity");
        heading.append(&severity_label);
        let location = gtk::Label::new(Some(&format!(
            "{file} · {}:{} · {category} · {}",
            diagnostic.line,
            diagnostic.column,
            diagnostic.language.label()
        )));
        location.set_xalign(0.0);
        location.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        location.set_hexpand(true);
        location.add_css_class("potoo-problem-location");
        heading.append(&location);
        content.append(&heading);

        let message = gtk::Label::new(Some(&diagnostic.message));
        message.set_xalign(0.0);
        message.set_wrap(true);
        message.set_selectable(true);
        message.add_css_class("potoo-problem-message");
        content.append(&message);
        button.set_child(Some(&content));

        let sender = sender.clone();
        let diagnostic = diagnostic.clone();
        button.connect_clicked(move |_| {
            sender.input(MainWindowMsg::NavigateToDiagnostic(diagnostic.clone()));
        });
        container.append(&button);
    }
}

fn build_debugger_learning(language: UiLanguage) -> DebuggerLearningUi {
    let analysis_stack = gtk::Stack::new();
    analysis_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    analysis_stack.set_transition_duration(180);
    analysis_stack.set_vexpand(true);
    analysis_stack.add_css_class("potoo-debugger-analysis");

    let spinner = gtk::Spinner::new();
    spinner.add_css_class("potoo-learning-spinner");
    let loading = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    loading.set_halign(gtk::Align::Center);
    loading.set_valign(gtk::Align::Start);
    loading.set_margin_top(16);
    loading.append(&spinner);
    let loading_label = gtk::Label::new(Some(tr(language, "Analisando linha…")));
    loading.append(&loading_label);
    analysis_stack.add_named(&loading, Some("loading"));

    let content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    content.set_margin_start(2);
    content.set_margin_end(2);
    content.set_margin_top(8);
    content.set_margin_bottom(12);

    let line_title = gtk::Label::new(Some(tr(language, "Linha 1 · programa")));
    line_title.set_xalign(0.0);
    line_title.add_css_class("heading");
    line_title.add_css_class("potoo-learning-line-title");
    content.append(&line_title);

    let explanation = gtk::Label::new(Some(""));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.set_selectable(true);
    explanation.add_css_class("potoo-learning-description");
    content.append(&explanation);

    let concept_section = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let concept = gtk::Box::new(gtk::Orientation::Vertical, 8);
    concept.add_css_class("potoo-card");
    concept.add_css_class("potoo-concept-card");
    concept.set_margin_start(2);
    concept.set_margin_end(2);
    concept.set_margin_top(4);
    concept.set_margin_bottom(4);
    let concept_title = gtk::Label::new(Some(tr(language, "Conceito: programa")));
    concept_title.set_xalign(0.0);
    concept_title.add_css_class("heading");
    let concept_text = gtk::Label::new(Some(""));
    concept_text.set_xalign(0.0);
    concept_text.set_wrap(true);
    concept_text.set_selectable(true);
    concept.append(&concept_title);
    concept.append(&concept_text);
    concept_section.append(&concept);
    concept_section.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&concept_section);

    let problems = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let problems_title = gtk::Label::new(Some(tr(language, "Problemas nesta linha")));
    problems_title.set_xalign(0.0);
    problems_title.add_css_class("heading");
    problems.append(&problems_title);
    let problem_list = gtk::Box::new(gtk::Orientation::Vertical, 8);
    problems.append(&problem_list);
    let problem_empty =
        gtk::Label::new(Some(tr(language, "Nenhum problema conhecido nesta linha.")));
    problem_empty.set_xalign(0.0);
    problem_empty.set_wrap(true);
    problem_empty.add_css_class("potoo-learning-problem-empty");
    problems.append(&problem_empty);
    content.append(&problems);

    let experiment_section = gtk::Box::new(gtk::Orientation::Vertical, 10);
    experiment_section.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    let experiment_title = gtk::Label::new(Some(tr(language, "Experimente")));
    experiment_title.set_xalign(0.0);
    experiment_title.add_css_class("heading");
    experiment_section.append(&experiment_title);
    let experiment = gtk::Label::new(Some(""));
    experiment.set_xalign(0.0);
    experiment.set_wrap(true);
    experiment.set_selectable(true);
    experiment.add_css_class("potoo-card");
    experiment.add_css_class("potoo-experiment-card");
    experiment.set_margin_start(2);
    experiment.set_margin_end(2);
    experiment.set_margin_top(4);
    experiment.set_margin_bottom(4);
    experiment_section.append(&experiment);
    content.append(&experiment_section);

    let content_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .vexpand(true)
        .child(&content)
        .build();
    content_scroll.set_propagate_natural_height(false);
    analysis_stack.add_named(&content_scroll, Some("content"));
    analysis_stack.set_visible_child_name("loading");
    analysis_stack.set_visible(false);

    DebuggerLearningUi {
        spinner,
        loading_label,
        analysis_stack,
        content_scroll,
        line_title,
        explanation,
        concept_section,
        concept_title,
        concept_text,
        experiment_section,
        experiment,
        problem_list,
        problem_empty,
    }
}

fn connect_debugger_start(
    button: &gtk::Button,
    controls: &gtk::Stack,
    on_start: impl Fn() + 'static,
) {
    let controls = controls.clone();
    button.connect_clicked(move |_| {
        controls.set_visible_child_name("controls");
        on_start();
    });
}

pub(crate) fn make_syntax_tags(buffer: &gtk::TextBuffer) -> SyntaxTags {
    let table = buffer.tag_table();
    let keyword = gtk::TextTag::builder().name("keyword").weight(700).build();
    let string = gtk::TextTag::builder().name("string").build();
    let number = gtk::TextTag::builder().name("number").build();
    let comment = gtk::TextTag::builder().name("comment").build();
    let current_line = gtk::TextTag::builder().name("current-line").build();
    let matching_delimiter = gtk::TextTag::builder()
        .name("matching-delimiter")
        .weight(700)
        .build();
    let diagnostic_error_line = gtk::TextTag::builder()
        .name("diagnostic-error-line")
        .build();
    let diagnostic_warning_line = gtk::TextTag::builder()
        .name("diagnostic-warning-line")
        .build();
    let diagnostic_error_range = gtk::TextTag::builder()
        .name("diagnostic-error-range")
        .build();
    diagnostic_error_range.set_property("underline", gtk::pango::Underline::Error);
    let diagnostic_warning_range = gtk::TextTag::builder()
        .name("diagnostic-warning-range")
        .build();
    diagnostic_warning_range.set_property("underline", gtk::pango::Underline::Error);
    table.add(&keyword);
    table.add(&string);
    table.add(&number);
    table.add(&comment);
    table.add(&current_line);
    table.add(&matching_delimiter);
    table.add(&diagnostic_error_line);
    table.add(&diagnostic_warning_line);
    table.add(&diagnostic_error_range);
    table.add(&diagnostic_warning_range);
    SyntaxTags {
        keyword,
        string,
        number,
        comment,
        current_line,
        matching_delimiter,
        diagnostic_error_line,
        diagnostic_warning_line,
        diagnostic_error_range,
        diagnostic_warning_range,
    }
}

pub(crate) fn set_syntax_palette(tags: &SyntaxTags, dark: bool) {
    tags.keyword
        .set_property("foreground", if dark { "#78d19c" } else { "#066b42" });
    tags.string
        .set_property("foreground", if dark { "#dfb47d" } else { "#825022" });
    tags.number
        .set_property("foreground", if dark { "#b9a9e8" } else { "#55468e" });
    tags.comment
        .set_property("foreground", if dark { "#9ca8a1" } else { "#718078" });
    tags.current_line.set_property(
        "paragraph-background",
        if dark { "#282f2b" } else { "#f2f7f3" },
    );
    tags.matching_delimiter
        .set_property("background", if dark { "#416a53" } else { "#cce8d8" });
    tags.matching_delimiter
        .set_property("foreground", if dark { "#ffffff" } else { "#075b3e" });
    let error_line = if dark {
        gtk::gdk::RGBA::new(0.62, 0.17, 0.17, 0.22)
    } else {
        gtk::gdk::RGBA::new(0.91, 0.28, 0.25, 0.14)
    };
    let warning_line = if dark {
        gtk::gdk::RGBA::new(0.58, 0.42, 0.13, 0.2)
    } else {
        gtk::gdk::RGBA::new(0.93, 0.68, 0.17, 0.16)
    };
    let error_underline = if dark {
        gtk::gdk::RGBA::new(1.0, 0.43, 0.39, 1.0)
    } else {
        gtk::gdk::RGBA::new(0.78, 0.12, 0.1, 1.0)
    };
    let warning_underline = if dark {
        gtk::gdk::RGBA::new(1.0, 0.78, 0.24, 1.0)
    } else {
        gtk::gdk::RGBA::new(0.66, 0.43, 0.03, 1.0)
    };
    tags.diagnostic_error_line
        .set_property("paragraph-background-rgba", error_line);
    tags.diagnostic_warning_line
        .set_property("paragraph-background-rgba", warning_line);
    tags.diagnostic_error_range
        .set_property("underline-rgba", error_underline);
    tags.diagnostic_warning_range
        .set_property("underline-rgba", warning_underline);
}

pub(crate) fn install_diagnostic_markers(view: &sourceview5::View, dark: bool) {
    for (category, icon_name, priority, background) in [
        (
            "potoo-diagnostic-error",
            "potoo-diagnostic-error-symbolic",
            40,
            gtk::gdk::RGBA::new(
                if dark { 0.62 } else { 0.91 },
                if dark { 0.17 } else { 0.28 },
                if dark { 0.17 } else { 0.25 },
                if dark { 0.22 } else { 0.14 },
            ),
        ),
        (
            "potoo-diagnostic-warning",
            "potoo-diagnostic-warning-symbolic",
            30,
            gtk::gdk::RGBA::new(
                if dark { 0.58 } else { 0.93 },
                if dark { 0.42 } else { 0.68 },
                if dark { 0.13 } else { 0.17 },
                if dark { 0.2 } else { 0.16 },
            ),
        ),
        (
            "potoo-diagnostic-information",
            "potoo-diagnostic-information-symbolic",
            20,
            gtk::gdk::RGBA::new(0.23, 0.48, 0.72, 0.12),
        ),
        (
            "potoo-diagnostic-hint",
            "potoo-diagnostic-hint-symbolic",
            10,
            gtk::gdk::RGBA::new(0.42, 0.48, 0.44, 0.1),
        ),
    ] {
        let attributes = sourceview5::MarkAttributes::new();
        attributes.set_icon_name(icon_name);
        attributes.set_background(&background);
        attributes.connect_query_tooltip_text(|_, mark| {
            mark.name()
                .map_or_else(String::new, |name| name.to_string())
        });
        view.set_mark_attributes(category, &attributes, priority);
    }
}

pub(crate) fn apply_diagnostics(
    _view: &sourceview5::View,
    buffer: &gtk::TextBuffer,
    diagnostics: &[EditorDiagnostic],
    tags: &SyntaxTags,
    _dark: bool,
) {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.remove_tag(&tags.diagnostic_error_line, &start, &end);
    buffer.remove_tag(&tags.diagnostic_warning_line, &start, &end);
    buffer.remove_tag(&tags.diagnostic_error_range, &start, &end);
    buffer.remove_tag(&tags.diagnostic_warning_range, &start, &end);

    let Some(source_buffer) = buffer.downcast_ref::<sourceview5::Buffer>() else {
        return;
    };
    for category in [
        "potoo-diagnostic-error",
        "potoo-diagnostic-warning",
        "potoo-diagnostic-information",
        "potoo-diagnostic-hint",
    ] {
        source_buffer.remove_source_marks(&start, &end, Some(category));
    }

    let mut by_line = BTreeMap::<usize, Vec<&EditorDiagnostic>>::new();
    for diagnostic in diagnostics {
        by_line
            .entry(diagnostic.line.max(1))
            .or_default()
            .push(diagnostic);
        let Some(range) = &diagnostic.range else {
            continue;
        };
        let limit = buffer.char_count().max(0) as usize;
        let first = range.start.min(limit);
        let last = range.end.min(limit);
        let tag = match diagnostic.severity {
            DiagnosticSeverity::Error => Some(&tags.diagnostic_error_range),
            DiagnosticSeverity::Warning => Some(&tags.diagnostic_warning_range),
            DiagnosticSeverity::Information | DiagnosticSeverity::Hint => None,
        };
        if let Some(tag) = tag {
            apply_tag_range(buffer, tag, first, last);
        }
    }

    for (line, diagnostics) in by_line {
        let line_index = line
            .saturating_sub(1)
            .min(buffer.line_count().saturating_sub(1) as usize) as i32;
        let Some(line_start) = buffer.iter_at_line(line_index) else {
            continue;
        };
        let mut line_end = line_start.clone();
        line_end.forward_to_line_end();
        let severity = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.severity)
            .max_by_key(|severity| severity_priority(*severity))
            .unwrap_or(DiagnosticSeverity::Hint);
        let line_tag = match severity {
            DiagnosticSeverity::Error => Some(&tags.diagnostic_error_line),
            DiagnosticSeverity::Warning => Some(&tags.diagnostic_warning_line),
            DiagnosticSeverity::Information | DiagnosticSeverity::Hint => None,
        };
        if let Some(tag) = line_tag {
            buffer.apply_tag(tag, &line_start, &line_end);
        }

        let category = marker_category(severity);
        let tooltip = diagnostics
            .iter()
            .map(|diagnostic| {
                format!(
                    "{}: {}",
                    severity_label(diagnostic.severity),
                    diagnostic.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        source_buffer.create_source_mark(Some(&tooltip), category, &line_start);
    }
}

fn severity_priority(severity: DiagnosticSeverity) -> u8 {
    match severity {
        DiagnosticSeverity::Error => 4,
        DiagnosticSeverity::Warning => 3,
        DiagnosticSeverity::Information => 2,
        DiagnosticSeverity::Hint => 1,
    }
}

fn marker_category(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "potoo-diagnostic-error",
        DiagnosticSeverity::Warning => "potoo-diagnostic-warning",
        DiagnosticSeverity::Information => "potoo-diagnostic-information",
        DiagnosticSeverity::Hint => "potoo-diagnostic-hint",
    }
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "Erro",
        DiagnosticSeverity::Warning => "Aviso",
        DiagnosticSeverity::Information => "Informação",
        DiagnosticSeverity::Hint => "Dica",
    }
}

pub(crate) fn highlight_buffer(buffer: &gtk::TextBuffer, language: Language, tags: &SyntaxTags) {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    for tag in [
        &tags.keyword,
        &tags.string,
        &tags.number,
        &tags.comment,
        &tags.current_line,
        &tags.matching_delimiter,
    ] {
        buffer.remove_tag(tag, &start, &end);
    }
    let source = buffer.text(&start, &end, false);
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        if matches!(language, Language::GPortugol | Language::Calango) && chars[index] == '/' {
            let comment_end = match chars.get(index + 1) {
                Some('/') => {
                    let mut end = index + 2;
                    while end < chars.len() && chars[end] != '\n' {
                        end += 1;
                    }
                    Some(end)
                }
                Some('*') => {
                    let mut end = index + 2;
                    while end + 1 < chars.len() && !(chars[end] == '*' && chars[end + 1] == '/') {
                        end += 1;
                    }
                    Some(if end + 1 < chars.len() {
                        end + 2
                    } else {
                        chars.len()
                    })
                }
                _ => None,
            };
            if let Some(end) = comment_end {
                apply_tag_range(buffer, &tags.comment, index, end);
                index = end;
                continue;
            }
        }

        if chars[index] == '"' {
            let first = index;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                let closes_string = chars[index] == '"';
                index += 1;
                if closes_string {
                    break;
                }
            }
            apply_tag_range(buffer, &tags.string, first, index);
            continue;
        }

        if language == Language::Calango && chars[index] == '\'' {
            let first = index;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                let closes_character = chars[index] == '\'';
                index += 1;
                if closes_character {
                    break;
                }
            }
            apply_tag_range(buffer, &tags.string, first, index);
            continue;
        }

        if chars[index].is_alphabetic() || chars[index] == '_' {
            let first = index;
            index += 1;
            while index < chars.len() && (chars[index].is_alphanumeric() || chars[index] == '_') {
                index += 1;
            }
            let word: String = chars[first..index].iter().collect();
            if language.keywords().contains(&word.as_str()) {
                apply_tag_range(buffer, &tags.keyword, first, index);
            }
            continue;
        }

        if chars[index].is_ascii_digit() {
            let first = index;
            index += 1;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
            }
            apply_tag_range(buffer, &tags.number, first, index);
            continue;
        }

        index += 1;
    }

    let cursor = buffer.iter_at_offset(buffer.cursor_position());
    if let Some(mut line) = buffer.iter_at_line(cursor.line()) {
        let first = line.offset() as usize;
        line.forward_to_line_end();
        apply_tag_range(buffer, &tags.current_line, first, line.offset() as usize);
    }

    if let Some((first, second)) = matching_delimiter_at_cursor(&chars, language, cursor.offset()) {
        apply_tag_range(buffer, &tags.matching_delimiter, first, first + 1);
        apply_tag_range(buffer, &tags.matching_delimiter, second, second + 1);
    }
}

fn matching_delimiter_at_cursor(
    chars: &[char],
    language: Language,
    cursor_offset: i32,
) -> Option<(usize, usize)> {
    let mut pairs = vec![None; chars.len()];
    let mut open = Vec::<(usize, char)>::new();
    let mut index = 0;

    while index < chars.len() {
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        if current == '"' || current == '\'' {
            let quote = current;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                let closes_string = chars[index] == quote;
                index += 1;
                if closes_string {
                    break;
                }
            }
            continue;
        }

        if current == '/' && next == Some('/') {
            index += 2;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }

        if language == Language::Potigol && current == '#' {
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }

        if matches!(
            language,
            Language::Portugol | Language::GPortugol | Language::Calango
        ) && current == '/'
            && next == Some('*')
        {
            index += 2;
            let mut closed = false;
            while index + 1 < chars.len() {
                if chars[index] == '*' && chars[index + 1] == '/' {
                    index += 2;
                    closed = true;
                    break;
                }
                index += 1;
            }
            if !closed {
                break;
            }
            continue;
        }

        if language == Language::Visualg && current == '{' {
            while index < chars.len() && chars[index] != '}' {
                index += 1;
            }
            index = (index + 1).min(chars.len());
            continue;
        }

        if matches!(current, '{' | '(' | '[') {
            open.push((index, current));
        } else if let Some(expected_open) = matching_open(current) {
            if open
                .last()
                .is_some_and(|(_, delimiter)| *delimiter == expected_open)
            {
                if let Some((opening_index, _)) = open.pop() {
                    pairs[opening_index] = Some(index);
                    pairs[index] = Some(opening_index);
                }
            }
        }

        index += 1;
    }

    let cursor = usize::try_from(cursor_offset).unwrap_or(0).min(chars.len());
    let current = (cursor < chars.len()).then_some(cursor);
    let previous = cursor.checked_sub(1).filter(|offset| *offset < chars.len());
    let delimiter = current
        .filter(|offset| pairs[*offset].is_some())
        .or_else(|| previous.filter(|offset| pairs[*offset].is_some()))?;
    Some((delimiter, pairs[delimiter]?))
}

fn matching_open(closing: char) -> Option<char> {
    match closing {
        '}' => Some('{'),
        ')' => Some('('),
        ']' => Some('['),
        _ => None,
    }
}

fn apply_tag_range(buffer: &gtk::TextBuffer, tag: &gtk::TextTag, first: usize, last: usize) {
    if first >= last {
        return;
    }
    let start = buffer.iter_at_offset(first as i32);
    let end = buffer.iter_at_offset(last as i32);
    buffer.apply_tag(tag, &start, &end);
}

#[cfg(test)]
#[path = "../../../tests/learning/problem_localization_tests.rs"]
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
    use crate::components::content_panel::Language;
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
        components::content_panel::Language,
        diagnostics::{DiagnosticCategory, DiagnosticSeverity, EditorDiagnostic},
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

use super::model::{Language, SourceDocument};
use crate::{
    app::{MainWindow, MainWindowMsg, UiLanguage, tr},
    components::{clear_box, icon},
};
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Clone)]
pub(crate) struct SyntaxTags {
    keyword: gtk::TextTag,
    string: gtk::TextTag,
    number: gtk::TextTag,
    current_line: gtk::TextTag,
    matching_delimiter: gtk::TextTag,
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

pub(crate) fn build_learning_panel(language: UiLanguage) -> gtk::Widget {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("potoo-learning-panel");
    panel.set_size_request(306, -1);

    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_margin_start(18);
    content.set_margin_end(18);
    content.set_margin_top(18);
    content.set_margin_bottom(18);
    panel.append(&content);

    let title = gtk::Label::new(Some(tr(language, "Aprenda enquanto programa")));
    title.set_xalign(0.0);
    title.add_css_class("potoo-learning-title");
    title.set_wrap(true);
    content.append(&title);

    let line_title = gtk::Label::new(Some(tr(language, "Linha 7 · decisão")));
    line_title.set_xalign(0.0);
    line_title.add_css_class("heading");
    line_title.add_css_class("potoo-learning-line-title");
    content.append(&line_title);

    let explanation = gtk::Label::new(Some(tr(
        language,
        "Uma condição verifica se uma expressão é verdadeira antes de executar um bloco.",
    )));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.set_selectable(true);
    explanation.add_css_class("potoo-learning-description");
    content.append(&explanation);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let experiment_title = gtk::Label::new(Some(tr(language, "Experimente")));
    experiment_title.set_xalign(0.0);
    experiment_title.add_css_class("heading");
    content.append(&experiment_title);

    let experiment = gtk::Label::new(Some(tr(
        language,
        "Troque o valor 18 por 16 e execute de novo. O resultado muda?",
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
    content.append(&experiment);

    let concept = gtk::Box::new(gtk::Orientation::Vertical, 8);
    concept.add_css_class("potoo-card");
    concept.add_css_class("potoo-concept-card");
    concept.set_margin_start(4);
    concept.set_margin_end(4);
    concept.set_margin_top(8);
    concept.set_margin_bottom(8);
    let concept_title = gtk::Label::new(Some(tr(language, "Conceito: condição")));
    concept_title.set_xalign(0.0);
    concept_title.add_css_class("heading");
    let concept_text = gtk::Label::new(Some(tr(
        language,
        "Uma comparação como idade >= 18 resulta em verdadeiro ou falso.",
    )));
    concept_text.set_xalign(0.0);
    concept_text.set_wrap(true);
    concept_text.set_selectable(true);
    concept.append(&concept_title);
    concept.append(&concept_text);
    content.append(&concept);

    panel.upcast()
}

pub(crate) fn build_output_panel(
    language: UiLanguage,
) -> (gtk::Box, gtk::TextBuffer, gtk::Button, gtk::TextView) {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 0);
    panel.add_css_class("potoo-output-panel");
    panel.set_vexpand(true);

    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    tabs.add_css_class("potoo-output-tabs");
    let output_tab = gtk::Button::with_label(tr(language, "Saída"));
    let problems_tab = gtk::Button::with_label(tr(language, "Problemas"));
    let debugger_tab = gtk::Button::with_label(tr(language, "Depurador"));
    let tab_buttons = [
        output_tab.clone(),
        problems_tab.clone(),
        debugger_tab.clone(),
    ];
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

    let problems = empty_tool_page(
        tr(language, "Nenhum problema detectado"),
        tr(language, "Erros de sintaxe e execução aparecerão aqui."),
    );
    stack.add_named(&problems, Some("problems"));

    let debugger = empty_tool_page(
        tr(language, "Depurador pronto"),
        tr(
            language,
            "Execute o programa passo a passo para acompanhar as variáveis.",
        ),
    );
    stack.add_named(&debugger, Some("debugger"));
    stack.set_visible_child_name("output");

    for (button, child_name) in tab_buttons.iter().zip(["output", "problems", "debugger"]) {
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
    (panel, output_buffer, output_tab, output)
}

fn empty_tool_page(title: &str, description: &str) -> gtk::Widget {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 4);
    page.add_css_class("potoo-output-body");
    page.set_margin_start(14);
    page.set_margin_end(14);
    page.set_margin_top(12);
    page.set_margin_bottom(12);
    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("potoo-output-problem-title");
    let description_label = gtk::Label::new(Some(description));
    description_label.set_xalign(0.0);
    description_label.set_wrap(true);
    description_label.add_css_class("potoo-output-placeholder");
    page.append(&title_label);
    page.append(&description_label);
    page.upcast()
}

pub(crate) fn make_syntax_tags(buffer: &gtk::TextBuffer) -> SyntaxTags {
    let table = buffer.tag_table();
    let keyword = gtk::TextTag::builder().name("keyword").weight(700).build();
    let string = gtk::TextTag::builder().name("string").build();
    let number = gtk::TextTag::builder().name("number").build();
    let current_line = gtk::TextTag::builder().name("current-line").build();
    let matching_delimiter = gtk::TextTag::builder()
        .name("matching-delimiter")
        .weight(700)
        .build();
    table.add(&keyword);
    table.add(&string);
    table.add(&number);
    table.add(&current_line);
    table.add(&matching_delimiter);
    SyntaxTags {
        keyword,
        string,
        number,
        current_line,
        matching_delimiter,
    }
}

pub(crate) fn set_syntax_palette(tags: &SyntaxTags, dark: bool) {
    tags.keyword
        .set_property("foreground", if dark { "#78d19c" } else { "#066b42" });
    tags.string
        .set_property("foreground", if dark { "#dfb47d" } else { "#825022" });
    tags.number
        .set_property("foreground", if dark { "#b9a9e8" } else { "#55468e" });
    tags.current_line.set_property(
        "paragraph-background",
        if dark { "#282f2b" } else { "#f2f7f3" },
    );
    tags.matching_delimiter
        .set_property("background", if dark { "#416a53" } else { "#cce8d8" });
    tags.matching_delimiter
        .set_property("foreground", if dark { "#ffffff" } else { "#075b3e" });
}

pub(crate) fn highlight_buffer(buffer: &gtk::TextBuffer, language: Language, tags: &SyntaxTags) {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.remove_all_tags(&start, &end);
    let source = buffer.text(&start, &end, false);
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;

    while index < chars.len() {
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

        if language == Language::Portugol && current == '/' && next == Some('*') {
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

pub(crate) fn update_line_numbers(buffer: &gtk::TextBuffer, gutter: &gtk::Label) {
    let count = buffer.line_count().max(1);
    let numbers = (1..=count)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    gutter.set_text(&numbers);
}

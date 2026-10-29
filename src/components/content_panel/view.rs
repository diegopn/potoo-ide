use super::model::{Language, SourceDocument};
use crate::{
    app::{MainWindow, MainWindowMsg, UiLanguage, tr},
    components::clear_box,
};
use gtk::prelude::*;
use relm4::prelude::*;

#[derive(Clone)]
pub(crate) struct SyntaxTags {
    keyword: gtk::TextTag,
    string: gtk::TextTag,
    number: gtk::TextTag,
    current_line: gtk::TextTag,
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
        if Some(document.id) == active_document_id {
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
        content.append(&gtk::Image::from_icon_name("text-x-generic-symbolic"));
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
        close_button.set_child(Some(&gtk::Image::from_icon_name("window-close-symbolic")));
        close_button.set_tooltip_text(Some(tr(language, "Fechar arquivo")));
        close_button.add_css_class("potoo-close-tab-button");
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
    panel.set_size_request(370, -1);

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
    content.append(&line_title);

    let explanation = gtk::Label::new(Some(tr(
        language,
        "Uma condição verifica se uma expressão é verdadeira antes de executar um bloco.",
    )));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.set_selectable(true);
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

pub(crate) fn build_output_panel(language: UiLanguage) -> (gtk::Notebook, gtk::TextBuffer) {
    let notebook = gtk::Notebook::new();
    notebook.add_css_class("potoo-output-notebook");
    notebook.set_tab_pos(gtk::PositionType::Top);
    notebook.set_vexpand(true);

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
    let output_buffer = output.buffer();
    output_buffer.set_text(tr(language, "A saída aparecerá aqui após a execução."));
    let output_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&output)
        .build();
    notebook.append_page(
        &output_scroll,
        Some(&gtk::Label::new(Some(tr(language, "Saída")))),
    );

    let problems = empty_tool_page(
        tr(language, "Nenhum problema detectado"),
        tr(language, "Os erros de sintaxe e execução aparecerão aqui."),
    );
    notebook.append_page(
        &problems,
        Some(&gtk::Label::new(Some(tr(language, "Problemas")))),
    );

    let debugger = empty_tool_page(
        tr(language, "Depurador"),
        tr(
            language,
            "Acompanhe a execução linha por linha e os valores das variáveis.",
        ),
    );
    notebook.append_page(
        &debugger,
        Some(&gtk::Label::new(Some(tr(language, "Depurador")))),
    );
    (notebook, output_buffer)
}

fn empty_tool_page(title: &str, description: &str) -> gtk::Widget {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    page.set_margin_start(16);
    page.set_margin_end(16);
    page.set_margin_top(16);
    page.set_margin_bottom(16);
    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("heading");
    let description_label = gtk::Label::new(Some(description));
    description_label.set_xalign(0.0);
    description_label.set_wrap(true);
    description_label.add_css_class("dim-label");
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
    table.add(&keyword);
    table.add(&string);
    table.add(&number);
    table.add(&current_line);
    SyntaxTags {
        keyword,
        string,
        number,
        current_line,
    }
}

pub(crate) fn set_syntax_palette(tags: &SyntaxTags, dark: bool) {
    tags.keyword
        .set_property("foreground", if dark { "#72cc99" } else { "#147a49" });
    tags.string
        .set_property("foreground", if dark { "#d6aa72" } else { "#8a5a23" });
    tags.number
        .set_property("foreground", if dark { "#b5a0eb" } else { "#6c57a3" });
    tags.current_line.set_property(
        "paragraph-background",
        if dark { "#2f4237" } else { "#e8f2ec" },
    );
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

    if let Some(mut line) = buffer.iter_at_line(6) {
        let first = line.offset() as usize;
        line.forward_to_line_end();
        apply_tag_range(buffer, &tags.current_line, first, line.offset() as usize);
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

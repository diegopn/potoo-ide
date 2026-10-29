use adw::prelude::*;
use std::{cell::Cell, path::Path, rc::Rc};

const PORTUGOL_SAMPLE: &str = "programa {\n  funcao inicio() {\n    inteiro idade\n\n    escreva(\"Qual é sua idade? \")\n    leia(idade)\n    se (idade >= 18) {\n      escreva(\"Pode votar.\")\n    } senao {\n      escreva(\"Ainda não pode votar.\")\n    }\n  }\n}";

const VISUALG_SAMPLE: &str = "algoritmo \"votacao\"\nvar\n  idade: inteiro\ninicio\n  escreva(\"Qual é sua idade? \")\n  leia(idade)\n  se idade >= 18 entao\n    escreval(\"Pode votar.\")\n  senao\n    escreval(\"Ainda não pode votar.\")\n  fimse\nfimalgoritmo";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    Portugol,
    Visualg,
}

impl Language {
    fn label(self) -> &'static str {
        match self {
            Self::Portugol => "Portugol",
            Self::Visualg => "VisuAlg",
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            Self::Portugol => "principal.por",
            Self::Visualg => "principal.alg",
        }
    }

    fn sample(self) -> &'static str {
        match self {
            Self::Portugol => PORTUGOL_SAMPLE,
            Self::Visualg => VISUALG_SAMPLE,
        }
    }

    fn keywords(self) -> &'static [&'static str] {
        match self {
            Self::Portugol => &[
                "programa", "funcao", "inicio", "inteiro", "escreva", "leia", "se", "senao",
            ],
            Self::Visualg => &[
                "algoritmo",
                "var",
                "inteiro",
                "inicio",
                "escreva",
                "escreval",
                "leia",
                "se",
                "entao",
                "senao",
                "fimse",
                "fimalgoritmo",
            ],
        }
    }
}

#[derive(Clone)]
struct SyntaxTags {
    keyword: gtk::TextTag,
    string: gtk::TextTag,
    number: gtk::TextTag,
    current_line: gtk::TextTag,
}

fn main() {
    let application = adw::Application::builder()
        .application_id("io.github.diegopn.potooide")
        .build();

    application.connect_activate(build_window);
    let _ = application.run();
}

fn build_window(application: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Potoo IDE")
        .default_width(1440)
        .default_height(900)
        .build();

    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::ForceLight);

    let language = Rc::new(Cell::new(Language::Portugol));
    let header = adw::HeaderBar::new();
    let brand = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    brand.append(&app_icon());
    let brand_label = gtk::Label::new(Some("Potoo IDE"));
    brand_label.add_css_class("heading");
    brand.append(&brand_label);
    header.pack_start(&brand);

    let project_title = gtk::Label::new(Some("Meu primeiro programa"));
    project_title.add_css_class("title");
    header.set_title_widget(Some(&project_title));

    let language_picker = gtk::DropDown::from_strings(&["Portugol", "VisuAlg"]);
    language_picker.set_tooltip_text(Some("Linguagem do programa"));

    let theme_button = gtk::ToggleButton::new();
    theme_button.set_child(Some(&gtk::Image::from_icon_name(
        "weather-clear-night-symbolic",
    )));
    theme_button.set_tooltip_text(Some("Alternar entre tema claro e escuro"));
    theme_button.add_css_class("flat");

    let run_button = gtk::Button::new();
    run_button.add_css_class("suggested-action");
    let run_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    run_content.append(&gtk::Image::from_icon_name("media-playback-start-symbolic"));
    run_content.append(&gtk::Label::new(Some("Executar")));
    run_button.set_child(Some(&run_content));
    run_button.set_tooltip_text(Some(
        "A execução real usa o interpretador da linguagem selecionada",
    ));

    let header_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header_actions.append(&language_picker);
    header_actions.append(&theme_button);
    header_actions.append(&run_button);
    header.pack_end(&header_actions);

    let (sidebar, sidebar_file_name) = build_file_sidebar();
    let file_name = gtk::Label::new(Some(Language::Portugol.file_name()));
    file_name.set_xalign(0.0);

    let code_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    code_buffer.set_text(Language::Portugol.sample());
    let syntax_tags = make_syntax_tags(&code_buffer);
    set_syntax_palette(&syntax_tags, false);
    highlight_buffer(&code_buffer, Language::Portugol, &syntax_tags);

    let line_numbers = gtk::Label::new(None);
    line_numbers.set_xalign(1.0);
    line_numbers.set_yalign(0.0);
    line_numbers.add_css_class("dim-label");
    line_numbers.add_css_class("monospace");
    line_numbers.set_margin_top(14);
    line_numbers.set_margin_bottom(14);
    line_numbers.set_margin_start(12);
    line_numbers.set_margin_end(12);
    update_line_numbers(&code_buffer, &line_numbers);

    let code_view = gtk::TextView::builder()
        .buffer(&code_buffer)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::None)
        .left_margin(12)
        .right_margin(16)
        .top_margin(14)
        .bottom_margin(14)
        .build();
    code_view.add_css_class("code-editor");

    let code_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&code_view)
        .build();

    let editor_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_row.add_css_class("view");
    editor_row.append(&line_numbers);
    editor_row.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    editor_row.append(&code_scroll);

    let editor_tab = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    editor_tab.set_margin_start(12);
    editor_tab.set_margin_end(12);
    editor_tab.set_margin_top(8);
    editor_tab.set_margin_bottom(8);
    editor_tab.append(&gtk::Image::from_icon_name("text-x-generic-symbolic"));
    editor_tab.append(&file_name);
    let tab_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tab_bar.add_css_class("toolbar");
    tab_bar.append(&editor_tab);
    tab_bar.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let editor = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor.append(&tab_bar);
    editor.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    editor.append(&editor_row);

    let (output_notebook, output_buffer) = build_output_panel();
    let output_status = gtk::Label::new(Some("Pronto para executar"));
    output_status.set_xalign(0.0);
    output_status.add_css_class("dim-label");
    let output_status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    output_status_row.set_margin_start(12);
    output_status_row.set_margin_end(12);
    output_status_row.set_margin_bottom(8);
    output_status_row.append(&gtk::Image::from_icon_name("emblem-synchronizing-symbolic"));
    output_status_row.append(&output_status);

    let output_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
    output_area.append(&output_notebook);
    output_area.append(&output_status_row);

    let editor_output = gtk::Paned::new(gtk::Orientation::Vertical);
    editor_output.set_wide_handle(true);
    editor_output.set_start_child(Some(&editor));
    editor_output.set_end_child(Some(&output_area));
    editor_output.set_resize_start_child(true);
    editor_output.set_resize_end_child(false);
    editor_output.set_shrink_start_child(false);
    editor_output.set_shrink_end_child(false);
    editor_output.set_position(570);

    let learning_panel = build_learning_panel();
    let editor_and_learning = gtk::Paned::new(gtk::Orientation::Horizontal);
    editor_and_learning.set_wide_handle(true);
    editor_and_learning.set_start_child(Some(&editor_output));
    editor_and_learning.set_end_child(Some(&learning_panel));
    editor_and_learning.set_resize_start_child(true);
    editor_and_learning.set_resize_end_child(false);
    editor_and_learning.set_shrink_start_child(false);
    editor_and_learning.set_shrink_end_child(false);
    editor_and_learning.set_position(820);

    let workspace = gtk::Paned::new(gtk::Orientation::Horizontal);
    workspace.set_wide_handle(true);
    workspace.set_start_child(Some(&sidebar));
    workspace.set_end_child(Some(&editor_and_learning));
    workspace.set_resize_start_child(false);
    workspace.set_resize_end_child(true);
    workspace.set_shrink_start_child(false);
    workspace.set_shrink_end_child(false);
    workspace.set_position(250);
    workspace.set_vexpand(true);

    let status_language = gtk::Label::new(Some("Portugol"));
    let status_details = gtk::Label::new(Some("UTF-8  ·  Espaços: 4"));
    status_language.add_css_class("dim-label");
    status_details.add_css_class("dim-label");
    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    status_bar.set_margin_start(12);
    status_bar.set_margin_end(12);
    status_bar.set_margin_top(6);
    status_bar.set_margin_bottom(6);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_language);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_details);
    status_bar.add_css_class("toolbar");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&workspace);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&status_bar);

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header);
    toolbar_view.set_content(Some(&content));
    window.set_content(Some(&toolbar_view));

    let code_language = language.clone();
    let file_name_for_language = file_name.clone();
    let sidebar_file_name_for_language = sidebar_file_name.clone();
    let status_language_for_language = status_language.clone();
    let code_buffer_for_language = code_buffer.clone();
    let output_status_for_language = output_status.clone();
    language_picker.connect_selected_notify(move |picker| {
        let selected = if picker.selected() == 1 {
            Language::Visualg
        } else {
            Language::Portugol
        };
        code_language.set(selected);
        file_name_for_language.set_text(selected.file_name());
        sidebar_file_name_for_language.set_text(selected.file_name());
        status_language_for_language.set_text(selected.label());
        code_buffer_for_language.set_text(selected.sample());
        output_status_for_language.set_text("Pronto para executar");
    });

    let style_manager_for_theme = style_manager.clone();
    let tags_for_theme = syntax_tags.clone();
    let buffer_for_theme = code_buffer.clone();
    let language_for_theme = language.clone();
    theme_button.connect_toggled(move |button| {
        let dark = button.is_active();
        style_manager_for_theme.set_color_scheme(if dark {
            adw::ColorScheme::ForceDark
        } else {
            adw::ColorScheme::ForceLight
        });
        set_syntax_palette(&tags_for_theme, dark);
        highlight_buffer(&buffer_for_theme, language_for_theme.get(), &tags_for_theme);
        button.set_child(Some(&gtk::Image::from_icon_name(if dark {
            "weather-clear-symbolic"
        } else {
            "weather-clear-night-symbolic"
        })));
        button.set_tooltip_text(Some(if dark {
            "Usar tema claro"
        } else {
            "Usar tema escuro"
        }));
    });

    let output_status_for_run = output_status.clone();
    let output_notebook_for_run = output_notebook.clone();
    run_button.connect_clicked(move |_| {
        output_buffer.set_text(
            "A execução real depende da integração com o interpretador oficial selecionado.",
        );
        output_status_for_run.set_text("Interpretador ainda não conectado nesta prévia");
        output_notebook_for_run.set_current_page(Some(0));
    });

    let tags_for_edit = syntax_tags.clone();
    let language_for_edit = language.clone();
    let line_numbers_for_edit = line_numbers.clone();
    code_buffer.connect_changed(move |buffer| {
        update_line_numbers(buffer, &line_numbers_for_edit);
        highlight_buffer(buffer, language_for_edit.get(), &tags_for_edit);
    });

    window.present();
}

fn app_icon() -> gtk::Image {
    let development_icon = "data/icons/hicolor/scalable/apps/io.github.diegopn.potooide.svg";
    let image = if Path::new(development_icon).exists() {
        gtk::Image::from_file(development_icon)
    } else {
        gtk::Image::from_icon_name("io.github.diegopn.potooide")
    };
    image.set_pixel_size(32);
    image
}

fn build_file_sidebar() -> (gtk::Widget, gtk::Label) {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar.add_css_class("sidebar");
    sidebar.set_size_request(220, -1);

    let heading = gtk::Label::new(Some("Arquivos"));
    heading.set_xalign(0.0);
    heading.add_css_class("heading");
    heading.set_margin_start(16);
    heading.set_margin_end(16);
    heading.set_margin_top(16);
    heading.set_margin_bottom(12);
    sidebar.append(&heading);

    let project = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    project.set_margin_start(12);
    project.set_margin_end(12);
    project.set_margin_top(4);
    project.set_margin_bottom(8);
    project.append(&gtk::Image::from_icon_name("folder-symbolic"));
    let project_label = gtk::Label::new(Some("Meu primeiro programa"));
    project_label.set_xalign(0.0);
    project_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    project.append(&project_label);
    sidebar.append(&project);

    let files = gtk::ListBox::new();
    files.add_css_class("navigation-sidebar");
    files.set_selection_mode(gtk::SelectionMode::Single);
    let row = gtk::ListBoxRow::new();
    row.add_css_class("activatable");
    let row_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row_content.set_margin_start(24);
    row_content.set_margin_end(12);
    row_content.set_margin_top(8);
    row_content.set_margin_bottom(8);
    row_content.append(&gtk::Image::from_icon_name("text-x-generic-symbolic"));
    let row_label = gtk::Label::new(Some(Language::Portugol.file_name()));
    row_label.set_xalign(0.0);
    row_content.append(&row_label);
    row.set_child(Some(&row_content));
    files.append(&row);
    files.select_row(Some(&row));
    sidebar.append(&files);

    (sidebar.upcast(), row_label)
}

fn build_learning_panel() -> gtk::Widget {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 16);
    panel.add_css_class("sidebar");
    panel.set_size_request(320, -1);
    panel.set_margin_start(18);
    panel.set_margin_end(18);
    panel.set_margin_top(18);
    panel.set_margin_bottom(18);

    let title = gtk::Label::new(Some("Aprenda enquanto programa"));
    title.set_xalign(0.0);
    title.add_css_class("title-3");
    title.set_wrap(true);
    panel.append(&title);

    let line_title = gtk::Label::new(Some("Linha 7 · decisão"));
    line_title.set_xalign(0.0);
    line_title.add_css_class("heading");
    panel.append(&line_title);

    let explanation = gtk::Label::new(Some(
        "Uma condição verifica se uma expressão é verdadeira antes de executar um bloco.",
    ));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.set_selectable(true);
    panel.append(&explanation);
    panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let experiment_title = gtk::Label::new(Some("Experimente"));
    experiment_title.set_xalign(0.0);
    experiment_title.add_css_class("heading");
    panel.append(&experiment_title);

    let experiment = gtk::Label::new(Some(
        "Troque o valor 18 por 16 e execute de novo. O resultado muda?",
    ));
    experiment.set_xalign(0.0);
    experiment.set_wrap(true);
    experiment.set_selectable(true);
    experiment.add_css_class("card");
    experiment.set_margin_start(4);
    experiment.set_margin_end(4);
    experiment.set_margin_top(4);
    experiment.set_margin_bottom(4);
    panel.append(&experiment);

    let concept = gtk::Box::new(gtk::Orientation::Vertical, 8);
    concept.add_css_class("card");
    concept.set_margin_start(4);
    concept.set_margin_end(4);
    concept.set_margin_top(8);
    concept.set_margin_bottom(8);
    let concept_title = gtk::Label::new(Some("Conceito: condição"));
    concept_title.set_xalign(0.0);
    concept_title.add_css_class("heading");
    let concept_text = gtk::Label::new(Some(
        "Uma comparação como idade >= 18 resulta em verdadeiro ou falso.",
    ));
    concept_text.set_xalign(0.0);
    concept_text.set_wrap(true);
    concept_text.set_selectable(true);
    concept.append(&concept_title);
    concept.append(&concept_text);
    panel.append(&concept);

    panel.upcast()
}

fn build_output_panel() -> (gtk::Notebook, gtk::TextBuffer) {
    let notebook = gtk::Notebook::new();
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
    output.add_css_class("view");
    let output_buffer = output.buffer();
    output_buffer.set_text("A saída aparecerá aqui após a execução.");
    let output_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&output)
        .build();
    notebook.append_page(&output_scroll, Some(&gtk::Label::new(Some("Saída"))));

    let problems = empty_tool_page(
        "Nenhum problema detectado",
        "Os erros de sintaxe e execução aparecerão aqui.",
    );
    notebook.append_page(&problems, Some(&gtk::Label::new(Some("Problemas"))));

    let debugger = empty_tool_page(
        "Depurador",
        "Acompanhe a execução linha por linha e os valores das variáveis.",
    );
    notebook.append_page(&debugger, Some(&gtk::Label::new(Some("Depurador"))));
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

fn make_syntax_tags(buffer: &gtk::TextBuffer) -> SyntaxTags {
    let table = buffer.tag_table();
    let keyword = gtk::TextTag::builder().name("keyword").build();
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

fn set_syntax_palette(tags: &SyntaxTags, dark: bool) {
    tags.keyword
        .set_property("foreground", if dark { "#79c0ff" } else { "#185abc" });
    tags.string
        .set_property("foreground", if dark { "#ff9b8a" } else { "#b3432b" });
    tags.number
        .set_property("foreground", if dark { "#a8dca0" } else { "#267a33" });
    tags.current_line
        .set_property("background", if dark { "#26394f" } else { "#e8f1ff" });
}

fn highlight_buffer(buffer: &gtk::TextBuffer, language: Language, tags: &SyntaxTags) {
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

fn update_line_numbers(buffer: &gtk::TextBuffer, gutter: &gtk::Label) {
    let count = buffer.line_count().max(1);
    let numbers = (1..=count)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    gutter.set_text(&numbers);
}

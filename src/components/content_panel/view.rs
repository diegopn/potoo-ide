use super::model::{Language, SourceDocument};
use crate::{
    app::{MainWindow, MainWindowMsg, RightPanelSection, UiLanguage, tr},
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

pub(crate) struct OutputPanelUi {
    pub(crate) panel: gtk::Box,
    pub(crate) debugger_page: gtk::Box,
    pub(crate) output_buffer: gtk::TextBuffer,
    pub(crate) output_tab: gtk::Button,
    pub(crate) output_view: gtk::TextView,
    pub(crate) problems_buffer: gtk::TextBuffer,
    pub(crate) problems_view: gtk::TextView,
    pub(crate) problems_tab: gtk::Button,
    pub(crate) debugger_start: gtk::Button,
    pub(crate) debugger_continue: gtk::Button,
    pub(crate) debugger_step: gtk::Button,
    pub(crate) debugger_stop: gtk::Button,
    pub(crate) debugger_position: gtk::Label,
    pub(crate) debugger_variables: gtk::TextBuffer,
}

pub(crate) struct LearningPanelUi {
    pub(crate) panel: gtk::Box,
    pub(crate) spinner: gtk::Spinner,
    pub(crate) analysis_stack: gtk::Stack,
    pub(crate) line_title: gtk::Label,
    pub(crate) explanation: gtk::Label,
    pub(crate) experiment: gtk::Label,
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
    heading.append(&spinner);
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
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

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
    content.append(&concept);

    let problems_title = gtk::Label::new(Some(tr(language, "Problemas nesta linha")));
    problems_title.set_xalign(0.0);
    problems_title.add_css_class("heading");
    content.append(&problems_title);

    let problem_list = gtk::Box::new(gtk::Orientation::Vertical, 8);
    content.append(&problem_list);
    let problem_empty =
        gtk::Label::new(Some(tr(language, "Nenhum problema conhecido nesta linha.")));
    problem_empty.set_xalign(0.0);
    problem_empty.set_wrap(true);
    problem_empty.add_css_class("potoo-learning-problem-empty");
    content.append(&problem_empty);

    let experiment_title = gtk::Label::new(Some(tr(language, "Experimente")));
    experiment_title.set_xalign(0.0);
    experiment_title.add_css_class("heading");
    content.append(&experiment_title);

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
    content.append(&experiment);

    let analysis_stack = gtk::Stack::new();
    analysis_stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    analysis_stack.set_transition_duration(120);
    analysis_stack.set_vexpand(true);
    analysis_stack.add_named(&scroll, Some("content"));
    let loading_message = gtk::Label::new(Some(tr(language, "Analisando a linha selecionada…")));
    loading_message.set_xalign(0.0);
    loading_message.set_wrap(true);
    loading_message.set_valign(gtk::Align::Start);
    loading_message.set_margin_start(18);
    loading_message.set_margin_end(18);
    loading_message.set_margin_top(12);
    loading_message.add_css_class("potoo-learning-loading");
    analysis_stack.add_named(&loading_message, Some("loading"));
    panel.append(&analysis_stack);

    LearningPanelUi {
        panel,
        spinner,
        analysis_stack,
        line_title,
        explanation,
        experiment,
        concept_title,
        concept_text,
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
    let sender_for_debugger = sender;
    debugger_button.connect_clicked(move |_| {
        sender_for_debugger.input(MainWindowMsg::SelectRightPanel(RightPanelSection::Debugger));
    });

    rail.append(&learning_button);
    rail.append(&debugger_button);
    panel.append(&rail);

    ToolPanelUi {
        panel,
        revealer,
        stack,
        learning_button,
        debugger_button,
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

    let problems = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(14)
        .right_margin(14)
        .top_margin(12)
        .bottom_margin(12)
        .build();
    problems.add_css_class("potoo-output-view");
    problems.add_css_class("potoo-output-placeholder-text");
    let problems_buffer = problems.buffer();
    problems_buffer.set_text(tr(language, "Nenhum problema detectado"));
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
    let controls = gtk::Grid::new();
    controls.set_column_spacing(6);
    controls.set_row_spacing(6);
    controls.set_hexpand(true);
    let debugger_start = gtk::Button::with_label(tr(language, "Iniciar depuração"));
    let debugger_continue = gtk::Button::with_label(tr(language, "Continuar"));
    let debugger_step = gtk::Button::with_label(tr(language, "Avançar um passo"));
    let debugger_stop = gtk::Button::with_label(tr(language, "Parar"));
    debugger_start.add_css_class("potoo-debugger-action");
    debugger_continue.add_css_class("potoo-debugger-action");
    debugger_step.add_css_class("potoo-debugger-action");
    debugger_stop.add_css_class("potoo-debugger-action");
    debugger_start.set_hexpand(true);
    debugger_continue.set_hexpand(true);
    debugger_step.set_hexpand(true);
    debugger_stop.set_hexpand(true);
    let sender_for_start = sender.clone();
    debugger_start.connect_clicked(move |_| {
        sender_for_start.input(MainWindowMsg::StartDebug);
    });
    let sender_for_continue = sender.clone();
    debugger_continue
        .connect_clicked(move |_| sender_for_continue.input(MainWindowMsg::ContinueDebug));
    let sender_for_step = sender.clone();
    debugger_step.connect_clicked(move |_| sender_for_step.input(MainWindowMsg::StepDebug));
    debugger_stop.connect_clicked(move |_| sender.input(MainWindowMsg::StopDebug));
    controls.attach(&debugger_start, 0, 0, 1, 1);
    controls.attach(&debugger_continue, 1, 0, 1, 1);
    controls.attach(&debugger_step, 0, 1, 1, 1);
    controls.attach(&debugger_stop, 1, 1, 1, 1);
    debugger.append(&controls);
    let debugger_position = gtk::Label::new(Some(tr(language, "Aguardando depuração")));
    debugger_position.set_xalign(0.0);
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
        .vexpand(true)
        .child(&variables_view)
        .build();
    debugger.append(&variables_scroll);
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
        problems_buffer,
        problems_view: problems,
        problems_tab,
        debugger_start,
        debugger_continue,
        debugger_step,
        debugger_stop,
        debugger_position,
        debugger_variables,
    }
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

#[cfg(test)]
#[path = "../../../tests/learning/problem_localization_tests.rs"]
mod problem_localization_tests;

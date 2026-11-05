use super::*;

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

pub(crate) fn show_learning_problems(
    list: &gtk::Box,
    empty: &gtk::Label,
    problems: &[crate::features::learning::LearningProblem],
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
            crate::features::learning::LearningSeverity::Error => "potoo-learning-problem-error",
            crate::features::learning::LearningSeverity::Warning => {
                "potoo-learning-problem-warning"
            }
        });
        card.set_hexpand(true);
        card.set_focus_on_click(false);
        let contents = gtk::Box::new(gtk::Orientation::Vertical, 6);
        contents.set_halign(gtk::Align::Fill);
        let title = gtk::Label::new(Some(match problem.kind {
            crate::features::learning::LearningProblemKind::Syntax => {
                tr(language, "Erro de sintaxe")
            }
            crate::features::learning::LearningProblemKind::Type => tr(language, "Erro de tipo"),
            crate::features::learning::LearningProblemKind::Runtime => {
                tr(language, "Erro durante a execução")
            }
            crate::features::learning::LearningProblemKind::Logic => {
                tr(language, "Índice fora dos limites")
            }
            crate::features::learning::LearningProblemKind::PossibleLogic => {
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

pub(crate) fn localize_learning_problem(
    problem: &crate::features::learning::LearningProblem,
    language: UiLanguage,
) -> (String, Option<String>) {
    use crate::features::learning::{LearningProblemKind, LearningProblemText};

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

pub(super) fn build_debugger_learning(language: UiLanguage) -> DebuggerLearningUi {
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

    let problems = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let problems_title = gtk::Label::new(Some(tr(language, "Problemas nesta linha")));
    problems_title.set_xalign(0.0);
    problems_title.add_css_class("heading");
    problems_title.set_margin_top(4);
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
        problem_list,
        problem_empty,
    }
}

pub(super) fn connect_debugger_start(
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

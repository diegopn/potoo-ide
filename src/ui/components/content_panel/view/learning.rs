use super::*;

mod diagnostics;
pub(crate) use self::diagnostics::localize_learning_problem;

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
            crate::features::learning::LearningProblemKind::StaticAnalysis => {
                tr(language, "Aviso de análise")
            }
            crate::features::learning::LearningProblemKind::Analysis => {
                tr(language, "Análise incompleta")
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

    let experiment_section = gtk::Box::new(gtk::Orientation::Vertical, 16);
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

#[cfg(test)]
mod tests {
    use super::{UiLanguage, localize_learning_problem};
    use crate::{
        features::learning::{LearningProblemKind, learning_hint},
        languages::model::Language,
    };

    #[test]
    fn constant_index_recommendation_includes_the_name_and_valid_range() {
        let source = r#"programa {
 funcao inicio() {
  inteiro valores[3]
  escreva(valores[3])
 }
}"#;
        let hint = learning_hint(source, 4, Language::Portugol);
        let problem = hint
            .problems
            .iter()
            .find(|problem| problem.kind == LearningProblemKind::Logic)
            .expect("the out-of-range index should be diagnosed");

        let (message, suggestion) = localize_learning_problem(problem, UiLanguage::English);
        assert!(message.contains("outside the bounds of `valores` (0 to 2)"));
        assert_eq!(
            suggestion.as_deref(),
            Some("Use an index between 0 and 2 in `valores`.")
        );
    }

    #[test]
    fn missing_jerimum_import_advice_is_localized() {
        let source = "def desenhe(g: Graphics2D): Nada\n  g.setColor(Cor.VERDE.color)\nfim";
        let problem = learning_hint(source, 2, Language::Potigol)
            .problems
            .into_iter()
            .find(|problem| problem.kind == LearningProblemKind::StaticAnalysis)
            .expect("a typed Jerimum call without its import should be reported");

        let (english, _) = localize_learning_problem(&problem, UiLanguage::English);
        assert!(english.contains("Jerimum library"));
        assert!(english.contains("use \"jerimum\""));

        let (spanish, _) = localize_learning_problem(&problem, UiLanguage::Spanish);
        assert!(spanish.contains("biblioteca Jerimum"));
        assert!(spanish.contains("use \"jerimum\""));
    }

    #[test]
    fn missing_potigol_output_advice_is_localized_with_its_interpreter_message() {
        for (source, line, command, english_tip, spanish_tip) in [
            (
                "escreva\nescreva 2",
                1,
                "escreva",
                "Provide the value or expression to display after `escreva`.",
                "Indica el valor o la expresión que se mostrará después de `escreva`.",
            ),
            (
                "imprima\nimprima 2",
                1,
                "imprima",
                "Provide the value or expression to display after `imprima`.",
                "Indica el valor o la expresión que se mostrará después de `imprima`.",
            ),
        ] {
            let hint = learning_hint(source, line, Language::Potigol);
            let problem = hint
                .problems
                .into_iter()
                .find(|problem| problem.kind == LearningProblemKind::Syntax)
                .expect("the empty output statement should remain a syntax problem");
            assert_eq!((problem.line, problem.column), (line, 1));

            let (english, english_suggestion) =
                localize_learning_problem(&problem, UiLanguage::English);
            assert_eq!(english, "Expected an expression", "{command}");
            assert_eq!(
                english_suggestion.as_deref(),
                Some(english_tip),
                "{command}"
            );

            let (spanish, spanish_suggestion) =
                localize_learning_problem(&problem, UiLanguage::Spanish);
            assert_eq!(spanish, "Se esperaba una expresión", "{command}");
            assert_eq!(
                spanish_suggestion.as_deref(),
                Some(spanish_tip),
                "{command}"
            );
        }
    }
}

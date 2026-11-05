use crate::{
    app::UiLanguage,
    features::learning::{
        LearningProblem, LearningProblemKind, LearningProblemText, LearningSeverity,
    },
    ui::components::content_panel::view::localize_learning_problem,
};

fn syntax_problem(message: &str) -> LearningProblem {
    LearningProblem {
        kind: LearningProblemKind::Syntax,
        severity: LearningSeverity::Error,
        line: 1,
        column: 1,
        text: LearningProblemText::InterpreterDiagnostic,
        message: message.to_owned(),
        suggestion: None,
    }
}

fn constant_index_problem() -> LearningProblem {
    LearningProblem {
        kind: LearningProblemKind::Logic,
        severity: LearningSeverity::Error,
        line: 3,
        column: 4,
        text: LearningProblemText::ConstantIndex {
            index: 7,
            name: "dados".to_owned(),
            lower_bound: 1,
            upper_bound: 5,
        },
        message: "O índice 7 está fora dos limites de `dados` (1 a 5).".to_owned(),
        suggestion: Some(
            "Use um índice dentro da faixa declarada para esse vetor ou matriz.".to_owned(),
        ),
    }
}

#[test]
fn translates_expected_token_with_context_into_english_and_spanish() {
    let problem = syntax_problem("esperado '{' após 'programa'");

    assert_eq!(
        localize_learning_problem(&problem, UiLanguage::English).0,
        "Expected '{' after 'programa'."
    );
    assert_eq!(
        localize_learning_problem(&problem, UiLanguage::Spanish).0,
        "Se esperaba '{' después de 'programa'."
    );
}

#[test]
fn translates_common_runtime_diagnostics_into_english_and_spanish() {
    let problem = LearningProblem {
        kind: LearningProblemKind::Runtime,
        ..syntax_problem("estouro na operação inteira")
    };

    assert_eq!(
        localize_learning_problem(&problem, UiLanguage::English).0,
        "Integer operation overflow."
    );
    assert_eq!(
        localize_learning_problem(&problem, UiLanguage::Spanish).0,
        "Desbordamiento en una operación entera."
    );
}

#[test]
fn keeps_unknown_interpreter_detail_labeled_in_portuguese() {
    let problem = syntax_problem("diagnóstico novo do interpretador");
    let (message, detail) = localize_learning_problem(&problem, UiLanguage::English);

    assert_eq!(message, "There is a syntax problem on this line.");
    assert_eq!(
        detail.as_deref(),
        Some("Original interpreter detail (Portuguese): diagnóstico novo do interpretador")
    );
}

#[test]
fn localizes_static_index_findings_and_suggestions() {
    let problem = constant_index_problem();
    let english = localize_learning_problem(&problem, UiLanguage::English);
    let spanish = localize_learning_problem(&problem, UiLanguage::Spanish);

    assert_eq!(
        english.0,
        "Index 7 is outside the bounds of `dados` (1 to 5)."
    );
    assert_eq!(
        english.1.as_deref(),
        Some("Use an index within the declared range for this array or matrix.")
    );
    assert_eq!(
        spanish.0,
        "El índice 7 está fuera de los límites de `dados` (1 a 5)."
    );
    assert_eq!(
        spanish.1.as_deref(),
        Some("Usa un índice dentro del rango declarado para este vector o matriz.")
    );
}

#[test]
fn localizes_constant_condition_findings_and_suggestions() {
    let problem = LearningProblem {
        kind: LearningProblemKind::PossibleLogic,
        severity: LearningSeverity::Warning,
        line: 4,
        column: 7,
        text: LearningProblemText::ConstantCondition { always_true: false },
        message: "A condição é sempre falsa; o bloco pode nunca ser executado.".to_owned(),
        suggestion: Some("Confira se a condição deveria comparar uma variável.".to_owned()),
    };
    let (message, suggestion) = localize_learning_problem(&problem, UiLanguage::English);

    assert_eq!(
        message,
        "This condition is always false; the block may never run."
    );
    assert_eq!(
        suggestion.as_deref(),
        Some("Check whether the condition should compare a variable.")
    );
}

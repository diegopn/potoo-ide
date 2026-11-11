use super::{CurrentLearningAnalysis, LearningAnalysisIdentity, invalidate_learning_hint};
use crate::features::learning::{LearningTopic, learning_hint};
use crate::languages::model::Language;

fn current(language: Language, document_id: u64, line: usize) -> CurrentLearningAnalysis {
    CurrentLearningAnalysis {
        generation: 7,
        active_document_id: Some(document_id),
        learning_document_id: Some(document_id),
        revision: 12,
        line,
        language: Some(language),
    }
}

fn result(language: Language, document_id: u64, line: usize) -> LearningAnalysisIdentity {
    LearningAnalysisIdentity {
        generation: 7,
        document_id,
        revision: 12,
        line,
        language,
    }
}

#[test]
fn accepts_only_the_result_for_the_current_line_and_document() {
    assert!(current(Language::Portugol, 42, 9).accepts(result(Language::Portugol, 42, 9)));
    assert!(!current(Language::Portugol, 42, 10).accepts(result(Language::Portugol, 42, 9)));
    assert!(!current(Language::Portugol, 43, 9).accepts(result(Language::Portugol, 42, 9)));
}

#[test]
fn rejects_results_from_an_old_dialect_revision_or_generation() {
    let current = current(Language::Visualg, 42, 9);
    let mut stale = result(Language::Portugol, 42, 9);
    assert!(!current.accepts(stale));

    stale = result(Language::Visualg, 42, 9);
    stale.revision -= 1;
    assert!(!current.accepts(stale));

    stale = result(Language::Visualg, 42, 9);
    stale.generation -= 1;
    assert!(!current.accepts(stale));
}

#[test]
fn rejects_results_when_no_matching_document_is_active_or_scheduled() {
    let result = result(Language::Portugol, 42, 9);
    let mut current = current(Language::Portugol, 42, 9);
    current.active_document_id = None;
    assert!(!current.accepts(result));

    current.active_document_id = Some(42);
    current.learning_document_id = None;
    assert!(!current.accepts(result));
}

#[test]
fn beginning_a_refresh_clears_the_previous_line_hint_and_invalidates_its_generation() {
    let mut generation = 6;
    let mut hint = Some(learning_hint("escreva(1)", 1, Language::Portugol));

    let next_generation = invalidate_learning_hint(&mut generation, &mut hint);

    assert_eq!(next_generation, 7);
    assert_eq!(generation, 7);
    assert!(hint.is_none());
}

#[test]
fn repeated_learning_requests_use_each_source_revision_without_reusing_previous_facts() {
    let first_revision = learning_hint("escreva \"versão antiga\"", 1, Language::Potigol);
    let next_revision = learning_hint("idade := 42", 1, Language::Potigol);
    let other_document = learning_hint("leia_inteiro()", 1, Language::Potigol);

    assert_eq!(first_revision.topic, LearningTopic::Output);
    assert_eq!(next_revision.topic, LearningTopic::Assignment);
    assert_eq!(next_revision.name.as_deref(), Some("idade"));
    assert_eq!(other_document.topic, LearningTopic::Input);
    assert_eq!(other_document.name.as_deref(), Some("leia_inteiro"));

    let mut current_revision = current(Language::Potigol, 42, 1);
    current_revision.revision += 1;
    assert!(!current_revision.accepts(result(Language::Potigol, 42, 1)));
    let other_document_state = current(Language::Potigol, 43, 1);
    assert!(!other_document_state.accepts(result(Language::Potigol, 42, 1)));
}

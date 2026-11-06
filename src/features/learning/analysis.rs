use super::*;

#[cfg(test)]
pub(crate) fn learning_hint(
    source: &str,
    one_based_line: usize,
    language: Language,
) -> LearningHint {
    learning_hint_with_runtime(source, one_based_line, language, &[])
}

#[cfg(test)]
pub(crate) fn learning_hint_with_runtime(
    source: &str,
    one_based_line: usize,
    language: Language,
    runtime_diagnostics: &[ExecutionDiagnostic],
) -> LearningHint {
    learning_hint_with_runtime_cancellable(
        source,
        one_based_line,
        language,
        runtime_diagnostics,
        || false,
    )
    .expect("an uncancellable analysis always returns a hint")
}

pub(crate) fn learning_hint_with_runtime_cancellable<F>(
    source: &str,
    one_based_line: usize,
    language: Language,
    runtime_diagnostics: &[ExecutionDiagnostic],
    is_cancelled: F,
) -> Option<LearningHint>
where
    F: Fn() -> bool,
{
    if is_cancelled() {
        return None;
    }
    if (language.profile().is_blank_or_line_comment)(source, one_based_line) {
        return Some(LearningHint {
            line: one_based_line.max(1),
            column: 1,
            language,
            topic: LearningTopic::Unrecognized,
            name: None,
            return_type: None,
            target: None,
            parameters: Vec::new(),
            variables: Vec::new(),
            expression: None,
            experiment_expression: None,
            call: None,
            problems: Vec::new(),
        });
    }
    let profile = language.profile();
    let code_line = (profile.source_line_code)(source, one_based_line);
    let line = code_line.trim();
    let (facts, column, mut problems) = match language.profile().learning {
        Some(adapter) => {
            let outcome = adapter(source, one_based_line, &is_cancelled);
            let mapped = recovering_learning_result(outcome, language, source, one_based_line)?;
            (mapped.facts, mapped.column, mapped.problems)
        }
        None => (None, 1, Vec::new()),
    };
    if is_cancelled() {
        return None;
    }
    let statement_range = (profile.source_statement_range)(source, one_based_line, column);
    let statement = statement_range
        .as_ref()
        .and_then(|range| source.get(range.clone()))
        .unwrap_or(line);
    let statement_column = if statement_range.is_some() {
        column
    } else {
        code_line
            .chars()
            .take_while(|character| character.is_whitespace())
            .count()
            + 1
    };
    let mut facts = facts.unwrap_or_else(unrecognized_facts);
    if matches!(facts.topic, LearningTopic::Call | LearningTopic::Input) && facts.call.is_none() {
        facts.call = call_from_source(source, one_based_line, column);
    }
    if facts.topic == LearningTopic::Call
        && facts
            .call
            .as_ref()
            .and_then(|call| catalog::lookup_library_call(language, call))
            .is_some()
    {
        facts.topic = LearningTopic::LibraryCall;
    }
    let expression = match facts.topic {
        LearningTopic::Decision => Some(extract_condition(statement)),
        LearningTopic::Loop => loop_condition(statement),
        _ => expression_for_line(
            statement,
            facts.topic,
            language,
            facts.name.as_deref(),
            facts.call.as_ref(),
        ),
    };
    let experiment_expression =
        matches!(facts.topic, LearningTopic::Decision | LearningTopic::Loop)
            .then(|| expression.as_deref().and_then(suggest_condition_change))
            .flatten();
    let target = if facts.topic == LearningTopic::Input {
        input_target(
            statement,
            language,
            facts.call.as_ref(),
            &facts.variables,
            facts.name.as_deref(),
        )
    } else {
        None
    };
    for diagnostic in runtime_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.line == one_based_line)
    {
        let category = match diagnostic.kind {
            ExecutionDiagnosticKind::Lexical => {
                crate::features::diagnostics::DiagnosticCategory::Lexical
            }
            ExecutionDiagnosticKind::Syntax => {
                crate::features::diagnostics::DiagnosticCategory::Syntax
            }
            ExecutionDiagnosticKind::Type => crate::features::diagnostics::DiagnosticCategory::Type,
            ExecutionDiagnosticKind::Runtime => {
                crate::features::diagnostics::DiagnosticCategory::Runtime
            }
        };
        let kind = learning_problem_kind(category);
        let message = crate::features::diagnostics::adapt_diagnostic_message(
            language,
            source,
            category,
            &diagnostic.message,
            diagnostic.line,
            diagnostic.column,
        );
        if problems.iter().any(|problem: &LearningProblem| {
            problem.kind == kind
                && problem.line == diagnostic.line
                && problem.column == diagnostic.column
                && problem.message == message
        }) {
            continue;
        }
        problems.push(LearningProblem {
            kind,
            severity: LearningSeverity::Error,
            line: diagnostic.line,
            column: diagnostic.column,
            text: LearningProblemText::InterpreterDiagnostic,
            message,
            suggestion: None,
        });
    }
    if facts.topic == LearningTopic::Decision
        && let Some(problem) = constant_condition_problem(statement, one_based_line)
    {
        problems.push(problem);
    }
    if facts.topic != LearningTopic::Variable && facts.topic != LearningTopic::Unrecognized {
        let index_problems = constant_index_problems(
            source,
            statement,
            one_based_line,
            statement_column,
            language,
            &is_cancelled,
        )?;
        problems.extend(index_problems);
    }
    if is_cancelled() {
        return None;
    }
    Some(LearningHint {
        line: one_based_line.max(1),
        column,
        language,
        topic: facts.topic,
        name: facts.name,
        return_type: facts.return_type,
        target,
        parameters: facts.parameters,
        variables: facts.variables,
        expression,
        experiment_expression,
        call: facts.call,
        problems,
    })
}

struct RecoveringLearningResult {
    facts: Option<facts::LearningFacts>,
    column: usize,
    problems: Vec<LearningProblem>,
}

fn recovering_learning_result(
    outcome: SourceLineAnalysisOutcome,
    language: Language,
    source: &str,
    selected_line: usize,
) -> Option<RecoveringLearningResult> {
    match outcome {
        SourceLineAnalysisOutcome::Complete {
            analysis,
            diagnostics,
        } => {
            let selected_diagnostic = diagnostics.iter().find(|diagnostic| {
                diagnostic.line == selected_line
                    && diagnostic.kind
                        != crate::languages::model::SourceDiagnosticKind::StaticAnalysis
            });
            let column = selected_diagnostic
                .map(|diagnostic| diagnostic.column.max(1))
                .or_else(|| analysis.as_ref().map(|analysis| analysis.column))
                .unwrap_or(1);
            let facts = if selected_diagnostic.is_some() {
                None
            } else {
                analysis.map(learning_facts)
            };
            Some(RecoveringLearningResult {
                facts,
                column,
                problems: learning_diagnostic_problems(
                    language,
                    source,
                    selected_line,
                    diagnostics,
                ),
            })
        }
        SourceLineAnalysisOutcome::Incomplete { diagnostics } => {
            let column = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.line == selected_line)
                .map_or(1, |diagnostic| diagnostic.column.max(1));
            let mut problems =
                learning_diagnostic_problems(language, source, selected_line, diagnostics);
            problems.push(LearningProblem {
                kind: LearningProblemKind::Analysis,
                severity: LearningSeverity::Warning,
                line: selected_line.max(1),
                column: 1,
                text: LearningProblemText::AnalysisIncomplete,
                message:
                    "A análise foi interrompida antes de recuperar toda a estrutura do programa."
                        .to_owned(),
                suggestion: None,
            });
            Some(RecoveringLearningResult {
                facts: None,
                column,
                problems,
            })
        }
        SourceLineAnalysisOutcome::Cancelled => None,
    }
}

fn learning_diagnostic_problems(
    language: Language,
    source: &str,
    selected_line: usize,
    diagnostics: Vec<crate::languages::model::SourceDiagnostic>,
) -> Vec<LearningProblem> {
    diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.line == selected_line)
        .map(|diagnostic| {
            let category = match diagnostic.kind {
                crate::languages::model::SourceDiagnosticKind::Lexical => {
                    crate::features::diagnostics::DiagnosticCategory::Lexical
                }
                crate::languages::model::SourceDiagnosticKind::Syntax => {
                    crate::features::diagnostics::DiagnosticCategory::Syntax
                }
                crate::languages::model::SourceDiagnosticKind::Type => {
                    crate::features::diagnostics::DiagnosticCategory::Type
                }
                crate::languages::model::SourceDiagnosticKind::Runtime => {
                    crate::features::diagnostics::DiagnosticCategory::Runtime
                }
                crate::languages::model::SourceDiagnosticKind::StaticAnalysis => {
                    crate::features::diagnostics::DiagnosticCategory::StaticAnalysis
                }
            };
            let message = crate::features::diagnostics::adapt_diagnostic_message(
                language,
                source,
                category,
                &diagnostic.message,
                diagnostic.line,
                diagnostic.column,
            );
            diagnostic_problem(
                learning_problem_kind(category),
                diagnostic.line,
                diagnostic.column,
                message,
            )
        })
        .collect()
}

#[cfg(test)]
mod recovering_adapter_tests {
    use super::*;
    use crate::languages::model::{SourceDiagnostic, SourceDiagnosticKind};

    #[test]
    fn incomplete_recovery_keeps_selected_diagnostic_and_adds_an_analysis_warning() {
        let mapped = recovering_learning_result(
            SourceLineAnalysisOutcome::Incomplete {
                diagnostics: vec![SourceDiagnostic {
                    kind: SourceDiagnosticKind::Syntax,
                    message: "erro sintático".to_owned(),
                    line: 2,
                    column: 3,
                    byte_range: None,
                }],
            },
            Language::Portugol,
            "programa {\n @\n}",
            2,
        )
        .expect("incomplete analysis is shown");

        assert!(mapped.facts.is_none());
        assert_eq!(mapped.column, 3);
        assert!(mapped.problems.iter().any(|problem| {
            problem.kind == LearningProblemKind::Syntax
                && problem.severity == LearningSeverity::Error
                && problem.line == 2
                && problem.column == 3
        }));
        assert!(mapped.problems.iter().any(|problem| {
            problem.kind == LearningProblemKind::Analysis
                && problem.severity == LearningSeverity::Warning
                && problem.line == 2
                && problem.text == LearningProblemText::AnalysisIncomplete
        }));
    }

    #[test]
    fn cancelled_recovery_produces_no_learning_result() {
        assert!(
            recovering_learning_result(
                SourceLineAnalysisOutcome::Cancelled,
                Language::Portugol,
                "programa {}",
                1,
            )
            .is_none()
        );
    }

    #[test]
    fn complete_recovery_keeps_facts_and_only_diagnostics_for_the_selected_line() {
        let mapped = recovering_learning_result(
            SourceLineAnalysisOutcome::Complete {
                analysis: Some(SourceLineAnalysis {
                    line: 3,
                    column: 4,
                    construct: SourceConstruct::Output,
                    name: None,
                    return_type: None,
                    parameters: Vec::new(),
                    variables: Vec::new(),
                    call: None,
                }),
                diagnostics: vec![
                    SourceDiagnostic {
                        kind: SourceDiagnosticKind::Syntax,
                        message: "erro em outra linha".to_owned(),
                        line: 1,
                        column: 1,
                        byte_range: None,
                    },
                    SourceDiagnostic {
                        kind: SourceDiagnosticKind::Syntax,
                        message: "erro em linha anterior".to_owned(),
                        line: 2,
                        column: 4,
                        byte_range: None,
                    },
                ],
            },
            Language::Portugol,
            "programa {\n @\n escreva(1)\n}",
            3,
        )
        .expect("complete analysis is shown");

        assert_eq!(mapped.column, 4);
        assert_eq!(
            mapped.facts.expect("AST facts").topic,
            LearningTopic::Output
        );
        assert!(mapped.problems.is_empty());

        let selected_error = recovering_learning_result(
            SourceLineAnalysisOutcome::Complete {
                analysis: Some(SourceLineAnalysis {
                    line: 2,
                    column: 4,
                    construct: SourceConstruct::Output,
                    name: None,
                    return_type: None,
                    parameters: Vec::new(),
                    variables: Vec::new(),
                    call: None,
                }),
                diagnostics: vec![SourceDiagnostic {
                    kind: SourceDiagnosticKind::Syntax,
                    message: "erro nesta linha".to_owned(),
                    line: 2,
                    column: 4,
                    byte_range: None,
                }],
            },
            Language::Portugol,
            "programa {\n @\n}",
            2,
        )
        .expect("selected error remains visible");
        assert!(selected_error.facts.is_none());
        assert_eq!(selected_error.problems.len(), 1);
        assert_eq!(selected_error.problems[0].line, 2);
    }
}

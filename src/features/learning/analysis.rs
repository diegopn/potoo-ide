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
    if is_blank_or_line_comment(source, one_based_line, language) {
        return Some(LearningHint {
            line: one_based_line.max(1),
            column: 1,
            language,
            topic: LearningTopic::Unrecognized,
            name: None,
            return_type: None,
            parameters: Vec::new(),
            variables: Vec::new(),
            expression: None,
            experiment_expression: None,
            call: None,
            problems: Vec::new(),
        });
    }
    let code_line = match language {
        Language::Portugol => {
            crate::languages::portugol::studio_20::syntax_analysis::views::source_line_code(
                source,
                one_based_line,
            )
        }
        Language::Visualg => {
            crate::languages::portugol::visualg::syntax_analysis::views::source_line_code(
                source,
                one_based_line,
            )
        }
        Language::GPortugol => {
            crate::languages::portugol::g_portugol::syntax_analysis::views::source_line_code(
                source,
                one_based_line,
            )
        }
        Language::Potigol => crate::languages::potigol::syntax_analysis::views::source_line_code(
            source,
            one_based_line,
        ),
        Language::Calango => crate::languages::calango::syntax_analysis::views::source_line_code(
            source,
            one_based_line,
        ),
        Language::Mapler => {
            crate::languages::portugol::mapler::syntax_analysis::views::source_line_code(
                source,
                one_based_line,
            )
        }
    };
    let line = code_line.trim();
    let (facts, column, mut problems) = match language {
        Language::Portugol => {
            match crate::languages::portugol::studio_20::learning::analyze_source_line_cancellable(
                source,
                one_based_line,
                &is_cancelled,
            ) {
                Ok(Some(analysis)) => {
                    let column = analysis.column;
                    (Some(learning_facts(analysis)), column, Vec::new())
                }
                Ok(None) => (None, 1, Vec::new()),
                Err(diagnostics) => {
                    let problems = diagnostics
                        .into_iter()
                        .filter(|diagnostic| diagnostic.line == one_based_line)
                        .map(|diagnostic| LearningProblem {
                            kind: LearningProblemKind::Syntax,
                            severity: LearningSeverity::Error,
                            line: diagnostic.line,
                            column: diagnostic.column,
                            text: LearningProblemText::InterpreterDiagnostic,
                            message: diagnostic.message,
                            suggestion: None,
                        })
                        .collect();
                    (None, 1, problems)
                }
            }
        }
        Language::Visualg => {
            match crate::languages::portugol::visualg::learning::analyze_source_line_cancellable(
                source,
                one_based_line,
                &is_cancelled,
            ) {
                Ok(Some(analysis)) => {
                    let column = analysis.column;
                    (Some(learning_facts(analysis)), column, Vec::new())
                }
                Ok(None) => (None, 1, Vec::new()),
                Err(diagnostics) => {
                    let problems = diagnostics
                        .into_iter()
                        .filter(|diagnostic| diagnostic.line == one_based_line)
                        .map(|diagnostic| LearningProblem {
                            kind: LearningProblemKind::Syntax,
                            severity: LearningSeverity::Error,
                            line: diagnostic.line,
                            column: diagnostic.column,
                            text: LearningProblemText::InterpreterDiagnostic,
                            message: diagnostic.message,
                            suggestion: None,
                        })
                        .collect();
                    (None, 1, problems)
                }
            }
        }
        Language::GPortugol => {
            match crate::languages::portugol::g_portugol::learning::analyze_source_line_cancellable(
                source,
                one_based_line,
                &is_cancelled,
            ) {
                Ok(Some(analysis)) => {
                    let column = analysis.column;
                    (Some(learning_facts(analysis)), column, Vec::new())
                }
                Ok(None) => (None, 1, Vec::new()),
                Err(diagnostics) => {
                    let problems = diagnostics
                        .into_iter()
                        .filter(|diagnostic| diagnostic.span.start.line == one_based_line)
                        .map(|diagnostic| LearningProblem {
                            kind: LearningProblemKind::Syntax,
                            severity: LearningSeverity::Error,
                            line: diagnostic.span.start.line,
                            column: diagnostic.span.start.column,
                            text: LearningProblemText::InterpreterDiagnostic,
                            message: diagnostic.message,
                            suggestion: None,
                        })
                        .collect();
                    (None, 1, problems)
                }
            }
        }
        Language::Potigol => {
            match crate::languages::potigol::learning::analyze_source_line_cancellable(
                source,
                one_based_line,
                &is_cancelled,
            ) {
                Ok(Some(analysis)) => {
                    let column = analysis.column;
                    (Some(learning_facts(analysis)), column, Vec::new())
                }
                Ok(None) => (None, 1, Vec::new()),
                Err(diagnostics) => {
                    let problems = diagnostics
                        .into_iter()
                        .filter(|diagnostic| diagnostic.span.start.line == one_based_line)
                        .map(|diagnostic| {
                            let category = match diagnostic.kind {
                                potoo_potigol_interpreter::DiagnosticKind::Lexical => {
                                    crate::features::diagnostics::DiagnosticCategory::Lexical
                                }
                                potoo_potigol_interpreter::DiagnosticKind::Syntax => {
                                    crate::features::diagnostics::DiagnosticCategory::Syntax
                                }
                                potoo_potigol_interpreter::DiagnosticKind::Type => {
                                    crate::features::diagnostics::DiagnosticCategory::Type
                                }
                                potoo_potigol_interpreter::DiagnosticKind::Runtime => {
                                    crate::features::diagnostics::DiagnosticCategory::Runtime
                                }
                            };
                            let message = crate::features::diagnostics::adapt_diagnostic_message(
                                Language::Potigol,
                                source,
                                category,
                                &diagnostic.message,
                                diagnostic.span.start.line,
                                diagnostic.span.start.column,
                            );
                            diagnostic_problem(
                                learning_problem_kind(category),
                                diagnostic.span.start.line,
                                diagnostic.span.start.column,
                                message,
                            )
                        })
                        .collect();
                    (None, 1, problems)
                }
            }
        }
        Language::Calango => {
            match crate::languages::calango::learning::analyze_source_line_cancellable(
                source,
                one_based_line,
                &is_cancelled,
            ) {
                Ok(Some(analysis)) => {
                    let column = analysis.column;
                    (Some(learning_facts(analysis)), column, Vec::new())
                }
                Ok(None) => (None, 1, Vec::new()),
                Err(diagnostics) => {
                    let problems = diagnostics
                        .into_iter()
                        .filter(|diagnostic| diagnostic.line == one_based_line)
                        .map(|diagnostic| {
                            let category = map_calango_diagnostic_kind(diagnostic.kind);
                            diagnostic_problem(
                                learning_problem_kind(category),
                                diagnostic.line,
                                diagnostic.column,
                                diagnostic.message,
                            )
                        })
                        .collect();
                    (None, 1, problems)
                }
            }
        }
        Language::Mapler => {
            match crate::languages::portugol::mapler::learning::analyze_source_line_cancellable(
                source,
                one_based_line,
                &is_cancelled,
            ) {
                Ok(Some(analysis)) => {
                    let column = analysis.column;
                    (Some(learning_facts(analysis)), column, Vec::new())
                }
                Ok(None) => (None, 1, Vec::new()),
                Err(diagnostics) => {
                    let problems = diagnostics
                        .into_iter()
                        .filter(|diagnostic| diagnostic.line == one_based_line)
                        .map(|diagnostic| {
                            let category = map_mapler_diagnostic_kind(diagnostic.kind);
                            diagnostic_problem(
                                learning_problem_kind(category),
                                diagnostic.line,
                                diagnostic.column,
                                diagnostic.message,
                            )
                        })
                        .collect();
                    (None, 1, problems)
                }
            }
        }
    };
    if is_cancelled() {
        return None;
    }
    let mut facts = facts.unwrap_or_else(unrecognized_facts);
    if facts.topic == LearningTopic::Call && facts.call.is_none() {
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
    let expression = (facts.topic == LearningTopic::Decision).then(|| extract_condition(line));
    let experiment_expression = expression.as_deref().and_then(suggest_condition_change);
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
        && let Some(problem) = constant_condition_problem(line, one_based_line)
    {
        problems.push(problem);
    }
    if facts.topic != LearningTopic::Variable && facts.topic != LearningTopic::Unrecognized {
        let index_problems =
            constant_index_problems(source, line, one_based_line, language, &is_cancelled)?;
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
        parameters: facts.parameters,
        variables: facts.variables,
        expression,
        experiment_expression,
        call: facts.call,
        problems,
    })
}

fn is_blank_or_line_comment(source: &str, line_number: usize, language: Language) -> bool {
    match language {
        Language::Portugol => {
            crate::languages::portugol::studio_20::syntax_analysis::views::is_blank_or_line_comment(
                source,
                line_number,
            )
        }
        Language::Visualg => {
            crate::languages::portugol::visualg::syntax_analysis::views::is_blank_or_line_comment(
                source,
                line_number,
            )
        }
        Language::GPortugol => {
            crate::languages::portugol::g_portugol::syntax_analysis::views::is_blank_or_line_comment(
                source,
                line_number,
            )
        }
        Language::Potigol => {
            crate::languages::potigol::syntax_analysis::views::is_blank_or_line_comment(
                source,
                line_number,
            )
        }
        Language::Calango => {
            crate::languages::calango::syntax_analysis::views::is_blank_or_line_comment(
                source,
                line_number,
            )
        }
        Language::Mapler => {
            crate::languages::portugol::mapler::syntax_analysis::views::is_blank_or_line_comment(
                source,
                line_number,
            )
        }
    }
}

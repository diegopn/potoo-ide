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
            parameters: Vec::new(),
            variables: Vec::new(),
            expression: None,
            experiment_expression: None,
            call: None,
            problems: Vec::new(),
        });
    }
    let code_line = (language.profile().source_line_code)(source, one_based_line);
    let line = code_line.trim();
    let adapter_result = language
        .profile()
        .learning
        .map(|adapter| adapter(source, one_based_line, &is_cancelled));
    let (facts, column, mut problems) = match adapter_result {
        Some(Ok(Some(analysis))) => {
            let column = analysis.column;
            (Some(learning_facts(analysis)), column, Vec::new())
        }
        Some(Ok(None)) | None => (None, 1, Vec::new()),
        Some(Err(diagnostics)) => (
            None,
            1,
            learning_diagnostic_problems(language, source, one_based_line, diagnostics),
        ),
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

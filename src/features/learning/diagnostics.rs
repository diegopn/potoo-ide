use super::*;
use crate::features::execution::{DiagnosticKind as ExecutionDiagnosticKind, ExecutionDiagnostic};
use crate::languages::model::{Language, SourceDiagnostic, SourceDiagnosticKind};

pub(super) fn learning_problem_kind(
    category: crate::features::diagnostics::DiagnosticCategory,
) -> LearningProblemKind {
    match category {
        crate::features::diagnostics::DiagnosticCategory::Lexical
        | crate::features::diagnostics::DiagnosticCategory::Syntax => LearningProblemKind::Syntax,
        crate::features::diagnostics::DiagnosticCategory::Type => LearningProblemKind::Type,
        crate::features::diagnostics::DiagnosticCategory::Runtime => LearningProblemKind::Runtime,
        crate::features::diagnostics::DiagnosticCategory::StaticAnalysis => {
            LearningProblemKind::StaticAnalysis
        }
    }
}

pub(super) fn diagnostic_problem(
    kind: LearningProblemKind,
    line: usize,
    column: usize,
    message: String,
) -> LearningProblem {
    let suggestion = diagnostic_suggestion(kind, &message);
    LearningProblem {
        kind,
        severity: if matches!(
            kind,
            LearningProblemKind::PossibleLogic | LearningProblemKind::StaticAnalysis
        ) {
            LearningSeverity::Warning
        } else {
            LearningSeverity::Error
        },
        line,
        column,
        text: LearningProblemText::InterpreterDiagnostic,
        message,
        suggestion,
    }
}

pub(super) fn diagnostic_suggestion(kind: LearningProblemKind, message: &str) -> Option<String> {
    match (kind, message) {
        (
            LearningProblemKind::Runtime,
            "divisão por zero"
            | "divisão inteira por zero"
            | "division by zero"
            | "integer division by zero",
        ) => Some("Confira se o divisor pode ser zero antes de fazer a divisão.".to_owned()),
        (LearningProblemKind::StaticAnalysis, "Comentário de bloco sem fechamento") => {
            Some("Adicione `*/` para fechar o comentário de bloco.".to_owned())
        }
        _ => None,
    }
}

pub(super) fn append_runtime_diagnostics(
    problems: &mut Vec<LearningProblem>,
    language: Language,
    source: &str,
    selected_line: usize,
    runtime_diagnostics: &[ExecutionDiagnostic],
) {
    for diagnostic in runtime_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.line == selected_line)
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
        if problems.iter().any(|problem| {
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
            suggestion: diagnostic_suggestion(kind, &message),
            message,
        });
    }
}

pub(super) fn learning_diagnostic_problems(
    language: Language,
    source: &str,
    selected_line: usize,
    diagnostics: Vec<SourceDiagnostic>,
) -> Vec<LearningProblem> {
    diagnostics
        .into_iter()
        .filter(|diagnostic| diagnostic.line == selected_line)
        .map(|diagnostic| {
            let category = match diagnostic.kind {
                SourceDiagnosticKind::Lexical => {
                    crate::features::diagnostics::DiagnosticCategory::Lexical
                }
                SourceDiagnosticKind::Syntax => {
                    crate::features::diagnostics::DiagnosticCategory::Syntax
                }
                SourceDiagnosticKind::Type => {
                    crate::features::diagnostics::DiagnosticCategory::Type
                }
                SourceDiagnosticKind::Runtime => {
                    crate::features::diagnostics::DiagnosticCategory::Runtime
                }
                SourceDiagnosticKind::StaticAnalysis => {
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
            let output_command = source
                .lines()
                .nth(diagnostic.line.saturating_sub(1))
                .map(str::trim)
                .filter(|command| matches!(*command, "escreva" | "imprima"));
            let output_suggestion = (language == Language::Potigol
                && category == crate::features::diagnostics::DiagnosticCategory::Syntax
                && message == "esperava uma expressão"
                && output_command.is_some())
            .then(|| {
                format!(
                    "Informe o valor ou a expressão que será exibida após `{}`.",
                    output_command.expect("the output command was checked")
                )
            });
            let mut problem = diagnostic_problem(
                learning_problem_kind(category),
                diagnostic.line,
                diagnostic.column,
                message,
            );
            if output_suggestion.is_some() {
                problem.suggestion = output_suggestion;
            }
            problem
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::execution::{
        DiagnosticKind as ExecutionDiagnosticKind, ExecutionDiagnostic,
    };
    use crate::languages::model::{Language, SourceDiagnostic, SourceDiagnosticKind};

    #[test]
    fn parser_conversion_keeps_location_and_potigol_output_advice() {
        let source = "escreva\n";
        let problems = learning_diagnostic_problems(
            Language::Potigol,
            source,
            1,
            vec![SourceDiagnostic {
                kind: SourceDiagnosticKind::Syntax,
                message: "esperava uma expressão".to_owned(),
                line: 1,
                column: 8,
                byte_range: None,
            }],
        );

        assert_eq!(problems.len(), 1);
        let problem = &problems[0];
        assert_eq!(problem.kind, LearningProblemKind::Syntax);
        assert_eq!(problem.severity, LearningSeverity::Error);
        assert_eq!((problem.line, problem.column), (1, 8));
        assert_eq!(problem.message, "esperava uma expressão");
        assert_eq!(
            problem.suggestion.as_deref(),
            Some("Informe o valor ou a expressão que será exibida após `escreva`.")
        );
    }

    #[test]
    fn parser_category_and_problem_conversion_keep_their_severity() {
        assert_eq!(
            learning_problem_kind(crate::features::diagnostics::DiagnosticCategory::Lexical),
            LearningProblemKind::Syntax
        );
        assert_eq!(
            learning_problem_kind(crate::features::diagnostics::DiagnosticCategory::Type),
            LearningProblemKind::Type
        );
        assert_eq!(
            diagnostic_problem(
                LearningProblemKind::StaticAnalysis,
                2,
                4,
                "Comentário de bloco sem fechamento".to_owned(),
            )
            .severity,
            LearningSeverity::Warning
        );
        assert_eq!(
            diagnostic_problem(
                LearningProblemKind::Runtime,
                2,
                4,
                "falha de execução".to_owned(),
            )
            .severity,
            LearningSeverity::Error
        );
    }

    #[test]
    fn diagnostic_suggestions_remain_specific_to_known_messages() {
        assert_eq!(
            diagnostic_suggestion(LearningProblemKind::Runtime, "division by zero").as_deref(),
            Some("Confira se o divisor pode ser zero antes de fazer a divisão.")
        );
        assert_eq!(
            diagnostic_suggestion(
                LearningProblemKind::StaticAnalysis,
                "Comentário de bloco sem fechamento",
            )
            .as_deref(),
            Some("Adicione `*/` para fechar o comentário de bloco.")
        );
        assert_eq!(
            diagnostic_suggestion(LearningProblemKind::Type, "tipo incompatível"),
            None
        );
    }

    #[test]
    fn duplicate_runtime_diagnostics_are_not_added_twice() {
        let language = Language::Visualg;
        let source = "linha anterior\nlinha atual";
        let message = "índice fora dos limites";
        let adapted = crate::features::diagnostics::adapt_diagnostic_message(
            language,
            source,
            crate::features::diagnostics::DiagnosticCategory::Runtime,
            message,
            2,
            4,
        );
        let mut problems = vec![diagnostic_problem(
            LearningProblemKind::Runtime,
            2,
            4,
            adapted,
        )];
        let runtime_diagnostics = [ExecutionDiagnostic {
            kind: ExecutionDiagnosticKind::Runtime,
            message: message.to_owned(),
            line: 2,
            column: 4,
        }];

        append_runtime_diagnostics(&mut problems, language, source, 2, &runtime_diagnostics);

        assert_eq!(problems.len(), 1);
    }
}

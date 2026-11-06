use crate::languages::model::{
    SourceCall, SourceConstruct, SourceDiagnostic, SourceDiagnosticKind, SourceDimension,
    SourceLineAnalysis, SourceLineAnalysisOutcome, SourceSymbol,
};
use crate::languages::source::{RecoveryStatus, SourceIndex, recover_parse};
use potoo_mapler_interpreter::{Diagnostic, DiagnosticKind, ast::*, parse};

pub(crate) fn profile_adapter(
    source: &str,
    one_based_line: usize,
    is_cancelled: &dyn Fn() -> bool,
) -> SourceLineAnalysisOutcome {
    if is_cancelled() {
        return SourceLineAnalysisOutcome::Cancelled;
    }
    let source_index = SourceIndex::new_lf_only(source);
    if one_based_line == 0 || one_based_line > source_index.line_count() {
        return SourceLineAnalysisOutcome::Complete {
            analysis: None,
            diagnostics: Vec::new(),
        };
    }

    let recovered = recover_parse(
        source,
        parse,
        |diagnostic: &Diagnostic| (diagnostic.line, diagnostic.column),
        crate::languages::portugol::mapler::syntax_analysis::views::mask_diagnostic_statement,
        is_cancelled,
    );
    let diagnostics = recovered
        .diagnostics
        .into_iter()
        .map(to_source_diagnostic)
        .collect::<Vec<_>>();

    match recovered.status {
        RecoveryStatus::Cancelled => SourceLineAnalysisOutcome::Cancelled,
        RecoveryStatus::Stalled | RecoveryStatus::LimitReached => {
            SourceLineAnalysisOutcome::Incomplete { diagnostics }
        }
        RecoveryStatus::Complete => {
            let Some(program) = recovered.program else {
                return SourceLineAnalysisOutcome::Incomplete { diagnostics };
            };
            let analysis = if diagnostics
                .iter()
                .any(|diagnostic| diagnostic.line == one_based_line)
            {
                None
            } else {
                analyze_program_line(&program, one_based_line)
            };
            SourceLineAnalysisOutcome::Complete {
                analysis,
                diagnostics,
            }
        }
    }
}

fn to_source_diagnostic(diagnostic: Diagnostic) -> SourceDiagnostic {
    let kind = match diagnostic.kind {
        DiagnosticKind::Lexical => SourceDiagnosticKind::Lexical,
        DiagnosticKind::Syntax => SourceDiagnosticKind::Syntax,
        DiagnosticKind::Type => SourceDiagnosticKind::Type,
        DiagnosticKind::Runtime => SourceDiagnosticKind::Runtime,
    };
    SourceDiagnostic::at_line(kind, diagnostic.message, diagnostic.line, diagnostic.column)
}

fn analyze_program_line(program: &Program, one_based_line: usize) -> Option<SourceLineAnalysis> {
    if let Some(variable) = program
        .variables
        .iter()
        .find(|variable| variable.line == one_based_line)
    {
        return Some(SourceLineAnalysis {
            line: variable.line,
            column: variable.column,
            construct: SourceConstruct::VariableDeclaration,
            name: None,
            return_type: None,
            parameters: Vec::new(),
            variables: program
                .variables
                .iter()
                .filter(|candidate| candidate.line == one_based_line)
                .map(source_variable)
                .collect(),
            call: None,
        });
    }

    if let Some(module) = program
        .modules
        .iter()
        .find(|module| module.line == one_based_line)
    {
        return Some(SourceLineAnalysis {
            line: module.line,
            column: module.column,
            construct: SourceConstruct::Unrecognized,
            name: Some(module.name.clone()),
            return_type: None,
            parameters: Vec::new(),
            variables: Vec::new(),
            call: None,
        });
    }

    for module in &program.modules {
        if let Some(analysis) = analyze_statements(&module.statements, one_based_line) {
            return Some(analysis);
        }
    }
    analyze_statements(&program.statements, one_based_line)
}

fn analyze_statements(statements: &[Statement], line: usize) -> Option<SourceLineAnalysis> {
    for statement in statements {
        if statement.line == line {
            let construct = match &statement.kind {
                StatementKind::Write(_) => SourceConstruct::Output,
                StatementKind::Read(_) => SourceConstruct::Input,
                StatementKind::Assign { .. } => SourceConstruct::Assignment,
                StatementKind::If { .. } => SourceConstruct::Decision,
                StatementKind::While { .. }
                | StatementKind::Repeat { .. }
                | StatementKind::For { .. } => SourceConstruct::Loop,
                StatementKind::CallModule(_) => SourceConstruct::Call,
            };
            let call = match &statement.kind {
                StatementKind::CallModule(name) => Some(SourceCall {
                    receiver: None,
                    member: name.clone(),
                    arguments: Vec::new(),
                    library: None,
                }),
                _ => None,
            };
            if construct != SourceConstruct::Unrecognized {
                return Some(SourceLineAnalysis {
                    line,
                    column: statement.column,
                    construct,
                    name: match &statement.kind {
                        StatementKind::Write(_) => Some("escrever".to_owned()),
                        StatementKind::Read(_) => Some("ler".to_owned()),
                        StatementKind::If { .. } => Some("se".to_owned()),
                        StatementKind::While { .. } => Some("enquanto".to_owned()),
                        StatementKind::Repeat { .. } => Some("repita".to_owned()),
                        StatementKind::For { .. } => Some("para".to_owned()),
                        StatementKind::CallModule(name) => Some(name.clone()),
                        StatementKind::Assign { target, .. } => expression_name(target),
                    },
                    return_type: None,
                    parameters: Vec::new(),
                    variables: Vec::new(),
                    call,
                });
            }
        }

        let nested = match &statement.kind {
            StatementKind::If {
                then_branch,
                else_branch,
                ..
            } => analyze_statements(then_branch, line)
                .or_else(|| analyze_statements(else_branch, line)),
            StatementKind::While { body, .. }
            | StatementKind::Repeat { body, .. }
            | StatementKind::For { body, .. } => analyze_statements(body, line),
            _ => None,
        };
        if nested.is_some() {
            return nested;
        }
    }
    None
}

fn expression_name(expression: &Expression) -> Option<String> {
    match &expression.kind {
        ExpressionKind::Identifier(name) | ExpressionKind::Index { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn source_variable(variable: &VariableDeclaration) -> SourceSymbol {
    SourceSymbol {
        name: variable.name.clone(),
        type_name: match variable.data_type {
            Type::Integer => "inteiro",
            Type::Real => "real",
            Type::Logical => "logico",
            Type::Chain => "cadeia",
            Type::Character => "caractere",
        }
        .to_owned(),
        is_constant: false,
        by_reference: false,
        dimensions: variable
            .vector
            .map(|bounds| {
                vec![SourceDimension {
                    lower_bound: Some(bounds.lower),
                    upper_bound: Some(bounds.upper),
                }]
            })
            .unwrap_or_default(),
        initializer: None,
    }
}

#[cfg(test)]
mod tests {
    use crate::features::learning::{LearningTopic, learning_hint};
    use crate::languages::model::{Language, SourceConstruct, SourceLineAnalysisOutcome};

    use super::profile_adapter;

    #[test]
    fn recognizes_mapler_output_and_module_calls() {
        let source = "variaveis\n  x: inteiro;\ninicio\n  escrever \"Olá\";\nfim.";
        assert_eq!(
            learning_hint(source, 4, Language::Mapler).topic,
            LearningTopic::Output
        );
    }

    #[test]
    fn shared_learning_recovery_preserves_mapler_error_kinds_and_later_lines() {
        let cases = [
            (
                "variaveis\nx: inteiro;\ninicio\n@;\nescrever \"depois\";\nfim.",
                4,
                5,
                potoo_mapler_interpreter::DiagnosticKind::Lexical,
            ),
            (
                "variaveis\nx inteiro;\ninicio\nescrever \"depois\";\nfim.",
                2,
                4,
                potoo_mapler_interpreter::DiagnosticKind::Syntax,
            ),
        ];

        for (source, error_line, analysis_line, expected_kind) in cases {
            let expected = potoo_mapler_interpreter::parse(source)
                .expect_err("the fixture has one original parser diagnostic")
                .remove(0);
            assert_eq!(expected.kind, expected_kind);
            let outcome = profile_adapter(source, analysis_line, &|| false);
            let SourceLineAnalysisOutcome::Complete {
                analysis: Some(analysis),
                diagnostics,
            } = outcome
            else {
                panic!("recovery should analyze a valid later line: {outcome:?}");
            };

            assert_eq!(analysis.construct, SourceConstruct::Output);
            let recovered_diagnostic = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.line == error_line)
                .expect("the original error remains visible after recovery");
            assert_eq!(recovered_diagnostic.message, expected.message);
            assert_eq!(recovered_diagnostic.column, expected.column);
            assert_eq!(recovered_diagnostic.byte_range, None);
            assert_eq!(
                recovered_diagnostic.kind,
                match expected_kind {
                    potoo_mapler_interpreter::DiagnosticKind::Lexical => {
                        crate::languages::model::SourceDiagnosticKind::Lexical
                    }
                    potoo_mapler_interpreter::DiagnosticKind::Syntax => {
                        crate::languages::model::SourceDiagnosticKind::Syntax
                    }
                    _ => unreachable!("the fixture covers lexical and syntax errors only"),
                }
            );
        }
    }
}

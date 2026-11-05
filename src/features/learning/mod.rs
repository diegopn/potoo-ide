use std::collections::HashMap;

use crate::{
    features::execution::{DiagnosticKind as ExecutionDiagnosticKind, ExecutionDiagnostic},
    languages::{
        model::{Language, SourceDimension},
        source::{read_identifier, read_integer, skip_spaces},
    },
};

pub(crate) use crate::languages::model::{
    SourceCall, SourceConstruct, SourceLineAnalysis, SourceSymbol,
};

pub(crate) mod catalog;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LearningTopic {
    Function,
    Procedure,
    Decision,
    Loop,
    Input,
    Output,
    Variable,
    Assignment,
    Call,
    LibraryCall,
    Import,
    Return,
    Program,
    Unrecognized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LearningProblemKind {
    Syntax,
    Type,
    Runtime,
    Logic,
    PossibleLogic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LearningProblemText {
    InterpreterDiagnostic,
    ConstantCondition {
        always_true: bool,
    },
    ConstantIndex {
        index: i64,
        name: String,
        lower_bound: i64,
        upper_bound: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LearningSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearningProblem {
    pub(crate) kind: LearningProblemKind,
    pub(crate) severity: LearningSeverity,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) text: LearningProblemText,
    pub(crate) message: String,
    pub(crate) suggestion: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LearningDimension {
    pub(crate) lower_bound: Option<i64>,
    pub(crate) upper_bound: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearningSymbol {
    pub(crate) name: String,
    pub(crate) type_name: String,
    pub(crate) is_constant: bool,
    pub(crate) by_reference: bool,
    pub(crate) dimensions: Vec<LearningDimension>,
    pub(crate) initializer: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearningHint {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) language: Language,
    pub(crate) topic: LearningTopic,
    pub(crate) name: Option<String>,
    pub(crate) return_type: Option<String>,
    pub(crate) parameters: Vec<LearningSymbol>,
    pub(crate) variables: Vec<LearningSymbol>,
    pub(crate) expression: Option<String>,
    pub(crate) experiment_expression: Option<String>,
    pub(crate) call: Option<SourceCall>,
    pub(crate) problems: Vec<LearningProblem>,
}

struct LearningFacts {
    topic: LearningTopic,
    name: Option<String>,
    return_type: Option<String>,
    parameters: Vec<LearningSymbol>,
    variables: Vec<LearningSymbol>,
    call: Option<SourceCall>,
}

pub(crate) fn source_call(
    source: &str,
    line: usize,
    column: usize,
    receiver: Option<String>,
    member: impl Into<String>,
    library: Option<String>,
) -> SourceCall {
    SourceCall {
        receiver,
        member: member.into(),
        arguments: call_arguments(source, line, column),
        library,
    }
}

fn call_arguments(source: &str, line: usize, column: usize) -> Vec<String> {
    let line_start = source
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    let Some(source_line) = source.lines().nth(line.saturating_sub(1)) else {
        return Vec::new();
    };
    let char_offset = column.saturating_sub(1);
    let byte_offset = source_line
        .char_indices()
        .nth(char_offset)
        .map_or(source_line.len(), |(offset, _)| offset);
    let Some(open_offset) = source.get(line_start + byte_offset..).and_then(|tail| {
        tail.char_indices()
            .find(|(_, character)| *character == '(')
            .map(|(offset, _)| line_start + byte_offset + offset)
    }) else {
        return Vec::new();
    };

    let mut arguments = Vec::new();
    let mut argument_start = open_offset + 1;
    let mut delimiter_depth = 1usize;
    let mut quote = None;
    let mut escaped = false;
    for (relative_offset, character) in source[open_offset + 1..].char_indices() {
        let offset = open_offset + 1 + relative_offset;
        if let Some(quote_character) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == quote_character {
                quote = None;
            }
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = Some(character);
            continue;
        }
        match character {
            '(' | '[' | '{' => delimiter_depth += 1,
            ')' if delimiter_depth == 1 => {
                let argument = source[argument_start..offset].trim();
                if !argument.is_empty() {
                    arguments.push(argument.to_owned());
                }
                return arguments;
            }
            ')' | ']' | '}' => delimiter_depth = delimiter_depth.saturating_sub(1),
            ',' if delimiter_depth == 1 => {
                arguments.push(source[argument_start..offset].trim().to_owned());
                argument_start = offset + character.len_utf8();
            }
            '#' | '\n' if delimiter_depth == 1 => return arguments,
            _ => {}
        }
    }
    arguments
}

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
                    (Some(portugol_facts(analysis)), column, Vec::new())
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
                    (Some(visualg_facts(analysis)), column, Vec::new())
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
                    (Some(gportugol_facts(analysis)), column, Vec::new())
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
                    (Some(potigol_facts(analysis)), column, Vec::new())
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
                    (Some(calango_facts(analysis)), column, Vec::new())
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
                    (Some(mapler_facts(analysis)), column, Vec::new())
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

fn portugol_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    let topic = match analysis.construct {
        SourceConstruct::Function => LearningTopic::Function,
        SourceConstruct::Procedure => LearningTopic::Procedure,
        SourceConstruct::VariableDeclaration => LearningTopic::Variable,
        SourceConstruct::Assignment => LearningTopic::Assignment,
        SourceConstruct::Input => LearningTopic::Input,
        SourceConstruct::Output => LearningTopic::Output,
        SourceConstruct::Decision => LearningTopic::Decision,
        SourceConstruct::Loop => LearningTopic::Loop,
        SourceConstruct::Return => LearningTopic::Return,
        SourceConstruct::Call | SourceConstruct::MethodCall => LearningTopic::Call,
        SourceConstruct::Import => LearningTopic::Import,
        SourceConstruct::Program => LearningTopic::Program,
        SourceConstruct::Unrecognized | SourceConstruct::Other => LearningTopic::Unrecognized,
    };
    LearningFacts {
        topic,
        name: analysis.name,
        return_type: analysis.return_type,
        parameters: analysis
            .parameters
            .into_iter()
            .map(learning_symbol)
            .collect(),
        variables: analysis
            .variables
            .into_iter()
            .map(learning_symbol)
            .collect(),
        call: analysis.call,
    }
}

fn visualg_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    let topic = match analysis.construct {
        SourceConstruct::Function => LearningTopic::Function,
        SourceConstruct::Procedure => LearningTopic::Procedure,
        SourceConstruct::VariableDeclaration => LearningTopic::Variable,
        SourceConstruct::Assignment => LearningTopic::Assignment,
        SourceConstruct::Input => LearningTopic::Input,
        SourceConstruct::Output => LearningTopic::Output,
        SourceConstruct::Decision => LearningTopic::Decision,
        SourceConstruct::Loop => LearningTopic::Loop,
        SourceConstruct::Return => LearningTopic::Return,
        SourceConstruct::Call | SourceConstruct::MethodCall => LearningTopic::Call,
        SourceConstruct::Import => LearningTopic::Import,
        SourceConstruct::Program => LearningTopic::Program,
        SourceConstruct::Unrecognized | SourceConstruct::Other => LearningTopic::Unrecognized,
    };
    LearningFacts {
        topic,
        name: analysis.name,
        return_type: analysis.return_type,
        parameters: analysis
            .parameters
            .into_iter()
            .map(learning_symbol)
            .collect(),
        variables: analysis
            .variables
            .into_iter()
            .map(learning_symbol)
            .collect(),
        call: analysis.call,
    }
}

fn gportugol_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    let topic = match analysis.construct {
        SourceConstruct::Function => LearningTopic::Function,
        SourceConstruct::Procedure => LearningTopic::Procedure,
        SourceConstruct::VariableDeclaration => LearningTopic::Variable,
        SourceConstruct::Assignment => LearningTopic::Assignment,
        SourceConstruct::Input => LearningTopic::Input,
        SourceConstruct::Output => LearningTopic::Output,
        SourceConstruct::Decision => LearningTopic::Decision,
        SourceConstruct::Loop => LearningTopic::Loop,
        SourceConstruct::Return => LearningTopic::Return,
        SourceConstruct::Call | SourceConstruct::MethodCall => LearningTopic::Call,
        SourceConstruct::Import => LearningTopic::Import,
        SourceConstruct::Program => LearningTopic::Program,
        SourceConstruct::Unrecognized | SourceConstruct::Other => LearningTopic::Unrecognized,
    };
    LearningFacts {
        topic,
        name: analysis.name,
        return_type: analysis.return_type,
        parameters: analysis
            .parameters
            .into_iter()
            .map(learning_symbol)
            .collect(),
        variables: analysis
            .variables
            .into_iter()
            .map(learning_symbol)
            .collect(),
        call: analysis.call,
    }
}

fn potigol_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    basic_facts(analysis)
}

fn calango_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    basic_facts(analysis)
}

fn mapler_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    basic_facts(analysis)
}

fn basic_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    let topic = match analysis.construct {
        SourceConstruct::Program => LearningTopic::Program,
        SourceConstruct::Function => LearningTopic::Function,
        SourceConstruct::Procedure => LearningTopic::Procedure,
        SourceConstruct::VariableDeclaration => LearningTopic::Variable,
        SourceConstruct::Assignment => LearningTopic::Assignment,
        SourceConstruct::Input => LearningTopic::Input,
        SourceConstruct::Output => LearningTopic::Output,
        SourceConstruct::Decision => LearningTopic::Decision,
        SourceConstruct::Loop => LearningTopic::Loop,
        SourceConstruct::Return => LearningTopic::Return,
        SourceConstruct::Call | SourceConstruct::MethodCall => LearningTopic::Call,
        SourceConstruct::Import => LearningTopic::Import,
        SourceConstruct::Unrecognized | SourceConstruct::Other => LearningTopic::Unrecognized,
    };
    LearningFacts {
        topic,
        name: analysis.name,
        return_type: analysis.return_type,
        parameters: analysis
            .parameters
            .into_iter()
            .map(learning_symbol)
            .collect(),
        variables: analysis
            .variables
            .into_iter()
            .map(learning_symbol)
            .collect(),
        call: analysis.call,
    }
}

fn learning_symbol(symbol: SourceSymbol) -> LearningSymbol {
    LearningSymbol {
        name: symbol.name,
        type_name: symbol.type_name,
        is_constant: symbol.is_constant,
        by_reference: symbol.by_reference,
        dimensions: symbol
            .dimensions
            .into_iter()
            .map(|dimension| LearningDimension {
                lower_bound: dimension.lower_bound,
                upper_bound: dimension.upper_bound,
            })
            .collect(),
        initializer: symbol.initializer,
    }
}

fn unrecognized_facts() -> LearningFacts {
    LearningFacts {
        topic: LearningTopic::Unrecognized,
        name: None,
        return_type: None,
        parameters: Vec::new(),
        variables: Vec::new(),
        call: None,
    }
}

fn call_from_source(source: &str, line: usize, column: usize) -> Option<SourceCall> {
    let source_line = source.lines().nth(line.saturating_sub(1))?;
    let start = source_line
        .char_indices()
        .nth(column.saturating_sub(1))
        .map_or(source_line.len(), |(offset, _)| offset);
    let remaining = source_line.get(start..)?;
    let open = remaining.find('(')?;
    let callee = remaining[..open].split_whitespace().last()?.trim();
    let (receiver, member) = callee
        .rsplit_once('.')
        .map_or((None, callee), |(receiver, member)| {
            (Some(receiver.to_owned()), member)
        });
    Some(source_call(source, line, column, receiver, member, None))
}

fn learning_problem_kind(
    category: crate::features::diagnostics::DiagnosticCategory,
) -> LearningProblemKind {
    match category {
        crate::features::diagnostics::DiagnosticCategory::Lexical
        | crate::features::diagnostics::DiagnosticCategory::Syntax => LearningProblemKind::Syntax,
        crate::features::diagnostics::DiagnosticCategory::Type => LearningProblemKind::Type,
        crate::features::diagnostics::DiagnosticCategory::Runtime => LearningProblemKind::Runtime,
        crate::features::diagnostics::DiagnosticCategory::StaticAnalysis => {
            LearningProblemKind::PossibleLogic
        }
    }
}

fn diagnostic_problem(
    kind: LearningProblemKind,
    line: usize,
    column: usize,
    message: String,
) -> LearningProblem {
    LearningProblem {
        kind,
        severity: LearningSeverity::Error,
        line,
        column,
        text: LearningProblemText::InterpreterDiagnostic,
        message,
        suggestion: None,
    }
}

fn map_calango_diagnostic_kind(
    kind: potoo_calango_interpreter::DiagnosticKind,
) -> crate::features::diagnostics::DiagnosticCategory {
    match kind {
        potoo_calango_interpreter::DiagnosticKind::Lexical => {
            crate::features::diagnostics::DiagnosticCategory::Lexical
        }
        potoo_calango_interpreter::DiagnosticKind::Syntax => {
            crate::features::diagnostics::DiagnosticCategory::Syntax
        }
        potoo_calango_interpreter::DiagnosticKind::Type => {
            crate::features::diagnostics::DiagnosticCategory::Type
        }
        potoo_calango_interpreter::DiagnosticKind::Runtime => {
            crate::features::diagnostics::DiagnosticCategory::Runtime
        }
    }
}

fn map_mapler_diagnostic_kind(
    kind: potoo_mapler_interpreter::DiagnosticKind,
) -> crate::features::diagnostics::DiagnosticCategory {
    match kind {
        potoo_mapler_interpreter::DiagnosticKind::Lexical => {
            crate::features::diagnostics::DiagnosticCategory::Lexical
        }
        potoo_mapler_interpreter::DiagnosticKind::Syntax => {
            crate::features::diagnostics::DiagnosticCategory::Syntax
        }
        potoo_mapler_interpreter::DiagnosticKind::Type => {
            crate::features::diagnostics::DiagnosticCategory::Type
        }
        potoo_mapler_interpreter::DiagnosticKind::Runtime => {
            crate::features::diagnostics::DiagnosticCategory::Runtime
        }
    }
}

fn constant_condition_problem(line: &str, line_number: usize) -> Option<LearningProblem> {
    let condition = extract_condition(line);
    let normalized = condition.to_lowercase();
    if !matches!(
        normalized.as_str(),
        "verdadeiro" | "falso" | "true" | "false"
    ) {
        return None;
    }
    Some(LearningProblem {
        kind: LearningProblemKind::PossibleLogic,
        severity: LearningSeverity::Warning,
        line: line_number,
        column: character_column(line, &condition),
        text: LearningProblemText::ConstantCondition {
            always_true: matches!(normalized.as_str(), "verdadeiro" | "true"),
        },
        message: if matches!(normalized.as_str(), "verdadeiro" | "true") {
            "A condição é sempre verdadeira; o ramo alternativo pode nunca ser executado."
                .to_owned()
        } else {
            "A condição é sempre falsa; o bloco pode nunca ser executado.".to_owned()
        },
        suggestion: Some("Confira se a condição deveria comparar uma variável.".to_owned()),
    })
}

fn constant_index_problems(
    source: &str,
    selected_line: &str,
    line_number: usize,
    language: Language,
    is_cancelled: &impl Fn() -> bool,
) -> Option<Vec<LearningProblem>> {
    let source_code = without_literals_and_comments(source, language);
    let arrays = declared_array_bounds(&source_code, language, is_cancelled)?;
    let code = source_code
        .lines()
        .nth(line_number.saturating_sub(1))
        .unwrap_or(selected_line);
    let characters = code.chars().collect::<Vec<_>>();
    let mut problems = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if is_cancelled() {
            return None;
        }
        skip_spaces(&characters, &mut index);
        let name_start = index;
        let Some(name) = read_identifier(&characters, &mut index) else {
            index += 1;
            continue;
        };
        let Some(dimensions) = arrays.get(&name.to_lowercase()) else {
            continue;
        };
        skip_spaces(&characters, &mut index);
        let mut dimension_index = 0;
        while characters.get(index) == Some(&'[') {
            index += 1;
            skip_spaces(&characters, &mut index);
            let Some(value) = read_integer(&characters, &mut index) else {
                break;
            };
            skip_spaces(&characters, &mut index);
            if characters.get(index) != Some(&']') {
                break;
            }
            index += 1;
            if let Some(dimension) = dimensions.get(dimension_index)
                && let (Some(lower), Some(upper)) = (dimension.lower_bound, dimension.upper_bound)
                && !(lower..=upper).contains(&value)
            {
                problems.push(LearningProblem {
                    kind: LearningProblemKind::Logic,
                    severity: LearningSeverity::Error,
                    line: line_number,
                    column: name_start + 1,
                    text: LearningProblemText::ConstantIndex {
                        index: value,
                        name: name.clone(),
                        lower_bound: lower,
                        upper_bound: upper,
                    },
                    message: format!(
                        "O índice {value} está fora dos limites de `{name}` ({lower} a {upper})."
                    ),
                    suggestion: Some(
                        "Use um índice dentro da faixa declarada para esse vetor ou matriz."
                            .to_owned(),
                    ),
                });
            }
            dimension_index += 1;
            skip_spaces(&characters, &mut index);
        }
    }
    Some(problems)
}

fn declared_array_bounds(
    source: &str,
    language: Language,
    is_cancelled: &impl Fn() -> bool,
) -> Option<HashMap<String, Vec<SourceDimension>>> {
    match language {
        Language::Portugol => {
            crate::languages::portugol::studio_20::learning::declared_array_bounds(
                source,
                is_cancelled,
            )
        }
        Language::Visualg => crate::languages::portugol::visualg::learning::declared_array_bounds(
            source,
            is_cancelled,
        ),
        Language::Calango => {
            crate::languages::calango::learning::declared_array_bounds(source, is_cancelled)
        }
        Language::Potigol | Language::GPortugol | Language::Mapler => Some(HashMap::new()),
    }
}

fn without_literals_and_comments(source: &str, language: Language) -> String {
    match language {
        Language::Portugol => {
            crate::languages::portugol::studio_20::syntax_analysis::views::without_literals_and_comments(source)
        }
        Language::Visualg => {
            crate::languages::portugol::visualg::syntax_analysis::views::without_literals_and_comments(source)
        }
        Language::GPortugol => {
            crate::languages::portugol::g_portugol::syntax_analysis::views::without_literals_and_comments(source)
        }
        Language::Potigol => {
            crate::languages::potigol::syntax_analysis::views::without_literals_and_comments(source)
        }
        Language::Calango => {
            crate::languages::calango::syntax_analysis::views::without_literals_and_comments(source)
        }
        Language::Mapler => {
            crate::languages::portugol::mapler::syntax_analysis::views::without_literals_and_comments(source)
        }
    }
}

fn character_column(line: &str, fragment: &str) -> usize {
    line.find(fragment)
        .map(|byte_offset| line[..byte_offset].chars().count() + 1)
        .unwrap_or(1)
}

fn extract_condition(line: &str) -> String {
    let mut condition = line.trim();
    let lowercase = condition.to_lowercase();
    let keyword_length = if lowercase.starts_with("se ") {
        Some(3)
    } else if lowercase.starts_with("escolha ") {
        Some("escolha ".len())
    } else {
        None
    };
    if let Some(keyword_length) = keyword_length {
        condition = condition[keyword_length..].trim();
    }
    if let Some(end) = condition
        .to_lowercase()
        .find(" então")
        .or_else(|| condition.to_lowercase().find(" entao"))
    {
        condition = condition[..end].trim();
    }
    condition = condition.trim_end_matches(['{', ';']).trim();
    condition.trim_matches(['(', ')', ' ']).to_owned()
}

fn suggest_condition_change(condition: &str) -> Option<String> {
    let (start, number) = condition.char_indices().find_map(|(index, character)| {
        if character.is_ascii_digit() {
            let end = condition[index..]
                .find(|next: char| !next.is_ascii_digit())
                .map_or(condition.len(), |offset| index + offset);
            condition[index..end]
                .parse::<i64>()
                .ok()
                .map(|number| (index, number))
        } else {
            None
        }
    })?;
    let end = condition[start..]
        .find(|next: char| !next.is_ascii_digit())
        .map_or(condition.len(), |offset| start + offset);
    let replacement = number.saturating_sub(2).to_string();
    Some(format!(
        "{}{}{}",
        &condition[..start],
        replacement,
        &condition[end..]
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        LearningProblemKind, LearningSeverity, LearningTopic, learning_hint,
        learning_hint_with_runtime,
    };
    use crate::features::execution::{
        DiagnosticKind as ExecutionDiagnosticKind, ExecutionDiagnostic,
    };
    use crate::languages::model::Language;

    #[test]
    fn learning_hint_tracks_active_line_and_updated_condition() {
        let source = "algoritmo \"votacao\"\nvar\nidade: inteiro\ninicio\nescreva(\"idade?\")\nleia(idade)\nse idade >= 18 entao\nescreval(\"sim\")\nfimse\nfimalgoritmo";
        let decision = learning_hint(source, 7, Language::Visualg);
        assert_eq!(decision.topic, LearningTopic::Decision);
        assert_eq!(decision.expression.as_deref(), Some("idade >= 18"));
        assert_eq!(
            decision.experiment_expression.as_deref(),
            Some("idade >= 16")
        );
        let input = learning_hint(source, 6, Language::Visualg);
        assert_eq!(input.topic, LearningTopic::Input);

        let edited = source.replace("18", "16");
        let changed = learning_hint(&edited, 7, Language::Visualg);
        assert_eq!(changed.expression.as_deref(), Some("idade >= 16"));
        assert_eq!(
            changed.experiment_expression.as_deref(),
            Some("idade >= 14")
        );
    }

    #[test]
    fn learning_hint_recognizes_output_loops_and_unclassified_lines() {
        assert_eq!(
            learning_hint(
                "algoritmo \"aula\"\ninicio\nescreval(1)\nfimalgoritmo",
                3,
                Language::Visualg
            )
            .topic,
            LearningTopic::Output
        );
        assert_eq!(
            learning_hint(
                "algoritmo \"aula\"\ninicio\nenquanto x < 3 faca\n escreval(x)\nfimenquanto\nfimalgoritmo",
                3,
                Language::Visualg
            )
            .topic,
            LearningTopic::Loop
        );
        assert_eq!(
            learning_hint(
                "programa {\n funcao inicio() {\n  escreva(1)\n }\n}",
                3,
                Language::Portugol
            )
            .topic,
            LearningTopic::Output
        );
        assert_eq!(
            learning_hint("fimse", 1, Language::Visualg).topic,
            LearningTopic::Unrecognized
        );
    }

    #[test]
    fn learning_hint_uses_ast_for_all_supported_languages_and_keeps_unknown_lines_unknown() {
        let potigol = learning_hint("escreva \"Olá\"", 1, Language::Potigol);
        assert_eq!(potigol.topic, LearningTopic::Output);

        let calango = "algoritmo aula;\nprincipal\n  escreval(\"Olá\");\nfimPrincipal";
        assert_eq!(
            learning_hint(calango, 3, Language::Calango).topic,
            LearningTopic::Output
        );

        let mapler = "variaveis\n  x: inteiro;\ninicio\n  escrever \"Olá\";\nfim.";
        assert_eq!(
            learning_hint(mapler, 4, Language::Mapler).topic,
            LearningTopic::Output
        );

        let unknown = learning_hint("??", 1, Language::Potigol);
        assert_eq!(unknown.topic, LearningTopic::Unrecognized);
    }

    #[test]
    fn blank_and_comment_lines_have_no_construct_or_problem_in_every_dialect() {
        let cases = [
            ("# comentário\n", Language::Potigol),
            ("// comentário\n", Language::Portugol),
            ("// comentário\n", Language::Visualg),
            ("// comentário\n", Language::GPortugol),
            ("// comentário\n", Language::Calango),
            ("// comentário\n", Language::Mapler),
        ];
        for (source, language) in cases {
            let hint = learning_hint(source, 1, language);
            assert_eq!(hint.topic, LearningTopic::Unrecognized, "{language:?}");
            assert!(hint.problems.is_empty(), "{language:?}: {source}");
            assert!(hint.call.is_none(), "{language:?}: {source}");
        }

        let incomplete_potigol_comment = learning_hint("# comentário", 1, Language::Potigol);
        assert!(
            !incomplete_potigol_comment.problems.is_empty(),
            "keep the Potigol parser's EOF diagnostic for an unterminated comment"
        );
    }

    #[test]
    fn learning_hint_recognizes_jerimum_calls_only_when_the_library_is_imported() {
        let imported = "use \"jerimum\"\ndesenhe_personagem(g: Graphics2D): Nada\n  g.setColor(Cor.VERDE.color)\nfim";
        let hint = learning_hint(imported, 3, Language::Potigol);
        assert_eq!(hint.topic, LearningTopic::LibraryCall);
        let call = hint
            .call
            .expect("the method call should be recovered from the AST");
        assert_eq!(call.receiver.as_deref(), Some("g"));
        assert_eq!(call.member, "setColor");
        assert_eq!(call.library.as_deref(), Some("jerimum"));
        assert_eq!(call.arguments, ["Cor.VERDE.color"]);

        let classic = learning_hint("g.setColor(Cor.VERDE.color)", 1, Language::Potigol);
        assert_eq!(classic.topic, LearningTopic::Call);
        assert_eq!(
            classic.call.and_then(|call| call.library),
            None,
            "a Jerimum API must not be inferred without its import"
        );

        let imported_but_untyped = "use \"jerimum\"\ng.setColor(Cor.VERDE.color)";
        let hint = learning_hint(imported_but_untyped, 2, Language::Potigol);
        assert_eq!(hint.topic, LearningTopic::Call);
        assert_eq!(hint.call.and_then(|call| call.library), None);
    }

    #[test]
    fn potigol_input_builtins_are_all_explained_as_input() {
        for name in [
            "leia_inteiro",
            "leia_real",
            "leia_numero",
            "leia_número",
            "leia_texto",
            "leia_caractere",
            "leia_logico",
            "leia_inteiros",
            "leia_reais",
            "leia_numeros",
            "leia_números",
            "leia_textos",
        ] {
            let source = format!("var valor := {name}()\n");
            let hint = learning_hint(&source, 1, Language::Potigol);
            assert_eq!(hint.topic, LearningTopic::Input, "{name}");
            assert_eq!(
                hint.call.as_ref().map(|call| call.member.as_str()),
                Some(name)
            );
        }
    }

    #[test]
    fn output_hints_preserve_the_dialect_command_name() {
        let cases = [
            ("escreva \"Olá\"", 1, Language::Potigol, "escreva"),
            ("imprima \"Olá\"", 1, Language::Potigol, "imprima"),
            (
                "algoritmo \"aula\"\ninicio\nescreva(\"Olá\")\nfimalgoritmo",
                3,
                Language::Visualg,
                "escreva",
            ),
            (
                "algoritmo \"aula\"\ninicio\nescreval(\"Olá\")\nfimalgoritmo",
                3,
                Language::Visualg,
                "escreval",
            ),
            (
                "programa {\n funcao inicio() {\n escreva(\"Olá\")\n }\n}",
                3,
                Language::Portugol,
                "escreva",
            ),
            (
                "algoritmo aula;\ninício\n imprima(\"Olá\");\nfim",
                3,
                Language::GPortugol,
                "imprima",
            ),
            (
                "algoritmo aula;\nprincipal\n escreval(\"Olá\");\nfimPrincipal",
                3,
                Language::Calango,
                "escreval",
            ),
            (
                "variaveis\n x: inteiro;\ninicio\n escrever \"Olá\";\nfim.",
                4,
                Language::Mapler,
                "escrever",
            ),
        ];

        for (source, line, language, command) in cases {
            let hint = learning_hint(source, line, language);
            assert_eq!(hint.topic, LearningTopic::Output, "{language:?}: {source}");
            assert_eq!(
                hint.name.as_deref(),
                Some(command),
                "{language:?}: {source}"
            );
        }
    }

    #[test]
    fn learning_hint_maps_dialect_specific_routine_and_array_facts() {
        let portugol = "programa {\n funcao vazio registrar(inteiro valor) {}\n funcao inicio() {\n  inteiro valores[3]\n }\n}";
        let routine = learning_hint(portugol, 2, Language::Portugol);
        assert_eq!(routine.topic, LearningTopic::Function);
        assert_eq!(routine.name.as_deref(), Some("registrar"));
        assert_eq!(routine.return_type.as_deref(), Some("vazio"));
        assert_eq!(routine.parameters[0].name, "valor");
        assert_eq!(routine.parameters[0].type_name, "inteiro");

        let visualg =
            "algoritmo \"analise\"\nvar\ndados: vetor[2..5] de inteiro\ninicio\nfimalgoritmo\n";
        let vector = learning_hint(visualg, 3, Language::Visualg);
        assert_eq!(vector.topic, LearningTopic::Variable);
        assert_eq!(vector.variables[0].name, "dados");
        assert_eq!(vector.variables[0].type_name, "inteiro");
        assert_eq!(vector.variables[0].dimensions[0].lower_bound, Some(2));
        assert_eq!(vector.variables[0].dimensions[0].upper_bound, Some(5));
    }

    #[test]
    fn gportugol_learning_hint_identifies_constructs_and_symbols() {
        let source = "algoritmo aula;\n\
variáveis\n\
  idade: inteiro;\n\
  valores: matriz[3] de inteiros;\n\
fim-variáveis\n\
início\n\
  idade := 17;\n\
  idade := leia();\n\
  imprima(idade);\n\
  enquanto idade > 0 faça\n\
    idade := idade - 1;\n\
  fim-enquanto\n\
  se idade >= 18 então\n\
    imprima(\"maior\");\n\
  fim-se\n\
fim\n\
função soma(a: inteiro, b: real): real\n\
início\n\
  retorne a + b;\n\
fim";

        let variable = learning_hint(source, 3, Language::GPortugol);
        assert_eq!(variable.topic, LearningTopic::Variable);
        assert_eq!(variable.variables[0].name, "idade");
        assert_eq!(variable.variables[0].type_name, "inteiro");

        let vector = learning_hint(source, 4, Language::GPortugol);
        assert_eq!(vector.variables[0].dimensions[0].lower_bound, Some(0));
        assert_eq!(vector.variables[0].dimensions[0].upper_bound, Some(2));

        assert_eq!(
            learning_hint(source, 7, Language::GPortugol).topic,
            LearningTopic::Assignment
        );
        assert_eq!(
            learning_hint(source, 8, Language::GPortugol).topic,
            LearningTopic::Input
        );

        let output = learning_hint(source, 9, Language::GPortugol);
        assert_eq!(output.topic, LearningTopic::Output);
        assert_eq!(
            learning_hint(source, 10, Language::GPortugol).topic,
            LearningTopic::Loop
        );

        let decision = learning_hint(source, 13, Language::GPortugol);
        assert_eq!(decision.topic, LearningTopic::Decision);
        assert_eq!(decision.expression.as_deref(), Some("idade >= 18"));
        assert_eq!(
            decision.experiment_expression.as_deref(),
            Some("idade >= 16")
        );

        let function = learning_hint(source, 17, Language::GPortugol);
        assert_eq!(function.topic, LearningTopic::Function);
        assert_eq!(function.name.as_deref(), Some("soma"));
        assert_eq!(function.return_type.as_deref(), Some("real"));
        assert_eq!(function.parameters[0].name, "a");
        assert_eq!(function.parameters[0].type_name, "inteiro");
        assert_eq!(function.parameters[1].name, "b");
        assert_eq!(function.parameters[1].type_name, "real");

        let return_value = learning_hint(source, 19, Language::GPortugol);
        assert_eq!(return_value.topic, LearningTopic::Return);
        assert_eq!(return_value.return_type.as_deref(), Some("real"));
    }

    #[test]
    fn gportugol_learning_hint_reports_syntax_errors_on_the_selected_line() {
        let source =
            "algoritmo aula;\ninício\n  se idade >= então\n    imprima(\"ok\");\n  fim-se\nfim";
        let hint = learning_hint(source, 3, Language::GPortugol);

        assert_eq!(hint.topic, LearningTopic::Unrecognized);
        assert!(hint.problems.iter().any(|problem| {
            problem.kind == LearningProblemKind::Syntax && problem.line == 3 && problem.column > 0
        }));
    }

    #[test]
    fn learning_hint_reports_line_specific_syntax_and_possible_logic_problems() {
        let source =
            "programa {\n funcao inicio() {\n  se (verdadeiro) {\n   escreva(\"ok\")\n  }\n }\n}";
        let hint = learning_hint(source, 3, Language::Portugol);
        assert!(hint.problems.iter().any(|problem| {
            problem.kind == LearningProblemKind::PossibleLogic
                && problem.severity == LearningSeverity::Warning
                && problem.line == 3
        }));
        let incomplete = learning_hint("programa {\n funcao inicio( {\n}", 2, Language::Portugol);
        assert_eq!(incomplete.topic, LearningTopic::Unrecognized);
        assert_eq!(incomplete.name, None);
        assert!(incomplete.parameters.is_empty());
        assert!(
            incomplete
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Syntax)
        );
    }

    #[test]
    fn parser_errors_stay_on_their_line_across_all_supported_dialects() {
        let cases = [
            ("escreva \"ok\"\n@\n", 2, Language::Potigol),
            ("programa {\n @\n}", 2, Language::Portugol),
            (
                "algoritmo \"aula\"\ninicio\n@\nfimalgoritmo",
                3,
                Language::Visualg,
            ),
            ("algoritmo aula;\ninício\n@\nfim", 3, Language::GPortugol),
            (
                "algoritmo aula;\nprincipal\n@\nfimPrincipal",
                3,
                Language::Calango,
            ),
            (
                "variaveis\n x: inteiro;\ninicio\n@\nfim.",
                4,
                Language::Mapler,
            ),
        ];
        for (source, error_line, language) in cases {
            let error = learning_hint(source, error_line, language);
            assert_eq!(error.topic, LearningTopic::Unrecognized, "{language:?}");
            assert!(
                error
                    .problems
                    .iter()
                    .any(|problem| problem.line == error_line),
                "{language:?}: {source}"
            );
            let valid = learning_hint(source, 1, language);
            assert!(
                valid.problems.is_empty(),
                "an error on another line leaked into {language:?}: {source}"
            );
        }
    }

    #[test]
    fn potigol_field_assignment_advice_uses_potigol_syntax_in_the_learning_panel() {
        let source = "use \"jerimum\"\njogo.titulo = \"teste\"\n";
        let error = learning_hint(source, 2, Language::Potigol);
        assert!(error.problems.iter().any(|problem| {
            problem.line == 2
                && problem.kind == LearningProblemKind::Syntax
                && problem.message.contains("usa ':='")
        }));

        let import_line = learning_hint(source, 1, Language::Potigol);
        assert!(import_line.problems.is_empty());
    }

    #[test]
    fn runtime_diagnostics_are_shown_only_on_their_source_line() {
        let diagnostics = [ExecutionDiagnostic {
            kind: ExecutionDiagnosticKind::Runtime,
            message: "índice fora dos limites".to_owned(),
            line: 2,
            column: 4,
        }];
        let source = "programa {\n inteiro x = 1\n}";
        let selected = learning_hint_with_runtime(source, 2, Language::Portugol, &diagnostics);
        let other_line = learning_hint_with_runtime(source, 3, Language::Portugol, &diagnostics);
        assert!(selected.problems.iter().any(|problem| {
            problem.kind == LearningProblemKind::Runtime
                && problem.message == "índice fora dos limites"
        }));
        assert!(
            !other_line
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Runtime)
        );
    }

    #[test]
    fn reports_only_proven_constant_indexes_outside_declared_bounds() {
        let portugol = "programa {\n funcao inicio() {\n  inteiro valores[3]\n  escreva(valores[3])\n  escreva(valores[2])\n }\n}";
        let out_of_range = learning_hint(portugol, 4, Language::Portugol);
        let declaration = learning_hint(portugol, 3, Language::Portugol);
        let valid = learning_hint(portugol, 5, Language::Portugol);
        assert!(out_of_range.problems.iter().any(|problem| {
            problem.kind == LearningProblemKind::Logic
                && problem.severity == LearningSeverity::Error
        }));
        assert!(
            !valid
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Logic)
        );
        assert!(
            !declaration
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Logic)
        );

        let visualg = "algoritmo \"analise\"\nvar\ndados: vetor[2..5] de inteiro\ninicio\n  escreva(dados[1])\nfimalgoritmo\n";
        let visualg_index = learning_hint(visualg, 5, Language::Visualg);
        assert!(
            visualg_index
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Logic)
        );

        let commented_index = "programa {\n funcao inicio() {\n  inteiro valores[3]\n  escreva(valores[2] /* valores[3] */)\n }\n}";
        let commented_index = learning_hint(commented_index, 4, Language::Portugol);
        assert!(
            !commented_index
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Logic),
            "indexes inside block comments must not produce learning problems"
        );
    }

    #[test]
    fn type_diagnostics_keep_their_own_learning_category() {
        let diagnostics = [ExecutionDiagnostic {
            kind: ExecutionDiagnosticKind::Type,
            message: "tipos incompatíveis: esperava Inteiro, recebeu Lógico".to_owned(),
            line: 1,
            column: 5,
        }];
        let hint = learning_hint_with_runtime(
            "valor = 1 + verdadeiro",
            1,
            Language::Potigol,
            &diagnostics,
        );
        assert!(
            hint.problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Type)
        );
    }
}

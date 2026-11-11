use std::collections::{HashMap, HashSet};

use crate::features::learning::source_call;
use crate::languages::{
    array_bounds::insert_unique_bounds,
    model::{
        SourceCall, SourceConstruct, SourceDiagnostic, SourceDiagnosticKind, SourceDimension,
        SourceLineAnalysis, SourceLineAnalysisOutcome, SourceSymbol,
    },
    source::{RecoveryStatus, SourceIndex, recover_parse},
};
use potoo_calango_interpreter::{Diagnostic, DiagnosticKind, ast::*, parse};

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
        crate::languages::calango::syntax_analysis::views::mask_diagnostic_statement,
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
                analyze_program_line(source, &program, one_based_line)
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

fn analyze_program_line(
    source: &str,
    program: &Program,
    line: usize,
) -> Option<SourceLineAnalysis> {
    if crate::languages::calango::syntax_analysis::views::source_line_code(source, line)
        .split_whitespace()
        .next()
        == Some("algoritmo")
    {
        return Some(empty_analysis(line, 1, SourceConstruct::Program));
    }
    for routine in &program.routines {
        if routine.line == line {
            return Some(analyze_routine(routine));
        }
        if let Some(analysis) = analyze_statements(source, &routine.body, line) {
            return Some(analysis);
        }
    }
    analyze_statements(source, &program.main, line)
}

fn analyze_routine(routine: &Routine) -> SourceLineAnalysis {
    SourceLineAnalysis {
        line: routine.line,
        column: routine.column,
        construct: if routine.return_type.is_some() {
            SourceConstruct::Function
        } else {
            SourceConstruct::Procedure
        },
        name: Some(routine.name.clone()),
        return_type: routine.return_type.as_ref().map(type_name),
        parameters: routine.parameters.iter().map(source_parameter).collect(),
        variables: Vec::new(),
        call: None,
    }
}

fn analyze_statements(
    source: &str,
    statements: &[Statement],
    line: usize,
) -> Option<SourceLineAnalysis> {
    for statement in statements {
        if statement.line == line {
            let mut analysis =
                empty_analysis(line, statement.column, SourceConstruct::Unrecognized);
            match &statement.kind {
                StatementKind::Declare(variables) => {
                    analysis.construct = SourceConstruct::VariableDeclaration;
                    analysis.variables = variables.iter().map(source_variable).collect();
                }
                StatementKind::Assign { target, value } => {
                    let is_input = expression_calls_input(value);
                    analysis.construct = if is_input {
                        SourceConstruct::Input
                    } else {
                        SourceConstruct::Assignment
                    };
                    analysis.name = if is_input {
                        match &value.kind {
                            ExpressionKind::Call { name, .. } if expression_calls_input(value) => {
                                Some(name.clone())
                            }
                            _ => None,
                        }
                    } else {
                        expression_name(target)
                    };
                    analysis.call = expression_call(source, value);
                }
                StatementKind::Write { newline, .. } => {
                    analysis.construct = SourceConstruct::Output;
                    analysis.name = Some(if *newline { "escreval" } else { "escreva" }.to_owned());
                }
                StatementKind::Read { character_only, .. } => {
                    analysis.construct = SourceConstruct::Input;
                    analysis.name = Some(
                        if *character_only {
                            "leiaCaracter"
                        } else {
                            "leia"
                        }
                        .to_owned(),
                    );
                }
                StatementKind::If { .. } => {
                    analysis.construct = SourceConstruct::Decision;
                    analysis.name = Some("se".to_owned());
                }
                StatementKind::Choose { .. } => {
                    analysis.construct = SourceConstruct::Decision;
                    analysis.name = Some("escolha".to_owned());
                }
                StatementKind::While { .. }
                | StatementKind::DoWhile { .. }
                | StatementKind::For { .. } => {
                    analysis.construct = SourceConstruct::Loop;
                    analysis.name = Some(
                        match &statement.kind {
                            StatementKind::While { .. } => "enquanto",
                            StatementKind::DoWhile { .. } => "faca",
                            _ => "para",
                        }
                        .to_owned(),
                    );
                }
                StatementKind::Call { name, .. } => {
                    analysis.construct = SourceConstruct::Call;
                    analysis.call = Some(source_call(
                        source,
                        line,
                        statement.column,
                        None,
                        name,
                        None,
                    ));
                }
                StatementKind::Return(_) => analysis.construct = SourceConstruct::Return,
                StatementKind::Break | StatementKind::ClearScreen => {}
            }
            if analysis.construct != SourceConstruct::Unrecognized {
                return Some(analysis);
            }
        }

        let nested = match &statement.kind {
            StatementKind::If {
                branches,
                otherwise,
            } => branches
                .iter()
                .find_map(|(_, body)| analyze_statements(source, body, line))
                .or_else(|| analyze_statements(source, otherwise, line)),
            StatementKind::While { body, .. }
            | StatementKind::DoWhile { body, .. }
            | StatementKind::For { body, .. } => analyze_statements(source, body, line),
            StatementKind::Choose {
                cases, otherwise, ..
            } => cases
                .iter()
                .find_map(|(_, body)| analyze_statements(source, body, line))
                .or_else(|| analyze_statements(source, otherwise, line)),
            _ => None,
        };
        if nested.is_some() {
            return nested;
        }
    }
    None
}

fn source_variable(variable: &VariableDeclaration) -> SourceSymbol {
    let (scalar, dimensions) = match &variable.ty {
        Type::Array(inner, sizes) => (
            inner.scalar(),
            sizes
                .iter()
                .map(|size| SourceDimension {
                    lower_bound: Some(0),
                    upper_bound: size.and_then(|size| i64::try_from(size).ok()?.checked_sub(1)),
                })
                .collect(),
        ),
        ty => (ty.scalar(), Vec::new()),
    };
    SourceSymbol {
        name: variable.name.clone(),
        type_name: type_name(scalar),
        is_constant: false,
        by_reference: false,
        dimensions,
        initializer: None,
    }
}

fn source_parameter(parameter: &Parameter) -> SourceSymbol {
    let (scalar, dimensions) = match &parameter.ty {
        Type::Array(inner, sizes) => (
            inner.scalar(),
            sizes
                .iter()
                .map(|size| SourceDimension {
                    lower_bound: Some(0),
                    upper_bound: size.and_then(|size| i64::try_from(size).ok()?.checked_sub(1)),
                })
                .collect(),
        ),
        ty => (ty.scalar(), Vec::new()),
    };
    SourceSymbol {
        name: parameter.name.clone(),
        type_name: type_name(scalar),
        is_constant: false,
        by_reference: parameter.by_reference,
        dimensions,
        initializer: None,
    }
}

fn expression_calls_input(expression: &Expression) -> bool {
    matches!(&expression.kind, ExpressionKind::Call { name, .. } if matches!(name.as_str(), "leia" | "leiaCaracter"))
}

fn expression_name(expression: &Expression) -> Option<String> {
    match &expression.kind {
        ExpressionKind::Variable(name) => Some(name.clone()),
        ExpressionKind::Index(target, _) => expression_name(target),
        _ => None,
    }
}

fn expression_call(source: &str, expression: &Expression) -> Option<SourceCall> {
    match &expression.kind {
        ExpressionKind::Call { name, .. } => Some(source_call(
            source,
            expression.line,
            expression.column,
            None,
            name,
            None,
        )),
        ExpressionKind::Binary { left, right, .. } => {
            expression_call(source, left).or_else(|| expression_call(source, right))
        }
        ExpressionKind::Unary { operand, .. }
        | ExpressionKind::Formatted { value: operand, .. } => expression_call(source, operand),
        ExpressionKind::Index(left, right) => {
            expression_call(source, left).or_else(|| expression_call(source, right))
        }
        _ => None,
    }
}

fn type_name(data_type: &Type) -> String {
    match data_type.scalar() {
        Type::Integer => "inteiro".to_owned(),
        Type::Real => "real".to_owned(),
        Type::Logical => "lógico".to_owned(),
        Type::Character => "caractere".to_owned(),
        Type::Text => "texto".to_owned(),
        Type::Array(_, _) => "vetor".to_owned(),
    }
}

fn empty_analysis(line: usize, column: usize, construct: SourceConstruct) -> SourceLineAnalysis {
    SourceLineAnalysis {
        line,
        column,
        construct,
        name: None,
        return_type: None,
        parameters: Vec::new(),
        variables: Vec::new(),
        call: None,
    }
}

fn collect_declared_array_bounds(
    source: &str,
    is_cancelled: &dyn Fn() -> bool,
) -> Option<HashMap<String, Vec<SourceDimension>>> {
    if is_cancelled() {
        return None;
    }
    let program = match parse(source) {
        Ok(program) => program,
        Err(_) => return Some(HashMap::new()),
    };
    if is_cancelled() {
        return None;
    }

    let mut arrays = HashMap::new();
    let mut ambiguous = HashSet::new();
    for routine in &program.routines {
        for parameter in &routine.parameters {
            if is_cancelled() {
                return None;
            }
            insert_symbol_array_bounds(&source_parameter(parameter), &mut arrays, &mut ambiguous);
        }
        collect_statement_array_bounds(&routine.body, is_cancelled, &mut arrays, &mut ambiguous)?;
    }
    collect_statement_array_bounds(&program.main, is_cancelled, &mut arrays, &mut ambiguous)?;
    if is_cancelled() { None } else { Some(arrays) }
}

fn collect_statement_array_bounds(
    statements: &[Statement],
    is_cancelled: &dyn Fn() -> bool,
    arrays: &mut HashMap<String, Vec<SourceDimension>>,
    ambiguous: &mut HashSet<String>,
) -> Option<()> {
    for statement in statements {
        if is_cancelled() {
            return None;
        }
        match &statement.kind {
            StatementKind::Declare(variables) => {
                for variable in variables {
                    if is_cancelled() {
                        return None;
                    }
                    insert_symbol_array_bounds(&source_variable(variable), arrays, ambiguous);
                }
            }
            StatementKind::If {
                branches,
                otherwise,
            } => {
                for (_, body) in branches {
                    collect_statement_array_bounds(body, is_cancelled, arrays, ambiguous)?;
                }
                collect_statement_array_bounds(otherwise, is_cancelled, arrays, ambiguous)?;
            }
            StatementKind::While { body, .. }
            | StatementKind::DoWhile { body, .. }
            | StatementKind::For { body, .. } => {
                collect_statement_array_bounds(body, is_cancelled, arrays, ambiguous)?;
            }
            StatementKind::Choose {
                cases, otherwise, ..
            } => {
                for (_, body) in cases {
                    collect_statement_array_bounds(body, is_cancelled, arrays, ambiguous)?;
                }
                collect_statement_array_bounds(otherwise, is_cancelled, arrays, ambiguous)?;
            }
            _ => {}
        }
    }
    Some(())
}

fn insert_symbol_array_bounds(
    symbol: &SourceSymbol,
    arrays: &mut HashMap<String, Vec<SourceDimension>>,
    ambiguous: &mut HashSet<String>,
) {
    if symbol.dimensions.is_empty()
        || symbol
            .dimensions
            .iter()
            .any(|dimension| dimension.lower_bound.is_none() || dimension.upper_bound.is_none())
    {
        return;
    }
    insert_unique_bounds(
        arrays,
        ambiguous,
        &symbol.name,
        symbol.dimensions.clone(),
        crate::languages::model::Language::Calango
            .profile()
            .case_sensitive_identifiers,
    );
}

pub(crate) fn profile_declared_array_bounds(
    source: &str,
    _masked_source: &str,
    is_cancelled: &dyn Fn() -> bool,
) -> Option<HashMap<String, Vec<SourceDimension>>> {
    collect_declared_array_bounds(source, is_cancelled)
}

#[cfg(test)]
#[path = "../../../tests/learning/calango_tests.rs"]
mod tests;

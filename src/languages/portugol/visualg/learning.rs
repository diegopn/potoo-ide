use std::collections::{HashMap, HashSet};

use crate::languages::{
    model::{
        SourceConstruct, SourceDiagnostic, SourceDiagnosticKind, SourceDimension,
        SourceLineAnalysis, SourceLineAnalysisOutcome, SourceSymbol,
    },
    source::{
        RecoveryStatus, SourceIndex, read_identifier, read_integer, recover_parse, skip_spaces,
    },
};
use potoo_visualg_interpreter::{Diagnostic, DiagnosticKind, ast::*, parse};

pub(crate) fn profile_adapter(
    source: &str,
    one_based_line: usize,
    is_cancelled: &dyn Fn() -> bool,
) -> SourceLineAnalysisOutcome {
    if is_cancelled() {
        return SourceLineAnalysisOutcome::Cancelled;
    }
    let source_index = SourceIndex::new(source);
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
        crate::languages::portugol::visualg::syntax_analysis::views::mask_diagnostic_statement,
        || is_cancelled(),
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
        DiagnosticKind::Runtime => SourceDiagnosticKind::Runtime,
    };
    SourceDiagnostic::at_line(kind, diagnostic.message, diagnostic.line, diagnostic.column)
}

fn analyze_program_line(
    source: &str,
    program: &Program,
    line: usize,
) -> Option<SourceLineAnalysis> {
    if crate::languages::portugol::visualg::syntax_analysis::views::source_line_code(source, line)
        .split_whitespace()
        .next()
        .is_some_and(|word| word.eq_ignore_ascii_case("algoritmo"))
    {
        return Some(SourceLineAnalysis {
            line,
            column: 1,
            construct: SourceConstruct::Program,
            name: None,
            return_type: None,
            parameters: Vec::new(),
            variables: Vec::new(),
            call: None,
        });
    }

    if let Some(routine) = program.routines.iter().find(|routine| routine.line == line) {
        return Some(analyze_routine(routine));
    }

    if let Some(variable) = program
        .globals
        .iter()
        .find(|variable| variable.line == line)
    {
        let variables = program
            .globals
            .iter()
            .filter(|candidate| candidate.line == line)
            .map(source_variable)
            .collect();
        return Some(SourceLineAnalysis {
            line,
            column: variable.column,
            construct: SourceConstruct::VariableDeclaration,
            name: None,
            return_type: None,
            parameters: Vec::new(),
            variables,
            call: None,
        });
    }

    for routine in &program.routines {
        if let Some(variable) = routine.locals.iter().find(|variable| variable.line == line) {
            let variables = routine
                .locals
                .iter()
                .filter(|candidate| candidate.line == line)
                .map(source_variable)
                .collect();
            return Some(SourceLineAnalysis {
                line,
                column: variable.column,
                construct: SourceConstruct::VariableDeclaration,
                name: None,
                return_type: None,
                parameters: Vec::new(),
                variables,
                call: None,
            });
        }
        if let Some(mut analysis) = analyze_statements(&routine.body, line) {
            if analysis.construct == SourceConstruct::Return {
                analysis.return_type = routine
                    .return_type
                    .map(|data_type| type_name(data_type).to_owned());
            }
            return Some(analysis);
        }
    }

    analyze_statements(&program.statements, line)
}

fn analyze_routine(routine: &Routine) -> SourceLineAnalysis {
    SourceLineAnalysis {
        line: routine.line,
        column: routine.column,
        construct: match routine.kind {
            RoutineKind::Procedure => SourceConstruct::Procedure,
            RoutineKind::Function => SourceConstruct::Function,
        },
        name: Some(routine.name.clone()),
        return_type: routine
            .return_type
            .map(|data_type| type_name(data_type).to_owned()),
        parameters: routine
            .parameters
            .iter()
            .map(|parameter| SourceSymbol {
                name: parameter.name.clone(),
                type_name: type_name(parameter.data_type).to_owned(),
                is_constant: false,
                by_reference: parameter.by_reference,
                dimensions: (0..parameter.dimensions)
                    .map(|_| SourceDimension {
                        lower_bound: None,
                        upper_bound: None,
                    })
                    .collect(),
                initializer: None,
            })
            .collect(),
        variables: Vec::new(),
        call: None,
    }
}

fn analyze_statements(statements: &[Statement], line: usize) -> Option<SourceLineAnalysis> {
    for statement in statements {
        if statement.line == line {
            let construct = match &statement.kind {
                StatementKind::Write { .. } => SourceConstruct::Output,
                StatementKind::Read(_) => SourceConstruct::Input,
                StatementKind::If { .. } | StatementKind::Choice { .. } => {
                    SourceConstruct::Decision
                }
                StatementKind::While { .. }
                | StatementKind::Repeat { .. }
                | StatementKind::For { .. } => SourceConstruct::Loop,
                StatementKind::Return(_) => SourceConstruct::Return,
                StatementKind::Expression(expression) => match &expression.kind {
                    ExpressionKind::Assign { .. } => SourceConstruct::Assignment,
                    ExpressionKind::Call { name, .. }
                        if matches!(name.as_str(), "escreva" | "escreval") =>
                    {
                        SourceConstruct::Output
                    }
                    ExpressionKind::Call { name, .. } if name == "leia" => SourceConstruct::Input,
                    ExpressionKind::Call { .. } => SourceConstruct::Call,
                    _ => SourceConstruct::Other,
                },
                _ => SourceConstruct::Other,
            };
            return Some(SourceLineAnalysis {
                line,
                column: statement.column,
                construct,
                name: match &statement.kind {
                    StatementKind::Write { newline, .. } => {
                        Some(if *newline { "escreval" } else { "escreva" }.to_owned())
                    }
                    StatementKind::Read(_) => Some("leia".to_owned()),
                    StatementKind::If { .. } => Some("se".to_owned()),
                    StatementKind::Choice { .. } => Some("escolha".to_owned()),
                    StatementKind::While { .. } => Some("enquanto".to_owned()),
                    StatementKind::Repeat { .. } => Some("repita".to_owned()),
                    StatementKind::For { variable, .. } => Some(format!("para {variable}")),
                    StatementKind::Expression(expression) => match &expression.kind {
                        ExpressionKind::Call { name, .. } => Some(name.clone()),
                        ExpressionKind::Assign { target, .. } => expression_name(target),
                        _ => None,
                    },
                    _ => None,
                },
                return_type: None,
                parameters: Vec::new(),
                variables: Vec::new(),
                call: None,
            });
        }

        let nested = match &statement.kind {
            StatementKind::If {
                then_branch,
                else_branch,
                ..
            } => analyze_statements(then_branch, line)
                .or_else(|| analyze_statements(else_branch, line)),
            StatementKind::Choice { cases, default, .. } => cases
                .iter()
                .find_map(|case| analyze_statements(&case.body, line))
                .or_else(|| analyze_statements(default, line)),
            StatementKind::While { body, .. } | StatementKind::For { body, .. } => {
                analyze_statements(body, line)
            }
            StatementKind::Repeat { body, .. } => analyze_statements(body, line),
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

fn source_variable(variable: &Variable) -> SourceSymbol {
    SourceSymbol {
        name: variable.name.clone(),
        type_name: type_name(variable.data_type).to_owned(),
        is_constant: false,
        by_reference: false,
        dimensions: variable
            .dimensions
            .iter()
            .map(|dimension| SourceDimension {
                lower_bound: Some(dimension.lower),
                upper_bound: Some(dimension.upper),
            })
            .collect(),
        initializer: None,
    }
}

fn type_name(data_type: Type) -> &'static str {
    match data_type {
        Type::Integer => "inteiro",
        Type::Real => "real",
        Type::Character => "caractere",
        Type::Logical => "logico",
    }
}

#[cfg(test)]
#[path = "../../../../tests/learning/visualg_tests.rs"]
mod tests;

pub(crate) fn declared_array_bounds(
    source: &str,
    is_cancelled: &impl Fn() -> bool,
) -> Option<HashMap<String, Vec<SourceDimension>>> {
    let mut arrays = HashMap::new();
    let mut ambiguous = HashSet::new();
    let source_index = SourceIndex::new(source);
    for line in 1..=source_index.line_count() {
        if is_cancelled() {
            return None;
        }
        let source_line = source_index.line_text(line).unwrap_or("");
        let characters = source_line.trim().chars().collect::<Vec<_>>();
        let mut cursor = 0;
        let mut variables = Vec::new();
        loop {
            skip_spaces(&characters, &mut cursor);
            let Some(name) = read_identifier(&characters, &mut cursor) else {
                break;
            };
            variables.push(name);
            skip_spaces(&characters, &mut cursor);
            if characters.get(cursor) != Some(&',') {
                break;
            }
            cursor += 1;
        }
        skip_spaces(&characters, &mut cursor);
        if characters.get(cursor) != Some(&':') {
            continue;
        }
        cursor += 1;
        skip_spaces(&characters, &mut cursor);
        let Some(container) = read_identifier(&characters, &mut cursor) else {
            continue;
        };
        if !container.eq_ignore_ascii_case("vetor") {
            continue;
        }
        let Some(dimensions) = read_dimensions(&characters, &mut cursor) else {
            continue;
        };
        if dimensions.is_empty() {
            continue;
        }
        for name in variables {
            insert_unique_array_bounds(&mut arrays, &mut ambiguous, &name, dimensions.clone());
        }
    }
    Some(arrays)
}

fn read_dimensions(characters: &[char], cursor: &mut usize) -> Option<Vec<SourceDimension>> {
    skip_spaces(characters, cursor);
    if characters.get(*cursor) != Some(&'[') {
        return None;
    }
    *cursor += 1;
    let mut dimensions = Vec::new();
    loop {
        skip_spaces(characters, cursor);
        let lower = read_integer(characters, cursor)?;
        skip_spaces(characters, cursor);
        if characters.get(*cursor) != Some(&'.') || characters.get(*cursor + 1) != Some(&'.') {
            return None;
        }
        *cursor += 2;
        skip_spaces(characters, cursor);
        let upper = read_integer(characters, cursor)?;
        dimensions.push(SourceDimension {
            lower_bound: Some(lower),
            upper_bound: Some(upper),
        });
        skip_spaces(characters, cursor);
        match characters.get(*cursor) {
            Some(',') => *cursor += 1,
            Some(']') => {
                *cursor += 1;
                return Some(dimensions);
            }
            _ => return None,
        }
    }
}

fn insert_unique_array_bounds(
    arrays: &mut HashMap<String, Vec<SourceDimension>>,
    ambiguous: &mut HashSet<String>,
    name: &str,
    dimensions: Vec<SourceDimension>,
) {
    let name = name.to_lowercase();
    if ambiguous.contains(&name) {
        return;
    }
    if arrays.remove(&name).is_some() {
        ambiguous.insert(name);
    } else {
        arrays.insert(name, dimensions);
    }
}

pub(crate) fn profile_declared_array_bounds(
    source: &str,
    is_cancelled: &dyn Fn() -> bool,
) -> Option<HashMap<String, Vec<SourceDimension>>> {
    declared_array_bounds(source, &|| is_cancelled())
}

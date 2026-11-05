use std::collections::{HashMap, HashSet};

use crate::languages::{
    model::{SourceConstruct, SourceDimension, SourceLineAnalysis, SourceSymbol},
    source::{read_identifier, read_integer, skip_spaces},
};
use potoo_visualg_interpreter::{Diagnostic, ast::*, parse};

/// Parses source with the interpreter AST and describes the construct at `one_based_line`.
pub(crate) fn analyze_source_line_cancellable(
    source: &str,
    one_based_line: usize,
    is_cancelled: impl Fn() -> bool,
) -> Result<Option<SourceLineAnalysis>, Vec<Diagnostic>> {
    if is_cancelled() {
        return Ok(None);
    }
    if one_based_line == 0 || one_based_line > source.lines().count() {
        return Ok(None);
    }

    let (program, diagnostics) = parse_recovering(source, &is_cancelled);
    if is_cancelled() {
        return Ok(None);
    }
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.line == one_based_line)
    {
        return Err(diagnostics);
    }
    let Some(program) = program else {
        return Err(diagnostics);
    };
    if source
        .lines()
        .nth(one_based_line - 1)
        .is_some_and(|line| line.trim_start().starts_with("algoritmo "))
    {
        return Ok(Some(SourceLineAnalysis {
            line: one_based_line,
            column: 1,
            construct: SourceConstruct::Program,
            name: None,
            return_type: None,
            parameters: Vec::new(),
            variables: Vec::new(),
            call: None,
        }));
    }

    Ok(analyze_program_line(&program, one_based_line))
}

fn parse_recovering<F>(source: &str, is_cancelled: &F) -> (Option<Program>, Vec<Diagnostic>)
where
    F: Fn() -> bool,
{
    let mut candidate = source.to_owned();
    let mut diagnostics = Vec::new();
    for attempt in 0..=64 {
        if is_cancelled() {
            return (None, diagnostics);
        }
        match parse(&candidate) {
            Ok(program) => return (Some(program), diagnostics),
            Err(parse_diagnostics) => {
                let Some(diagnostic) = parse_diagnostics.into_iter().next() else {
                    return (None, diagnostics);
                };
                let line = diagnostic.line;
                let column = diagnostic.column;
                if attempt == 64 {
                    diagnostics.push(diagnostic);
                    break;
                }
                let recovered = crate::languages::portugol::visualg::syntax_analysis::views::mask_diagnostic_statement(
                    &candidate,
                    line,
                    column,
                );
                diagnostics.push(diagnostic);
                if recovered == candidate {
                    break;
                }
                candidate = recovered;
            }
        }
    }
    (None, diagnostics)
}

fn analyze_program_line(program: &Program, line: usize) -> Option<SourceLineAnalysis> {
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
    for source_line in source.lines() {
        if is_cancelled() {
            return None;
        }
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

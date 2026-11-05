use std::collections::{HashMap, HashSet};

use crate::features::learning::source_call;
use crate::languages::{
    model::{SourceCall, SourceConstruct, SourceDimension, SourceLineAnalysis, SourceSymbol},
    source::{read_identifier, read_integer, skip_spaces, skip_to_top_level_comma},
};
use potoo_calango_interpreter::{Diagnostic, ast::*, parse};

pub(crate) fn analyze_source_line_cancellable(
    source: &str,
    one_based_line: usize,
    is_cancelled: impl Fn() -> bool,
) -> Result<Option<SourceLineAnalysis>, Vec<Diagnostic>> {
    if is_cancelled() || one_based_line == 0 || one_based_line > source.lines().count() {
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
        return if diagnostics.is_empty() {
            Ok(None)
        } else {
            Err(diagnostics)
        };
    };
    if source
        .lines()
        .nth(one_based_line - 1)
        .is_some_and(|line| line.trim_start().starts_with("algoritmo "))
    {
        return Ok(Some(empty_analysis(
            one_based_line,
            1,
            SourceConstruct::Program,
        )));
    }
    for routine in &program.routines {
        if routine.line == one_based_line {
            return Ok(Some(analyze_routine(routine)));
        }
        if let Some(analysis) = analyze_statements(source, &routine.body, one_based_line) {
            return Ok(Some(analysis));
        }
    }
    Ok(analyze_statements(source, &program.main, one_based_line))
}

pub(crate) fn profile_adapter(
    source: &str,
    one_based_line: usize,
    is_cancelled: &dyn Fn() -> bool,
) -> Result<Option<SourceLineAnalysis>, Vec<crate::languages::model::SourceDiagnostic>> {
    analyze_source_line_cancellable(source, one_based_line, is_cancelled).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| {
                let kind = match diagnostic.kind {
                    potoo_calango_interpreter::DiagnosticKind::Lexical => {
                        crate::languages::model::SourceDiagnosticKind::Lexical
                    }
                    potoo_calango_interpreter::DiagnosticKind::Syntax => {
                        crate::languages::model::SourceDiagnosticKind::Syntax
                    }
                    potoo_calango_interpreter::DiagnosticKind::Type => {
                        crate::languages::model::SourceDiagnosticKind::Type
                    }
                    potoo_calango_interpreter::DiagnosticKind::Runtime => {
                        crate::languages::model::SourceDiagnosticKind::Runtime
                    }
                };
                crate::languages::model::SourceDiagnostic::at_line(
                    kind,
                    diagnostic.message,
                    diagnostic.line,
                    diagnostic.column,
                )
            })
            .collect()
    })
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
                let recovered =
                    crate::languages::calango::syntax_analysis::views::mask_diagnostic_statement(
                        &candidate, line, column,
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
        let Some(type_name) = read_identifier(&characters, &mut cursor) else {
            continue;
        };
        if !matches!(
            type_name.to_lowercase().as_str(),
            "inteiro" | "real" | "logico" | "caracter" | "caractere" | "cadeia"
        ) {
            continue;
        }

        read_array_declarations(&characters, cursor, &mut arrays, &mut ambiguous);
    }
    Some(arrays)
}

fn read_array_declarations(
    characters: &[char],
    mut cursor: usize,
    arrays: &mut HashMap<String, Vec<SourceDimension>>,
    ambiguous: &mut HashSet<String>,
) {
    loop {
        skip_spaces(characters, &mut cursor);
        let Some(name) = read_identifier(characters, &mut cursor) else {
            break;
        };
        skip_spaces(characters, &mut cursor);
        let mut dimensions = Vec::new();
        while characters.get(cursor) == Some(&'[') {
            cursor += 1;
            skip_spaces(characters, &mut cursor);
            let Some(size) = read_integer(characters, &mut cursor) else {
                break;
            };
            skip_spaces(characters, &mut cursor);
            if characters.get(cursor) != Some(&']') {
                break;
            }
            cursor += 1;
            dimensions.push(SourceDimension {
                lower_bound: (size > 0).then_some(0),
                upper_bound: (size > 0).then_some(size - 1),
            });
            skip_spaces(characters, &mut cursor);
        }
        if !dimensions.is_empty()
            && dimensions
                .iter()
                .all(|dimension| dimension.upper_bound.is_some())
        {
            insert_unique_array_bounds(arrays, ambiguous, &name, dimensions);
        }
        cursor = skip_to_top_level_comma(characters, cursor);
        if characters.get(cursor) != Some(&',') {
            break;
        }
        cursor += 1;
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

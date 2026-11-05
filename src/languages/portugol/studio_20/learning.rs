use std::collections::{HashMap, HashSet};

use crate::languages::{
    model::{SourceConstruct, SourceDimension, SourceLineAnalysis, SourceSymbol},
    source::{read_identifier, read_integer, skip_spaces, skip_to_top_level_comma},
};
use potoo_portugol_interpreter::{Diagnostic, ast::*, parse};

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
        .is_some_and(|line| line.trim_start().starts_with("programa"))
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
                let recovered = crate::languages::portugol::studio_20::syntax_analysis::views::mask_diagnostic_statement(
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
    if let Some(function) = program
        .functions
        .iter()
        .find(|function| function.line == line)
    {
        return Some(analyze_function(function));
    }

    for function in &program.functions {
        if let Some(mut analysis) = analyze_statement(&function.body, line) {
            if analysis.construct == SourceConstruct::Return {
                analysis.return_type = Some(function.return_type.map_or_else(
                    || "vazio".to_owned(),
                    |data_type| type_name(data_type).to_owned(),
                ));
            }
            return Some(analysis);
        }
    }
    analyze_statements(&program.statements, line)
}

fn analyze_function(function: &Function) -> SourceLineAnalysis {
    SourceLineAnalysis {
        line: function.line,
        column: function.column,
        construct: SourceConstruct::Function,
        name: Some(function.name.clone()),
        return_type: Some(function.return_type.map_or_else(
            || "vazio".to_owned(),
            |data_type| type_name(data_type).to_owned(),
        )),
        parameters: function
            .parameters
            .iter()
            .map(|parameter| SourceSymbol {
                name: parameter.name.clone(),
                type_name: type_name(parameter.ty).to_owned(),
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
    statements
        .iter()
        .find_map(|statement| analyze_statement(statement, line))
}

fn analyze_statement(statement: &Statement, line: usize) -> Option<SourceLineAnalysis> {
    if statement.line == line {
        if let StatementKind::Variables(variables) = &statement.kind {
            return Some(SourceLineAnalysis {
                line,
                column: statement.column,
                construct: SourceConstruct::VariableDeclaration,
                name: None,
                return_type: None,
                parameters: Vec::new(),
                variables: variables.iter().map(source_variable).collect(),
                call: None,
            });
        }

        let construct = match &statement.kind {
            StatementKind::Expression(expression) => expression_construct(expression),
            StatementKind::If { .. } | StatementKind::Switch { .. } => SourceConstruct::Decision,
            StatementKind::While { .. }
            | StatementKind::DoWhile { .. }
            | StatementKind::For { .. } => SourceConstruct::Loop,
            StatementKind::Return(_) => SourceConstruct::Return,
            _ => SourceConstruct::Other,
        };
        return Some(SourceLineAnalysis {
            line,
            column: statement.column,
            construct,
            name: match &statement.kind {
                StatementKind::Expression(expression)
                    if matches!(construct, SourceConstruct::Input | SourceConstruct::Output) =>
                {
                    expression_command(expression)
                }
                StatementKind::Expression(Expression {
                    kind: ExpressionKind::Assign { target, .. },
                    ..
                }) => expression_name(target),
                StatementKind::If { .. } => Some("se".to_owned()),
                StatementKind::Switch { .. } => Some("escolha".to_owned()),
                StatementKind::While { .. } => Some("enquanto".to_owned()),
                StatementKind::DoWhile { .. } => Some("faca".to_owned()),
                StatementKind::For { .. } => Some("para".to_owned()),
                _ => None,
            },
            return_type: None,
            parameters: Vec::new(),
            variables: Vec::new(),
            call: None,
        });
    }

    match &statement.kind {
        StatementKind::Block(statements) => analyze_statements(statements, line),
        StatementKind::If {
            then_branch,
            else_branch,
            ..
        } => analyze_statement(then_branch, line).or_else(|| {
            else_branch
                .as_deref()
                .and_then(|branch| analyze_statement(branch, line))
        }),
        StatementKind::While { body, .. } | StatementKind::DoWhile { body, .. } => {
            analyze_statement(body, line)
        }
        StatementKind::For {
            initializer, body, ..
        } => initializer
            .as_deref()
            .and_then(|statement| analyze_statement(statement, line))
            .or_else(|| analyze_statement(body, line)),
        StatementKind::Switch { cases, .. } => cases
            .iter()
            .find_map(|case| analyze_statements(&case.body, line)),
        StatementKind::Variables(_)
        | StatementKind::Expression(_)
        | StatementKind::Return(_)
        | StatementKind::Break => None,
    }
}

fn expression_construct(expression: &Expression) -> SourceConstruct {
    match &expression.kind {
        ExpressionKind::Assign { .. } => SourceConstruct::Assignment,
        ExpressionKind::Call { callee, .. } => match &callee.kind {
            ExpressionKind::Variable(name) if name.eq_ignore_ascii_case("leia") => {
                SourceConstruct::Input
            }
            ExpressionKind::Variable(name)
                if name.eq_ignore_ascii_case("escreva")
                    || name.eq_ignore_ascii_case("escreval") =>
            {
                SourceConstruct::Output
            }
            ExpressionKind::Member { name, .. }
                if name.eq_ignore_ascii_case("leia")
                    || name.eq_ignore_ascii_case("escreva")
                    || name.eq_ignore_ascii_case("escreval") =>
            {
                if name.eq_ignore_ascii_case("leia") {
                    SourceConstruct::Input
                } else {
                    SourceConstruct::Output
                }
            }
            ExpressionKind::Member { .. } => SourceConstruct::MethodCall,
            _ => SourceConstruct::Call,
        },
        _ => SourceConstruct::Other,
    }
}

fn expression_command(expression: &Expression) -> Option<String> {
    let ExpressionKind::Call { callee, .. } = &expression.kind else {
        return None;
    };
    match &callee.kind {
        ExpressionKind::Variable(name) | ExpressionKind::Member { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn expression_name(expression: &Expression) -> Option<String> {
    match &expression.kind {
        ExpressionKind::Variable(name) => Some(name.clone()),
        ExpressionKind::Member { target, name } => {
            Some(format!("{}.{}", expression_name(target)?, name))
        }
        ExpressionKind::Index { target, .. } => expression_name(target),
        _ => None,
    }
}

fn source_variable(variable: &Variable) -> SourceSymbol {
    SourceSymbol {
        name: variable.name.clone(),
        type_name: type_name(variable.ty).to_owned(),
        is_constant: variable.is_constant,
        by_reference: false,
        dimensions: variable
            .dimensions
            .iter()
            .map(|dimension| match dimension {
                Some(expression) => SourceDimension {
                    lower_bound: Some(0),
                    upper_bound: match expression.kind {
                        ExpressionKind::Integer(size) => Some(size.saturating_sub(1)),
                        _ => None,
                    },
                },
                None => SourceDimension {
                    lower_bound: None,
                    upper_bound: None,
                },
            })
            .collect(),
        initializer: variable.initializer.as_ref().map(expression_text),
    }
}

fn expression_text(expression: &Expression) -> String {
    match &expression.kind {
        ExpressionKind::Integer(value) => value.to_string(),
        ExpressionKind::Real(value) => value.to_string(),
        ExpressionKind::Logical(value) => if *value { "verdadeiro" } else { "falso" }.to_owned(),
        ExpressionKind::Character(value) => format!("'{value}'"),
        ExpressionKind::String(value) => format!("\"{value}\""),
        ExpressionKind::ArrayLiteral(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(expression_text)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ExpressionKind::Variable(name) => name.clone(),
        _ => "expressão".to_owned(),
    }
}

fn type_name(data_type: Type) -> &'static str {
    match data_type {
        Type::Integer => "inteiro",
        Type::Real => "real",
        Type::Logical => "logico",
        Type::Character => "caracter",
        Type::String => "cadeia",
        Type::Void => "vazio",
    }
}

#[cfg(test)]
#[path = "../../../../tests/learning/portugol_tests.rs"]
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

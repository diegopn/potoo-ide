use super::{SourceConstruct, SourceDimension, SourceLineAnalysis, SourceSymbol};
use potoo_gportugol_interpreter::{DataType, Diagnostic, ast::*, parse};
use std::collections::HashSet;

pub(super) fn source_line_code(source: &str, one_based_line: usize) -> String {
    let mut in_block_comment = false;
    for (index, line) in source.lines().enumerate() {
        let selected = index + 1 == one_based_line;
        let mut code = String::with_capacity(if selected { line.len() } else { 0 });
        let mut characters = line.chars().peekable();
        let mut quote = None;
        let mut escaped = false;
        while let Some(character) = characters.next() {
            if in_block_comment {
                if character == '*' && characters.peek() == Some(&'/') {
                    characters.next();
                    if selected {
                        code.push_str("  ");
                    }
                    in_block_comment = false;
                } else if selected {
                    code.push(' ');
                }
                continue;
            }
            if let Some(quote_character) = quote {
                if selected {
                    code.push(character);
                }
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
                if selected {
                    code.push(character);
                }
            } else if character == '/' && characters.peek() == Some(&'*') {
                characters.next();
                in_block_comment = true;
                if selected {
                    code.push_str("  ");
                }
            } else if character == '/' && characters.peek() == Some(&'/') {
                break;
            } else if selected {
                code.push(character);
            }
        }
        if selected {
            return code;
        }
    }
    String::new()
}

pub(super) fn analyze_source_line_cancellable(
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

    let Some((program, diagnostics)) = parse_recovering(source, &is_cancelled) else {
        return Ok(None);
    };
    if is_cancelled() {
        return Ok(None);
    }
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.span.start.line == one_based_line)
    {
        return Err(diagnostics);
    }
    let Some(program) = program else {
        return Err(diagnostics);
    };
    Ok(analyze_program_line(&program, one_based_line))
}

fn parse_recovering<F>(source: &str, is_cancelled: &F) -> Option<(Option<Program>, Vec<Diagnostic>)>
where
    F: Fn() -> bool,
{
    let mut lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    let mut masked_lines = HashSet::new();
    // shortcut: recover at most 64 malformed lines per refresh; raise this if large broken files need complete hints.
    for _ in 0..lines.len().min(64) {
        if is_cancelled() {
            return None;
        }
        match parse(&lines.join("\n")) {
            Ok(program) => return Some((Some(program), diagnostics)),
            Err(parse_diagnostics) => {
                let Some(diagnostic) = parse_diagnostics.into_iter().next() else {
                    return Some((None, diagnostics));
                };
                let line_index = diagnostic.span.start.line.saturating_sub(1);
                if line_index >= lines.len() || !masked_lines.insert(line_index) {
                    diagnostics.push(diagnostic);
                    break;
                }
                diagnostics.push(diagnostic);
                lines[line_index].clear();
            }
        }
    }
    Some((None, diagnostics))
}

fn analyze_program_line(program: &Program, line: usize) -> Option<SourceLineAnalysis> {
    if program.name_span.start.line == line {
        return Some(empty_analysis(
            line,
            program.name_span.start.column,
            SourceConstruct::Program,
        ));
    }
    if let Some(analysis) = analyze_declarations(&program.globals, line) {
        return Some(analysis);
    }
    if program.main.span.start.line == line || program.main.span.end.line == line {
        return Some(empty_analysis(line, 1, SourceConstruct::Program));
    }
    if let Some(analysis) = analyze_statements(&program.main.statements, line, None) {
        return Some(analysis);
    }

    for function in &program.functions {
        if let Some(analysis) = analyze_declarations(&function.locals, line) {
            return Some(analysis);
        }
        if line >= function.span.start.line && line < function.body.span.start.line {
            return Some(SourceLineAnalysis {
                line,
                column: if line == function.name_span.start.line {
                    function.name_span.start.column
                } else {
                    1
                },
                construct: SourceConstruct::Function,
                name: Some(function.name.clone()),
                return_type: function
                    .return_type
                    .map(|data_type| type_name(data_type).to_owned()),
                parameters: function
                    .parameters
                    .iter()
                    .flat_map(|declaration| source_symbols(declaration))
                    .collect(),
                variables: Vec::new(),
            });
        }
        if function.body.span.start.line == line || function.body.span.end.line == line {
            return Some(empty_analysis(line, 1, SourceConstruct::Program));
        }
        if let Some(analysis) = analyze_statements(
            &function.body.statements,
            line,
            function.return_type.map(type_name),
        ) {
            return Some(analysis);
        }
    }
    None
}

fn analyze_declarations(declarations: &[VariableDecl], line: usize) -> Option<SourceLineAnalysis> {
    let matching = declarations
        .iter()
        .filter(|declaration| declaration.span.start.line == line)
        .collect::<Vec<_>>();
    let first = *matching.first()?;
    Some(SourceLineAnalysis {
        line,
        column: first
            .names
            .first()
            .map_or(first.span.start.column, |name| name.span.start.column),
        construct: SourceConstruct::VariableDeclaration,
        name: None,
        return_type: None,
        parameters: Vec::new(),
        variables: matching.into_iter().flat_map(source_symbols).collect(),
    })
}

fn source_symbols(declaration: &VariableDecl) -> Vec<SourceSymbol> {
    declaration
        .names
        .iter()
        .map(|name| SourceSymbol {
            name: name.name.clone(),
            type_name: type_name(declaration.data_type).to_owned(),
            is_constant: false,
            by_reference: false,
            dimensions: declaration
                .dimensions
                .iter()
                .map(|size| SourceDimension {
                    lower_bound: Some(0),
                    upper_bound: size
                        .checked_sub(1)
                        .and_then(|upper| i64::try_from(upper).ok()),
                })
                .collect(),
            initializer: None,
        })
        .collect()
}

fn analyze_statements(
    statements: &[Statement],
    line: usize,
    return_type: Option<&str>,
) -> Option<SourceLineAnalysis> {
    statements
        .iter()
        .find_map(|statement| analyze_statement(statement, line, return_type))
}

fn analyze_statement(
    statement: &Statement,
    line: usize,
    return_type: Option<&str>,
) -> Option<SourceLineAnalysis> {
    if let Statement::Sequence { statements, .. } = statement
        && let Some(analysis) = analyze_statements(statements, line, return_type)
    {
        return Some(analysis);
    }
    let span = statement.span();
    if span.start.line == line {
        let (construct, name) = match statement {
            Statement::Assign { target, value, .. } => (
                if is_call_named(value, "leia") {
                    SourceConstruct::Input
                } else {
                    SourceConstruct::Assignment
                },
                Some(target.name.clone()),
            ),
            Statement::Call { call, .. } => (
                if is_call_named(call, "imprima") {
                    SourceConstruct::Output
                } else {
                    SourceConstruct::Other
                },
                None,
            ),
            Statement::Return { .. } => (SourceConstruct::Return, None),
            Statement::If { .. } => (SourceConstruct::Decision, None),
            Statement::While { .. } | Statement::Repeat { .. } => (SourceConstruct::Loop, None),
            Statement::For { target, .. } => (SourceConstruct::Loop, Some(target.name.clone())),
            Statement::Sequence { .. } => (SourceConstruct::Other, None),
        };
        return Some(SourceLineAnalysis {
            line,
            column: span.start.column,
            construct,
            name,
            return_type: if construct == SourceConstruct::Return {
                return_type.map(str::to_owned)
            } else {
                None
            },
            parameters: Vec::new(),
            variables: Vec::new(),
        });
    }

    match statement {
        Statement::If {
            then_branch,
            else_branch,
            ..
        } => analyze_statement(then_branch, line, return_type).or_else(|| {
            else_branch
                .as_deref()
                .and_then(|branch| analyze_statement(branch, line, return_type))
        }),
        Statement::While { body, .. }
        | Statement::Repeat { body, .. }
        | Statement::For { body, .. } => analyze_statement(body, line, return_type),
        Statement::Sequence { statements, .. } => analyze_statements(statements, line, return_type),
        Statement::Assign { .. } | Statement::Call { .. } | Statement::Return { .. } => None,
    }
}

fn is_call_named(expression: &Expr, expected: &str) -> bool {
    matches!(&expression.kind, ExprKind::Call { name, .. } if name.eq_ignore_ascii_case(expected))
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
    }
}

fn type_name(data_type: DataType) -> &'static str {
    match data_type {
        DataType::Integer => "inteiro",
        DataType::Real => "real",
        DataType::Character => "caractere",
        DataType::Literal => "literal",
        DataType::Logical => "lógico",
    }
}

#[path = "../../tests/learning/gportugol_tests.rs"]
#[cfg(test)]
mod tests;

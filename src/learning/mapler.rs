use super::{SourceCall, SourceConstruct, SourceDimension, SourceLineAnalysis, SourceSymbol};
use potoo_mapler_interpreter::{Diagnostic, ast::*, parse};

pub(super) fn analyze_source_line_cancellable(
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

    if let Some(variable) = program
        .variables
        .iter()
        .find(|variable| variable.line == one_based_line)
    {
        return Ok(Some(SourceLineAnalysis {
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
        }));
    }

    if let Some(module) = program
        .modules
        .iter()
        .find(|module| module.line == one_based_line)
    {
        return Ok(Some(SourceLineAnalysis {
            line: module.line,
            column: module.column,
            construct: SourceConstruct::Unrecognized,
            name: Some(module.name.clone()),
            return_type: None,
            parameters: Vec::new(),
            variables: Vec::new(),
            call: None,
        }));
    }

    for module in &program.modules {
        if let Some(analysis) = analyze_statements(&module.statements, one_based_line) {
            return Ok(Some(analysis));
        }
    }
    Ok(analyze_statements(&program.statements, one_based_line))
}

fn parse_recovering<F>(source: &str, is_cancelled: &F) -> (Option<Program>, Vec<Diagnostic>)
where
    F: Fn() -> bool,
{
    let mut lines = source.lines().map(str::to_owned).collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    let mut masked_lines = std::collections::HashSet::new();
    for _ in 0..lines.len().min(64) {
        if is_cancelled() {
            return (None, diagnostics);
        }
        match parse(&lines.join("\n")) {
            Ok(program) => return (Some(program), diagnostics),
            Err(parse_diagnostics) => {
                let Some(diagnostic) = parse_diagnostics.into_iter().next() else {
                    return (None, diagnostics);
                };
                let line_index = diagnostic.line.saturating_sub(1);
                if line_index >= lines.len() || !masked_lines.insert(line_index) {
                    diagnostics.push(diagnostic);
                    break;
                }
                diagnostics.push(diagnostic);
                lines[line_index].clear();
            }
        }
    }
    (None, diagnostics)
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
            Type::Logical => "lógico",
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
    use super::super::{LearningTopic, learning_hint};
    use crate::components::content_panel::Language;

    #[test]
    fn recognizes_mapler_output_and_module_calls() {
        let source = "variaveis\n  x: inteiro;\ninicio\n  escrever \"Olá\";\nfim.";
        assert_eq!(
            learning_hint(source, 4, Language::Mapler).topic,
            LearningTopic::Output
        );
    }
}

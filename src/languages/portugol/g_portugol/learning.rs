use std::collections::{HashMap, HashSet};

use crate::languages::model::{
    SourceConstruct, SourceDiagnostic, SourceDiagnosticKind, SourceDimension, SourceLineAnalysis,
    SourceLineAnalysisOutcome, SourceSymbol,
};
use crate::languages::source::{RecoveryStatus, SourceIndex, recover_parse};
use potoo_gportugol_interpreter::{DataType, Diagnostic, DiagnosticKind, ast::*, parse};

pub(crate) fn profile_adapter(
    source: &str,
    one_based_line: usize,
    is_cancelled: &dyn Fn() -> bool,
) -> SourceLineAnalysisOutcome {
    if is_cancelled() {
        return SourceLineAnalysisOutcome::Cancelled;
    }
    let source_index = SourceIndex::new_ignoring_leading_bom(source);
    if one_based_line == 0 || one_based_line > source_index.line_count() {
        return SourceLineAnalysisOutcome::Complete {
            analysis: None,
            diagnostics: Vec::new(),
        };
    }

    let recovered = recover_parse(
        source,
        parse,
        |diagnostic: &Diagnostic| (diagnostic.span.start.line, diagnostic.span.start.column),
        crate::languages::portugol::g_portugol::syntax_analysis::views::mask_diagnostic_statement,
        is_cancelled,
    );
    let mut diagnostics = recovered
        .diagnostics
        .into_iter()
        .map(to_source_diagnostic)
        .collect::<Vec<_>>();
    if !is_cancelled()
        && let Some(warning) =
            crate::languages::portugol::g_portugol::diagnostics::unclosed_block_comment_diagnostic(
                source,
            )
    {
        diagnostics.push(warning);
    }

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

pub(crate) fn profile_declared_array_bounds(
    source: &str,
    _masked_source: &str,
    is_cancelled: &dyn Fn() -> bool,
) -> Option<HashMap<String, Vec<SourceDimension>>> {
    if is_cancelled() {
        return None;
    }
    let program = parse(source).ok();
    if is_cancelled() {
        return None;
    }
    let Some(program) = program else {
        return Some(HashMap::new());
    };

    let mut bounds = HashMap::new();
    let mut ambiguous = HashSet::new();
    for declaration in program.globals.iter().chain(
        program
            .functions
            .iter()
            .flat_map(|function| function.locals.iter()),
    ) {
        if is_cancelled() {
            return None;
        }
        for symbol in source_symbols(declaration) {
            if symbol.dimensions.is_empty()
                || symbol
                    .dimensions
                    .iter()
                    .any(|dimension| dimension.upper_bound.is_none())
            {
                continue;
            }
            let name = symbol.name.to_lowercase();
            if ambiguous.contains(&name) {
                continue;
            }
            if bounds.insert(name.clone(), symbol.dimensions).is_some() {
                bounds.remove(&name);
                ambiguous.insert(name);
            }
        }
    }
    if is_cancelled() { None } else { Some(bounds) }
}

fn to_source_diagnostic(diagnostic: Diagnostic) -> SourceDiagnostic {
    let kind = match diagnostic.kind {
        DiagnosticKind::Lexical => SourceDiagnosticKind::Lexical,
        DiagnosticKind::Syntax => SourceDiagnosticKind::Syntax,
        DiagnosticKind::Type => SourceDiagnosticKind::Type,
        DiagnosticKind::Runtime => SourceDiagnosticKind::Runtime,
    };
    SourceDiagnostic {
        kind,
        message: diagnostic.message,
        line: diagnostic.span.start.line,
        column: diagnostic.span.start.column,
        byte_range: Some(diagnostic.span.start.byte_offset..diagnostic.span.end.byte_offset),
    }
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
                call: None,
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
        call: None,
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
            Statement::Assign { target, value, .. } => {
                let is_input = is_call_named(value, "leia");
                (
                    if is_input {
                        SourceConstruct::Input
                    } else {
                        SourceConstruct::Assignment
                    },
                    Some(if is_input {
                        "leia".to_owned()
                    } else {
                        target.name.clone()
                    }),
                )
            }
            Statement::Call { call, .. } => {
                let command = match &call.kind {
                    ExprKind::Call { name, .. } => Some(name.clone()),
                    _ => None,
                };
                let construct = if is_call_named(call, "imprima") {
                    SourceConstruct::Output
                } else if is_call_named(call, "leia") {
                    SourceConstruct::Input
                } else {
                    SourceConstruct::Call
                };
                (
                    construct,
                    (matches!(construct, SourceConstruct::Input | SourceConstruct::Output))
                        .then(|| command.unwrap_or_default()),
                )
            }
            Statement::Return { .. } => (SourceConstruct::Return, None),
            Statement::If { .. } => (SourceConstruct::Decision, Some("se".to_owned())),
            Statement::While { .. } => (SourceConstruct::Loop, Some("enquanto".to_owned())),
            Statement::Repeat { .. } => (SourceConstruct::Loop, Some("repita".to_owned())),
            Statement::For { .. } => (SourceConstruct::Loop, Some("para".to_owned())),
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
            call: None,
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
        call: None,
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

#[path = "../../../../tests/learning/gportugol_tests.rs"]
#[cfg(test)]
mod tests;

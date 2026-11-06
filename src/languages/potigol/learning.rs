use crate::features::learning::{catalog, source_call};
use crate::languages::model::{
    Language, SourceCall, SourceConstruct, SourceDiagnostic, SourceDiagnosticKind, SourceDimension,
    SourceLineAnalysis, SourceLineAnalysisOutcome, SourceSymbol,
};
use crate::languages::source::{RecoveryStatus, SourceIndex, recover_parse};
use potoo_potigol_interpreter::{Diagnostic, DiagnosticKind, ast::*, parse};

pub(crate) fn profile_adapter(
    source: &str,
    one_based_line: usize,
    is_cancelled: &dyn Fn() -> bool,
) -> SourceLineAnalysisOutcome {
    if is_cancelled() {
        return SourceLineAnalysisOutcome::Cancelled;
    }
    if one_based_line == 0 || one_based_line > SourceIndex::new(source).line_count() {
        return SourceLineAnalysisOutcome::Complete {
            analysis: None,
            diagnostics: Vec::new(),
        };
    }

    let recovered = recover_parse(
        source,
        parse,
        |diagnostic: &Diagnostic| (diagnostic.span.start.line, diagnostic.span.start.column),
        crate::languages::potigol::syntax_analysis::views::mask_diagnostic_statement,
        is_cancelled,
    );
    let mut diagnostics = recovered
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
            if let Some(diagnostic) = analysis
                .as_ref()
                .and_then(|analysis| missing_jerimum_import_diagnostic(&program, analysis))
            {
                diagnostics.push(diagnostic);
            }
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
    SourceDiagnostic::at_line(
        kind,
        diagnostic.message,
        diagnostic.span.start.line,
        diagnostic.span.start.column,
    )
}

fn analyze_program_line(
    source: &str,
    program: &Program,
    line: usize,
) -> Option<SourceLineAnalysis> {
    let jerimum_imported = jerimum_imported(program);
    let mut analysis = analyze_statements(source, &program.statements, line, jerimum_imported);
    if jerimum_imported
        && let Some(call) = analysis
            .as_mut()
            .and_then(|analysis| analysis.call.as_mut())
        && matches!(call.member.as_str(), "setColor" | "fillRect" | "drawString")
        && graphics_receiver_at_line(program, line, call.receiver.as_deref())
    {
        call.library = Some("jerimum".to_owned());
    }
    analysis
}

fn jerimum_imported(program: &Program) -> bool {
    program.statements.iter().any(|statement| {
        statement.span.start.column == 1
            && matches!(&statement.kind, StmtKind::Import(name) if name.eq_ignore_ascii_case("jerimum"))
    })
}

fn missing_jerimum_import_diagnostic(
    program: &Program,
    analysis: &SourceLineAnalysis,
) -> Option<SourceDiagnostic> {
    if jerimum_imported(program) {
        return None;
    }
    let call = analysis.call.as_ref()?;
    if !catalog::is_jerimum_call_candidate(Language::Potigol, call) {
        return None;
    }
    let confirmed_jerimum_call = match (call.receiver.as_deref(), call.member.as_str()) {
        (Some("jogo"), "iniciar") => call.arguments.len() == 6,
        (Some("Desenho"), "incluir") => call.arguments.len() == 2,
        (Some(receiver), "setColor" | "fillRect" | "drawString") => {
            graphics_receiver_at_line(program, analysis.line, Some(receiver))
        }
        _ => false,
    };
    confirmed_jerimum_call.then(|| {
        SourceDiagnostic::at_line(
            SourceDiagnosticKind::StaticAnalysis,
            "A chamada pertence à biblioteca Jerimum, mas ela não foi importada. Adicione `use \"jerimum\"` no início do arquivo.".to_owned(),
            analysis.line,
            analysis.column,
        )
    })
}

fn graphics_receiver_at_line(program: &Program, line: usize, receiver: Option<&str>) -> bool {
    let Some(receiver) = receiver else {
        return false;
    };
    program.statements.iter().any(|statement| {
        let StmtKind::Function(function) = &statement.kind else {
            return false;
        };
        let FunctionBody::Block(block) = &function.body else {
            return false;
        };
        block.span.start.line <= line
            && block.span.end.line >= line
            && function.parameters.iter().any(|parameter| {
                parameter.name == receiver
                    && parameter.annotation.as_ref().is_some_and(|annotation| {
                        matches!(
                            &annotation.kind,
                            TypeExprKind::Named(name, arguments)
                                if name == "Graphics2D" && arguments.is_empty()
                        )
                    })
            })
    })
}

fn analyze_statements(
    source: &str,
    statements: &[Stmt],
    line: usize,
    jerimum_imported: bool,
) -> Option<SourceLineAnalysis> {
    for statement in statements {
        if statement.span.start.line == line {
            let analysis = analyze_statement(source, statement, jerimum_imported);
            if analysis.construct != SourceConstruct::Unrecognized {
                return Some(analysis);
            }
        }
        if let Some(analysis) = analyze_nested_statement(source, statement, line, jerimum_imported)
        {
            return Some(analysis);
        }
    }
    None
}

fn analyze_nested_statement(
    source: &str,
    statement: &Stmt,
    line: usize,
    jerimum_imported: bool,
) -> Option<SourceLineAnalysis> {
    match &statement.kind {
        StmtKind::Function(function) => match &function.body {
            FunctionBody::Block(block) => {
                analyze_statements(source, &block.statements, line, jerimum_imported)
            }
            FunctionBody::Expression(expression) => (expression.span.start.line == line)
                .then(|| analyze_expression(source, expression, jerimum_imported)),
            FunctionBody::Abstract => None,
        },
        StmtKind::Block(block) => {
            analyze_statements(source, &block.statements, line, jerimum_imported)
        }
        StmtKind::While { body, .. } | StmtKind::For { body, .. } => {
            analyze_statements(source, &body.statements, line, jerimum_imported)
        }
        StmtKind::Expression(expression) => {
            expression.nested_block_at_line(line).and_then(|block| {
                analyze_statements(source, &block.statements, line, jerimum_imported)
            })
        }
        _ => None,
    }
}

trait ExpressionBlocks {
    fn nested_block_at_line(&self, line: usize) -> Option<&Block>;
}

impl ExpressionBlocks for Expr {
    fn nested_block_at_line(&self, line: usize) -> Option<&Block> {
        match &self.kind {
            ExprKind::Conditional {
                then_branch,
                else_branch,
                ..
            } => {
                if then_branch.span.start.line <= line && then_branch.span.end.line >= line {
                    Some(then_branch)
                } else {
                    else_branch.as_ref().filter(|block| {
                        block.span.start.line <= line && block.span.end.line >= line
                    })
                }
            }
            ExprKind::While { body, .. } | ExprKind::For { body, .. } | ExprKind::Block(body) => {
                (body.span.start.line <= line && body.span.end.line >= line).then_some(body)
            }
            _ => None,
        }
    }
}

fn analyze_statement(source: &str, statement: &Stmt, jerimum_imported: bool) -> SourceLineAnalysis {
    let line = statement.span.start.line;
    let column = statement.span.start.column;
    let mut analysis = empty_analysis(line, column, SourceConstruct::Unrecognized);
    match &statement.kind {
        StmtKind::Binding {
            pattern,
            annotation,
            mutable,
            value,
        } => {
            let input_call = call_in_expression(source, value, jerimum_imported);
            analysis.construct = if input_call.as_ref().is_some_and(is_potigol_input) {
                SourceConstruct::Input
            } else {
                SourceConstruct::VariableDeclaration
            };
            analysis.variables = pattern_symbols(pattern, annotation.as_ref(), *mutable);
            analysis.call = input_call;
        }
        StmtKind::Assignment { target, value } => {
            analysis.call = call_in_expression(source, value, jerimum_imported);
            if let Some(call) = &analysis.call
                && is_potigol_input(call)
            {
                analysis.construct = SourceConstruct::Input;
                analysis.name = Some(call.member.clone());
            } else {
                analysis.construct = SourceConstruct::Assignment;
                analysis.name = expression_path(target);
            }
        }
        StmtKind::Function(function) => {
            analysis.construct = SourceConstruct::Function;
            analysis.name = Some(function.name.clone());
            analysis.return_type = function.return_type.as_ref().map(type_name);
            analysis.parameters = function
                .parameters
                .iter()
                .map(|parameter| SourceSymbol {
                    name: parameter.name.clone(),
                    type_name: parameter
                        .annotation
                        .as_ref()
                        .map(type_name)
                        .unwrap_or_else(|| "?".to_owned()),
                    is_constant: !parameter.mutable,
                    by_reference: false,
                    dimensions: Vec::new(),
                    initializer: None,
                })
                .collect();
        }
        StmtKind::Type(_) | StmtKind::TypeAlias { .. } => {}
        StmtKind::Import(library) => {
            analysis.construct = SourceConstruct::Import;
            analysis.name = Some(library.clone());
        }
        StmtKind::Block(block) => {
            analysis.construct = SourceConstruct::Unrecognized;
            if let Some(nested) =
                analyze_statements(source, &block.statements, line, jerimum_imported)
            {
                return nested;
            }
        }
        StmtKind::Output { newline, .. } => {
            analysis.construct = SourceConstruct::Output;
            analysis.name = Some(if *newline { "escreva" } else { "imprima" }.to_owned());
        }
        StmtKind::Return(_) => analysis.construct = SourceConstruct::Return,
        StmtKind::While { .. } => {
            analysis.construct = SourceConstruct::Loop;
            analysis.name = Some("enquanto".to_owned());
        }
        StmtKind::For { .. } => {
            analysis.construct = SourceConstruct::Loop;
            analysis.name = Some("para".to_owned());
        }
        StmtKind::Expression(expression) => {
            analysis = analyze_expression(source, expression, jerimum_imported);
        }
    }
    analysis
}

fn analyze_expression(
    source: &str,
    expression: &Expr,
    jerimum_imported: bool,
) -> SourceLineAnalysis {
    let mut analysis = empty_analysis(
        expression.span.start.line,
        expression.span.start.column,
        SourceConstruct::Unrecognized,
    );
    match &expression.kind {
        ExprKind::Call { callee, .. } => {
            let call = make_call(source, expression, callee, jerimum_imported);
            let is_input = is_potigol_input(&call);
            analysis.construct = if is_input {
                SourceConstruct::Input
            } else if call.receiver.is_some() {
                SourceConstruct::MethodCall
            } else {
                SourceConstruct::Call
            };
            if is_input {
                analysis.name = Some(call.member.clone());
            }
            analysis.call = Some(call);
        }
        ExprKind::Conditional { .. } => {
            analysis.construct = SourceConstruct::Decision;
            analysis.name = Some("se".to_owned());
        }
        ExprKind::While { .. } => {
            analysis.construct = SourceConstruct::Loop;
            analysis.name = Some("enquanto".to_owned());
        }
        ExprKind::For { .. } => {
            analysis.construct = SourceConstruct::Loop;
            analysis.name = Some("para".to_owned());
        }
        _ => {}
    }
    analysis
}

fn call_in_expression(
    source: &str,
    expression: &Expr,
    jerimum_imported: bool,
) -> Option<SourceCall> {
    match &expression.kind {
        ExprKind::Call { callee, .. } => {
            Some(make_call(source, expression, callee, jerimum_imported))
        }
        ExprKind::Binary { left, right, .. } => call_in_expression(source, left, jerimum_imported)
            .or_else(|| call_in_expression(source, right, jerimum_imported)),
        ExprKind::Unary { operand, .. } => call_in_expression(source, operand, jerimum_imported),
        ExprKind::Member { receiver, .. } | ExprKind::Index { receiver, .. } => {
            call_in_expression(source, receiver, jerimum_imported)
        }
        ExprKind::Conditional {
            condition,
            then_branch,
            else_branch,
        } => call_in_expression(source, condition, jerimum_imported)
            .or_else(|| {
                then_branch
                    .statements
                    .iter()
                    .find_map(|statement| call_in_statement(source, statement, jerimum_imported))
            })
            .or_else(|| {
                else_branch.as_ref().and_then(|block| {
                    block.statements.iter().find_map(|statement| {
                        call_in_statement(source, statement, jerimum_imported)
                    })
                })
            }),
        _ => None,
    }
}

fn call_in_statement(source: &str, statement: &Stmt, jerimum_imported: bool) -> Option<SourceCall> {
    match &statement.kind {
        StmtKind::Expression(expression)
        | StmtKind::Output {
            value: expression, ..
        }
        | StmtKind::Return(Some(expression)) => {
            call_in_expression(source, expression, jerimum_imported)
        }
        StmtKind::Binding { value, .. } | StmtKind::Assignment { value, .. } => {
            call_in_expression(source, value, jerimum_imported)
        }
        _ => None,
    }
}

fn make_call(source: &str, expression: &Expr, callee: &Expr, jerimum_imported: bool) -> SourceCall {
    let path = expression_path(callee).unwrap_or_else(|| "?".to_owned());
    let (receiver, member) = path
        .rsplit_once('.')
        .map_or((None, path.as_str()), |(receiver, member)| {
            (Some(receiver.to_owned()), member)
        });
    let library = (jerimum_imported
        && matches!(
            (receiver.as_deref(), member),
            (Some("jogo"), "iniciar") | (Some("Desenho"), "incluir")
        ))
    .then(|| "jerimum".to_owned());
    source_call(
        source,
        expression.span.start.line,
        expression.span.start.column,
        receiver,
        member,
        library,
    )
}

fn expression_path(expression: &Expr) -> Option<String> {
    match &expression.kind {
        ExprKind::Identifier(name) => Some(name.clone()),
        ExprKind::Member { receiver, name } => {
            Some(format!("{}.{}", expression_path(receiver)?, name))
        }
        _ => None,
    }
}

fn is_potigol_input(call: &SourceCall) -> bool {
    matches!(
        call.member.as_str(),
        "leia_inteiro"
            | "leia_real"
            | "leia_numero"
            | "leia_número"
            | "leia_texto"
            | "leia_caractere"
            | "leia_logico"
            | "leia_inteiros"
            | "leia_reais"
            | "leia_numeros"
            | "leia_números"
            | "leia_textos"
    )
}

fn pattern_symbols(
    pattern: &Pattern,
    annotation: Option<&TypeExpr>,
    mutable: bool,
) -> Vec<SourceSymbol> {
    match &pattern.kind {
        PatternKind::Binding(name) => vec![SourceSymbol {
            name: name.clone(),
            type_name: annotation.map(type_name).unwrap_or_else(|| "?".to_owned()),
            is_constant: !mutable,
            by_reference: false,
            dimensions: Vec::<SourceDimension>::new(),
            initializer: None,
        }],
        PatternKind::Tuple(patterns) | PatternKind::List(patterns) => patterns
            .iter()
            .flat_map(|pattern| pattern_symbols(pattern, annotation, mutable))
            .collect(),
        PatternKind::Constructor { fields, .. } => fields
            .iter()
            .flat_map(|pattern| pattern_symbols(pattern, annotation, mutable))
            .collect(),
        _ => Vec::new(),
    }
}

fn type_name(type_expr: &TypeExpr) -> String {
    match &type_expr.kind {
        TypeExprKind::Named(name, arguments) if arguments.is_empty() => name.clone(),
        TypeExprKind::Named(name, arguments) => format!(
            "{}[{}]",
            name,
            arguments
                .iter()
                .map(type_name)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        TypeExprKind::Tuple(items) => format!(
            "({})",
            items.iter().map(type_name).collect::<Vec<_>>().join(", ")
        ),
        TypeExprKind::Function(arguments, result) => format!(
            "({}) -> {}",
            arguments
                .iter()
                .map(type_name)
                .collect::<Vec<_>>()
                .join(", "),
            type_name(result)
        ),
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

#[cfg(test)]
#[path = "../../../tests/learning/potigol_tests.rs"]
mod tests;

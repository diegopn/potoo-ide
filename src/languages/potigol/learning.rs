use crate::features::learning::source_call;
use crate::languages::model::{
    SourceCall, SourceConstruct, SourceDimension, SourceLineAnalysis, SourceSymbol,
};
use potoo_potigol_interpreter::{Diagnostic, ast::*, parse};

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
        .any(|diagnostic| diagnostic.span.start.line == one_based_line)
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
    let jerimum_imported = crate::languages::potigol::syntax_analysis::views::is_jerimum_import(
        source,
    ) && program.statements.iter().any(|statement| {
        matches!(&statement.kind, StmtKind::Import(name) if name.eq_ignore_ascii_case("jerimum"))
    });
    let mut analysis = analyze_statements(
        source,
        &program.statements,
        one_based_line,
        jerimum_imported,
    );
    if jerimum_imported
        && let Some(call) = analysis
            .as_mut()
            .and_then(|analysis| analysis.call.as_mut())
        && matches!(call.member.as_str(), "setColor" | "fillRect" | "drawString")
        && graphics_receiver_at_line(&program, one_based_line, call.receiver.as_deref())
    {
        call.library = Some("jerimum".to_owned());
    }
    Ok(analysis)
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
                let line = diagnostic.span.start.line;
                let column = diagnostic.span.start.column;
                if attempt == 64 {
                    diagnostics.push(diagnostic);
                    break;
                }
                let recovered =
                    crate::languages::potigol::syntax_analysis::views::mask_diagnostic_statement(
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

use crate::languages::model::{SourceDiagnostic, SourceDiagnosticKind};

pub(crate) fn analyze(source: &str) -> Vec<SourceDiagnostic> {
    let errors = match potoo_gportugol_interpreter::parse(source) {
        Ok(program) => potoo_gportugol_interpreter::check(&program)
            .err()
            .unwrap_or_default(),
        Err(errors) => errors,
    };
    errors
        .into_iter()
        .map(|diagnostic| SourceDiagnostic {
            kind: match diagnostic.kind {
                potoo_gportugol_interpreter::DiagnosticKind::Lexical => {
                    SourceDiagnosticKind::Lexical
                }
                potoo_gportugol_interpreter::DiagnosticKind::Syntax => SourceDiagnosticKind::Syntax,
                potoo_gportugol_interpreter::DiagnosticKind::Type => SourceDiagnosticKind::Type,
                potoo_gportugol_interpreter::DiagnosticKind::Runtime => {
                    SourceDiagnosticKind::Runtime
                }
            },
            message: diagnostic.message,
            line: diagnostic.span.start.line,
            column: diagnostic.span.start.column,
            byte_range: Some(diagnostic.span.start.byte_offset..diagnostic.span.end.byte_offset),
        })
        .collect()
}

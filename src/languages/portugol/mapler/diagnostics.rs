use crate::languages::model::{SourceDiagnostic, SourceDiagnosticKind};

pub(crate) fn analyze(source: &str) -> Vec<SourceDiagnostic> {
    let errors = match potoo_mapler_interpreter::parse(source) {
        Ok(program) => potoo_mapler_interpreter::check(&program)
            .err()
            .unwrap_or_default(),
        Err(errors) => errors,
    };
    errors
        .into_iter()
        .map(|diagnostic| SourceDiagnostic {
            kind: match diagnostic.kind {
                potoo_mapler_interpreter::DiagnosticKind::Lexical => SourceDiagnosticKind::Lexical,
                potoo_mapler_interpreter::DiagnosticKind::Syntax => SourceDiagnosticKind::Syntax,
                potoo_mapler_interpreter::DiagnosticKind::Type => SourceDiagnosticKind::Type,
                potoo_mapler_interpreter::DiagnosticKind::Runtime => SourceDiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
            byte_range: None,
        })
        .collect()
}

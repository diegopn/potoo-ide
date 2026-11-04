use crate::languages::model::{SourceDiagnostic, SourceDiagnosticKind};

pub(crate) fn analyze(source: &str) -> Vec<SourceDiagnostic> {
    potoo_calango_interpreter::parse(source)
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|diagnostic| SourceDiagnostic {
            kind: match diagnostic.kind {
                potoo_calango_interpreter::DiagnosticKind::Lexical => SourceDiagnosticKind::Lexical,
                potoo_calango_interpreter::DiagnosticKind::Syntax => SourceDiagnosticKind::Syntax,
                potoo_calango_interpreter::DiagnosticKind::Type => SourceDiagnosticKind::Type,
                potoo_calango_interpreter::DiagnosticKind::Runtime => SourceDiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
            byte_range: None,
        })
        .collect()
}

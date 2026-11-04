use crate::languages::model::{SourceDiagnostic, SourceDiagnosticKind};

pub(crate) fn analyze(source: &str) -> Vec<SourceDiagnostic> {
    potoo_visualg_interpreter::parse(source)
        .err()
        .unwrap_or_default()
        .into_iter()
        .map(|diagnostic| SourceDiagnostic {
            kind: match diagnostic.kind {
                potoo_visualg_interpreter::DiagnosticKind::Lexical => SourceDiagnosticKind::Lexical,
                potoo_visualg_interpreter::DiagnosticKind::Syntax => SourceDiagnosticKind::Syntax,
                potoo_visualg_interpreter::DiagnosticKind::Runtime => SourceDiagnosticKind::Runtime,
            },
            message: diagnostic.message,
            line: diagnostic.line,
            column: diagnostic.column,
            byte_range: None,
        })
        .collect()
}

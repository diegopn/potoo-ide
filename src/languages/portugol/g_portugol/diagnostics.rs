use crate::languages::model::{SourceDiagnostic, SourceDiagnosticKind};

pub(crate) fn analyze(source: &str) -> Vec<SourceDiagnostic> {
    let errors = match potoo_gportugol_interpreter::parse(source) {
        Ok(program) => potoo_gportugol_interpreter::check(&program)
            .err()
            .unwrap_or_default(),
        Err(errors) => errors,
    };
    let mut diagnostics = errors
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
        .collect::<Vec<_>>();

    if let Some((start, line, column)) =
        super::syntax_analysis::views::unclosed_block_comment(source)
    {
        let byte_start = source
            .char_indices()
            .nth(start)
            .map_or(source.len(), |(offset, _)| offset);
        diagnostics.push(SourceDiagnostic {
            kind: SourceDiagnosticKind::StaticAnalysis,
            message: "Comentário de bloco sem fechamento".to_owned(),
            line,
            column,
            byte_range: Some(byte_start..source.len()),
        });
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::analyze;
    use crate::languages::model::SourceDiagnosticKind;

    #[test]
    fn reports_unclosed_block_comments_as_static_analysis() {
        let source = "algoritmo teste; início imprima(); /* comentário aberto";
        let diagnostics = analyze(source);

        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == SourceDiagnosticKind::StaticAnalysis)
            .expect("the open comment should be reported by the G-Portugol profile");

        assert_eq!(diagnostic.message, "Comentário de bloco sem fechamento");
        assert_eq!(diagnostic.line, 1);
        assert_eq!(
            diagnostic.byte_range,
            Some(source.find("/*").unwrap()..source.len())
        );
        assert_eq!(
            diagnostic.column,
            source[..source.find("/*").unwrap()].chars().count() + 1
        );
    }
}

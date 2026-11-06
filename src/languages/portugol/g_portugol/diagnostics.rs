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

    if let Some(warning) = unclosed_block_comment_diagnostic(source) {
        diagnostics.push(warning);
    }

    diagnostics
}

pub(crate) fn unclosed_block_comment_diagnostic(source: &str) -> Option<SourceDiagnostic> {
    let (start, line, column) = super::syntax_analysis::views::unclosed_block_comment(source)?;
    let byte_start = source
        .char_indices()
        .nth(start)
        .map_or(source.len(), |(offset, _)| offset);
    Some(SourceDiagnostic {
        kind: SourceDiagnosticKind::StaticAnalysis,
        message: "Comentário de bloco sem fechamento".to_owned(),
        line,
        column,
        byte_range: Some(byte_start..source.len()),
    })
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

    #[test]
    fn unclosed_comment_coordinates_handle_cr_and_crlf_lines() {
        for newline in ["\r", "\r\n"] {
            let source = format!("algoritmo teste;{newline}/* comentário aberto");
            let diagnostics = analyze(&source);
            let diagnostic = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.kind == SourceDiagnosticKind::StaticAnalysis)
                .expect("the open comment should be reported");

            assert_eq!(diagnostic.line, 2, "newline {newline:?}");
            assert_eq!(diagnostic.column, 1, "newline {newline:?}");
            assert_eq!(
                diagnostic.byte_range,
                Some(source.find("/*").unwrap()..source.len()),
                "newline {newline:?}"
            );
        }
    }

    #[test]
    fn block_comment_marker_inside_bare_cr_line_comment_is_not_reported() {
        let source = "algoritmo teste;\rinício\r// comentário\r/*\n@;\rimprima(1);\rfim";
        let diagnostics = analyze(source);

        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.kind != SourceDiagnosticKind::StaticAnalysis),
            "a block marker inside a G-Portugol line comment is not an open comment"
        );
    }

    #[test]
    fn unclosed_comment_after_a_leading_bom_uses_parser_columns_and_byte_offsets() {
        let source = "\u{feff}algoritmo teste; início /* comentário aberto";
        let diagnostics = analyze(source);
        let marker = source.find("/*").unwrap();
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind == SourceDiagnosticKind::StaticAnalysis)
            .expect("the open comment should be reported");

        assert_eq!(diagnostic.line, 1);
        assert_eq!(
            diagnostic.column,
            source["\u{feff}".len()..marker].chars().count() + 1
        );
        assert_eq!(diagnostic.byte_range, Some(marker..source.len()));
    }
}

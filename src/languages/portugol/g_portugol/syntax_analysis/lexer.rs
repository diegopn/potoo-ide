use crate::languages::source::{
    LexerConfig, LineCommentTerminator, LineEndingPolicy, QuoteRule, SourceIndex, SourceToken,
    source_view, tokenize,
};

pub(super) const CONFIG: LexerConfig<'static> = LexerConfig {
    line_comments: &["//"],
    line_comment_terminator: LineCommentTerminator::LfOrCrlf,
    line_endings: LineEndingPolicy::Universal,
    block_comments: &[("/*", "*/")],
    quote_rules: &[
        QuoteRule {
            delimiter: '"',
            escape: Some('\\'),
            doubled_delimiter: false,
            allows_newlines: false,
        },
        QuoteRule {
            delimiter: '\'',
            escape: Some('\\'),
            doubled_delimiter: false,
            allows_newlines: true,
        },
    ],
    hyphenated_words: &["fim-variáveis", "fim-enquanto", "fim-para", "fim-se"],
    ignore_leading_bom: true,
};

pub(super) fn tokens(source: &str) -> Vec<SourceToken> {
    tokenize(source, &CONFIG)
}

pub(crate) fn source_line_code(source: &str, line: usize) -> String {
    let view = source_view(source, &tokens(source), true);
    SourceIndex::new_ignoring_leading_bom(&view)
        .line_text(line)
        .unwrap_or("")
        .to_owned()
}

pub(crate) fn without_literals_and_comments(source: &str) -> String {
    source_view(source, &tokens(source), false)
}

#[cfg(test)]
mod tests {
    use super::{source_line_code, tokens, without_literals_and_comments};
    use crate::languages::source::{SourceTokenKind, token_text};

    #[test]
    fn shared_tokens_keep_gportugol_accents_and_hyphenated_closers() {
        let source = "algoritmo teste; variáveis x: inteiro; fim-variáveis início se verdadeiro então fim-se até";
        let tokens = tokens(source);
        let words = tokens
            .iter()
            .filter(|token| token.kind == SourceTokenKind::Word)
            .filter_map(|token| token_text(source, token))
            .collect::<Vec<_>>();

        for keyword in ["variáveis", "fim-variáveis", "início", "fim-se", "até"] {
            assert!(words.contains(&keyword), "missing {keyword:?}: {words:?}");
        }
    }

    #[test]
    fn hyphens_are_preserved_only_in_known_closing_keywords() {
        let source = "se-1 fim-se fim-enquanto fim-sexta";
        let tokens = tokens(source);
        let words = tokens
            .iter()
            .filter(|token| token.kind == SourceTokenKind::Word)
            .filter_map(|token| token_text(source, token))
            .collect::<Vec<_>>();

        assert_eq!(words, ["se", "fim-se", "fim-enquanto", "fim", "sexta"]);
        assert_eq!(tokens[1].kind, SourceTokenKind::Symbol('-'));
        assert_eq!(tokens[6].kind, SourceTokenKind::Symbol('-'));
    }

    #[test]
    fn bom_and_multiline_character_literals_keep_interpreter_coordinates() {
        let source = "\u{feff}início\nimprima('\n');\n@";
        let tokens = tokens(source);
        let first = tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Word)
            .expect("the BOM is not a source column in G-Portugol");
        assert_eq!(token_text(source, first), Some("início"));
        assert_eq!(
            (first.line, first.column, first.byte_range.start),
            (1, 1, 3)
        );
        assert_eq!(source_line_code(source, 1), "início");

        let literal = tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Literal)
            .expect("a G-Portugol character literal may contain a raw newline");
        assert_eq!(token_text(source, literal), Some("'\n'"));
        let at = tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Symbol('@'))
            .expect("the invalid token after the literal remains visible");
        assert_eq!((at.line, at.column), (4, 1));
    }

    #[test]
    fn shared_views_preserve_literals_and_mask_comments_without_losing_newlines() {
        let source = "início // comentário\n imprima(\"fim-se; // literal\"); /* se\n fim-enquanto */ imprima(1);\nfim";
        let selected = source_line_code(source, 2);
        assert!(selected.contains("fim-se; // literal"));
        assert!(!selected.contains("fim-enquanto"));

        let masked = without_literals_and_comments(source);
        assert!(!masked.contains("literal"));
        assert!(!masked.contains("comentário"));
        assert!(masked.contains("imprima"));
        assert_eq!(masked.lines().count(), source.lines().count());
    }

    #[test]
    fn source_line_code_handles_bare_carriage_returns_and_crlf() {
        let source = "início\r imprima(\"CR\");\r\n imprima(\"CRLF\");\rfim";

        assert_eq!(source_line_code(source, 2).trim(), "imprima(\"CR\");");
        assert_eq!(source_line_code(source, 3).trim(), "imprima(\"CRLF\");");
        assert_eq!(source_line_code(source, 4).trim(), "fim");
    }
}

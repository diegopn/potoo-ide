use crate::languages::source::{
    LexerConfig, LineCommentTerminator, LineEndingPolicy, QuoteRule, SourceIndex, SourceToken,
    source_view, tokenize,
};

pub(super) const CONFIG: LexerConfig<'static> = LexerConfig {
    line_comments: &["//"],
    line_comment_terminator: LineCommentTerminator::LfOnly,
    line_endings: LineEndingPolicy::LfOnly,
    block_comments: &[("/*", "*/")],
    quote_rules: &[
        QuoteRule {
            delimiter: '"',
            escape: Some('\\'),
            doubled_delimiter: false,
            allows_newlines: false,
            allows_escaped_newlines: true,
            allows_carriage_returns: true,
        },
        QuoteRule {
            delimiter: '\'',
            escape: Some('\\'),
            doubled_delimiter: false,
            allows_newlines: false,
            allows_escaped_newlines: true,
            allows_carriage_returns: true,
        },
    ],
    hyphenated_words: &[],
    ignore_leading_bom: false,
};

pub(super) fn tokens(source: &str) -> Vec<SourceToken> {
    tokenize(source, &CONFIG)
}

pub(crate) fn source_line_code(source: &str, line: usize) -> String {
    let view = source_view(source, &tokens(source), true);
    SourceIndex::new_lf_only(&view)
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
    use crate::languages::source::{SourceIndex, SourceTokenKind, token_text};
    use potoo_calango_interpreter::parse;

    #[test]
    fn shared_tokens_match_calango_lf_only_unicode_coordinates() {
        let source = "algoritmo aula;\r\nprincipal\r\n/* comentário */\nescreval(\"ação\"); @; // fimPrincipal\rtexto\nfimPrincipal";
        let expected = parse(source)
            .expect_err("the token after a complete statement is invalid")
            .remove(0);
        let tokens = tokens(source);
        let at = tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Symbol('@'))
            .expect("the invalid token remains visible");

        assert_eq!((at.line, at.column), (expected.line, expected.column));
        assert_eq!(
            SourceIndex::new_lf_only(source).byte_to_line_column(at.byte_range.start),
            Some((expected.line, expected.column))
        );
        assert!(source_line_code(source, 4).contains("ação"));
        assert!(!source_line_code(source, 4).contains("fimPrincipal"));
    }

    #[test]
    fn strings_and_characters_hide_escaped_quotes_and_comment_markers() {
        let source = r#"escreval("fimSe; \"// literal /* valor */\""); escreva('x'); /* fimPrincipal; */ // fimSe"#;
        let tokens = tokens(source);
        let words = tokens
            .iter()
            .filter(|token| token.kind == SourceTokenKind::Word)
            .filter_map(|token| token_text(source, token))
            .collect::<Vec<_>>();

        assert_eq!(words, ["escreval", "escreva"]);
        let view = without_literals_and_comments(source);
        assert!(!view.contains("literal"));
        assert!(!view.contains("valor"));
        assert!(view.contains("escreval"));
        assert!(view.contains("escreva"));
    }

    #[test]
    fn escaped_lf_and_raw_cr_inside_literals_follow_the_pinned_parser() {
        for source in ["escreval(\"a\\\nb\"); @", "escreval(\"a\rb\"); @"] {
            let expected = parse(&format!(
                "algoritmo aula;\nprincipal\n{source}\nfimPrincipal"
            ))
            .expect_err("the trailing token should be diagnosed")
            .remove(0);
            let wrapped = format!("algoritmo aula;\nprincipal\n{source}\nfimPrincipal");
            let at = tokens(&wrapped)
                .into_iter()
                .find(|token| token.kind == SourceTokenKind::Symbol('@'))
                .expect("the trailing invalid token remains visible");

            assert_eq!((at.line, at.column), (expected.line, expected.column));
        }
    }
}

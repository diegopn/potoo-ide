use crate::languages::source::{
    LexerConfig, LineCommentTerminator, LineEndingPolicy, QuoteRule, SourceIndex, SourceToken,
    source_view, tokenize,
};

pub(super) const CONFIG: LexerConfig<'static> = LexerConfig {
    line_comments: &["#"],
    line_comment_terminator: LineCommentTerminator::LfOrCrlfRejectStandaloneCr,
    line_endings: LineEndingPolicy::Universal,
    bare_cr_advances_line_without_separator: true,
    interpolated_strings: true,
    block_comments: &[],
    quote_rules: &[
        QuoteRule {
            delimiter: '"',
            escape: Some('\\'),
            doubled_delimiter: false,
            allows_newlines: true,
            allows_escaped_newlines: false,
            allows_carriage_returns: true,
        },
        QuoteRule {
            delimiter: '\'',
            escape: None,
            doubled_delimiter: false,
            allows_newlines: false,
            allows_escaped_newlines: false,
            allows_carriage_returns: false,
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
    SourceIndex::new(&view)
        .line_text(line)
        .unwrap_or("")
        .to_owned()
}

pub(crate) fn without_literals_and_comments(source: &str) -> String {
    source_view(source, &tokens(source), false)
}

#[cfg(test)]
mod tests {
    use super::tokens;
    use crate::languages::source::{SourceIndex, SourceTokenKind, token_text};
    use potoo_potigol_interpreter::parse;

    #[test]
    fn token_positions_match_the_parser_for_unicode_newlines_and_bare_carriage_returns() {
        for source in ["escreva \"ação\"\r\n@\r\n", "escreva 1\r@"] {
            let expected = parse(source)
                .expect_err("the source contains an invalid token")
                .remove(0);
            let at = tokens(source)
                .into_iter()
                .find(|token| token.kind == SourceTokenKind::Symbol('@'))
                .expect("the invalid token remains visible");

            assert_eq!(
                (at.line, at.column),
                (expected.span.start.line, expected.span.start.column),
                "{source:?}"
            );
            assert_eq!(
                SourceIndex::new(source).byte_to_line_column(at.byte_range.start),
                Some((expected.span.start.line, expected.span.start.column)),
                "{source:?}"
            );
        }
    }

    #[test]
    fn only_hash_comments_are_removed_and_eof_comments_keep_their_error() {
        let source = "# comentário\r\n@\n";
        let expected = parse(source)
            .expect_err("the token after the valid comment is invalid")
            .remove(0);
        let at = tokens(source)
            .into_iter()
            .find(|token| token.kind == SourceTokenKind::Symbol('@'))
            .expect("the token after CRLF remains visible");
        assert_eq!(
            (at.line, at.column),
            (expected.span.start.line, expected.span.start.column)
        );

        let unterminated = "# comentário";
        let expected = parse(unterminated)
            .expect_err("the comment requires LF or CRLF")
            .remove(0);
        assert!(tokens(unterminated).is_empty());
        assert_eq!(
            (expected.span.start.line, expected.span.start.column),
            (1, 1)
        );

        let malformed = "# comentário\r@\n";
        let expected = parse(malformed)
            .expect_err("a standalone CR is not a valid comment terminator")
            .remove(0);
        assert_eq!(
            (expected.span.start.line, expected.span.start.column),
            (1, 1)
        );
        let malformed_tokens = tokens(malformed);
        let at = malformed_tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Symbol('@'))
            .expect("code after the malformed comment remains visible");
        assert_eq!((at.line, at.column), (2, 1));

        let slash_comment = "escreva 1 // não é comentário\n";
        assert!(
            tokens(slash_comment)
                .iter()
                .any(|token| { token.kind == SourceTokenKind::Symbol('/') })
        );
    }

    #[test]
    fn interpolated_text_keeps_nested_strings_and_comment_markers_atomic() {
        for source in [
            r#"escreva "valor {"a;b"}"
@
"#,
            "escreva \"valor {1 # ; } comentário\n + 2}\"\n@\n",
            "escreva \"valor {1 // ; } comentário\n + 2}\"\n@\n",
        ] {
            let expected = parse(source)
                .expect_err("the token after the complete interpolated string is invalid")
                .remove(0);
            let tokens = tokens(source);
            let literal = tokens
                .iter()
                .find(|token| token.kind == SourceTokenKind::Literal)
                .expect("the outer string remains one literal token");
            let at = tokens
                .iter()
                .find(|token| token.kind == SourceTokenKind::Symbol('@'))
                .expect("the invalid token after the string remains visible");

            assert!(token_text(source, literal).unwrap().contains("}"));
            assert_eq!(
                (at.line, at.column),
                (expected.span.start.line, expected.span.start.column),
                "{source:?}"
            );
        }
    }
}

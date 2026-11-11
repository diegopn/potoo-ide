use crate::languages::source::{
    LexerConfig, LineCommentTerminator, LineEndingPolicy, QuoteRule, SourceIndex, SourceToken,
    source_view, tokenize,
};

pub(super) const CONFIG: LexerConfig<'static> = LexerConfig {
    line_comments: &["//"],
    line_comment_terminator: LineCommentTerminator::LfOnly,
    line_endings: LineEndingPolicy::LfOnly,
    bare_cr_advances_line_without_separator: false,
    interpolated_strings: false,
    block_comments: &[],
    quote_rules: &[QuoteRule {
        delimiter: '"',
        escape: None,
        doubled_delimiter: false,
        allows_newlines: false,
        allows_escaped_newlines: false,
        allows_carriage_returns: false,
    }],
    hyphenated_words: &[],
    ignore_leading_bom: false,
};

pub(super) fn tokens(source: &str) -> Vec<SourceToken> {
    tokenize(source, &CONFIG)
}

pub(crate) fn source_line_code(source: &str, line: usize) -> String {
    let tokens = tokens(source);
    let view = source_view(source, &tokens, true);
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
    use crate::languages::source::token_text;

    use super::{source_line_code, tokens, without_literals_and_comments};

    #[test]
    fn mapler_comments_end_only_at_lf_and_keep_parser_coordinates() {
        let source = "variaveis\ninicio\r// comentário\r @;\nescrever \"ok\";\nfim.";
        let tokens = tokens(source);
        assert!(
            !tokens
                .iter()
                .any(|token| token_text(source, token) == Some("@"))
        );
        let write = tokens
            .iter()
            .find(|token| token_text(source, token) == Some("escrever"))
            .expect("the statement after LF remains code");
        assert_eq!((write.line, write.column), (3, 1));
        assert!(source_line_code(source, 2).starts_with("inicio"));
        assert!(potoo_mapler_interpreter::parse(source).is_ok());
    }

    #[test]
    fn mapler_view_does_not_treat_unsupported_block_comments_as_comments() {
        let view = without_literals_and_comments("valor /* valor */");

        assert_eq!(view, "valor /* valor */");
    }
}

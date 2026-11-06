use crate::languages::source::{
    LexerConfig, LineCommentTerminator, LineEndingPolicy, QuoteRule, SourceIndex, SourceToken,
    source_view, tokenize,
};

pub(super) const CONFIG: LexerConfig<'static> = LexerConfig {
    line_comments: &["//"],
    line_comment_terminator: LineCommentTerminator::AnyNewline,
    line_endings: LineEndingPolicy::Universal,
    block_comments: &[],
    quote_rules: &[QuoteRule {
        delimiter: '"',
        escape: None,
        doubled_delimiter: true,
        allows_newlines: false,
    }],
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
    use crate::languages::source::{SourceTokenKind, token_text};
    use potoo_visualg_interpreter::parse;

    #[test]
    fn shared_tokens_match_visualg_unicode_and_universal_line_coordinates() {
        let source = "algoritmo \"aula\"\r\ninicio\r  ação @\rfimalgoritmo";
        let diagnostic = parse(source)
            .expect_err("the invalid character after a Unicode identifier is diagnosed")
            .remove(0);
        let tokens = tokens(source);
        let action = tokens
            .iter()
            .find(|token| {
                token.kind == SourceTokenKind::Word && token_text(source, token) == Some("ação")
            })
            .expect("the shared scanner retains the identifier text");
        let invalid = tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Symbol('@'))
            .expect("the invalid character remains a token for range selection");

        assert_eq!((action.line, action.column), (3, 3));
        assert_eq!(
            (invalid.line, invalid.column),
            (diagnostic.line, diagnostic.column)
        );
        assert_eq!((diagnostic.line, diagnostic.column), (3, 8));
    }

    #[test]
    fn doubled_quotes_keep_comment_and_block_words_inside_one_literal() {
        let source = "escreval(\"fimse \"\"// literal fimpara\"\"\") // fimse";
        let tokens = tokens(source);
        let literals = tokens
            .iter()
            .filter(|token| token.kind == SourceTokenKind::Literal)
            .collect::<Vec<_>>();

        assert_eq!(literals.len(), 1);
        assert_eq!(
            token_text(source, literals[0]),
            Some("\"fimse \"\"// literal fimpara\"\"\"")
        );
        assert_eq!(
            tokens
                .iter()
                .filter(|token| token.kind == SourceTokenKind::Word)
                .filter_map(|token| token_text(source, token))
                .collect::<Vec<_>>(),
            ["escreval"]
        );
    }
}

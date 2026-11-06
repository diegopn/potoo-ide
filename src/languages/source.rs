pub(crate) fn read_identifier(characters: &[char], cursor: &mut usize) -> Option<String> {
    skip_spaces(characters, cursor);
    let start = *cursor;
    if !characters
        .get(*cursor)
        .is_some_and(|character| character.is_alphabetic() || *character == '_')
    {
        return None;
    }
    *cursor += 1;
    while characters
        .get(*cursor)
        .is_some_and(|character| character.is_alphanumeric() || *character == '_')
    {
        *cursor += 1;
    }
    Some(characters[start..*cursor].iter().collect())
}

pub(crate) fn read_integer(characters: &[char], cursor: &mut usize) -> Option<i64> {
    let start = *cursor;
    if matches!(characters.get(*cursor), Some('-' | '+')) {
        *cursor += 1;
    }
    while characters
        .get(*cursor)
        .is_some_and(|character| character.is_ascii_digit())
    {
        *cursor += 1;
    }
    if *cursor == start || (*cursor == start + 1 && matches!(characters[start], '-' | '+')) {
        *cursor = start;
        return None;
    }
    characters[start..*cursor]
        .iter()
        .collect::<String>()
        .parse()
        .ok()
}

pub(crate) fn skip_spaces(characters: &[char], cursor: &mut usize) {
    while characters
        .get(*cursor)
        .is_some_and(|character| character.is_whitespace())
    {
        *cursor += 1;
    }
}

pub(crate) fn skip_to_top_level_comma(characters: &[char], mut cursor: usize) -> usize {
    let mut square_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    while let Some(character) = characters.get(cursor) {
        match character {
            '[' => square_depth += 1,
            ']' => square_depth = square_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if square_depth == 0 && brace_depth == 0 && paren_depth == 0 => return cursor,
            _ => {}
        }
        cursor += 1;
    }
    cursor
}

#[cfg(test)]
mod shared_source_tests {
    use std::cell::Cell;

    use super::{
        LexerConfig, LineCommentTerminator, LineEndingPolicy, MaskRules, QuoteRule, RecoveryStatus,
        SourceIndex, SourceTokenKind, StatementRules, close_open_blocks,
        mask_range_preserving_positions, mask_range_preserving_positions_with_rules, recover_parse,
        statement_range, token_text, tokenize,
    };

    #[test]
    fn source_index_maps_unicode_tabs_crlf_and_eof_without_accepting_invalid_coordinates() {
        let source = "açã\tB\r\nfim";
        let index = SourceIndex::new(source);

        assert_eq!(index.line_column_to_byte(1, 1), Some(0));
        assert_eq!(index.line_column_to_byte(1, 2), Some(1));
        assert_eq!(index.line_column_to_byte(1, 3), Some(3));
        assert_eq!(index.line_column_to_byte(1, 6), Some(7));
        assert_eq!(index.line_column_to_byte(1, 7), None);
        assert_eq!(index.line_column_to_byte(1, 8), None);
        assert_eq!(index.line_column_to_byte(2, 1), Some(9));
        assert_eq!(index.line_column_to_byte(2, 4), Some(source.len()));
        assert_eq!(index.line_column_to_byte(0, 1), None);
        assert_eq!(index.line_column_to_byte(1, 0), None);
        assert_eq!(index.line_column_to_byte(3, 1), None);

        assert_eq!(index.byte_to_line_column(0), Some((1, 1)));
        assert_eq!(index.byte_to_line_column(2), None);
        assert_eq!(index.byte_to_line_column(3), Some((1, 3)));
        assert_eq!(index.byte_to_line_column(7), Some((1, 6)));
        assert_eq!(index.byte_to_line_column(8), Some((1, 6)));
        assert_eq!(index.byte_to_line_column(9), Some((2, 1)));
        assert_eq!(index.byte_to_line_column(source.len()), Some((2, 4)));
        assert_eq!(index.byte_to_line_column(source.len() + 1), None);
    }

    #[test]
    fn source_index_treats_empty_input_and_a_trailing_newline_as_real_lines() {
        let empty = SourceIndex::new("");
        assert_eq!(empty.line_count(), 1);
        assert_eq!(empty.line_text(1), Some(""));
        assert_eq!(empty.line_column_to_byte(1, 1), Some(0));
        assert_eq!(empty.line_column_to_byte(1, 2), None);
        assert_eq!(empty.line_column_to_byte(2, 1), None);
        assert_eq!(empty.byte_to_line_column(0), Some((1, 1)));

        let trailing_newline = SourceIndex::new("a\n");
        assert_eq!(trailing_newline.line_count(), 2);
        assert_eq!(trailing_newline.line_text(2), Some(""));
        assert_eq!(trailing_newline.line_column_to_byte(2, 1), Some(2));
        assert_eq!(trailing_newline.byte_to_line_column(2), Some((2, 1)));

        let carriage_return = SourceIndex::new("a\rb");
        assert_eq!(carriage_return.line_count(), 2);
        assert_eq!(carriage_return.line_text(1), Some("a"));
        assert_eq!(carriage_return.line_text(2), Some("b"));
        assert_eq!(carriage_return.line_column_to_byte(2, 1), Some(2));
        assert_eq!(carriage_return.byte_to_line_column(2), Some((2, 1)));

        let mixed = SourceIndex::new("a\r\nb\rc\n");
        assert_eq!(mixed.line_count(), 4);
        assert_eq!(mixed.line_text(1), Some("a"));
        assert_eq!(mixed.line_text(2), Some("b"));
        assert_eq!(mixed.line_text(3), Some("c"));
        assert_eq!(mixed.line_text(4), Some(""));
    }

    #[test]
    fn lf_only_source_index_counts_carriage_returns_as_columns() {
        let source = "a\r\nb\rc\n";
        let index = SourceIndex::new_lf_only(source);

        assert_eq!(index.line_count(), 3);
        assert_eq!(index.line_text(1), Some("a"));
        assert_eq!(index.line_text(2), Some("b\rc"));
        assert_eq!(index.line_text(3), Some(""));
        assert_eq!(index.line_column_to_byte(1, 1), Some(0));
        assert_eq!(index.line_column_to_byte(1, 2), Some(1));
        assert_eq!(index.line_column_to_byte(1, 3), Some(2));
        assert_eq!(index.line_column_to_byte(1, 4), None);
        assert_eq!(index.line_column_to_byte(2, 1), Some(3));
        assert_eq!(index.line_column_to_byte(2, 3), Some(5));
        assert_eq!(index.byte_to_line_column(1), Some((1, 2)));
        assert_eq!(index.byte_to_line_column(2), Some((1, 3)));
        assert_eq!(index.byte_to_line_column(3), Some((2, 1)));
    }

    #[test]
    fn source_index_can_ignore_a_leading_bom_for_language_coordinates() {
        let source = "\u{feff}ação\r\nfim";
        let index = SourceIndex::new_ignoring_leading_bom(source);

        assert_eq!(index.line_text(1), Some("ação"));
        assert_eq!(index.line_column_to_byte(1, 1), Some(3));
        assert_eq!(index.line_column_to_byte(1, 2), Some(4));
        assert_eq!(index.byte_to_line_column(3), Some((1, 1)));
        assert_eq!(index.line_column_to_byte(2, 1), Some(11));
    }

    #[test]
    fn tokenizer_skips_comments_and_keeps_escaped_and_doubled_quote_literals_atomic() {
        let source = r#"ação fim-se "fim \"se"; // para
x 'd''e' # enquanto
/* fim
se */ z"#;
        let config = LexerConfig {
            line_comments: &["//", "#"],
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
                    escape: None,
                    doubled_delimiter: true,
                    allows_newlines: false,
                },
            ],
            hyphenated_words: &["fim-se"],
            line_comment_terminator: LineCommentTerminator::AnyNewline,
            line_endings: LineEndingPolicy::Universal,
            ignore_leading_bom: false,
        };

        let tokens = tokenize(source, &config);
        let texts = tokens
            .iter()
            .map(|token| token_text(source, token).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(
            texts,
            [
                "ação",
                "fim-se",
                r#""fim \"se""#,
                ";",
                "\n",
                "x",
                "'d''e'",
                "\n",
                "\n",
                "z",
            ]
        );
        assert_eq!(tokens[0].kind, SourceTokenKind::Word);
        assert_eq!(tokens[1].kind, SourceTokenKind::Word);
        assert_eq!(tokens[2].kind, SourceTokenKind::Literal);
        assert_eq!(tokens[6].kind, SourceTokenKind::Literal);
        assert!(
            tokens
                .iter()
                .all(|token| source.is_char_boundary(token.byte_range.start)
                    && source.is_char_boundary(token.byte_range.end))
        );
        assert_eq!((tokens[0].line, tokens[0].column), (1, 1));
        assert_eq!((tokens[5].line, tokens[5].column), (2, 1));
        assert_eq!((tokens[9].line, tokens[9].column), (4, 7));
    }

    #[test]
    fn tokenizer_supports_language_specific_comment_and_quote_line_rules() {
        let source = "a\r// comentário\r/*\n@";
        let config = LexerConfig {
            line_comments: &["//"],
            line_comment_terminator: LineCommentTerminator::LfOrCrlf,
            block_comments: &[("/*", "*/")],
            quote_rules: &[QuoteRule {
                delimiter: '\'',
                escape: Some('\\'),
                doubled_delimiter: false,
                allows_newlines: true,
            }],
            hyphenated_words: &[],
            line_endings: LineEndingPolicy::Universal,
            ignore_leading_bom: false,
        };

        let tokens = tokenize(source, &config);
        let at = tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Symbol('@'))
            .expect("the code after the line comment remains tokenized");
        assert_eq!((at.line, at.column), (4, 1));
        let literal_source = "'b\nc' @";
        let literal_tokens = tokenize(literal_source, &config);
        let literal = literal_tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Literal)
            .expect("a single-quoted literal may span lines in this dialect");
        assert_eq!(token_text(literal_source, literal), Some("'b\nc'"));
        let after_literal = literal_tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Symbol('@'))
            .expect("code after the multiline literal remains tokenized");
        assert_eq!((after_literal.line, after_literal.column), (2, 4));
    }

    #[test]
    fn lf_only_tokenizer_keeps_bare_carriage_returns_inside_the_current_line() {
        let config = LexerConfig {
            line_comments: &["//"],
            line_comment_terminator: LineCommentTerminator::LfOnly,
            line_endings: LineEndingPolicy::LfOnly,
            block_comments: &[],
            quote_rules: &[QuoteRule {
                delimiter: '"',
                escape: None,
                doubled_delimiter: false,
                allows_newlines: false,
            }],
            hyphenated_words: &[],
            ignore_leading_bom: false,
        };

        let source = "a\r// comentário\rtexto\n@";
        let tokens = tokenize(source, &config);
        let at = tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Symbol('@'))
            .expect("code after the LF-terminated comment remains tokenized");
        assert_eq!((at.line, at.column), (2, 1));

        let source = "a\r@";
        let tokens = tokenize(source, &config);
        let at = tokens
            .iter()
            .find(|token| token.kind == SourceTokenKind::Symbol('@'))
            .expect("a bare carriage return is whitespace, not a line ending");
        assert_eq!((at.line, at.column), (1, 3));
    }

    #[test]
    fn statement_range_respects_nested_delimiters_semicolons_and_line_continuations() {
        let source = "first; second(x; y); third\nvalue = a +\n  b\nnext";
        let config = LexerConfig {
            line_comments: &[],
            line_comment_terminator: LineCommentTerminator::AnyNewline,
            block_comments: &[],
            quote_rules: &[],
            hyphenated_words: &[],
            line_endings: LineEndingPolicy::Universal,
            ignore_leading_bom: false,
        };
        let tokens = tokenize(source, &config);
        let rules = StatementRules {
            semicolon_separator: true,
            block_pairs: &[],
            root_pairs: &[],
            multiline_continuations: &["+"],
            brace_delimited_blocks: false,
            case_sensitive_keywords: true,
        };

        let second = statement_range(source, &tokens, 1, 8, &rules).unwrap();
        assert_eq!(&source[second], "second(x; y);");
        let continuation = statement_range(source, &tokens, 3, 3, &rules).unwrap();
        assert_eq!(&source[continuation], "value = a +\n  b");
        assert!(statement_range(source, &tokens, 8, 1, &rules).is_none());

        let adjacent_source = "a;@;z";
        let adjacent_tokens = tokenize(adjacent_source, &config);
        let adjacent = statement_range(adjacent_source, &adjacent_tokens, 1, 3, &rules).unwrap();
        assert_eq!(&adjacent_source[adjacent], "@;");

        let ambiguous_source = "first(\n  unfinished";
        let ambiguous_tokens = tokenize(ambiguous_source, &config);
        assert!(statement_range(ambiguous_source, &ambiguous_tokens, 2, 3, &rules).is_none());
    }

    #[test]
    fn statement_range_includes_a_diagnostic_at_the_statement_end_column() {
        let source = "x <-";
        let config = LexerConfig {
            line_comments: &[],
            line_comment_terminator: LineCommentTerminator::AnyNewline,
            block_comments: &[],
            quote_rules: &[],
            hyphenated_words: &[],
            line_endings: LineEndingPolicy::Universal,
            ignore_leading_bom: false,
        };
        let tokens = tokenize(source, &config);
        let rules = StatementRules {
            semicolon_separator: false,
            block_pairs: &[],
            root_pairs: &[],
            multiline_continuations: &[],
            brace_delimited_blocks: false,
            case_sensitive_keywords: false,
        };

        let range = statement_range(source, &tokens, 1, 5, &rules)
            .expect("the parser can locate a missing expression at end of line");
        assert_eq!(&source[range], "x <-");
    }

    #[test]
    fn statement_range_selects_one_statement_inside_nested_blocks() {
        let source = "if outer then\n  if inner then\n    broken @; after\n  end if\nend if";
        let config = LexerConfig {
            line_comments: &[],
            line_comment_terminator: LineCommentTerminator::AnyNewline,
            block_comments: &[],
            quote_rules: &[],
            hyphenated_words: &[],
            line_endings: LineEndingPolicy::Universal,
            ignore_leading_bom: false,
        };
        let tokens = tokenize(source, &config);
        let rules = StatementRules {
            semicolon_separator: true,
            block_pairs: &[("if then", "end if")],
            root_pairs: &[],
            multiline_continuations: &[],
            brace_delimited_blocks: false,
            case_sensitive_keywords: true,
        };

        let broken = statement_range(source, &tokens, 3, 5, &rules).unwrap();
        assert_eq!(&source[broken], "broken @;");
    }

    #[test]
    fn close_open_blocks_closes_nested_word_and_brace_scopes_in_order() {
        let source = "program begin\n  if then\n    work()";
        let config = LexerConfig {
            line_comments: &[],
            line_comment_terminator: LineCommentTerminator::AnyNewline,
            block_comments: &[],
            quote_rules: &[],
            hyphenated_words: &[],
            line_endings: LineEndingPolicy::Universal,
            ignore_leading_bom: false,
        };
        let tokens = tokenize(source, &config);
        let rules = StatementRules {
            semicolon_separator: false,
            block_pairs: &[("if then", "end if")],
            root_pairs: &[("program begin", "end program")],
            multiline_continuations: &[],
            brace_delimited_blocks: false,
            case_sensitive_keywords: true,
        };

        assert_eq!(
            close_open_blocks(source, &tokens, &rules).as_deref(),
            Some("program begin\n  if then\n    work()\nend if\nend program")
        );

        let braced_source = "program\n{\n  if then\n    work()";
        let braced_tokens = tokenize(braced_source, &config);
        let braced_rules = StatementRules {
            semicolon_separator: false,
            block_pairs: &[("if then", "end if")],
            root_pairs: &[("program", "end program")],
            multiline_continuations: &[],
            brace_delimited_blocks: true,
            case_sensitive_keywords: true,
        };
        assert_eq!(
            close_open_blocks(braced_source, &braced_tokens, &braced_rules).as_deref(),
            Some("program\n{\n  if then\n    work()\nend if\n}\nend program")
        );

        let overlapping_source = "while do\n  work\nend if";
        let overlapping_tokens = tokenize(overlapping_source, &config);
        let overlapping_rules = StatementRules {
            semicolon_separator: false,
            block_pairs: &[("if then", "end"), ("while do", "end if")],
            root_pairs: &[],
            multiline_continuations: &[],
            brace_delimited_blocks: false,
            case_sensitive_keywords: true,
        };
        assert_eq!(
            close_open_blocks(overlapping_source, &overlapping_tokens, &overlapping_rules)
                .as_deref(),
            Some(overlapping_source)
        );
    }

    #[test]
    fn masking_preserves_newlines_and_source_after_the_selected_range() {
        let source = "áx\r\nruim\nfinal";
        let masked = mask_range_preserving_positions(source, 5..9).unwrap();
        assert_eq!(masked, "áx\r\n    \nfinal");
        assert_eq!(masked.len(), source.len());
        assert_eq!(masked.lines().count(), source.lines().count());
        assert!(mask_range_preserving_positions(source, 1..2).is_none());
        assert!(mask_range_preserving_positions(source, 7..6).is_none());

        let accented_source = "á界 + fim";
        let accented_mask = mask_range_preserving_positions(accented_source, 0..5).unwrap();
        assert_eq!(
            accented_mask.chars().count(),
            accented_source.chars().count()
        );
        let original_suffix_column = SourceIndex::new(accented_source)
            .byte_to_line_column(accented_source.find(" + fim").unwrap())
            .unwrap();
        let masked_suffix_column = SourceIndex::new(&accented_mask)
            .byte_to_line_column(accented_mask.find(" + fim").unwrap())
            .unwrap();
        assert_eq!(masked_suffix_column, original_suffix_column);

        let byte_preserving_mask = mask_range_preserving_positions_with_rules(
            accented_source,
            0..5,
            MaskRules::byte_offsets(Some('\u{00a0}'), Some('\u{3000}')),
        )
        .unwrap();
        assert_eq!(byte_preserving_mask.len(), accented_source.len());
        assert_eq!(
            SourceIndex::new(&byte_preserving_mask).byte_to_line_column(5),
            SourceIndex::new(accented_source).byte_to_line_column(5)
        );
        assert!(
            byte_preserving_mask
                .chars()
                .take(2)
                .all(char::is_whitespace)
        );
        assert!(mask_range_preserving_positions("😀 x", 0..4).is_some());
        assert!(
            mask_range_preserving_positions_with_rules(
                "😀 x",
                0..4,
                MaskRules::byte_offsets(Some('\u{00a0}'), Some('\u{3000}')),
            )
            .is_none()
        );

        let g_portugol_mask_rules = MaskRules::byte_offsets(Some('\u{00a0}'), None);
        assert!(
            mask_range_preserving_positions_with_rules(
                accented_source,
                0..2,
                g_portugol_mask_rules,
            )
            .is_some()
        );
        assert!(
            mask_range_preserving_positions_with_rules(
                accented_source,
                2..5,
                g_portugol_mask_rules,
            )
            .is_none()
        );
    }

    #[test]
    fn recovery_returns_all_discovered_diagnostics_after_a_later_success() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct Diagnostic(usize, usize, &'static str);

        let parse_calls = Cell::new(0);
        let mask_calls = Cell::new(0);
        let outcome = recover_parse(
            "x",
            |candidate| {
                let calls = parse_calls.get() + 1;
                parse_calls.set(calls);
                if calls == 3 {
                    Ok(candidate.to_owned())
                } else {
                    Err(vec![Diagnostic(1, calls, "erro")])
                }
            },
            |diagnostic| (diagnostic.0, diagnostic.1),
            |candidate, _, _| {
                mask_calls.set(mask_calls.get() + 1);
                Some(format!("{candidate}#"))
            },
            || false,
        );

        assert_eq!(outcome.status, RecoveryStatus::Complete);
        assert_eq!(outcome.program.as_deref(), Some("x##"));
        assert_eq!(outcome.diagnostics.len(), 2);
        assert_eq!(parse_calls.get(), 3);
        assert_eq!(mask_calls.get(), 2);
    }

    #[test]
    fn recovery_accepts_clean_source_without_masking() {
        let parse_calls = Cell::new(0);
        let outcome = recover_parse(
            "clean",
            |candidate| {
                parse_calls.set(parse_calls.get() + 1);
                Ok::<_, Vec<()>>(candidate.to_owned())
            },
            |_| (1, 1),
            |_, _, _| panic!("clean source must not be masked"),
            || false,
        );

        assert_eq!(outcome.status, RecoveryStatus::Complete);
        assert_eq!(outcome.program.as_deref(), Some("clean"));
        assert!(outcome.diagnostics.is_empty());
        assert_eq!(parse_calls.get(), 1);
    }

    #[test]
    fn recovery_can_succeed_after_exactly_sixty_four_changed_masks() {
        let parse_calls = Cell::new(0);
        let mask_calls = Cell::new(0);
        let outcome = recover_parse(
            "x",
            |candidate| {
                let calls = parse_calls.get() + 1;
                parse_calls.set(calls);
                if calls == 65 {
                    Ok(candidate.to_owned())
                } else {
                    Err(vec![(calls, 1)])
                }
            },
            |diagnostic| *diagnostic,
            |candidate, _, _| {
                mask_calls.set(mask_calls.get() + 1);
                Some(format!("{candidate}#"))
            },
            || false,
        );

        assert_eq!(outcome.status, RecoveryStatus::Complete);
        assert_eq!(parse_calls.get(), 65);
        assert_eq!(mask_calls.get(), 64);
        assert_eq!(outcome.diagnostics.len(), 64);
    }

    #[test]
    fn recovery_reports_stall_cancellation_and_the_exact_mask_limit() {
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct Diagnostic(usize, usize);

        let stalled = recover_parse(
            "x",
            |_| Err::<String, _>(vec![Diagnostic(1, 1)]),
            |diagnostic| (diagnostic.0, diagnostic.1),
            |_, _, _| None,
            || false,
        );
        assert_eq!(stalled.status, RecoveryStatus::Stalled);
        assert_eq!(stalled.diagnostics.len(), 1);

        let unchanged = recover_parse(
            "x",
            |_| Err::<String, _>(vec![Diagnostic(1, 1)]),
            |diagnostic| (diagnostic.0, diagnostic.1),
            |candidate, _, _| Some(candidate.to_owned()),
            || false,
        );
        assert_eq!(unchanged.status, RecoveryStatus::Stalled);
        assert_eq!(unchanged.diagnostics.len(), 1);

        let parse_calls = Cell::new(0);
        let cancellation_checks = Cell::new(0);
        let cancelled = recover_parse(
            "x",
            |_| {
                parse_calls.set(parse_calls.get() + 1);
                Err::<String, _>(vec![Diagnostic(1, 1)])
            },
            |diagnostic| (diagnostic.0, diagnostic.1),
            |candidate, _, _| Some(format!("{candidate}#")),
            || {
                let checks = cancellation_checks.get() + 1;
                cancellation_checks.set(checks);
                checks == 2
            },
        );
        assert_eq!(cancelled.status, RecoveryStatus::Cancelled);
        assert_eq!(parse_calls.get(), 1);

        let parse_calls = Cell::new(0);
        let mask_calls = Cell::new(0);
        let limited = recover_parse(
            "x",
            |_| {
                parse_calls.set(parse_calls.get() + 1);
                Err::<String, _>(vec![Diagnostic(1, 1)])
            },
            |diagnostic| (diagnostic.0, diagnostic.1),
            |candidate, _, _| {
                mask_calls.set(mask_calls.get() + 1);
                Some(format!("{candidate}#"))
            },
            || false,
        );
        assert_eq!(limited.status, RecoveryStatus::LimitReached);
        assert_eq!(parse_calls.get(), 65);
        assert_eq!(mask_calls.get(), 64);
        assert_eq!(limited.diagnostics.len(), 65);
    }
}
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineEndingPolicy {
    Universal,
    LfOnly,
}

#[derive(Clone, Debug)]
pub(crate) struct SourceIndex<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
    line_endings: LineEndingPolicy,
}

impl<'a> SourceIndex<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self::with_first_line_start(source, 0, LineEndingPolicy::Universal)
    }

    pub(crate) fn new_lf_only(source: &'a str) -> Self {
        Self::with_first_line_start(source, 0, LineEndingPolicy::LfOnly)
    }

    pub(crate) fn new_ignoring_leading_bom(source: &'a str) -> Self {
        let first_line_start = source
            .strip_prefix('\u{feff}')
            .map_or(0, |without_bom| source.len() - without_bom.len());
        Self::with_first_line_start(source, first_line_start, LineEndingPolicy::Universal)
    }

    fn with_first_line_start(
        source: &'a str,
        first_line_start: usize,
        line_endings: LineEndingPolicy,
    ) -> Self {
        let mut line_starts = vec![first_line_start];
        let bytes = source.as_bytes();
        let mut offset = first_line_start;
        while offset < bytes.len() {
            let width = match (line_endings, bytes[offset]) {
                (LineEndingPolicy::Universal, b'\r') if bytes.get(offset + 1) == Some(&b'\n') => 2,
                (LineEndingPolicy::Universal, b'\r' | b'\n')
                | (LineEndingPolicy::LfOnly, b'\n') => 1,
                _ => {
                    offset += 1;
                    continue;
                }
            };
            offset += width;
            line_starts.push(offset);
        }
        Self {
            source,
            line_starts,
            line_endings,
        }
    }

    pub(crate) fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub(crate) fn line_text(&self, line: usize) -> Option<&'a str> {
        let start = *self.line_starts.get(line.checked_sub(1)?)?;
        let next_start = self
            .line_starts
            .get(line)
            .copied()
            .unwrap_or(self.source.len());
        self.source
            .get(start..line_text_end(self.source, start, next_start, self.line_endings))
    }

    pub(crate) fn line_column_to_byte(&self, line: usize, column: usize) -> Option<usize> {
        if line == 0 || column == 0 {
            return None;
        }
        let line_index = line - 1;
        let start = *self.line_starts.get(line_index)?;
        let next_start = self
            .line_starts
            .get(line_index + 1)
            .copied()
            .unwrap_or(self.source.len());
        let end = line_coordinate_end(self.source, start, next_start, self.line_endings);
        let text = self.source.get(start..end)?;
        let target_character = column - 1;

        for (character_index, (byte_offset, _)) in text.char_indices().enumerate() {
            if character_index == target_character {
                return Some(start + byte_offset);
            }
        }

        (target_character == text.chars().count()).then_some(end)
    }

    pub(crate) fn byte_to_line_column(&self, byte: usize) -> Option<(usize, usize)> {
        if byte > self.source.len() || !self.source.is_char_boundary(byte) {
            return None;
        }
        let line_index = self
            .line_starts
            .partition_point(|line_start| *line_start <= byte)
            .saturating_sub(1);
        let start = *self.line_starts.get(line_index)?;
        let next_start = self
            .line_starts
            .get(line_index + 1)
            .copied()
            .unwrap_or(self.source.len());
        let end = line_coordinate_end(self.source, start, next_start, self.line_endings);
        let position = byte.min(end);
        let column = self.source.get(start..position)?.chars().count() + 1;
        Some((line_index + 1, column))
    }
}

fn line_coordinate_end(
    source: &str,
    start: usize,
    next_start: usize,
    line_endings: LineEndingPolicy,
) -> usize {
    let bytes = source.as_bytes();
    let mut end = next_start;
    if end > start && bytes.get(end - 1) == Some(&b'\n') {
        end -= 1;
        if line_endings == LineEndingPolicy::Universal
            && end > start
            && bytes.get(end - 1) == Some(&b'\r')
        {
            end -= 1;
        }
    } else if line_endings == LineEndingPolicy::Universal
        && end > start
        && bytes.get(end - 1) == Some(&b'\r')
    {
        end -= 1;
    }
    end
}

fn line_text_end(
    source: &str,
    start: usize,
    next_start: usize,
    line_endings: LineEndingPolicy,
) -> usize {
    let mut end = line_coordinate_end(source, start, next_start, line_endings);
    if line_endings == LineEndingPolicy::LfOnly
        && end > start
        && source.as_bytes().get(end - 1) == Some(&b'\r')
        && source.as_bytes().get(next_start.saturating_sub(1)) == Some(&b'\n')
    {
        end -= 1;
    }
    end
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceTokenKind {
    Word,
    Literal,
    Symbol(char),
    Newline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceToken {
    pub(crate) kind: SourceTokenKind,
    pub(crate) byte_range: Range<usize>,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QuoteRule {
    pub(crate) delimiter: char,
    pub(crate) escape: Option<char>,
    pub(crate) doubled_delimiter: bool,
    pub(crate) allows_newlines: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineCommentTerminator {
    AnyNewline,
    LfOrCrlf,
    LfOnly,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LexerConfig<'a> {
    pub(crate) line_comments: &'a [&'a str],
    pub(crate) line_comment_terminator: LineCommentTerminator,
    pub(crate) line_endings: LineEndingPolicy,
    pub(crate) block_comments: &'a [(&'a str, &'a str)],
    pub(crate) quote_rules: &'a [QuoteRule],
    pub(crate) hyphenated_words: &'a [&'a str],
    pub(crate) ignore_leading_bom: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct StatementRules<'a> {
    pub(crate) semicolon_separator: bool,
    pub(crate) block_pairs: &'a [(&'a str, &'a str)],
    pub(crate) root_pairs: &'a [(&'a str, &'a str)],
    pub(crate) multiline_continuations: &'a [&'a str],
    pub(crate) brace_delimited_blocks: bool,
    pub(crate) case_sensitive_keywords: bool,
}

pub(crate) fn token_text<'a>(source: &'a str, token: &SourceToken) -> Option<&'a str> {
    source.get(token.byte_range.clone())
}

pub(crate) fn source_view(source: &str, tokens: &[SourceToken], preserve_literals: bool) -> String {
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;

    for token in tokens {
        let range = token.byte_range.clone();
        let Some(gap) = source.get(cursor..range.start) else {
            continue;
        };
        result.extend(gap.chars().map(mask_comment_character));
        let Some(text) = token_text(source, token) else {
            continue;
        };
        if token.kind == SourceTokenKind::Literal && !preserve_literals {
            result.extend(text.chars().map(mask_literal_character));
        } else {
            result.push_str(text);
        }
        cursor = range.end;
    }

    if let Some(gap) = source.get(cursor..) {
        result.extend(gap.chars().map(mask_comment_character));
    }
    result
}

fn mask_comment_character(character: char) -> char {
    if character.is_whitespace() || character == '\u{feff}' {
        character
    } else {
        ' '
    }
}

fn mask_literal_character(character: char) -> char {
    if matches!(character, '\n' | '\r' | '\t') {
        character
    } else {
        ' '
    }
}

pub(crate) fn tokenize(source: &str, config: &LexerConfig<'_>) -> Vec<SourceToken> {
    let mut tokens = Vec::new();
    let mut offset = if config.ignore_leading_bom {
        source
            .strip_prefix('\u{feff}')
            .map_or(0, |without_bom| source.len() - without_bom.len())
    } else {
        0
    };
    let (mut line, mut column) = (1, 1);

    while offset < source.len() {
        if newline_width(source, offset, config.line_endings).is_some() {
            push_newline(
                source,
                &mut tokens,
                &mut offset,
                &mut line,
                &mut column,
                config.line_endings,
            );
            continue;
        }

        let Some(character) = char_at(source, offset) else {
            break;
        };

        if character.is_whitespace() {
            advance_character(&mut offset, &mut column, character);
            continue;
        }

        if let Some(comment) = config
            .line_comments
            .iter()
            .filter(|comment| !comment.is_empty() && source[offset..].starts_with(**comment))
            .max_by_key(|comment| comment.len())
        {
            advance_text(&mut offset, &mut column, comment);
            while offset < source.len()
                && !line_comment_ends(
                    source,
                    offset,
                    config.line_comment_terminator,
                    config.line_endings,
                )
            {
                if config.line_comment_terminator == LineCommentTerminator::LfOrCrlf
                    && source[offset..].starts_with('\r')
                {
                    offset += 1;
                    line += 1;
                    column = 1;
                    continue;
                }
                let Some(current) = char_at(source, offset) else {
                    break;
                };
                advance_character(&mut offset, &mut column, current);
            }
            continue;
        }

        if let Some((opener, closer)) = config
            .block_comments
            .iter()
            .filter(|(opener, _)| !opener.is_empty() && source[offset..].starts_with(*opener))
            .max_by_key(|(opener, _)| opener.len())
        {
            advance_text(&mut offset, &mut column, opener);
            while offset < source.len() {
                if source[offset..].starts_with(*closer) {
                    advance_text(&mut offset, &mut column, closer);
                    break;
                }
                if newline_width(source, offset, config.line_endings).is_some() {
                    push_newline(
                        source,
                        &mut tokens,
                        &mut offset,
                        &mut line,
                        &mut column,
                        config.line_endings,
                    );
                    continue;
                }
                let Some(current) = char_at(source, offset) else {
                    break;
                };
                advance_character(&mut offset, &mut column, current);
            }
            continue;
        }

        if let Some(rule) = config
            .quote_rules
            .iter()
            .find(|rule| rule.delimiter == character)
        {
            let start = offset;
            let start_line = line;
            let start_column = column;
            advance_character(&mut offset, &mut column, character);

            while offset < source.len() {
                if let Some(width) = newline_width(source, offset, config.line_endings) {
                    if !rule.allows_newlines {
                        break;
                    }
                    offset += width;
                    line += 1;
                    column = 1;
                    continue;
                }
                let Some(current) = char_at(source, offset) else {
                    break;
                };
                if !rule.allows_newlines && current == '\r' {
                    break;
                }
                if rule.escape == Some(current) {
                    advance_character(&mut offset, &mut column, current);
                    if newline_width(source, offset, config.line_endings).is_some()
                        || (!rule.allows_newlines && char_at(source, offset) == Some('\r'))
                    {
                        break;
                    }
                    if let Some(escaped) = char_at(source, offset) {
                        advance_character(&mut offset, &mut column, escaped);
                    }
                    continue;
                }
                if current == rule.delimiter {
                    let after_delimiter = offset + current.len_utf8();
                    if rule.doubled_delimiter
                        && source[after_delimiter..].starts_with(rule.delimiter)
                    {
                        advance_character(&mut offset, &mut column, current);
                        advance_character(&mut offset, &mut column, rule.delimiter);
                        continue;
                    }
                    advance_character(&mut offset, &mut column, current);
                    break;
                }
                advance_character(&mut offset, &mut column, current);
            }

            tokens.push(SourceToken {
                kind: SourceTokenKind::Literal,
                byte_range: start..offset,
                line: start_line,
                column: start_column,
            });
            continue;
        }

        if is_word_start(character) {
            let start = offset;
            let start_line = line;
            let start_column = column;

            if let Some(keyword) = config.hyphenated_words.iter().find(|keyword| {
                source[offset..].starts_with(**keyword)
                    && char_at(source, offset + keyword.len())
                        .is_none_or(|next| !is_word_continue(next))
            }) {
                advance_text(&mut offset, &mut column, keyword);
                tokens.push(SourceToken {
                    kind: SourceTokenKind::Word,
                    byte_range: start..offset,
                    line: start_line,
                    column: start_column,
                });
                continue;
            }

            advance_character(&mut offset, &mut column, character);
            while let Some(current) = char_at(source, offset) {
                if is_word_continue(current) {
                    advance_character(&mut offset, &mut column, current);
                    continue;
                }
                break;
            }
            tokens.push(SourceToken {
                kind: SourceTokenKind::Word,
                byte_range: start..offset,
                line: start_line,
                column: start_column,
            });
            continue;
        }

        let start = offset;
        let start_line = line;
        let start_column = column;
        advance_character(&mut offset, &mut column, character);
        tokens.push(SourceToken {
            kind: SourceTokenKind::Symbol(character),
            byte_range: start..offset,
            line: start_line,
            column: start_column,
        });
    }

    tokens
}

fn line_comment_ends(
    source: &str,
    offset: usize,
    terminator: LineCommentTerminator,
    line_endings: LineEndingPolicy,
) -> bool {
    match terminator {
        LineCommentTerminator::AnyNewline => newline_width(source, offset, line_endings).is_some(),
        LineCommentTerminator::LfOrCrlf => {
            source[offset..].starts_with('\n') || source[offset..].starts_with("\r\n")
        }
        LineCommentTerminator::LfOnly => source[offset..].starts_with('\n'),
    }
}

fn char_at(source: &str, offset: usize) -> Option<char> {
    source.get(offset..)?.chars().next()
}

fn newline_width(source: &str, offset: usize, line_endings: LineEndingPolicy) -> Option<usize> {
    let remaining = source.get(offset..)?;
    match line_endings {
        LineEndingPolicy::Universal if remaining.starts_with("\r\n") => Some(2),
        LineEndingPolicy::Universal
            if remaining.starts_with('\n') || remaining.starts_with('\r') =>
        {
            Some(1)
        }
        LineEndingPolicy::LfOnly if remaining.starts_with('\n') => Some(1),
        _ => None,
    }
}

fn advance_character(offset: &mut usize, column: &mut usize, character: char) {
    *offset += character.len_utf8();
    *column += 1;
}

fn advance_text(offset: &mut usize, column: &mut usize, text: &str) {
    *offset += text.len();
    *column += text.chars().count();
}

fn push_newline(
    source: &str,
    tokens: &mut Vec<SourceToken>,
    offset: &mut usize,
    line: &mut usize,
    column: &mut usize,
    line_endings: LineEndingPolicy,
) {
    let start = *offset;
    let width = newline_width(source, start, line_endings).expect("caller checked for a newline");
    tokens.push(SourceToken {
        kind: SourceTokenKind::Newline,
        byte_range: start..start + width,
        line: *line,
        column: *column,
    });
    *offset += width;
    *line += 1;
    *column = 1;
}

fn is_word_start(character: char) -> bool {
    character.is_alphabetic() || character == '_'
}

fn is_word_continue(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

pub(crate) fn statement_range(
    source: &str,
    tokens: &[SourceToken],
    line: usize,
    column: usize,
    rules: &StatementRules<'_>,
) -> Option<Range<usize>> {
    let index = SourceIndex::new(source);
    statement_range_with_index(source, tokens, line, column, rules, &index)
}

pub(crate) fn statement_range_with_index(
    source: &str,
    tokens: &[SourceToken],
    line: usize,
    column: usize,
    rules: &StatementRules<'_>,
    index: &SourceIndex<'_>,
) -> Option<Range<usize>> {
    let target = index.line_column_to_byte(line, column)?;
    let mut ranges = Vec::new();
    let mut statement_start = None;
    let mut statement_end = 0;
    let (mut parentheses, mut brackets, mut braces) = (0usize, 0usize, 0usize);
    let mut previous: Option<&SourceToken> = None;

    for token in tokens {
        if token.kind == SourceTokenKind::Newline {
            let continued = parentheses > 0
                || brackets > 0
                || (!rules.brace_delimited_blocks && braces > 0)
                || previous.is_some_and(|previous| {
                    token_text(source, previous).is_some_and(|text| {
                        rules
                            .multiline_continuations
                            .iter()
                            .any(|continuation| keyword_matches(text, continuation, rules))
                    })
                });
            if !continued && let Some(start) = statement_start.take() {
                ranges.push(start..statement_end);
            }
            previous = None;
            continue;
        }

        statement_start.get_or_insert(token.byte_range.start);
        match token.kind {
            SourceTokenKind::Symbol('(') => parentheses += 1,
            SourceTokenKind::Symbol(')') => parentheses = parentheses.saturating_sub(1),
            SourceTokenKind::Symbol('[') => brackets += 1,
            SourceTokenKind::Symbol(']') => brackets = brackets.saturating_sub(1),
            SourceTokenKind::Symbol('{') if !rules.brace_delimited_blocks => braces += 1,
            SourceTokenKind::Symbol('}') if !rules.brace_delimited_blocks => {
                braces = braces.saturating_sub(1);
            }
            _ => {}
        }
        statement_end = token.byte_range.end;

        if rules.semicolon_separator
            && token.kind == SourceTokenKind::Symbol(';')
            && parentheses == 0
            && brackets == 0
            && (rules.brace_delimited_blocks || braces == 0)
        {
            if let Some(start) = statement_start.take() {
                ranges.push(start..statement_end);
            }
            previous = None;
        } else {
            previous = Some(token);
        }
    }

    if parentheses > 0 || brackets > 0 || (!rules.brace_delimited_blocks && braces > 0) {
        return None;
    }
    if let Some(start) = statement_start {
        ranges.push(start..statement_end);
    }

    ranges
        .iter()
        .find(|range| range.start <= target && target < range.end)
        .cloned()
        .or_else(|| {
            ranges
                .iter()
                .find(|range| {
                    range.end == target
                        && index
                            .byte_to_line_column(range.end)
                            .is_some_and(|(end_line, _)| end_line == line)
                })
                .cloned()
        })
        .or_else(|| {
            ranges
                .iter()
                .find(|range| {
                    index
                        .byte_to_line_column(range.start)
                        .is_some_and(|(start_line, _)| start_line == line && target < range.start)
                })
                .cloned()
        })
        .or_else(|| {
            ranges
                .iter()
                .rev()
                .find(|range| {
                    index
                        .byte_to_line_column(range.end)
                        .is_some_and(|(end_line, _)| end_line == line && target > range.end)
                })
                .cloned()
        })
}

fn keyword_matches(left: &str, right: &str, rules: &StatementRules<'_>) -> bool {
    if rules.case_sensitive_keywords {
        left == right
    } else {
        left.to_lowercase() == right.to_lowercase()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MaskRules {
    position_policy: MaskPositionPolicy,
    two_byte_whitespace: Option<char>,
    three_byte_whitespace: Option<char>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MaskPositionPolicy {
    SourceColumns,
    ByteOffsets,
}

impl MaskRules {
    /// Use only when the parser reports Unicode-scalar columns.
    pub(crate) const SOURCE_COLUMNS: Self = Self {
        position_policy: MaskPositionPolicy::SourceColumns,
        two_byte_whitespace: None,
        three_byte_whitespace: None,
    };

    /// Replacement characters must be accepted by the selected parser's lexer.
    pub(crate) const fn byte_offsets(
        two_byte_whitespace: Option<char>,
        three_byte_whitespace: Option<char>,
    ) -> Self {
        Self {
            position_policy: MaskPositionPolicy::ByteOffsets,
            two_byte_whitespace,
            three_byte_whitespace,
        }
    }
}

/// Replaces a range with ASCII whitespace while preserving source line and scalar columns.
/// Byte offsets can change; use the configured variant when byte spans must stay stable.
pub(crate) fn mask_range_preserving_positions(source: &str, range: Range<usize>) -> Option<String> {
    mask_range_preserving_positions_with_rules(source, range, MaskRules::SOURCE_COLUMNS)
}

/// Applies configured replacements without assuming that Unicode whitespace is universal.
pub(crate) fn mask_range_preserving_positions_with_rules(
    source: &str,
    range: Range<usize>,
    rules: MaskRules,
) -> Option<String> {
    if range.start > range.end
        || range.end > source.len()
        || !source.is_char_boundary(range.start)
        || !source.is_char_boundary(range.end)
    {
        return None;
    }

    let mut masked = String::with_capacity(source.len());
    for (offset, character) in source.char_indices() {
        if range.contains(&offset) {
            if matches!(character, '\n' | '\r' | '\t') {
                masked.push(character);
            } else if rules.position_policy == MaskPositionPolicy::SourceColumns {
                masked.push(' ');
            } else {
                let replacement = match character.len_utf8() {
                    1 => Some(' '),
                    2 => rules.two_byte_whitespace,
                    3 => rules.three_byte_whitespace,
                    _ => None,
                }?;
                if replacement.len_utf8() != character.len_utf8() || !replacement.is_whitespace() {
                    return None;
                }
                masked.push(replacement);
            }
        } else {
            masked.push(character);
        }
    }
    Some(masked)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpectedCloser<'a> {
    Words(&'a str),
    Brace,
}

#[derive(Clone, Copy, Debug)]
enum StructuralToken<'a> {
    Word(&'a str),
    BraceOpen,
    BraceClose,
}

pub(crate) fn close_open_blocks(
    source: &str,
    tokens: &[SourceToken],
    rules: &StatementRules<'_>,
) -> Option<String> {
    let structural = tokens
        .iter()
        .filter_map(|token| match token.kind {
            SourceTokenKind::Word => token_text(source, token).map(StructuralToken::Word),
            SourceTokenKind::Symbol('{') if rules.brace_delimited_blocks => {
                Some(StructuralToken::BraceOpen)
            }
            SourceTokenKind::Symbol('}') if rules.brace_delimited_blocks => {
                Some(StructuralToken::BraceClose)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut stack = Vec::<ExpectedCloser<'_>>::new();
    let pairs = rules
        .block_pairs
        .iter()
        .chain(rules.root_pairs.iter())
        .copied()
        .collect::<Vec<_>>();

    let mut cursor = 0;
    while cursor < structural.len() {
        match structural[cursor] {
            StructuralToken::BraceOpen => {
                stack.push(ExpectedCloser::Brace);
                cursor += 1;
            }
            StructuralToken::BraceClose => {
                if stack.pop() != Some(ExpectedCloser::Brace) {
                    return None;
                }
                cursor += 1;
            }
            StructuralToken::Word(_) => {
                if let Some((closer, consumed)) = pairs
                    .iter()
                    .filter_map(|closer| {
                        let parts = phrase_parts(closer.1);
                        phrase_matches(&structural, cursor, &parts, rules)
                            .then_some((closer.1, parts.len()))
                    })
                    .max_by_key(|(_, consumed)| *consumed)
                {
                    if stack.last().copied() != Some(ExpectedCloser::Words(closer)) {
                        return None;
                    }
                    stack.pop();
                    cursor += consumed;
                    continue;
                }

                if let Some((closer, opener_length)) = pairs
                    .iter()
                    .filter_map(|(opener, closer)| {
                        let parts = phrase_parts(opener);
                        phrase_matches(&structural, cursor, &parts, rules)
                            .then_some((*closer, parts.len()))
                    })
                    .max_by_key(|(_, opener_length)| *opener_length)
                {
                    stack.push(ExpectedCloser::Words(closer));
                    cursor += opener_length;
                    continue;
                }
                cursor += 1;
            }
        }
    }

    if stack.is_empty() {
        return Some(source.to_owned());
    }

    let mut completed = source.to_owned();
    for closer in stack.iter().rev() {
        if !completed.ends_with('\n') && !completed.ends_with('\r') {
            completed.push('\n');
        }
        match closer {
            ExpectedCloser::Words(words) => completed.push_str(words),
            ExpectedCloser::Brace => completed.push('}'),
        }
    }
    Some(completed)
}

fn phrase_parts(phrase: &str) -> Vec<String> {
    phrase.split_whitespace().map(str::to_owned).collect()
}

fn phrase_matches(
    tokens: &[StructuralToken<'_>],
    cursor: usize,
    phrase: &[String],
    rules: &StatementRules<'_>,
) -> bool {
    !phrase.is_empty()
        && tokens
            .get(cursor..cursor.saturating_add(phrase.len()))
            .is_some_and(|sequence| {
                sequence.iter().zip(phrase).all(|(token, expected)| {
                    matches!(token, StructuralToken::Word(actual) if keyword_matches(actual, expected, rules))
                })
            })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecoveryStatus {
    Complete,
    Stalled,
    LimitReached,
    Cancelled,
}

pub(crate) struct RecoveryOutcome<P, D> {
    pub(crate) program: Option<P>,
    pub(crate) diagnostics: Vec<D>,
    pub(crate) status: RecoveryStatus,
}

const MAX_RECOVERY_MASKS: usize = 64;

pub(crate) fn recover_parse<P, D>(
    source: &str,
    mut parse: impl FnMut(&str) -> Result<P, Vec<D>>,
    diagnostic_position: impl Fn(&D) -> (usize, usize),
    mut mask: impl FnMut(&str, usize, usize) -> Option<String>,
    is_cancelled: impl Fn() -> bool,
) -> RecoveryOutcome<P, D> {
    let mut candidate = source.to_owned();
    let mut diagnostics = Vec::new();
    let mut masks = 0;

    loop {
        if is_cancelled() {
            return RecoveryOutcome {
                program: None,
                diagnostics,
                status: RecoveryStatus::Cancelled,
            };
        }

        match parse(&candidate) {
            Ok(program) => {
                return RecoveryOutcome {
                    program: Some(program),
                    diagnostics,
                    status: RecoveryStatus::Complete,
                };
            }
            Err(parse_diagnostics) => {
                let Some(diagnostic) = parse_diagnostics.into_iter().next() else {
                    return RecoveryOutcome {
                        program: None,
                        diagnostics,
                        status: RecoveryStatus::Stalled,
                    };
                };
                let (line, column) = diagnostic_position(&diagnostic);
                diagnostics.push(diagnostic);

                if masks == MAX_RECOVERY_MASKS {
                    return RecoveryOutcome {
                        program: None,
                        diagnostics,
                        status: RecoveryStatus::LimitReached,
                    };
                }

                let Some(recovered) = mask(&candidate, line, column) else {
                    return RecoveryOutcome {
                        program: None,
                        diagnostics,
                        status: RecoveryStatus::Stalled,
                    };
                };
                if recovered == candidate {
                    return RecoveryOutcome {
                        program: None,
                        diagnostics,
                        status: RecoveryStatus::Stalled,
                    };
                }
                candidate = recovered;
                masks += 1;
            }
        }
    }
}

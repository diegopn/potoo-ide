use std::ops::Range;

use super::lexer;
use crate::languages::source::{
    MaskRules, SourceIndex, SourceTokenKind, StatementRules, close_open_blocks,
    mask_range_preserving_positions_with_rules,
    statement_range_with_index as shared_statement_range, token_text,
};

const STRUCTURE_RULES: StatementRules<'static> = StatementRules {
    semicolon_separator: true,
    block_pairs: &[
        ("se", "fim-se"),
        ("enquanto", "fim-enquanto"),
        ("para", "fim-para"),
        ("repita", "até"),
        ("início", "fim"),
        ("variáveis", "fim-variáveis"),
    ],
    root_pairs: &[],
    multiline_continuations: &[],
    brace_delimited_blocks: false,
    case_sensitive_keywords: true,
};

// These scopes have safe synthetic closers. Loop blocks are intentionally omitted: a prefix
// cut inside a loop must fail the worker's parse validation before any prefix is executed.
const DEBUGGER_CLOSE_RULES: StatementRules<'static> = StatementRules {
    semicolon_separator: true,
    block_pairs: &[
        ("se", "fim-se"),
        ("início", "fim"),
        ("variáveis", "fim-variáveis"),
    ],
    root_pairs: &[],
    multiline_continuations: &[],
    brace_delimited_blocks: false,
    case_sensitive_keywords: true,
};

const MASK_RULES: MaskRules = MaskRules::byte_offsets(Some('\u{00a0}'), None);

pub(super) fn statement_range(source: &str, line: usize, column: usize) -> Option<Range<usize>> {
    let tokens = lexer::tokens(source);
    let index = SourceIndex::new_ignoring_leading_bom(source);
    shared_statement_range(source, &tokens, line, column, &STRUCTURE_RULES, &index)
}

pub(super) fn mask_diagnostic_statement(
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    let range = statement_range(source, line, column)?;
    mask_range_preserving_positions_with_rules(source, range, MASK_RULES)
}

pub(super) fn debugger_prefix(source: &str, line: usize, column: usize) -> Option<String> {
    statement_range(source, line, column)?;
    let target = SourceIndex::new_ignoring_leading_bom(source).line_column_to_byte(line, column)?;
    let prefix = source.get(..target)?;
    let tokens = lexer::tokens(prefix);
    // A truncated routine can still be called from the complete main block; do not execute it.
    if tokens.iter().any(|token| {
        token.kind == SourceTokenKind::Word
            && token_text(prefix, token).is_some_and(|word| word.to_lowercase() == "função")
    }) {
        return None;
    }
    close_open_blocks(prefix, &tokens, &DEBUGGER_CLOSE_RULES)
}

#[cfg(test)]
mod tests {
    use super::{
        STRUCTURE_RULES, close_open_blocks, debugger_prefix, mask_diagnostic_statement,
        statement_range,
    };
    use crate::languages::portugol::g_portugol::syntax_analysis::{lexer, views};
    use crate::languages::source::{SourceIndex, token_text};
    use potoo_gportugol_interpreter::parse;

    #[test]
    fn statement_ranges_respect_hyphenated_blocks_comments_literals_and_semicolons() {
        let source = r#"algoritmo teste;
variáveis x: inteiro; fim-variáveis
início
  se verdadeiro então
    imprima("fim-se; ; // literal"); /* ; fim-para */ imprima(1); @; imprima(2);
  fim-se
fim"#;
        let diagnostic =
            parse(source).expect_err("the selected statement has an invalid token")[0].clone();
        let range = statement_range(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("a complete semicolon-delimited statement can be selected");

        assert_eq!(&source[range], "@;");
        let masked = mask_diagnostic_statement(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("the statement can be masked without shifting bytes");
        assert_eq!(masked.len(), source.len());
        assert!(masked.contains("imprima(\"fim-se; ; // literal\")"));
        assert!(parse(&masked).is_ok(), "{masked}");
    }

    #[test]
    fn byte_preserving_mask_uses_only_whitespace_accepted_by_gportugol() {
        let source = "algoritmo teste; início\n  imprima(\"ação\") @; imprima(1);\nfim";
        let diagnostic =
            parse(source).expect_err("the token after the accented literal is invalid")[0].clone();
        let masked = mask_diagnostic_statement(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("NBSP keeps two-byte accents byte-aligned");

        assert_eq!(masked.len(), source.len());
        assert!(masked.contains('\u{00a0}'));
        assert!(parse(&masked).is_ok(), "{masked:?}");
        assert_eq!(
            SourceIndex::new(&masked).byte_to_line_column(diagnostic.span.start.byte_offset),
            Some((diagnostic.span.start.line, diagnostic.span.start.column))
        );
    }

    #[test]
    fn debugger_prefix_closes_accented_main_and_nested_if_but_not_a_loop() {
        let source = "algoritmo teste;\ninício\n se verdadeiro então\n  imprima(\"antes\"); @\n  imprima(\"depois\");\n fim-se\nfim";
        let diagnostic =
            parse(source).expect_err("the token after the first output is invalid")[0].clone();
        let prefix = debugger_prefix(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("the main and if blocks have safe synthetic closers");
        assert!(prefix.ends_with("fim-se\nfim"), "{prefix:?}");
        assert!(parse(&prefix).is_ok(), "{prefix:?}");

        let source = "algoritmo teste;\nvariáveis i: inteiro; fim-variáveis\ninício\n enquanto verdadeiro faça\n  imprima(\"antes\"); @\n  imprima(\"depois\");\n fim-enquanto\nfim";
        let diagnostic =
            parse(source).expect_err("the loop body contains an invalid token")[0].clone();
        let prefix = debugger_prefix(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("the safe outer block closure can be proposed");
        assert!(parse(&prefix).is_err(), "an open loop must fail validation");

        for source in [
            "algoritmo teste;\nvariáveis i: inteiro; fim-variáveis\ninício\n para i de 1 até 2 faça\n  imprima(\"antes\"); @\n  imprima(\"depois\");\n fim-para\nfim",
            "algoritmo teste;\ninício\n repita\n  imprima(\"antes\"); @\n até verdadeiro;\nfim",
        ] {
            let diagnostic = parse(source)
                .expect_err("a loop body with an invalid token must fail parsing")
                .remove(0);
            let prefix = debugger_prefix(
                source,
                diagnostic.span.start.line,
                diagnostic.span.start.column,
            )
            .expect("the safe outer block closure can be proposed");
            assert!(parse(&prefix).is_err(), "an open loop must fail validation");
        }
    }

    #[test]
    fn repeat_structure_matches_until_with_a_condition_expression() {
        let source = "algoritmo teste;\ninício\nrepita\n imprima(1);\naté idade > 3;\nfim";
        let tokens = lexer::tokens(source);

        assert_eq!(
            close_open_blocks(source, &tokens, &STRUCTURE_RULES).as_deref(),
            Some(source)
        );
    }

    #[test]
    fn selected_line_views_keep_code_and_mask_comments_and_literals() {
        let source =
            "início // comentário\n imprima(\"fim-se\"); /* bloco\n   conteúdo */ imprima(1);\nfim";
        let code = lexer::tokens(source);
        let words = code
            .iter()
            .filter(|token| token.kind == crate::languages::source::SourceTokenKind::Word)
            .filter_map(|token| token_text(source, token))
            .collect::<Vec<_>>();
        let literals = code
            .iter()
            .filter(|token| token.kind == crate::languages::source::SourceTokenKind::Literal)
            .filter_map(|token| token_text(source, token))
            .collect::<Vec<_>>();
        assert!(words.contains(&"imprima"));
        assert!(!words.contains(&"fim-se"));
        assert!(!words.contains(&"fim-enquanto"));
        assert_eq!(literals, ["\"fim-se\""]);
        let selected = views::source_line_code(source, 2);
        assert!(selected.contains("\"fim-se\""));
        assert!(!selected.contains("bloco"));
        let after_block_comment = views::source_line_code(source, 3);
        assert!(after_block_comment.contains("imprima(1)"));
        assert!(!after_block_comment.contains("conteúdo"));
    }
}

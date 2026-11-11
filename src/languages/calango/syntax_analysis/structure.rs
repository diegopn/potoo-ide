use std::ops::Range;

use super::lexer;
use crate::languages::source::{
    SourceIndex, StatementRules, close_open_blocks, mask_range_preserving_positions,
    statement_range_with_index,
};

const STATEMENT_RULES: StatementRules<'static> = StatementRules {
    semicolon_separator: true,
    block_pairs: &[],
    root_pairs: &[],
    multiline_continuations: &[],
    brace_delimited_blocks: false,
    case_sensitive_keywords: true,
};

const DEBUGGER_CLOSE_RULES: StatementRules<'static> = StatementRules {
    semicolon_separator: true,
    block_pairs: &[("se", "fimSe")],
    root_pairs: &[("principal", "fimPrincipal")],
    multiline_continuations: &[],
    brace_delimited_blocks: false,
    case_sensitive_keywords: true,
};

pub(crate) fn statement_range(source: &str, line: usize, column: usize) -> Option<Range<usize>> {
    let tokens = lexer::tokens(source);
    let index = SourceIndex::new_lf_only(source);
    statement_range_with_index(source, &tokens, line, column, &STATEMENT_RULES, &index)
}

pub(super) fn mask_diagnostic_statement(
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    let range = statement_range(source, line, column)?;
    mask_range_preserving_positions(source, range)
}

pub(super) fn debugger_prefix(source: &str, line: usize, column: usize) -> Option<String> {
    statement_range(source, line, column)?;
    let target = SourceIndex::new_lf_only(source).line_column_to_byte(line, column)?;
    let prefix = source.get(..target)?;
    close_open_blocks(prefix, &lexer::tokens(prefix), &DEBUGGER_CLOSE_RULES)
}

#[cfg(test)]
mod tests {
    use super::{debugger_prefix, mask_diagnostic_statement, statement_range};
    use crate::languages::source::SourceIndex;
    use potoo_calango_interpreter::parse;

    #[test]
    fn statement_range_masks_one_semicolon_statement_and_preserves_later_code() {
        let source = "algoritmo teste;\nprincipal\nse verdadeiro entao\nescreval(\"fimSe; ; // literal\"); /* ; fimPara */ @; escreva(2);\nfimSe\nfimPrincipal";
        let diagnostic = parse(source)
            .expect_err("the selected statement has an invalid token")
            .remove(0);
        let range = statement_range(source, diagnostic.line, diagnostic.column)
            .expect("the semicolon-delimited statement has an unambiguous range");

        assert_eq!(&source[range], "@;");
        let masked = mask_diagnostic_statement(source, diagnostic.line, diagnostic.column)
            .expect("the statement can be masked with parser coordinates intact");
        assert_eq!(masked.len(), source.len());
        assert!(masked.contains("fimSe; ; // literal"));
        assert!(masked.contains("escreva(2);"));
        assert!(parse(&masked).is_ok(), "{masked}");
    }

    #[test]
    fn debugger_prefix_closes_only_if_and_main_not_loops_or_routines() {
        let source = "algoritmo teste;\nprincipal\nse verdadeiro entao\nescreval(\"antes\"); @;\nescreval(\"depois\");\nfimSe\nfimPrincipal";
        let diagnostic = parse(source)
            .expect_err("the invalid token follows a complete output statement")
            .remove(0);
        let prefix = debugger_prefix(source, diagnostic.line, diagnostic.column)
            .expect("the if and principal wrapper are safe to close");
        assert!(prefix.ends_with("fimSe\nfimPrincipal"), "{prefix:?}");
        assert!(parse(&prefix).is_ok(), "{prefix:?}");

        let source = "algoritmo teste;\nprincipal\ninteiro i;\npara (i de 1 ate 2 passo 1) faca\nescreval(\"antes\"); @;\nfimPara\nfimPrincipal";
        let diagnostic = parse(source)
            .expect_err("the loop body contains an invalid token")
            .remove(0);
        let prefix = debugger_prefix(source, diagnostic.line, diagnostic.column)
            .expect("the candidate boundary is known");
        assert!(parse(&prefix).is_err(), "an open loop must fail validation");
        assert!(!prefix.contains("fimPara"), "{prefix:?}");
    }

    #[test]
    fn debugger_closers_do_not_confuse_uppercase_identifiers_with_keywords() {
        let source = "algoritmo teste;\nprincipal\ninteiro Se;\nSe = 1; @;\nfimPrincipal";
        let diagnostic = parse(source)
            .expect_err("the invalid token follows a valid assignment")
            .remove(0);
        let prefix = debugger_prefix(source, diagnostic.line, diagnostic.column)
            .expect("the main wrapper can be closed");
        assert!(parse(&prefix).is_ok(), "{prefix:?}");
        assert!(!prefix.contains("fimSe"), "{prefix:?}");
        assert_eq!(SourceIndex::new_lf_only(&prefix).line_count(), 5);
    }
}

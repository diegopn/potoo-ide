use std::ops::Range;

use super::lexer;
use crate::languages::source::{
    SourceIndex, StatementRules, close_open_blocks, mask_range_preserving_positions,
    statement_range as shared_statement_range,
};

const STATEMENT_RULES: StatementRules<'static> = StatementRules {
    // VisuAlg uses line endings to separate statements. A semicolon is optional only for
    // declarations and never separates two statements on one line.
    semicolon_separator: false,
    block_pairs: &[],
    root_pairs: &[],
    multiline_continuations: &[],
    brace_delimited_blocks: false,
    case_sensitive_keywords: false,
};

const DEBUGGER_CLOSE_RULES: StatementRules<'static> = StatementRules {
    semicolon_separator: false,
    // An unfinished branch may safely stop after the statements before the diagnostic.
    // Loops, selections, and routines are intentionally left open and fail parser validation.
    block_pairs: &[("se", "fimse")],
    root_pairs: &[("algoritmo", "fimalgoritmo")],
    multiline_continuations: &[],
    brace_delimited_blocks: false,
    case_sensitive_keywords: false,
};

pub(crate) fn statement_range(source: &str, line: usize, column: usize) -> Option<Range<usize>> {
    shared_statement_range(
        source,
        &lexer::tokens(source),
        line,
        column,
        &STATEMENT_RULES,
    )
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
    // Require an unambiguous statement boundary, then cut exactly at the parser's original
    // scalar-column coordinate. The worker validates this candidate before running it.
    statement_range(source, line, column)?;
    let target = SourceIndex::new(source).line_column_to_byte(line, column)?;
    let prefix = source.get(..target)?;
    close_open_blocks(prefix, &lexer::tokens(prefix), &DEBUGGER_CLOSE_RULES)
}

#[cfg(test)]
mod tests {
    use super::{debugger_prefix, statement_range};
    use potoo_visualg_interpreter::parse;

    #[test]
    fn visualg_statement_range_keeps_a_multiline_expression_together() {
        let source = "algoritmo \"linhas\"\ninicio\nescreval(\n  1 +\n  @\n  2\n)\nfimalgoritmo";
        let diagnostic = parse(source)
            .expect_err("the parser reports the newline that interrupts this expression")
            .remove(0);
        let range = statement_range(source, diagnostic.line, diagnostic.column)
            .expect("balanced parentheses define one source statement range");

        assert_eq!(&source[range], "escreval(\n  1 +\n  @\n  2\n)");
    }

    #[test]
    fn debugger_prefix_closes_if_and_algorithm_but_leaves_loops_open() {
        let source = "algoritmo \"if\"\ninicio\nse verdadeiro entao\nescreval(\"ANTES\")\n@\nescreval(\"DEPOIS\")\nfimse\nfimalgoritmo";
        let diagnostic = parse(source)
            .expect_err("the conditional body contains an invalid character")
            .remove(0);
        let prefix = debugger_prefix(source, diagnostic.line, diagnostic.column)
            .expect("the if and algorithm can be closed safely");

        assert!(parse(&prefix).is_ok(), "{prefix:?}");
        assert!(prefix.ends_with("fimse\nfimalgoritmo"), "{prefix:?}");

        let source = "algoritmo \"loop\"\nvar\ni: inteiro\ninicio\ni <- 0\nenquanto i < 1 faca\ni <- i + 1\nescreval(\"DENTRO\")\n@\nescreval(\"DEPOIS\")\nfimenquanto\nfimalgoritmo";
        let diagnostic = parse(source)
            .expect_err("the loop body contains an invalid character")
            .remove(0);
        let prefix = debugger_prefix(source, diagnostic.line, diagnostic.column)
            .expect("the candidate boundary is known even though the loop is incomplete");

        assert!(
            parse(&prefix).is_err(),
            "an unfinished loop must fail validation"
        );
        assert!(!prefix.contains("fimenquanto"), "{prefix:?}");
    }
}

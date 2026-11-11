use std::ops::Range;

use crate::languages::source::{
    SourceIndex, StatementRules, close_open_blocks, mask_range_preserving_positions,
    statement_range_with_index,
};

use super::lexer;

const STRUCTURE_RULES: StatementRules<'static> = StatementRules {
    semicolon_separator: true,
    block_pairs: &[
        ("se", "fim se"),
        ("enquanto", "fim enquanto"),
        ("para", "fim para"),
        ("repita", "ate"),
    ],
    root_pairs: &[("inicio", "fim")],
    multiline_continuations: &[],
    brace_delimited_blocks: false,
    case_sensitive_keywords: true,
};

// Track loops so a completed loop before the error remains valid, then refuse to synthesize
// their closers. Closing an open loop could execute side effects from a body that contains it.
const DEBUGGER_CLOSE_RULES: StatementRules<'static> = StatementRules {
    semicolon_separator: true,
    block_pairs: &[
        ("se", "fim se"),
        ("enquanto", "fim enquanto"),
        ("para", "fim para"),
    ],
    root_pairs: &[("inicio", "fim")],
    multiline_continuations: &[],
    brace_delimited_blocks: false,
    case_sensitive_keywords: true,
};

pub(crate) fn statement_range(source: &str, line: usize, column: usize) -> Option<Range<usize>> {
    let index = SourceIndex::new_lf_only(source);
    statement_range_with_index(
        source,
        &lexer::tokens(source),
        line,
        column,
        &STRUCTURE_RULES,
        &index,
    )
}

pub(crate) fn mask_diagnostic_statement(
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    mask_range_preserving_positions(source, statement_range(source, line, column)?)
}

pub(crate) fn debugger_prefix(source: &str, line: usize, column: usize) -> Option<String> {
    let index = SourceIndex::new_lf_only(source);
    let target = index.line_column_to_byte(line, column)?;
    let prefix = source.get(..target)?;
    let completed = close_open_blocks(prefix, &lexer::tokens(prefix), &DEBUGGER_CLOSE_RULES)?;
    let synthetic_closers = completed.strip_prefix(prefix)?;
    if synthetic_closers
        .lines()
        .any(|line| matches!(line.trim(), "fim enquanto" | "fim para"))
    {
        return None;
    }
    let synthetic_closers = synthetic_closers
        .lines()
        .map(|line| {
            if line.trim() == "fim se" {
                "fim se;"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    Some(format!("{prefix}{synthetic_closers}"))
}

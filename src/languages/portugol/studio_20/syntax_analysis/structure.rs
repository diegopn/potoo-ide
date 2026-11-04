use std::ops::Range;

use super::lexer::{self, TokenKind};

pub(super) fn statement_range(source: &str, line: usize, column: usize) -> Range<usize> {
    let line_start = source
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    let selected_line = source
        .split_inclusive('\n')
        .nth(line.saturating_sub(1))
        .unwrap_or("");
    let column_offset = selected_line
        .char_indices()
        .nth(column.saturating_sub(1))
        .map_or(
            selected_line.trim_end_matches(['\r', '\n']).len(),
            |(offset, _)| offset,
        );
    let target = line_start + column_offset;
    let line_end = line_start + selected_line.trim_end_matches(['\r', '\n']).len();
    let tokens = lexer::tokens(source);

    let mut start = line_start;
    let mut paren_depth = 0usize;
    let mut bracket_depth = 0usize;
    for token in tokens
        .iter()
        .filter(|token| token.start >= line_start && token.start < target)
    {
        match token.kind {
            TokenKind::Symbol('(') => paren_depth += 1,
            TokenKind::Symbol(')') => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::Symbol('[') => bracket_depth += 1,
            TokenKind::Symbol(']') => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Symbol(';') if paren_depth == 0 && bracket_depth == 0 => {
                start = token.end;
            }
            _ => {}
        }
    }

    let mut end = line_end;
    paren_depth = 0;
    bracket_depth = 0;
    for token in tokens
        .iter()
        .filter(|token| token.start >= target && token.start <= line_end)
    {
        match token.kind {
            TokenKind::Symbol('(') => paren_depth += 1,
            TokenKind::Symbol(')') => paren_depth = paren_depth.saturating_sub(1),
            TokenKind::Symbol('[') => bracket_depth += 1,
            TokenKind::Symbol(']') => bracket_depth = bracket_depth.saturating_sub(1),
            TokenKind::Symbol(';') if paren_depth == 0 && bracket_depth == 0 => {
                end = token.end;
                break;
            }
            _ => {}
        }
    }

    start.min(end)..end
}

pub(super) fn mask_diagnostic_statement(source: &str, line: usize, column: usize) -> String {
    lexer::mask_range(source, statement_range(source, line, column))
}

pub(super) fn debugger_prefix(source: &str, line: usize, column: usize) -> String {
    let range = statement_range(source, line, column);
    source[..range.start.min(source.len())].to_owned()
}

pub(super) fn unmatched_braces(source: &str) -> usize {
    let mut balance = 0usize;
    for token in lexer::tokens(source) {
        match token.kind {
            TokenKind::Symbol('{') => balance += 1,
            TokenKind::Symbol('}') => balance = balance.saturating_sub(1),
            _ => {}
        }
    }
    balance
}

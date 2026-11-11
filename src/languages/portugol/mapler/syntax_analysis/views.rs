use super::structure;

pub(crate) fn mask_diagnostic_statement(
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    structure::mask_diagnostic_statement(source, line, column)
}

pub(crate) fn source_line_code(source: &str, line: usize) -> String {
    super::lexer::source_line_code(source, line)
}

pub(crate) fn without_literals_and_comments(source: &str) -> String {
    super::lexer::without_literals_and_comments(source)
}

pub(crate) fn is_blank_or_line_comment(source: &str, line_number: usize) -> bool {
    source_line_code(source, line_number).trim().is_empty()
}

pub(crate) fn debugger_prefix(source: &str, line: usize, column: usize) -> Option<String> {
    structure::debugger_prefix(source, line, column)
}

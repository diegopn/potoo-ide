use super::structure;

pub(crate) fn mask_diagnostic_statement(source: &str, line: usize, column: usize) -> String {
    structure::mask_diagnostic_statement(source, line, column)
}

pub(crate) fn source_line_code(source: &str, line: usize) -> String {
    super::lexer::line_without_comments(source, line)
}

pub(crate) fn debugger_prefix(source: &str, line: usize, column: usize) -> String {
    let mut prefix = structure::debugger_prefix(source, line, column);
    for _ in 0..structure::unmatched_braces(&prefix) {
        if !prefix.ends_with('\n') {
            prefix.push('\n');
        }
        prefix.push('}');
    }
    prefix
}

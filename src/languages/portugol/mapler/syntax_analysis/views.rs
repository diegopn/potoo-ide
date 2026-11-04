use super::structure;

pub(crate) fn mask_diagnostic_statement(source: &str, line: usize, column: usize) -> String {
    structure::mask_diagnostic_statement(source, line, column)
}

pub(crate) fn source_line_code(source: &str, line: usize) -> String {
    super::lexer::line_without_comments(source, line)
}

pub(crate) fn debugger_prefix(source: &str, line: usize, column: usize) -> String {
    structure::close_blocks(
        &structure::debugger_prefix(source, line, column),
        &[
            ("se", "fim se;"),
            ("enquanto", "fim enquanto;"),
            ("para", "fim para;"),
            ("repita", "ate verdadeiro;"),
        ],
        Some(("inicio", "fim.")),
    )
}

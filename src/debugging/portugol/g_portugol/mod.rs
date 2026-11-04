use crate::debugging::{before_line, close_blocks};

pub(crate) fn prefix(source: &str, line: usize) -> String {
    close_blocks(
        &before_line(source, line),
        &[
            ("se", "fim-se"),
            ("enquanto", "fim-enquanto"),
            ("para", "fim-para"),
            ("repita", "ate verdadeiro;"),
            ("inicio", "fim"),
        ],
        None,
    )
}

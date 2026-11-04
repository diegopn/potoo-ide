use crate::debugging::{before_line, close_blocks};

pub(super) fn prefix(source: &str, line: usize) -> String {
    close_blocks(
        &before_line(source, line),
        &[
            ("def", "fim"),
            ("se", "fim"),
            ("enquanto", "fim"),
            ("para", "fim"),
            ("faca", "fim"),
        ],
        None,
    )
}

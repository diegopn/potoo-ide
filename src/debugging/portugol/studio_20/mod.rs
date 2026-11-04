use crate::debugging::{before_line, unmatched_braces};

pub(crate) fn prefix(source: &str, line: usize) -> String {
    let mut prefix = before_line(source, line);
    for _ in 0..unmatched_braces(&prefix) {
        prefix.push_str("\n}");
    }
    prefix
}

use super::{lexer, structure};
use crate::languages::source::SourceIndex;

pub(crate) fn without_literals_and_comments(source: &str) -> String {
    lexer::without_literals_and_comments(source)
}

pub(crate) fn is_blank_or_line_comment(source: &str, line_number: usize) -> bool {
    if hash_comment_has_invalid_terminator(source, line_number) {
        return false;
    }
    let line = lexer::source_line_code(source, line_number);
    line.trim().is_empty()
}

fn hash_comment_has_invalid_terminator(source: &str, line_number: usize) -> bool {
    let index = SourceIndex::new(source);
    let Some(start) = index.line_column_to_byte(line_number, 1) else {
        return false;
    };
    if !index
        .line_text(line_number)
        .is_some_and(|line| line.trim_start().starts_with('#'))
    {
        return false;
    }

    let Some(remaining) = source.get(start..) else {
        return true;
    };
    let Some((offset, terminator)) = remaining
        .char_indices()
        .find(|(_, character)| matches!(character, '\r' | '\n'))
    else {
        return true;
    };
    terminator == '\r' && !remaining[offset..].starts_with("\r\n")
}

pub(crate) fn mask_diagnostic_statement(
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    structure::mask_diagnostic_statement(source, line, column)
}

pub(crate) fn source_line_code(source: &str, line: usize) -> String {
    lexer::source_line_code(source, line)
}

pub(crate) fn debugger_prefix(source: &str, line: usize, column: usize) -> Option<String> {
    structure::debugger_prefix(source, line, column)
}

#[cfg(test)]
mod tests {
    use super::{is_blank_or_line_comment, source_line_code, without_literals_and_comments};

    #[test]
    fn hash_comments_are_hidden_but_block_comment_markers_remain_code() {
        let source = "# comentário\nescreva \"# literal\"\n/* não é comentário */";
        assert!(is_blank_or_line_comment(source, 1));
        assert!(source_line_code(source, 2).contains("# literal"));
        assert_eq!(source_line_code(source, 3).trim(), "/* não é comentário */");
        assert!(without_literals_and_comments(source).contains("/* não é comentário */"));
    }

    #[test]
    fn unterminated_hash_comment_remains_selectable_as_an_error() {
        assert!(!is_blank_or_line_comment("# comentário", 1));
        assert!(is_blank_or_line_comment("# comentário\r\n", 1));
        assert!(!is_blank_or_line_comment("# comentário\r", 1));
        assert!(!is_blank_or_line_comment("# comentário\r@\n", 1));
    }
}

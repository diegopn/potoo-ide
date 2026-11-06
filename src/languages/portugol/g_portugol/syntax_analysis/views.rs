use super::{lexer, structure};
use crate::languages::source::SourceIndex;

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

pub(crate) fn without_literals_and_comments(source: &str) -> String {
    lexer::without_literals_and_comments(source)
}

pub(crate) fn is_blank_or_line_comment(source: &str, line_number: usize) -> bool {
    if unclosed_block_comment(source).is_some_and(|(_, open_line, _)| open_line == line_number) {
        return false;
    }
    source_line_code(source, line_number).trim().is_empty()
}

pub(crate) fn unclosed_block_comment(source: &str) -> Option<(usize, usize, usize)> {
    let characters = source.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < characters.len() {
        if matches!(characters[index], '"' | '\'') {
            let quote = characters[index];
            index += 1;
            while index < characters.len() {
                if matches!(characters[index], '\r' | '\n') {
                    if quote == '"' {
                        break;
                    }
                    if characters[index] == '\r' && characters.get(index + 1) == Some(&'\n') {
                        index += 2;
                    } else {
                        index += 1;
                    }
                    continue;
                }
                if characters[index] == '\\' {
                    index += 1;
                    if index < characters.len() && !matches!(characters[index], '\r' | '\n') {
                        index += 1;
                    } else {
                        break;
                    }
                } else if characters[index] == quote {
                    index += 1;
                    break;
                } else {
                    index += 1;
                }
            }
        } else if characters[index] == '/' && characters.get(index + 1) == Some(&'/') {
            index += 2;
            while index < characters.len() && characters[index] != '\n' {
                index += 1;
            }
        } else if characters[index] == '/' && characters.get(index + 1) == Some(&'*') {
            let start = index;
            index += 2;
            while index + 1 < characters.len()
                && !(characters[index] == '*' && characters[index + 1] == '/')
            {
                index += 1;
            }
            if index + 1 >= characters.len() {
                let prefix = characters[..start].iter().collect::<String>();
                let (line, column) = SourceIndex::new_ignoring_leading_bom(&prefix)
                    .byte_to_line_column(prefix.len())?;
                return Some((start, line, column));
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    None
}

pub(crate) fn debugger_prefix(source: &str, line: usize, column: usize) -> Option<String> {
    structure::debugger_prefix(source, line, column)
}

#[cfg(test)]
mod tests {
    use super::{is_blank_or_line_comment, source_line_code};

    #[test]
    fn recognizes_comment_only_lines_without_hiding_an_unclosed_comment_warning() {
        let source = "// linha\n/* comentário fechado */\ninício\n/* comentário aberto";
        assert!(is_blank_or_line_comment(source, 1));
        assert!(is_blank_or_line_comment(source, 2));
        assert!(!is_blank_or_line_comment(source, 3));
        assert!(!is_blank_or_line_comment(source, 4));
        assert!(source_line_code(source, 2).trim().is_empty());
        assert!(source_line_code(source, 4).trim().is_empty());
    }
}

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

fn unclosed_block_comment(source: &str) -> Option<(usize, usize, usize)> {
    let characters = source.char_indices().collect::<Vec<_>>();
    let mut index = 0;

    while index < characters.len() {
        let (offset, character) = characters[index];
        if matches!(character, '"' | '\'') {
            let delimiter = character;
            index += 1;
            while index < characters.len() {
                let current = characters[index].1;
                if current == '\\' {
                    index = (index + 2).min(characters.len());
                } else if current == delimiter {
                    index += 1;
                    break;
                } else if current == '\n' {
                    break;
                } else {
                    index += 1;
                }
            }
        } else if character == '/' && characters.get(index + 1).is_some_and(|(_, ch)| *ch == '/') {
            index += 2;
            while index < characters.len() && characters[index].1 != '\n' {
                index += 1;
            }
        } else if character == '/' && characters.get(index + 1).is_some_and(|(_, ch)| *ch == '*') {
            let start = offset;
            index += 2;
            while index + 1 < characters.len()
                && !(characters[index].1 == '*' && characters[index + 1].1 == '/')
            {
                index += 1;
            }
            if index + 1 >= characters.len() {
                let (line, column) = SourceIndex::new_lf_only(source).byte_to_line_column(start)?;
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
    use super::{is_blank_or_line_comment, source_line_code, unclosed_block_comment};

    #[test]
    fn line_views_follow_lf_only_comments_without_truncating_escaped_literals() {
        let source = "principal\r  escreval(\"ação \\\"// literal\\\"\"); // comentário\r ainda comentário\n@";

        let first_line = source_line_code(source, 1);
        assert!(first_line.contains("ação"), "{first_line:?}");
        assert!(first_line.contains("// literal"), "{first_line:?}");
        assert!(!first_line.contains("comentário"), "{first_line:?}");
        assert_eq!(source_line_code(source, 2), "@");
        assert!(is_blank_or_line_comment(source, 1) == false);
        assert!(is_blank_or_line_comment(
            "// comentário\r ainda comentário\n",
            1
        ));
    }

    #[test]
    fn unclosed_block_comment_keeps_its_diagnostic_line_selectable() {
        let source = "algoritmo aula;\nprincipal\n/* comentário aberto\nfimPrincipal";

        assert_eq!(unclosed_block_comment(source), Some((26, 3, 1)));
        assert!(!is_blank_or_line_comment(source, 3));
        assert!(is_blank_or_line_comment(source, 4));
    }
}

use super::structure;

pub(crate) fn mask_diagnostic_statement(source: &str, line: usize, column: usize) -> String {
    structure::mask_diagnostic_statement(source, line, column)
}

pub(crate) fn source_line_code(source: &str, line: usize) -> String {
    super::lexer::line_without_comments(source, line)
}

pub(crate) fn without_literals_and_comments(source: &str) -> String {
    super::lexer::without_literals_and_comments(source)
}

pub(crate) fn is_blank_or_line_comment(source: &str, line_number: usize) -> bool {
    let line = source
        .lines()
        .nth(line_number.saturating_sub(1))
        .unwrap_or("")
        .trim_start();
    line.is_empty() || line.starts_with("//")
}

pub(crate) fn unclosed_block_comment(source: &str) -> Option<(usize, usize, usize)> {
    let characters = source.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < characters.len() {
        if matches!(characters[index], '"' | '\'') {
            let quote = characters[index];
            index += 1;
            while index < characters.len() {
                if characters[index] == '\\' {
                    index = (index + 2).min(characters.len());
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
                let line = prefix
                    .chars()
                    .filter(|character| *character == '\n')
                    .count()
                    + 1;
                let column = prefix
                    .rsplit('\n')
                    .next()
                    .map_or(1, |last_line| last_line.chars().count() + 1);
                return Some((start, line, column));
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    None
}

pub(crate) fn debugger_prefix(source: &str, line: usize, column: usize) -> String {
    structure::close_blocks(
        &structure::debugger_prefix(source, line, column),
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

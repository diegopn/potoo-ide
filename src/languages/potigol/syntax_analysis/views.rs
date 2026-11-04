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
            ("def", "fim"),
            ("se", "fim"),
            ("enquanto", "fim"),
            ("para", "fim"),
            ("faca", "fim"),
        ],
        None,
    )
}

use super::lexer::{self, TokenKind};

pub(crate) fn is_jerimum_import(source: &str) -> bool {
    let tokens = lexer::tokens(source);
    let mut block_depth = 0usize;
    let mut current_line = 0usize;
    let mut first_on_line = true;

    for (index, token) in tokens.iter().enumerate() {
        if token.line != current_line {
            current_line = token.line;
            first_on_line = true;
        }
        if token.kind == TokenKind::Newline {
            continue;
        }

        if first_on_line
            && block_depth == 0
            && token.kind == TokenKind::Word
            && token.text == "use"
            && token.column == 1
            && tokens[index + 1..]
                .iter()
                .take_while(|next| next.line == token.line)
                .find(|next| next.kind != TokenKind::Newline)
                .is_some_and(|next| {
                    next.kind == TokenKind::String && next.text.eq_ignore_ascii_case("jerimum")
                })
        {
            return true;
        }

        if token.kind == TokenKind::Word {
            match token.text.as_str() {
                "def" | "se" | "enquanto" | "para" | "faca" => block_depth += 1,
                "fim" => block_depth = block_depth.saturating_sub(1),
                _ => {}
            }
        }
        first_on_line = false;
    }
    false
}

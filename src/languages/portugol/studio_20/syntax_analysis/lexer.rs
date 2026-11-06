use crate::languages::source::{LexerConfig, QuoteRule, SourceToken, tokenize};

pub(super) const SHARED_CONFIG: LexerConfig<'static> = LexerConfig {
    line_comments: &["//"],
    block_comments: &[("/*", "*/")],
    quote_rules: &[
        QuoteRule {
            delimiter: '"',
            escape: Some('\\'),
            doubled_delimiter: false,
        },
        QuoteRule {
            delimiter: '\'',
            escape: Some('\\'),
            doubled_delimiter: false,
        },
    ],
    hyphenated_identifiers: false,
};

pub(super) fn shared_tokens(source: &str) -> Vec<SourceToken> {
    tokenize(source, &SHARED_CONFIG)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TokenKind {
    Word,
    String,
    Symbol(char),
    Newline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Token {
    pub(super) kind: TokenKind,
    pub(super) text: String,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) line: usize,
    pub(super) column: usize,
}

const ALLOW_HYPHENATED_WORDS: bool = false;
const HASH_STARTS_LINE_COMMENT: bool = false;

pub(super) fn tokens(source: &str) -> Vec<Token> {
    let characters = source.char_indices().collect::<Vec<_>>();
    let mut result = Vec::new();
    let (mut cursor, mut line, mut column) = (0, 1, 1);

    while cursor < characters.len() {
        let (start, character) = characters[cursor];

        if character == '\n' {
            result.push(Token {
                kind: TokenKind::Newline,
                text: "\n".to_owned(),
                start,
                end: start + character.len_utf8(),
                line,
                column,
            });
            cursor += 1;
            line += 1;
            column = 1;
            continue;
        }

        if character.is_whitespace() {
            cursor += 1;
            column += 1;
            continue;
        }

        if character == '/' && next_is(&characters, cursor, '/') {
            cursor += 2;
            column += 2;
            while cursor < characters.len() && characters[cursor].1 != '\n' {
                column += 1;
                cursor += 1;
            }
            continue;
        }

        if HASH_STARTS_LINE_COMMENT && character == '#' {
            while cursor < characters.len() && characters[cursor].1 != '\n' {
                column += 1;
                cursor += 1;
            }
            continue;
        }

        if character == '/' && next_is(&characters, cursor, '*') {
            cursor += 2;
            column += 2;
            while cursor < characters.len() {
                let (offset, current) = characters[cursor];
                if current == '*' && next_is(&characters, cursor, '/') {
                    cursor += 2;
                    column += 2;
                    break;
                }
                if current == '\n' {
                    result.push(Token {
                        kind: TokenKind::Newline,
                        text: "\n".to_owned(),
                        start: offset,
                        end: offset + current.len_utf8(),
                        line,
                        column,
                    });
                    cursor += 1;
                    line += 1;
                    column = 1;
                } else {
                    cursor += 1;
                    column += 1;
                }
            }
            continue;
        }

        if matches!(character, '"' | '\'') {
            let delimiter = character;
            let mut text = String::new();
            cursor += 1;
            column += 1;
            let mut escaped = false;
            while cursor < characters.len() {
                let current = characters[cursor].1;
                cursor += 1;
                if current == '\n' {
                    line += 1;
                    column = 1;
                } else {
                    column += 1;
                }
                if escaped {
                    text.push(current);
                    escaped = false;
                } else if current == '\\' {
                    escaped = true;
                } else if current == delimiter {
                    break;
                } else {
                    text.push(current);
                }
            }
            let end = characters
                .get(cursor.saturating_sub(1))
                .map_or(source.len(), |(offset, current)| {
                    offset + current.len_utf8()
                });
            result.push(Token {
                kind: TokenKind::String,
                text,
                start,
                end,
                line: line_for_offset(source, start),
                column: column_for_offset(source, start),
            });
            continue;
        }

        if is_word_character(character) {
            let start_line = line;
            let start_column = column;
            let mut word = String::new();
            while cursor < characters.len() {
                let current = characters[cursor].1;
                let accepted = is_word_character(current)
                    || (ALLOW_HYPHENATED_WORDS
                        && current == '-'
                        && cursor + 1 < characters.len()
                        && is_word_character(characters[cursor + 1].1));
                if !accepted {
                    break;
                }
                word.push(current);
                cursor += 1;
                column += 1;
            }
            let end = characters
                .get(cursor.saturating_sub(1))
                .map_or(source.len(), |(offset, current)| {
                    offset + current.len_utf8()
                });
            result.push(Token {
                kind: TokenKind::Word,
                text: normalize_word(&word),
                start,
                end,
                line: start_line,
                column: start_column,
            });
            continue;
        }

        result.push(Token {
            kind: TokenKind::Symbol(character),
            text: character.to_string(),
            start,
            end: start + character.len_utf8(),
            line,
            column,
        });
        cursor += 1;
        column += 1;
    }

    result
}

pub(super) fn without_literals_and_comments(source: &str) -> String {
    let mut result = source
        .chars()
        .map(|character| {
            if matches!(character, '\n' | '\r') {
                character
            } else {
                ' '
            }
        })
        .collect::<Vec<_>>();

    let mut byte_cursor = 0;
    let mut character_cursor = 0;
    for token in tokens(source) {
        while byte_cursor < token.start {
            let character = source[byte_cursor..]
                .chars()
                .next()
                .expect("token offset must be within the source");
            byte_cursor += character.len_utf8();
            character_cursor += 1;
        }
        if matches!(token.kind, TokenKind::String | TokenKind::Newline) {
            continue;
        }
        let start = character_cursor;
        let mut token_character_count = 0;
        for (offset, character) in source[token.start..token.end].chars().enumerate() {
            result[start + offset] = character;
            token_character_count += 1;
        }
        character_cursor += token_character_count;
        byte_cursor = token.end;
    }

    result.into_iter().collect()
}

pub(super) fn line_without_comments(source: &str, selected_line: usize) -> String {
    let mut result = String::new();
    let mut line = 1usize;
    let mut in_block_comment = false;
    let mut in_line_comment = false;
    let mut quote = None;
    let mut escaped = false;
    let mut characters = source.chars().peekable();

    while let Some(character) = characters.next() {
        let selected = line == selected_line;
        if character == '\n' {
            if selected {
                return result;
            }
            line += 1;
            in_line_comment = false;
            quote = None;
            escaped = false;
            continue;
        }

        if in_line_comment {
            if selected {
                result.push(' ');
            }
            continue;
        }
        if in_block_comment {
            if character == '*' && characters.peek() == Some(&'/') {
                characters.next();
                if selected {
                    result.push_str("  ");
                }
                in_block_comment = false;
            } else if selected {
                result.push(' ');
            }
            continue;
        }
        if let Some(delimiter) = quote {
            if selected {
                result.push(character);
            }
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            continue;
        }

        if character == '/' && characters.peek() == Some(&'*') {
            characters.next();
            in_block_comment = true;
            if selected {
                result.push_str("  ");
            }
        } else if character == '/' && characters.peek() == Some(&'/') {
            characters.next();
            in_line_comment = true;
            if selected {
                result.push_str("  ");
            }
        } else if HASH_STARTS_LINE_COMMENT && character == '#' {
            in_line_comment = true;
            if selected {
                result.push(' ');
            }
        } else {
            if selected {
                result.push(character);
            }
            if matches!(character, '\'' | '"') {
                quote = Some(character);
            }
        }
    }

    result
}

pub(super) fn normalize_word(value: &str) -> String {
    value
        .to_lowercase()
        .replace('á', "a")
        .replace('à', "a")
        .replace('ã', "a")
        .replace('â', "a")
        .replace('é', "e")
        .replace('ê', "e")
        .replace('í', "i")
        .replace('ó', "o")
        .replace('ô', "o")
        .replace('õ', "o")
        .replace('ú', "u")
        .replace('ç', "c")
}

fn is_word_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

fn next_is(characters: &[(usize, char)], cursor: usize, expected: char) -> bool {
    characters
        .get(cursor + 1)
        .is_some_and(|(_, character)| *character == expected)
}

fn line_for_offset(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .chars()
        .filter(|character| *character == '\n')
        .count()
        + 1
}

fn column_for_offset(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .rsplit('\n')
        .next()
        .map_or(1, |line| line.chars().count() + 1)
}

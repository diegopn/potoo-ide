pub(crate) fn read_identifier(characters: &[char], cursor: &mut usize) -> Option<String> {
    skip_spaces(characters, cursor);
    let start = *cursor;
    if !characters
        .get(*cursor)
        .is_some_and(|character| character.is_alphabetic() || *character == '_')
    {
        return None;
    }
    *cursor += 1;
    while characters
        .get(*cursor)
        .is_some_and(|character| character.is_alphanumeric() || *character == '_')
    {
        *cursor += 1;
    }
    Some(characters[start..*cursor].iter().collect())
}

pub(crate) fn read_integer(characters: &[char], cursor: &mut usize) -> Option<i64> {
    let start = *cursor;
    if matches!(characters.get(*cursor), Some('-' | '+')) {
        *cursor += 1;
    }
    while characters
        .get(*cursor)
        .is_some_and(|character| character.is_ascii_digit())
    {
        *cursor += 1;
    }
    if *cursor == start || (*cursor == start + 1 && matches!(characters[start], '-' | '+')) {
        *cursor = start;
        return None;
    }
    characters[start..*cursor]
        .iter()
        .collect::<String>()
        .parse()
        .ok()
}

pub(crate) fn skip_spaces(characters: &[char], cursor: &mut usize) {
    while characters
        .get(*cursor)
        .is_some_and(|character| character.is_whitespace())
    {
        *cursor += 1;
    }
}

pub(crate) fn skip_to_top_level_comma(characters: &[char], mut cursor: usize) -> usize {
    let mut square_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    while let Some(character) = characters.get(cursor) {
        match character {
            '[' => square_depth += 1,
            ']' => square_depth = square_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if square_depth == 0 && brace_depth == 0 && paren_depth == 0 => return cursor,
            _ => {}
        }
        cursor += 1;
    }
    cursor
}

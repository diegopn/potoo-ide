use super::*;

pub(crate) fn make_syntax_tags(buffer: &gtk::TextBuffer) -> SyntaxTags {
    let table = buffer.tag_table();
    let keyword = gtk::TextTag::builder().name("keyword").weight(700).build();
    let string = gtk::TextTag::builder().name("string").build();
    let number = gtk::TextTag::builder().name("number").build();
    let comment = gtk::TextTag::builder().name("comment").build();
    let current_line = gtk::TextTag::builder().name("current-line").build();
    let matching_delimiter = gtk::TextTag::builder()
        .name("matching-delimiter")
        .weight(700)
        .build();
    let diagnostic_error_line = gtk::TextTag::builder()
        .name("diagnostic-error-line")
        .build();
    let diagnostic_warning_line = gtk::TextTag::builder()
        .name("diagnostic-warning-line")
        .build();
    let diagnostic_error_range = gtk::TextTag::builder()
        .name("diagnostic-error-range")
        .build();
    diagnostic_error_range.set_property("underline", gtk::pango::Underline::Error);
    let diagnostic_warning_range = gtk::TextTag::builder()
        .name("diagnostic-warning-range")
        .build();
    diagnostic_warning_range.set_property("underline", gtk::pango::Underline::Error);
    table.add(&keyword);
    table.add(&string);
    table.add(&number);
    table.add(&comment);
    table.add(&current_line);
    table.add(&matching_delimiter);
    table.add(&diagnostic_error_line);
    table.add(&diagnostic_warning_line);
    table.add(&diagnostic_error_range);
    table.add(&diagnostic_warning_range);
    SyntaxTags {
        keyword,
        string,
        number,
        comment,
        current_line,
        matching_delimiter,
        diagnostic_error_line,
        diagnostic_warning_line,
        diagnostic_error_range,
        diagnostic_warning_range,
    }
}

pub(crate) fn set_syntax_palette(tags: &SyntaxTags, dark: bool) {
    tags.keyword
        .set_property("foreground", if dark { "#78d19c" } else { "#066b42" });
    tags.string
        .set_property("foreground", if dark { "#dfb47d" } else { "#825022" });
    tags.number
        .set_property("foreground", if dark { "#b9a9e8" } else { "#55468e" });
    tags.comment
        .set_property("foreground", if dark { "#9ca8a1" } else { "#718078" });
    tags.current_line.set_property(
        "paragraph-background",
        if dark { "#282f2b" } else { "#f2f7f3" },
    );
    tags.matching_delimiter
        .set_property("background", if dark { "#416a53" } else { "#cce8d8" });
    tags.matching_delimiter
        .set_property("foreground", if dark { "#ffffff" } else { "#075b3e" });
    let error_line = if dark {
        gtk::gdk::RGBA::new(0.62, 0.17, 0.17, 0.22)
    } else {
        gtk::gdk::RGBA::new(0.91, 0.28, 0.25, 0.14)
    };
    let warning_line = if dark {
        gtk::gdk::RGBA::new(0.58, 0.42, 0.13, 0.2)
    } else {
        gtk::gdk::RGBA::new(0.93, 0.68, 0.17, 0.16)
    };
    let error_underline = if dark {
        gtk::gdk::RGBA::new(1.0, 0.43, 0.39, 1.0)
    } else {
        gtk::gdk::RGBA::new(0.78, 0.12, 0.1, 1.0)
    };
    let warning_underline = if dark {
        gtk::gdk::RGBA::new(1.0, 0.78, 0.24, 1.0)
    } else {
        gtk::gdk::RGBA::new(0.66, 0.43, 0.03, 1.0)
    };
    tags.diagnostic_error_line
        .set_property("paragraph-background-rgba", error_line);
    tags.diagnostic_warning_line
        .set_property("paragraph-background-rgba", warning_line);
    tags.diagnostic_error_range
        .set_property("underline-rgba", error_underline);
    tags.diagnostic_warning_range
        .set_property("underline-rgba", warning_underline);
}

pub(crate) fn highlight_buffer(buffer: &gtk::TextBuffer, language: Language, tags: &SyntaxTags) {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    for tag in [
        &tags.keyword,
        &tags.string,
        &tags.number,
        &tags.comment,
        &tags.current_line,
        &tags.matching_delimiter,
    ] {
        buffer.remove_tag(tag, &start, &end);
    }
    let source = buffer.text(&start, &end, false);
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        if matches!(language, Language::GPortugol | Language::Calango) && chars[index] == '/' {
            let comment_end = match chars.get(index + 1) {
                Some('/') => {
                    let mut end = index + 2;
                    while end < chars.len() && chars[end] != '\n' {
                        end += 1;
                    }
                    Some(end)
                }
                Some('*') => {
                    let mut end = index + 2;
                    while end + 1 < chars.len() && !(chars[end] == '*' && chars[end + 1] == '/') {
                        end += 1;
                    }
                    Some(if end + 1 < chars.len() {
                        end + 2
                    } else {
                        chars.len()
                    })
                }
                _ => None,
            };
            if let Some(end) = comment_end {
                apply_tag_range(buffer, &tags.comment, index, end);
                index = end;
                continue;
            }
        }

        if chars[index] == '"' {
            let first = index;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                let closes_string = chars[index] == '"';
                index += 1;
                if closes_string {
                    break;
                }
            }
            apply_tag_range(buffer, &tags.string, first, index);
            continue;
        }

        if language == Language::Calango && chars[index] == '\'' {
            let first = index;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                let closes_character = chars[index] == '\'';
                index += 1;
                if closes_character {
                    break;
                }
            }
            apply_tag_range(buffer, &tags.string, first, index);
            continue;
        }

        if chars[index].is_alphabetic() || chars[index] == '_' {
            let first = index;
            index += 1;
            while index < chars.len() && (chars[index].is_alphanumeric() || chars[index] == '_') {
                index += 1;
            }
            let word: String = chars[first..index].iter().collect();
            if language.keywords().contains(&word.as_str()) {
                apply_tag_range(buffer, &tags.keyword, first, index);
            }
            continue;
        }

        if chars[index].is_ascii_digit() {
            let first = index;
            index += 1;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
            }
            apply_tag_range(buffer, &tags.number, first, index);
            continue;
        }

        index += 1;
    }

    let cursor = buffer.iter_at_offset(buffer.cursor_position());
    if let Some(mut line) = buffer.iter_at_line(cursor.line()) {
        let first = line.offset() as usize;
        line.forward_to_line_end();
        apply_tag_range(buffer, &tags.current_line, first, line.offset() as usize);
    }

    if let Some((first, second)) = matching_delimiter_at_cursor(&chars, language, cursor.offset()) {
        apply_tag_range(buffer, &tags.matching_delimiter, first, first + 1);
        apply_tag_range(buffer, &tags.matching_delimiter, second, second + 1);
    }
}

fn matching_delimiter_at_cursor(
    chars: &[char],
    language: Language,
    cursor_offset: i32,
) -> Option<(usize, usize)> {
    let mut pairs = vec![None; chars.len()];
    let mut open = Vec::<(usize, char)>::new();
    let mut index = 0;

    while index < chars.len() {
        let current = chars[index];
        let next = chars.get(index + 1).copied();

        if current == '"' || current == '\'' {
            let quote = current;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                let closes_string = chars[index] == quote;
                index += 1;
                if closes_string {
                    break;
                }
            }
            continue;
        }

        if current == '/' && next == Some('/') {
            index += 2;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }

        if language == Language::Potigol && current == '#' {
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }

        if matches!(
            language,
            Language::Portugol | Language::GPortugol | Language::Calango
        ) && current == '/'
            && next == Some('*')
        {
            index += 2;
            let mut closed = false;
            while index + 1 < chars.len() {
                if chars[index] == '*' && chars[index + 1] == '/' {
                    index += 2;
                    closed = true;
                    break;
                }
                index += 1;
            }
            if !closed {
                break;
            }
            continue;
        }

        if language == Language::Visualg && current == '{' {
            while index < chars.len() && chars[index] != '}' {
                index += 1;
            }
            index = (index + 1).min(chars.len());
            continue;
        }

        if matches!(current, '{' | '(' | '[') {
            open.push((index, current));
        } else if let Some(expected_open) = matching_open(current) {
            if open
                .last()
                .is_some_and(|(_, delimiter)| *delimiter == expected_open)
            {
                if let Some((opening_index, _)) = open.pop() {
                    pairs[opening_index] = Some(index);
                    pairs[index] = Some(opening_index);
                }
            }
        }

        index += 1;
    }

    let cursor = usize::try_from(cursor_offset).unwrap_or(0).min(chars.len());
    let current = (cursor < chars.len()).then_some(cursor);
    let previous = cursor.checked_sub(1).filter(|offset| *offset < chars.len());
    let delimiter = current
        .filter(|offset| pairs[*offset].is_some())
        .or_else(|| previous.filter(|offset| pairs[*offset].is_some()))?;
    Some((delimiter, pairs[delimiter]?))
}

fn matching_open(closing: char) -> Option<char> {
    match closing {
        '}' => Some('{'),
        ')' => Some('('),
        ']' => Some('['),
        _ => None,
    }
}

pub(super) fn apply_tag_range(
    buffer: &gtk::TextBuffer,
    tag: &gtk::TextTag,
    first: usize,
    last: usize,
) {
    if first >= last {
        return;
    }
    let start = buffer.iter_at_offset(first as i32);
    let end = buffer.iter_at_offset(last as i32);
    buffer.apply_tag(tag, &start, &end);
}

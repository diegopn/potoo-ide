mod portugol;
mod potigol;

use crate::components::content_panel::Language;

pub(crate) fn prefix(language: Language, source: &str, line: usize) -> String {
    match language {
        Language::Portugol => portugol::studio_20::prefix(source, line),
        Language::Visualg => portugol::visualg::prefix(source, line),
        Language::Potigol => potigol::prefix(source, line),
        Language::GPortugol => portugol::g_portugol::prefix(source, line),
        Language::Calango => portugol::calango::prefix(source, line),
        Language::Mapler => portugol::mapler::prefix(source, line),
    }
}

pub(super) fn before_line(source: &str, line: usize) -> String {
    source
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .collect::<Vec<_>>()
        .concat()
}

pub(super) fn close_blocks(
    source: &str,
    pairs: &[(&str, &str)],
    root: Option<(&str, &str)>,
) -> String {
    // shortcut: closes common block keywords; replace with token-aware recovery for ambiguous inline blocks.
    let mut stack = Vec::new();
    let source_words = words(source);
    let mut index = 0;
    while index < source_words.len() {
        let word = &source_words[index];
        if let Some((_, closer)) = pairs.iter().find(|(opener, _)| *opener == word.as_str()) {
            stack.push(*closer);
        } else if let Some(closer) = stack.last() {
            let closer_words = words(closer);
            if source_words[index..].starts_with(&closer_words) {
                stack.pop();
                index += closer_words.len().saturating_sub(1);
            }
        }
        index += 1;
    }
    if let Some((opener, closer)) = root
        && source_words.iter().any(|word| word.as_str() == opener)
    {
        stack.insert(0, closer);
    }
    let mut result = source.to_owned();
    for closer in stack.into_iter().rev() {
        result.push('\n');
        result.push_str(closer);
    }
    result
}

fn words(source: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;
    let mut block_comment = false;
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_comment = false;
            }
            continue;
        }
        if comment {
            if ch == '\n' {
                comment = false;
            }
            continue;
        }
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == delimiter {
                quote = None;
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            comment = true;
        } else if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_comment = true;
        } else if ch == '#' {
            comment = true;
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch.is_alphanumeric() || ch == '_' || ch == '-' {
            word.push(ch);
        } else if !word.is_empty() {
            words.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        words.push(word);
    }
    words
        .into_iter()
        .map(|word| {
            word.to_lowercase()
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
        })
        .collect()
}

pub(super) fn unmatched_braces(source: &str) -> usize {
    let mut open = 0_usize;
    let mut close = 0_usize;
    let mut quote = None;
    let mut escaped = false;
    let mut comment = false;
    let mut block_comment = false;
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                block_comment = false;
            }
            continue;
        }
        if comment {
            if ch == '\n' {
                comment = false;
            }
            continue;
        }
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == delimiter {
                quote = None;
            }
            continue;
        }
        if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            comment = true;
        } else if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            block_comment = true;
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        } else if ch == '{' {
            open += 1;
        } else if ch == '}' {
            close += 1;
        }
    }
    open.saturating_sub(close)
}

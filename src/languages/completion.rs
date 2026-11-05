use super::model::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompletionSnippet {
    pub(crate) label: &'static str,
    pub(crate) text: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompletionCatalog {
    pub(crate) keywords: &'static [&'static str],
    pub(crate) functions: &'static [&'static str],
    pub(crate) snippets: &'static [CompletionSnippet],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LibraryImportContext {
    pub(crate) start: usize,
    pub(crate) prefix: String,
    pub(crate) needs_quote: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CursorLexicalContext {
    pub(crate) in_comment: bool,
    pub(crate) in_string: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CompletionSyntax {
    pub(crate) block_comments: bool,
    pub(crate) hash_comments: bool,
    pub(crate) brace_comments: bool,
}

pub(crate) fn catalog(language: Language) -> CompletionCatalog {
    match language {
        Language::Portugol => super::portugol::studio_20::completion::catalog(),
        Language::Visualg => super::portugol::visualg::completion::catalog(),
        Language::Potigol => super::potigol::completion::catalog(),
        Language::GPortugol => super::portugol::g_portugol::completion::catalog(),
        Language::Calango => super::calango::completion::catalog(),
        Language::Mapler => super::portugol::mapler::completion::catalog(),
    }
}

pub(crate) fn cursor_lexical_context(
    language: Language,
    source: &str,
    cursor: usize,
) -> CursorLexicalContext {
    match language {
        Language::Portugol => {
            super::portugol::studio_20::completion::cursor_lexical_context(source, cursor)
        }
        Language::Visualg => {
            super::portugol::visualg::completion::cursor_lexical_context(source, cursor)
        }
        Language::Potigol => super::potigol::completion::cursor_lexical_context(source, cursor),
        Language::GPortugol => {
            super::portugol::g_portugol::completion::cursor_lexical_context(source, cursor)
        }
        Language::Calango => super::calango::completion::cursor_lexical_context(source, cursor),
        Language::Mapler => {
            super::portugol::mapler::completion::cursor_lexical_context(source, cursor)
        }
    }
}

pub(crate) fn library_import_context(
    language: Language,
    chars: &[char],
    cursor: usize,
) -> Option<LibraryImportContext> {
    match language {
        Language::Portugol => {
            super::portugol::studio_20::completion::library_import_context(chars, cursor)
        }
        Language::Potigol => super::potigol::completion::library_import_context(chars, cursor),
        Language::Visualg | Language::GPortugol | Language::Calango | Language::Mapler => None,
    }
}

pub(crate) fn libraries(language: Language) -> &'static [&'static str] {
    match language {
        Language::Portugol => super::portugol::studio_20::completion::libraries(),
        Language::Potigol => super::potigol::completion::libraries(),
        Language::Visualg | Language::GPortugol | Language::Calango | Language::Mapler => &[],
    }
}

pub(crate) fn library_members(
    language: Language,
    source: &str,
    cursor: usize,
    owner: &str,
) -> &'static [&'static str] {
    match language {
        Language::Portugol => {
            super::portugol::studio_20::completion::library_members(source, cursor, owner)
        }
        Language::Potigol => super::potigol::completion::library_members(source, cursor, owner),
        Language::Visualg | Language::GPortugol | Language::Calango | Language::Mapler => &[],
    }
}

pub(crate) fn declared_identifiers(language: Language, source: &str) -> Vec<String> {
    match language {
        Language::Portugol => super::portugol::studio_20::completion::declared_identifiers(source),
        Language::Visualg => super::portugol::visualg::completion::declared_identifiers(source),
        Language::Potigol => super::potigol::completion::declared_identifiers(source),
        Language::GPortugol => {
            super::portugol::g_portugol::completion::declared_identifiers(source)
        }
        Language::Calango => super::calango::completion::declared_identifiers(source),
        Language::Mapler => super::portugol::mapler::completion::declared_identifiers(source),
    }
}

pub(crate) fn cursor_lexical_context_for(
    source: &str,
    cursor: usize,
    syntax: CompletionSyntax,
) -> CursorLexicalContext {
    let chars: Vec<char> = source.chars().collect();
    let cursor = cursor.min(chars.len());
    CursorLexicalContext {
        in_comment: cursor_is_in_comment(&chars, cursor, syntax),
        in_string: cursor_is_in_string(&chars, cursor, syntax),
    }
}

fn cursor_is_in_comment(chars: &[char], cursor: usize, syntax: CompletionSyntax) -> bool {
    let mut index = 0;
    while index < cursor {
        if chars[index] == '"' || chars[index] == '\'' {
            let quote = chars[index];
            index += 1;
            while index < cursor {
                if chars[index] == '\\' {
                    index = (index + 2).min(cursor);
                    continue;
                }
                let closes = chars[index] == quote;
                index += 1;
                if closes {
                    break;
                }
            }
            continue;
        }
        if chars[index] == '/' && chars.get(index + 1) == Some(&'/') {
            while index < cursor && chars[index] != '\n' {
                index += 1;
            }
            if index == cursor {
                return true;
            }
            index += 1;
            continue;
        }
        if syntax.block_comments && chars[index] == '/' && chars.get(index + 1) == Some(&'*') {
            index += 2;
            while index + 1 < cursor && !(chars[index] == '*' && chars[index + 1] == '/') {
                index += 1;
            }
            if index + 1 >= cursor {
                return true;
            }
            index += 2;
            continue;
        }
        if syntax.hash_comments && chars[index] == '#' {
            while index < cursor && chars[index] != '\n' {
                index += 1;
            }
            if index == cursor {
                return true;
            }
            index += 1;
            continue;
        }
        if syntax.brace_comments && chars[index] == '{' {
            while index < cursor && chars[index] != '}' {
                index += 1;
            }
            if index == cursor {
                return true;
            }
            index += 1;
            continue;
        }
        index += 1;
    }
    false
}

fn cursor_is_in_string(chars: &[char], cursor: usize, syntax: CompletionSyntax) -> bool {
    let mut index = 0;
    let mut quote = None;
    while index < cursor {
        if let Some(active_quote) = quote {
            if chars[index] == '\\' {
                index = (index + 2).min(cursor);
                continue;
            }
            if chars[index] == active_quote {
                quote = None;
            }
            index += 1;
            continue;
        }

        if chars[index] == '/' && chars.get(index + 1) == Some(&'/') {
            while index < cursor && chars[index] != '\n' {
                index += 1;
            }
            index += usize::from(index < cursor);
            continue;
        }
        if syntax.block_comments && chars[index] == '/' && chars.get(index + 1) == Some(&'*') {
            index += 2;
            while index + 1 < cursor && !(chars[index] == '*' && chars[index + 1] == '/') {
                index += 1;
            }
            index = (index + 2).min(cursor);
            continue;
        }
        if syntax.hash_comments && chars[index] == '#' {
            while index < cursor && chars[index] != '\n' {
                index += 1;
            }
            index += usize::from(index < cursor);
            continue;
        }
        if syntax.brace_comments && chars[index] == '{' {
            while index < cursor && chars[index] != '}' {
                index += 1;
            }
            if index < cursor {
                index += 1;
            }
            continue;
        }
        if matches!(chars[index], '"' | '\'') {
            quote = Some(chars[index]);
        }
        index += 1;
    }
    quote.is_some()
}

pub(crate) fn current_word(chars: &[char], cursor: usize) -> (usize, String) {
    let mut start = cursor;
    while start > 0 && is_identifier_char(chars[start - 1]) {
        start -= 1;
    }
    (start, chars[start..cursor].iter().collect())
}

pub(crate) fn is_identifier_char(character: char) -> bool {
    character == '_' || character.is_alphanumeric()
}

pub(crate) fn line_bounds(chars: &[char], cursor: usize) -> (usize, usize) {
    let start = chars[..cursor]
        .iter()
        .rposition(|character| *character == '\n')
        .map_or(0, |index| index + 1);
    let end = chars[cursor..]
        .iter()
        .position(|character| *character == '\n')
        .map_or(chars.len(), |index| cursor + index);
    (start, end)
}

pub(crate) fn dot_context(chars: &[char], cursor: usize) -> Option<(String, String)> {
    let (word_start, prefix) = current_word(chars, cursor);
    if word_start == 0 || chars[word_start - 1] != '.' {
        return None;
    }
    let owner_end = word_start - 1;
    let mut owner_start = owner_end;
    while owner_start > 0 && is_identifier_char(chars[owner_start - 1]) {
        owner_start -= 1;
    }
    (owner_start < owner_end).then(|| (chars[owner_start..owner_end].iter().collect(), prefix))
}

pub(crate) fn identifier_at_start(source: &str) -> &str {
    let source = source.trim_start();
    let end = source
        .char_indices()
        .find_map(|(index, character)| (!is_identifier_char(character)).then_some(index))
        .unwrap_or(source.len());
    &source[..end]
}

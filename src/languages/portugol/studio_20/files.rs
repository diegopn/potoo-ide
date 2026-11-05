use std::fs::File;
use std::path::{Path, PathBuf};

use gtk::glib::{Regex, RegexCompileFlags, RegexMatchFlags};

use crate::features::execution::MAX_FILE_BYTES;

#[cfg(windows)]
pub(crate) const PORTUGOL_FILE_LINE_ENDING: &str = "\r\n";
#[cfg(not(windows))]
pub(crate) const PORTUGOL_FILE_LINE_ENDING: &str = "\n";

pub(crate) fn normalize_file_selection_suffixes(suffixes: &str) -> Option<Vec<String>> {
    let mut normalized = Vec::new();
    for suffix in suffixes.split(',') {
        let suffix = suffix.trim();
        if matches!(suffix, "*" | "*.*") {
            return None;
        }
        let suffix = suffix.trim_start_matches('*').trim_start_matches('.');
        if !suffix.is_empty() {
            normalized.push(suffix.to_owned());
        }
    }
    Some(normalized)
}
pub(crate) fn portugol_file_selection_matches(
    path: &str,
    formats: &[String],
    allow_all_files: bool,
) -> bool {
    allow_all_files
        || formats.is_empty()
        || formats.iter().any(|format| {
            let suffixes = format
                .split_once('|')
                .map_or(format.as_str(), |(_, suffixes)| suffixes);
            normalize_file_selection_suffixes(suffixes).is_none_or(|suffixes| {
                suffixes.iter().any(|suffix| {
                    path.to_lowercase()
                        .ends_with(&format!(".{}", suffix.to_lowercase()))
                })
            })
        })
}
pub(crate) fn split_file_lines(contents: &str) -> Vec<String> {
    let bytes = contents.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        if matches!(bytes[index], b'\r' | b'\n') {
            lines.push(contents[start..index].to_owned());
            if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                index += 1;
            }
            index += 1;
            start = index;
        } else {
            index += 1;
        }
    }
    if start < bytes.len() {
        lines.push(contents[start..].to_owned());
    }
    lines
}
pub(crate) enum OpenPortugolFile {
    Read {
        path: PathBuf,
        lines: Vec<String>,
        next_line: usize,
        eof: bool,
    },
    Write {
        path: PathBuf,
        file: File,
        bytes_written: u64,
    },
    Append {
        path: PathBuf,
        file: File,
        bytes_written: u64,
    },
}
impl OpenPortugolFile {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::Read { path, .. } | Self::Write { path, .. } | Self::Append { path, .. } => path,
        }
    }
}

pub(crate) fn replace_file_text_regex(
    source: &str,
    pattern: &str,
    replacement: &str,
    only_first: bool,
) -> Result<String, String> {
    // shortcut: PCRE differs from Java Pattern beyond this alias; replace the engine for full compatibility.
    let pattern = pattern.replace(r"\p{javaLowerCase}", r"\p{Lowercase}");
    let regex = Regex::new(
        &pattern,
        RegexCompileFlags::empty(),
        RegexMatchFlags::empty(),
    )
    .map_err(|error| format!("expressão regular inválida: {error}"))?
    .ok_or_else(|| "não foi possível compilar a expressão regular".to_owned())?;
    let source = gtk::glib::GString::from_string_checked(source.to_owned())
        .map_err(|_| "não é possível processar arquivos com byte nulo".to_owned())?;
    let match_info = regex
        .match_(source.as_gstr(), RegexMatchFlags::empty())
        .map_err(|error| format!("falha ao pesquisar o arquivo: {error}"))?;
    if !match_info.matches() {
        return Ok(source.to_string());
    }
    let replacement = java_replacement_for_glib(replacement, &regex)?;

    let mut output = String::with_capacity(source.len().min(MAX_FILE_BYTES as usize));
    let mut output_bytes = 0usize;
    let mut previous_end = 0usize;
    let match_info = match_info;
    while match_info.matches() {
        let (start, end) = match_info
            .fetch_pos(0)
            .ok_or_else(|| "a expressão regular retornou uma posição inválida".to_owned())?;
        let start = usize::try_from(start)
            .map_err(|_| "a expressão regular retornou uma posição inválida".to_owned())?;
        let end = usize::try_from(end)
            .map_err(|_| "a expressão regular retornou uma posição inválida".to_owned())?;
        append_latin1_bounded(&mut output, &mut output_bytes, &source[previous_end..start])?;
        let expanded = match_info
            .expand_references(&replacement)
            .map_err(|error| format!("referência de substituição inválida: {error}"))?
            .ok_or_else(|| "referência de substituição inválida".to_owned())?;
        append_latin1_bounded(&mut output, &mut output_bytes, expanded.as_str())?;
        previous_end = end;
        if only_first || !match_info.next().map_err(|error| error.to_string())? {
            break;
        }
    }
    append_latin1_bounded(&mut output, &mut output_bytes, &source[previous_end..])?;
    Ok(output)
}

fn java_replacement_for_glib(replacement: &str, regex: &Regex) -> Result<String, String> {
    let mut output = String::with_capacity(replacement.len());
    let mut characters = replacement.chars().peekable();
    while let Some(character) = characters.next() {
        match character {
            '\\' => {
                if let Some(escaped) = characters.next() {
                    if escaped == '\\' {
                        output.push_str("\\\\");
                    } else {
                        output.push(escaped);
                    }
                } else {
                    return Err("barra invertida sem caractere para escapar".to_owned());
                }
            }
            '$' => match characters.peek().copied() {
                Some('{') => {
                    characters.next();
                    let mut name = String::new();
                    let mut closed = false;
                    for next in characters.by_ref() {
                        if next == '}' {
                            closed = true;
                            break;
                        }
                        name.push(next);
                    }
                    if !closed || name.is_empty() || regex.string_number(&name) < 0 {
                        return Err(format!("grupo nomeado '${{{name}}}' não existe"));
                    }
                    output.push_str("\\g<");
                    output.push_str(&name);
                    output.push('>');
                }
                Some(first @ '0'..='9') => {
                    let mut group = first.to_digit(10).unwrap_or(0) as usize;
                    characters.next();
                    while let Some(next @ '0'..='9') = characters.peek().copied() {
                        let candidate = group
                            .saturating_mul(10)
                            .saturating_add(next.to_digit(10).unwrap_or(0) as usize);
                        if candidate > regex.capture_count() as usize {
                            break;
                        }
                        group = candidate;
                        characters.next();
                    }
                    if group > regex.capture_count() as usize {
                        return Err(format!("grupo '${group}' não existe"));
                    }
                    output.push('\\');
                    output.push_str(&group.to_string());
                }
                _ => return Err("referência de grupo inválida na substituição".to_owned()),
            },
            character => output.push(character),
        }
    }
    Ok(output)
}

fn append_latin1_bounded(
    output: &mut String,
    output_bytes: &mut usize,
    text: &str,
) -> Result<(), String> {
    for character in text.chars() {
        *output_bytes = output_bytes
            .checked_add(1)
            .ok_or_else(|| "arquivo excede o limite de 1 MiB".to_owned())?;
        if *output_bytes > MAX_FILE_BYTES as usize {
            return Err("arquivo excede o limite de 1 MiB".to_owned());
        }
        output.push(if character as u32 > u8::MAX as u32 {
            '?'
        } else {
            character
        });
    }
    Ok(())
}

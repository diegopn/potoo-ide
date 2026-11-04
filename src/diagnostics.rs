use std::ops::Range;
use std::path::PathBuf;

use crate::components::content_panel::Language;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiagnosticSeverity {
    Error,
    Warning,
    #[allow(dead_code)]
    Information,
    Hint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DiagnosticCategory {
    Lexical,
    Syntax,
    Type,
    Runtime,
    StaticAnalysis,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EditorDiagnostic {
    pub(crate) document_id: u64,
    pub(crate) source_name: String,
    pub(crate) source_path: Option<PathBuf>,
    pub(crate) language: Language,
    pub(crate) severity: DiagnosticSeverity,
    pub(crate) category: DiagnosticCategory,
    pub(crate) message: String,
    pub(crate) offset: usize,
    pub(crate) range: Option<Range<usize>>,
    pub(crate) line: usize,
    pub(crate) column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AnalysisIdentity {
    pub(crate) generation: u64,
    pub(crate) document_id: u64,
    pub(crate) revision: u64,
    pub(crate) language: Language,
}

pub(crate) fn is_current_analysis(
    result: AnalysisIdentity,
    generation: u64,
    active_document_id: u64,
    revision: u64,
    language: Language,
) -> bool {
    result.generation == generation
        && result.document_id == active_document_id
        && result.revision == revision
        && result.language == language
}

pub(crate) fn byte_offset_to_char_offset(source: &str, byte_offset: usize) -> Option<usize> {
    Some(source.get(..byte_offset)?.chars().count())
}

pub(crate) fn line_column_to_char_offset(source: &str, line: usize, column: usize) -> usize {
    let (mut current_line, mut current_column, mut offset) = (1, 1, 0);
    for character in source.chars() {
        if current_line == line && current_column == column {
            return offset;
        }
        offset += 1;
        if character == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
    }
    offset
}

pub(crate) fn analyze(
    document_id: u64,
    source_name: &str,
    source_path: Option<&std::path::Path>,
    language: Language,
    source: &str,
) -> Vec<EditorDiagnostic> {
    let mut diagnostics = match language {
        Language::Portugol => match potoo_portugol_interpreter::parse(source) {
            Ok(_) => Vec::new(),
            Err(errors) => errors
                .into_iter()
                .map(|error| {
                    from_line_column(
                        document_id,
                        source_name,
                        source_path,
                        language,
                        source,
                        map_portugol_kind(error.kind),
                        error.message,
                        error.line,
                        error.column,
                    )
                })
                .collect(),
        },
        Language::Visualg => potoo_visualg_interpreter::parse(source)
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|error| {
                from_line_column(
                    document_id,
                    source_name,
                    source_path,
                    language,
                    source,
                    match error.kind {
                        potoo_visualg_interpreter::DiagnosticKind::Lexical => {
                            DiagnosticCategory::Lexical
                        }
                        potoo_visualg_interpreter::DiagnosticKind::Syntax => {
                            DiagnosticCategory::Syntax
                        }
                        potoo_visualg_interpreter::DiagnosticKind::Runtime => {
                            DiagnosticCategory::Runtime
                        }
                    },
                    error.message,
                    error.line,
                    error.column,
                )
            })
            .collect(),
        Language::GPortugol => match potoo_gportugol_interpreter::parse(source) {
            Ok(program) => potoo_gportugol_interpreter::check(&program)
                .err()
                .unwrap_or_default()
                .into_iter()
                .map(|error| {
                    from_gportugol(
                        document_id,
                        source_name,
                        source_path,
                        language,
                        source,
                        error,
                    )
                })
                .collect(),
            Err(errors) => errors
                .into_iter()
                .map(|error| {
                    from_gportugol(
                        document_id,
                        source_name,
                        source_path,
                        language,
                        source,
                        error,
                    )
                })
                .collect(),
        },
        Language::Potigol => match potoo_potigol_interpreter::parse(source) {
            Ok(program) => potoo_potigol_interpreter::check(&program)
                .err()
                .unwrap_or_default()
                .into_iter()
                .map(|error| {
                    from_potigol(
                        document_id,
                        source_name,
                        source_path,
                        language,
                        source,
                        error,
                    )
                })
                .collect(),
            Err(errors) => errors
                .into_iter()
                .map(|error| {
                    from_potigol(
                        document_id,
                        source_name,
                        source_path,
                        language,
                        source,
                        error,
                    )
                })
                .collect(),
        },
        Language::Calango => potoo_calango_interpreter::parse(source)
            .err()
            .unwrap_or_default()
            .into_iter()
            .map(|error| {
                from_line_column(
                    document_id,
                    source_name,
                    source_path,
                    language,
                    source,
                    match error.kind {
                        potoo_calango_interpreter::DiagnosticKind::Lexical => {
                            DiagnosticCategory::Lexical
                        }
                        potoo_calango_interpreter::DiagnosticKind::Syntax => {
                            DiagnosticCategory::Syntax
                        }
                        potoo_calango_interpreter::DiagnosticKind::Type => DiagnosticCategory::Type,
                        potoo_calango_interpreter::DiagnosticKind::Runtime => {
                            DiagnosticCategory::Runtime
                        }
                    },
                    error.message,
                    error.line,
                    error.column,
                )
            })
            .collect(),
        Language::Mapler => match potoo_mapler_interpreter::parse(source) {
            Ok(program) => potoo_mapler_interpreter::check(&program)
                .err()
                .unwrap_or_default()
                .into_iter()
                .map(|error| {
                    from_line_column(
                        document_id,
                        source_name,
                        source_path,
                        language,
                        source,
                        map_mapler_kind(error.kind),
                        error.message,
                        error.line,
                        error.column,
                    )
                })
                .collect(),
            Err(errors) => errors
                .into_iter()
                .map(|error| {
                    from_line_column(
                        document_id,
                        source_name,
                        source_path,
                        language,
                        source,
                        map_mapler_kind(error.kind),
                        error.message,
                        error.line,
                        error.column,
                    )
                })
                .collect(),
        },
    };

    normalize_parser_syntax_location(language, source, &mut diagnostics);

    if language == Language::GPortugol
        && let Some((start, line, column)) = unclosed_block_comment(source)
    {
        diagnostics.push(EditorDiagnostic {
            document_id,
            source_name: source_name.to_owned(),
            source_path: source_path.map(PathBuf::from),
            language,
            severity: DiagnosticSeverity::Warning,
            category: DiagnosticCategory::StaticAnalysis,
            message: "Comentário de bloco sem fechamento".to_owned(),
            offset: start,
            range: Some(start..source.chars().count()),
            line,
            column,
        });
    }
    diagnostics
}

#[derive(Clone, Debug)]
struct ParserSyntaxDiagnostic {
    message: String,
    line: usize,
    column: usize,
}

fn normalize_parser_syntax_location(
    language: Language,
    source: &str,
    diagnostics: &mut [EditorDiagnostic],
) {
    if !diagnostics
        .iter()
        .any(|diagnostic| diagnostic.category == DiagnosticCategory::Syntax)
    {
        return;
    }
    let Some(original) = first_parser_syntax_diagnostic(language, source) else {
        return;
    };
    let Some(editor_diagnostic) = diagnostics.iter_mut().find(|diagnostic| {
        diagnostic.category == DiagnosticCategory::Syntax
            && diagnostic.line == original.line
            && diagnostic.column == original.column
    }) else {
        return;
    };
    let Some(line_count) = original.line.checked_sub(1) else {
        return;
    };
    let lines = source.lines().collect::<Vec<_>>();
    if line_count == 0 || line_count > lines.len() {
        return;
    }

    let mut prefix = lines[..line_count].join("\n");
    prefix.push('\n');
    let Some(prefix_error) = first_parser_syntax_diagnostic(language, &prefix) else {
        return;
    };
    if prefix_error.message != original.message {
        return;
    }

    let location = if prefix_error.line <= line_count {
        Some((prefix_error.line, prefix_error.column))
    } else if prefix_error.line == line_count + 1 && prefix_error.column == 1 {
        last_code_location(source, language, line_count)
    } else {
        None
    };
    let Some((line, column)) = location.filter(|(line, _)| *line < original.line) else {
        return;
    };

    // The same parser error in the source prefix means later lines only moved its reported token.
    editor_diagnostic.line = line;
    editor_diagnostic.column = column;
    editor_diagnostic.offset = line_column_to_char_offset(source, line, column);
    editor_diagnostic.range = None;
}

fn first_parser_syntax_diagnostic(
    language: Language,
    source: &str,
) -> Option<ParserSyntaxDiagnostic> {
    match language {
        Language::Portugol => potoo_portugol_interpreter::parse(source)
            .err()?
            .into_iter()
            .find(|error| error.kind == potoo_portugol_interpreter::DiagnosticKind::Syntax)
            .map(|error| ParserSyntaxDiagnostic {
                message: error.message,
                line: error.line,
                column: error.column,
            }),
        Language::Visualg => potoo_visualg_interpreter::parse(source)
            .err()?
            .into_iter()
            .find(|error| error.kind == potoo_visualg_interpreter::DiagnosticKind::Syntax)
            .map(|error| ParserSyntaxDiagnostic {
                message: error.message,
                line: error.line,
                column: error.column,
            }),
        Language::GPortugol => potoo_gportugol_interpreter::parse(source)
            .err()?
            .into_iter()
            .find(|error| error.kind == potoo_gportugol_interpreter::DiagnosticKind::Syntax)
            .map(|error| ParserSyntaxDiagnostic {
                message: error.message,
                line: error.span.start.line,
                column: error.span.start.column,
            }),
        Language::Potigol => potoo_potigol_interpreter::parse(source)
            .err()?
            .into_iter()
            .find(|error| error.kind == potoo_potigol_interpreter::DiagnosticKind::Syntax)
            .map(|error| ParserSyntaxDiagnostic {
                message: error.message,
                line: error.span.start.line,
                column: error.span.start.column,
            }),
        Language::Calango => potoo_calango_interpreter::parse(source)
            .err()?
            .into_iter()
            .find(|error| error.kind == potoo_calango_interpreter::DiagnosticKind::Syntax)
            .map(|error| ParserSyntaxDiagnostic {
                message: error.message,
                line: error.line,
                column: error.column,
            }),
        Language::Mapler => potoo_mapler_interpreter::parse(source)
            .err()?
            .into_iter()
            .find(|error| error.kind == potoo_mapler_interpreter::DiagnosticKind::Syntax)
            .map(|error| ParserSyntaxDiagnostic {
                message: error.message,
                line: error.line,
                column: error.column,
            }),
    }
}

fn last_code_location(
    source: &str,
    language: Language,
    line_count: usize,
) -> Option<(usize, usize)> {
    let has_block_comments = matches!(
        language,
        Language::Portugol | Language::GPortugol | Language::Calango
    );
    let mut inside_block_comment = false;
    let mut quote = None;
    let mut escaped = false;
    let mut last = None;

    for (line_index, line) in source.lines().take(line_count).enumerate() {
        let characters = line.chars().collect::<Vec<_>>();
        let mut index = 0;
        let mut first_code_column = None;
        while index < characters.len() {
            if inside_block_comment {
                if characters[index] == '*' && characters.get(index + 1) == Some(&'/') {
                    index += 2;
                    inside_block_comment = false;
                } else {
                    index += 1;
                }
                continue;
            }

            let character = characters[index];
            if let Some(delimiter) = quote {
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == delimiter {
                    quote = None;
                }
                index += 1;
                continue;
            }

            if (language == Language::Potigol && character == '#')
                || (character == '/' && characters.get(index + 1) == Some(&'/'))
            {
                break;
            }
            if has_block_comments && character == '/' && characters.get(index + 1) == Some(&'*') {
                index += 2;
                inside_block_comment = true;
                continue;
            }
            if matches!(character, '"' | '\'') {
                first_code_column.get_or_insert(index + 1);
                quote = Some(character);
            } else if !character.is_whitespace() {
                first_code_column.get_or_insert(index + 1);
            }
            index += 1;
        }
        if let Some(column) = first_code_column {
            last = Some((line_index + 1, column));
        }
    }

    last
}

pub(crate) fn from_execution(
    document_id: u64,
    source_name: &str,
    source_path: Option<&std::path::Path>,
    language: Language,
    source: &str,
    diagnostics: &[crate::execution::ExecutionDiagnostic],
) -> Vec<EditorDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let category = match diagnostic.kind {
                crate::execution::DiagnosticKind::Lexical => DiagnosticCategory::Lexical,
                crate::execution::DiagnosticKind::Syntax => DiagnosticCategory::Syntax,
                crate::execution::DiagnosticKind::Type => DiagnosticCategory::Type,
                crate::execution::DiagnosticKind::Runtime => DiagnosticCategory::Runtime,
            };
            from_line_column(
                document_id,
                source_name,
                source_path,
                language,
                source,
                category,
                diagnostic.message.clone(),
                diagnostic.line,
                diagnostic.column,
            )
        })
        .collect()
}

fn from_line_column(
    document_id: u64,
    source_name: &str,
    source_path: Option<&std::path::Path>,
    language: Language,
    source: &str,
    category: DiagnosticCategory,
    message: String,
    line: usize,
    column: usize,
) -> EditorDiagnostic {
    EditorDiagnostic {
        document_id,
        source_name: source_name.to_owned(),
        source_path: source_path.map(PathBuf::from),
        language,
        severity: DiagnosticSeverity::Error,
        category,
        message: adapt_diagnostic_message(language, source, category, &message, line, column),
        offset: line_column_to_char_offset(source, line, column),
        range: None,
        line,
        column,
    }
}

pub(crate) fn adapt_diagnostic_message(
    language: Language,
    source: &str,
    category: DiagnosticCategory,
    message: &str,
    line: usize,
    column: usize,
) -> String {
    let is_potigol_field_assignment = language == Language::Potigol
        && category == DiagnosticCategory::Syntax
        && message == "esperava '=' na declaração de valor"
        && line
            .checked_sub(1)
            .and_then(|line| source.lines().nth(line))
            .is_some_and(|source_line| {
                let Some((dot_offset, '.')) = column
                    .checked_sub(1)
                    .and_then(|column| source_line.char_indices().nth(column))
                else {
                    return false;
                };
                source_line
                    .get(dot_offset + 1..)
                    .is_some_and(has_potigol_assignment_operator)
            });
    if is_potigol_field_assignment {
        "A atribuição em Potigol usa ':='; substitua '=' por ':='.".to_owned()
    } else {
        message.to_owned()
    }
}

fn has_potigol_assignment_operator(source: &str) -> bool {
    let mut quote = None;
    let mut escaped = false;
    let mut previous = None;
    for (offset, character) in source.char_indices() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
            continue;
        }
        if character == '#' || (character == '/' && source[offset..].starts_with("//")) {
            break;
        }
        if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if character == '='
            && !matches!(previous, Some(':' | '=' | '!' | '<' | '>'))
            && source[offset + character.len_utf8()..].chars().next() != Some('=')
        {
            return true;
        }
        previous = Some(character);
    }
    false
}

fn from_gportugol(
    document_id: u64,
    source_name: &str,
    source_path: Option<&std::path::Path>,
    language: Language,
    source: &str,
    error: potoo_gportugol_interpreter::Diagnostic,
) -> EditorDiagnostic {
    let start = byte_offset_to_char_offset(source, error.span.start.byte_offset);
    let end = byte_offset_to_char_offset(source, error.span.end.byte_offset);
    let offset = start.unwrap_or_else(|| {
        line_column_to_char_offset(source, error.span.start.line, error.span.start.column)
    });
    EditorDiagnostic {
        document_id,
        source_name: source_name.to_owned(),
        source_path: source_path.map(PathBuf::from),
        language,
        severity: DiagnosticSeverity::Error,
        category: match error.kind {
            potoo_gportugol_interpreter::DiagnosticKind::Lexical => DiagnosticCategory::Lexical,
            potoo_gportugol_interpreter::DiagnosticKind::Syntax => DiagnosticCategory::Syntax,
            potoo_gportugol_interpreter::DiagnosticKind::Type => DiagnosticCategory::Type,
            potoo_gportugol_interpreter::DiagnosticKind::Runtime => DiagnosticCategory::Runtime,
        },
        message: error.message,
        offset,
        range: match (start, end) {
            (Some(start), Some(end)) if end > start => Some(start..end),
            _ => None,
        },
        line: error.span.start.line,
        column: error.span.start.column,
    }
}

fn from_potigol(
    document_id: u64,
    source_name: &str,
    source_path: Option<&std::path::Path>,
    language: Language,
    source: &str,
    error: potoo_potigol_interpreter::Diagnostic,
) -> EditorDiagnostic {
    let missing_output_argument = potigol_missing_output_argument(source, &error);
    let start = byte_offset_to_char_offset(source, error.span.start.byte_offset);
    let end = byte_offset_to_char_offset(source, error.span.end.byte_offset);
    let (offset, line, column, range) = if let Some((offset, line, column)) =
        missing_output_argument
    {
        (offset, line, column, None)
    } else {
        (
            start.unwrap_or_else(|| {
                line_column_to_char_offset(source, error.span.start.line, error.span.start.column)
            }),
            error.span.start.line,
            error.span.start.column,
            match (start, end) {
                (Some(start), Some(end)) if end > start => Some(start..end),
                _ => None,
            },
        )
    };
    let category = match error.kind {
        potoo_potigol_interpreter::DiagnosticKind::Lexical => DiagnosticCategory::Lexical,
        potoo_potigol_interpreter::DiagnosticKind::Syntax => DiagnosticCategory::Syntax,
        potoo_potigol_interpreter::DiagnosticKind::Type => DiagnosticCategory::Type,
        potoo_potigol_interpreter::DiagnosticKind::Runtime => DiagnosticCategory::Runtime,
    };
    let message =
        adapt_diagnostic_message(language, source, category, &error.message, line, column);
    EditorDiagnostic {
        document_id,
        source_name: source_name.to_owned(),
        source_path: source_path.map(PathBuf::from),
        language,
        severity: DiagnosticSeverity::Error,
        category,
        message,
        offset,
        range,
        line,
        column,
    }
}

fn potigol_missing_output_argument(
    source: &str,
    error: &potoo_potigol_interpreter::Diagnostic,
) -> Option<(usize, usize, usize)> {
    if error.kind != potoo_potigol_interpreter::DiagnosticKind::Syntax
        || error.message != "esperava uma expressão"
    {
        return None;
    }

    // shortcut: reanchor only a bare preceding `escreva`; widen if missing-operand spans become public.
    let line_index = error.span.start.line.checked_sub(1)?;
    let lines = source.lines().take(line_index).collect::<Vec<_>>();
    let (command_index, command_line) = lines
        .iter()
        .enumerate()
        .rev()
        .find(|(_, line)| !line.trim().is_empty())?;
    if command_line.trim() != "escreva" {
        return None;
    }

    let line = command_index + 1;
    let column = command_line
        .chars()
        .take_while(|character| character.is_whitespace())
        .count()
        + 1;
    Some((
        line_column_to_char_offset(source, line, column),
        line,
        column,
    ))
}

fn map_portugol_kind(kind: potoo_portugol_interpreter::DiagnosticKind) -> DiagnosticCategory {
    match kind {
        potoo_portugol_interpreter::DiagnosticKind::Lexical => DiagnosticCategory::Lexical,
        potoo_portugol_interpreter::DiagnosticKind::Syntax => DiagnosticCategory::Syntax,
        potoo_portugol_interpreter::DiagnosticKind::Type => DiagnosticCategory::Type,
        potoo_portugol_interpreter::DiagnosticKind::Runtime => DiagnosticCategory::Runtime,
    }
}

fn map_mapler_kind(kind: potoo_mapler_interpreter::DiagnosticKind) -> DiagnosticCategory {
    match kind {
        potoo_mapler_interpreter::DiagnosticKind::Lexical => DiagnosticCategory::Lexical,
        potoo_mapler_interpreter::DiagnosticKind::Syntax => DiagnosticCategory::Syntax,
        potoo_mapler_interpreter::DiagnosticKind::Type => DiagnosticCategory::Type,
        potoo_mapler_interpreter::DiagnosticKind::Runtime => DiagnosticCategory::Runtime,
    }
}

fn unclosed_block_comment(source: &str) -> Option<(usize, usize, usize)> {
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

#[cfg(test)]
mod tests {
    use super::{
        AnalysisIdentity, DiagnosticCategory, DiagnosticSeverity, adapt_diagnostic_message,
        analyze, byte_offset_to_char_offset, is_current_analysis, line_column_to_char_offset,
    };
    use crate::components::content_panel::Language;

    #[test]
    fn converts_utf8_byte_offsets_to_gtk_character_offsets() {
        assert_eq!(byte_offset_to_char_offset("éerro", 2), Some(1));
        assert_eq!(byte_offset_to_char_offset("éerro", 1), None);
    }

    #[test]
    fn converts_line_and_column_after_accented_text() {
        assert_eq!(line_column_to_char_offset("ação\n  ruim", 2, 3), 7);
    }

    #[test]
    fn rejects_results_from_old_document_revisions() {
        let result = AnalysisIdentity {
            generation: 8,
            document_id: 3,
            revision: 12,
            language: Language::GPortugol,
        };
        assert!(!is_current_analysis(result, 9, 3, 12, Language::GPortugol));
        assert!(!is_current_analysis(result, 8, 3, 13, Language::GPortugol));
        assert!(!is_current_analysis(result, 8, 4, 12, Language::GPortugol));
        assert!(is_current_analysis(result, 8, 3, 12, Language::GPortugol));
    }

    #[test]
    fn gportugol_unclosed_block_comment_is_a_nonfatal_warning() {
        let source = "algoritmo teste; início imprima(; /* comentário aberto";
        let diagnostics = analyze(3, "teste.gpt", None, Language::GPortugol, source);
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.category == DiagnosticCategory::Syntax)
        );
        let warning = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.category == DiagnosticCategory::StaticAnalysis)
            .expect("the open comment should be reported");

        assert_eq!(warning.severity, DiagnosticSeverity::Warning);
        assert_eq!(warning.document_id, 3);
        assert_eq!(warning.source_name, "teste.gpt");
        assert_eq!(warning.line, 1);
        assert_eq!(warning.column, 35);
        assert_eq!(warning.range.as_ref().map(|range| range.start), Some(34));
    }

    #[test]
    fn missing_potigol_output_argument_marks_its_command_line() {
        let diagnostics = analyze(
            8,
            "teste.poti",
            None,
            Language::Potigol,
            "escreva\nvalor := 1",
        );
        let diagnostic = diagnostics
            .first()
            .expect("the invalid output statement should be reported");

        assert_eq!(diagnostic.category, DiagnosticCategory::Syntax);
        assert_eq!(diagnostic.line, 1);
        assert_eq!(diagnostic.column, 1);
        assert_eq!(diagnostic.offset, 0);
        assert!(diagnostic.range.is_none());
    }

    #[test]
    fn assignment_guidance_respects_potigol_operators_and_ignores_strings_and_comments() {
        let raw = "esperava '=' na declaração de valor";
        let assignment = adapt_diagnostic_message(
            Language::Potigol,
            "jogo.titulo = \"teste\"",
            DiagnosticCategory::Syntax,
            raw,
            1,
            5,
        );
        assert!(assignment.contains("usa ':='"));

        for source in [
            "jogo.titulo := \"teste\"",
            "jogo.titulo == \"teste\"",
            "jogo.titulo >= 1",
            "jogo.titulo \"texto = valor\"",
            "jogo.titulo # comentário =",
        ] {
            assert_eq!(
                adapt_diagnostic_message(
                    Language::Potigol,
                    source,
                    DiagnosticCategory::Syntax,
                    raw,
                    1,
                    5,
                ),
                raw,
                "{source}"
            );
        }
        assert_eq!(
            adapt_diagnostic_message(
                Language::Visualg,
                "jogo.titulo = \"teste\"",
                DiagnosticCategory::Syntax,
                raw,
                1,
                5,
            ),
            raw,
            "only Potigol gets Potigol-specific syntax advice"
        );
    }

    #[test]
    fn lexical_error_after_potigol_output_keeps_its_source_line() {
        let diagnostics = analyze(9, "teste.poti", None, Language::Potigol, "escreva\n@");
        let diagnostic = diagnostics
            .first()
            .expect("the invalid character should be reported");

        assert_eq!(diagnostic.category, DiagnosticCategory::Lexical);
        assert_eq!(diagnostic.line, 2);
    }

    #[test]
    fn lexical_errors_from_all_languages_use_the_common_model() {
        for language in [
            Language::Portugol,
            Language::Visualg,
            Language::GPortugol,
            Language::Potigol,
        ] {
            let diagnostics = analyze(7, "teste", None, language, "@");
            assert!(diagnostics.iter().any(|diagnostic| {
                diagnostic.language == language
                    && diagnostic.severity == DiagnosticSeverity::Error
                    && diagnostic.category == DiagnosticCategory::Lexical
            }));
        }
    }
}

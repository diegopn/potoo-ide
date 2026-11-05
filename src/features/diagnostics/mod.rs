use std::ops::Range;
use std::path::PathBuf;

use crate::languages::model::Language;
use crate::languages::model::{SourceDiagnostic, SourceDiagnosticKind};

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
    let mut diagnostics = language_diagnostics(language, source)
        .into_iter()
        .map(|diagnostic| {
            from_source_diagnostic(
                document_id,
                source_name,
                source_path,
                language,
                source,
                diagnostic,
            )
        })
        .collect::<Vec<_>>();

    normalize_parser_syntax_location(language, source, &mut diagnostics);

    diagnostics
}

fn language_diagnostics(language: Language, source: &str) -> Vec<SourceDiagnostic> {
    (language.profile().diagnostics)(source)
}

fn from_source_diagnostic(
    document_id: u64,
    source_name: &str,
    source_path: Option<&std::path::Path>,
    language: Language,
    source: &str,
    diagnostic: SourceDiagnostic,
) -> EditorDiagnostic {
    let (severity, category) = match diagnostic.kind {
        SourceDiagnosticKind::Lexical => (DiagnosticSeverity::Error, DiagnosticCategory::Lexical),
        SourceDiagnosticKind::Syntax => (DiagnosticSeverity::Error, DiagnosticCategory::Syntax),
        SourceDiagnosticKind::Type => (DiagnosticSeverity::Error, DiagnosticCategory::Type),
        SourceDiagnosticKind::Runtime => (DiagnosticSeverity::Error, DiagnosticCategory::Runtime),
        SourceDiagnosticKind::StaticAnalysis => (
            DiagnosticSeverity::Warning,
            DiagnosticCategory::StaticAnalysis,
        ),
    };
    let span = diagnostic.byte_range.and_then(|range| {
        Some((
            byte_offset_to_char_offset(source, range.start)?,
            byte_offset_to_char_offset(source, range.end)?,
        ))
    });
    let (offset, range) = match span {
        Some((start, end)) => (start, (end > start).then_some(start..end)),
        None => (
            line_column_to_char_offset(source, diagnostic.line, diagnostic.column),
            None,
        ),
    };
    EditorDiagnostic {
        document_id,
        source_name: source_name.to_owned(),
        source_path: source_path.map(PathBuf::from),
        language,
        severity,
        category,
        message: adapt_diagnostic_message(
            language,
            source,
            category,
            &diagnostic.message,
            diagnostic.line,
            diagnostic.column,
        ),
        offset,
        range,
        line: diagnostic.line,
        column: diagnostic.column,
    }
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

fn first_parser_syntax_diagnostic(language: Language, source: &str) -> Option<SourceDiagnostic> {
    language_diagnostics(language, source)
        .into_iter()
        .find(|diagnostic| diagnostic.kind == SourceDiagnosticKind::Syntax)
}

fn last_code_location(
    source: &str,
    language: Language,
    line_count: usize,
) -> Option<(usize, usize)> {
    let mut last = None;
    for line in 1..=line_count {
        let code = (language.profile().source_line_code)(source, line);
        if let Some(column) = code
            .chars()
            .position(|character| !character.is_whitespace())
            .map(|index| index + 1)
        {
            last = Some((line, column));
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
    diagnostics: &[crate::features::execution::ExecutionDiagnostic],
) -> Vec<EditorDiagnostic> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let category = match diagnostic.kind {
                crate::features::execution::DiagnosticKind::Lexical => DiagnosticCategory::Lexical,
                crate::features::execution::DiagnosticKind::Syntax => DiagnosticCategory::Syntax,
                crate::features::execution::DiagnosticKind::Type => DiagnosticCategory::Type,
                crate::features::execution::DiagnosticKind::Runtime => DiagnosticCategory::Runtime,
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
    if language == Language::Potigol && category == DiagnosticCategory::Syntax {
        crate::languages::potigol::diagnostics::adapt_message(source, message, line, column)
    } else {
        message.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AnalysisIdentity, DiagnosticCategory, DiagnosticSeverity, adapt_diagnostic_message,
        analyze, byte_offset_to_char_offset, is_current_analysis, line_column_to_char_offset,
    };
    use crate::languages::model::Language;

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

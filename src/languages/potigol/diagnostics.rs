use crate::languages::model::{SourceDiagnostic, SourceDiagnosticKind};

pub(crate) fn analyze(source: &str) -> Vec<SourceDiagnostic> {
    let errors = match potoo_potigol_interpreter::parse(source) {
        Ok(program) => potoo_potigol_interpreter::check(&program)
            .err()
            .unwrap_or_default(),
        Err(errors) => errors,
    };
    errors
        .into_iter()
        .map(|diagnostic| {
            let kind = match diagnostic.kind {
                potoo_potigol_interpreter::DiagnosticKind::Lexical => SourceDiagnosticKind::Lexical,
                potoo_potigol_interpreter::DiagnosticKind::Syntax => SourceDiagnosticKind::Syntax,
                potoo_potigol_interpreter::DiagnosticKind::Type => SourceDiagnosticKind::Type,
                potoo_potigol_interpreter::DiagnosticKind::Runtime => SourceDiagnosticKind::Runtime,
            };
            let (line, column, byte_range) = if let Some((line, column)) =
                missing_output_argument_location(
                    source,
                    kind,
                    &diagnostic.message,
                    diagnostic.span.start.line,
                ) {
                (line, column, None)
            } else {
                (
                    diagnostic.span.start.line,
                    diagnostic.span.start.column,
                    Some(diagnostic.span.start.byte_offset..diagnostic.span.end.byte_offset),
                )
            };
            SourceDiagnostic {
                kind,
                message: diagnostic.message,
                line,
                column,
                byte_range,
            }
        })
        .collect()
}

pub(crate) fn adapt_message(source: &str, message: &str, line: usize, column: usize) -> String {
    let is_field_assignment = message == "esperava '=' na declaração de valor"
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
                    .is_some_and(has_assignment_operator)
            });

    if is_field_assignment {
        "A atribuição em Potigol usa ':='; substitua '=' por ':='.".to_owned()
    } else {
        message.to_owned()
    }
}

pub(crate) fn missing_output_argument_location(
    source: &str,
    kind: SourceDiagnosticKind,
    message: &str,
    diagnostic_line: usize,
) -> Option<(usize, usize)> {
    if kind != SourceDiagnosticKind::Syntax || message != "esperava uma expressão" {
        return None;
    }
    let lines = source
        .lines()
        .take(diagnostic_line.checked_sub(1)?)
        .collect::<Vec<_>>();
    let (command_index, command_line) = lines
        .iter()
        .enumerate()
        .rev()
        .find(|(_, line)| !line.trim().is_empty())?;
    if !matches!(command_line.trim(), "escreva" | "imprima") {
        return None;
    }
    let line = command_index + 1;
    let column = command_line
        .chars()
        .take_while(|character| character.is_whitespace())
        .count()
        + 1;
    Some((line, column))
}

fn has_assignment_operator(source: &str) -> bool {
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

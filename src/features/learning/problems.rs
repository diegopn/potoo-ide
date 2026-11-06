use super::*;

pub(super) fn learning_problem_kind(
    category: crate::features::diagnostics::DiagnosticCategory,
) -> LearningProblemKind {
    match category {
        crate::features::diagnostics::DiagnosticCategory::Lexical
        | crate::features::diagnostics::DiagnosticCategory::Syntax => LearningProblemKind::Syntax,
        crate::features::diagnostics::DiagnosticCategory::Type => LearningProblemKind::Type,
        crate::features::diagnostics::DiagnosticCategory::Runtime => LearningProblemKind::Runtime,
        crate::features::diagnostics::DiagnosticCategory::StaticAnalysis => {
            LearningProblemKind::PossibleLogic
        }
    }
}

pub(super) fn diagnostic_problem(
    kind: LearningProblemKind,
    line: usize,
    column: usize,
    message: String,
) -> LearningProblem {
    LearningProblem {
        kind,
        severity: LearningSeverity::Error,
        line,
        column,
        text: LearningProblemText::InterpreterDiagnostic,
        message,
        suggestion: None,
    }
}

pub(super) fn constant_condition_problem(
    line: &str,
    line_number: usize,
) -> Option<LearningProblem> {
    let condition = extract_condition(line);
    let normalized = condition.to_lowercase();
    if !matches!(
        normalized.as_str(),
        "verdadeiro" | "falso" | "true" | "false"
    ) {
        return None;
    }
    Some(LearningProblem {
        kind: LearningProblemKind::PossibleLogic,
        severity: LearningSeverity::Warning,
        line: line_number,
        column: character_column(line, &condition),
        text: LearningProblemText::ConstantCondition {
            always_true: matches!(normalized.as_str(), "verdadeiro" | "true"),
        },
        message: if matches!(normalized.as_str(), "verdadeiro" | "true") {
            "A condição é sempre verdadeira; o ramo alternativo pode nunca ser executado."
                .to_owned()
        } else {
            "A condição é sempre falsa; o bloco pode nunca ser executado.".to_owned()
        },
        suggestion: Some("Confira se a condição deveria comparar uma variável.".to_owned()),
    })
}

pub(super) fn constant_index_problems(
    source: &str,
    selected_line: &str,
    line_number: usize,
    language: Language,
    is_cancelled: &impl Fn() -> bool,
) -> Option<Vec<LearningProblem>> {
    let source_code = (language.profile().without_literals_and_comments)(source);
    let arrays = declared_array_bounds(&source_code, language, is_cancelled)?;
    let code = source_code
        .lines()
        .nth(line_number.saturating_sub(1))
        .unwrap_or(selected_line);
    let characters = code.chars().collect::<Vec<_>>();
    let mut problems = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        if is_cancelled() {
            return None;
        }
        skip_spaces(&characters, &mut index);
        let name_start = index;
        let Some(name) = read_identifier(&characters, &mut index) else {
            index += 1;
            continue;
        };
        let Some(dimensions) = arrays.get(&name.to_lowercase()) else {
            continue;
        };
        skip_spaces(&characters, &mut index);
        let mut dimension_index = 0;
        while characters.get(index) == Some(&'[') {
            index += 1;
            skip_spaces(&characters, &mut index);
            let Some(value) = read_integer(&characters, &mut index) else {
                break;
            };
            skip_spaces(&characters, &mut index);
            if characters.get(index) != Some(&']') {
                break;
            }
            index += 1;
            if let Some(dimension) = dimensions.get(dimension_index)
                && let (Some(lower), Some(upper)) = (dimension.lower_bound, dimension.upper_bound)
                && !(lower..=upper).contains(&value)
            {
                problems.push(LearningProblem {
                    kind: LearningProblemKind::Logic,
                    severity: LearningSeverity::Error,
                    line: line_number,
                    column: name_start + 1,
                    text: LearningProblemText::ConstantIndex {
                        index: value,
                        name: name.clone(),
                        lower_bound: lower,
                        upper_bound: upper,
                    },
                    message: format!(
                        "O índice {value} está fora dos limites de `{name}` ({lower} a {upper})."
                    ),
                    suggestion: Some(format!(
                        "Use um índice entre {lower} e {upper} em `{name}`."
                    )),
                });
            }
            dimension_index += 1;
            skip_spaces(&characters, &mut index);
        }
    }
    Some(problems)
}

fn declared_array_bounds(
    source: &str,
    language: Language,
    is_cancelled: &impl Fn() -> bool,
) -> Option<HashMap<String, Vec<SourceDimension>>> {
    match language.profile().declared_array_bounds {
        Some(adapter) => adapter(source, is_cancelled),
        None => Some(HashMap::new()),
    }
}

fn character_column(line: &str, fragment: &str) -> usize {
    line.find(fragment)
        .map(|byte_offset| line[..byte_offset].chars().count() + 1)
        .unwrap_or(1)
}

pub(super) fn extract_condition(line: &str) -> String {
    let mut condition = line.trim();
    let lowercase = condition.to_lowercase();
    let keyword_length = if lowercase.starts_with("se ") {
        Some(3)
    } else if lowercase.starts_with("escolha ") {
        Some("escolha ".len())
    } else {
        None
    };
    if let Some(keyword_length) = keyword_length {
        condition = condition[keyword_length..].trim();
    }
    if let Some(end) = condition
        .to_lowercase()
        .find(" então")
        .or_else(|| condition.to_lowercase().find(" entao"))
    {
        condition = condition[..end].trim();
    }
    condition = condition.trim_end_matches(['{', ';']).trim();
    condition.trim_matches(['(', ')', ' ']).to_owned()
}

pub(super) fn suggest_condition_change(condition: &str) -> Option<String> {
    let mut index = 0;
    let mut quote: Option<char> = None;
    while index < condition.len() {
        let character = condition[index..].chars().next()?;
        let next_index = index + character.len_utf8();

        if let Some(active_quote) = quote {
            if character == '\\' {
                index = condition[next_index..]
                    .chars()
                    .next()
                    .map_or(next_index, |escaped| next_index + escaped.len_utf8());
            } else {
                if character == active_quote {
                    quote = None;
                }
                index = next_index;
            }
            continue;
        }

        if matches!(character, '\'' | '"') {
            quote = Some(character);
            index = next_index;
            continue;
        }
        if character == '#'
            || condition[index..].starts_with("//")
            || condition[index..].starts_with("/*")
        {
            break;
        }
        if !character.is_ascii_digit() {
            index = next_index;
            continue;
        }

        let end = condition[index..]
            .find(|next: char| !next.is_ascii_digit())
            .map_or(condition.len(), |offset| index + offset);
        let previous = condition[..index].chars().next_back();
        let next = condition[end..].chars().next();
        let is_exponent_continuation = matches!(previous, Some('+' | '-'))
            && condition[..index]
                .chars()
                .rev()
                .nth(1)
                .is_some_and(|character| matches!(character, 'e' | 'E'));
        let is_identifier_continuation = previous.is_some_and(is_identifier_character)
            || next.is_some_and(is_identifier_character)
            || is_exponent_continuation;

        if !is_identifier_continuation && let Ok(number) = condition[index..end].parse::<i64>() {
            let replacement = if number > 2 {
                number.saturating_sub(2)
            } else {
                number.saturating_add(2)
            }
            .to_string();
            return Some(format!(
                "{}{}{}",
                &condition[..index],
                replacement,
                &condition[end..]
            ));
        }
        index = end;
    }
    None
}

fn is_identifier_character(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod condition_suggestion_tests {
    use super::suggest_condition_change;

    #[test]
    fn does_not_change_digits_in_identifiers_strings_or_comments() {
        assert_eq!(
            suggest_condition_change("nota1 > 0"),
            Some("nota1 > 2".to_owned())
        );
        assert_eq!(suggest_condition_change("texto == \"nota1\""), None);
        assert_eq!(suggest_condition_change("texto == palavra // nota1"), None);
    }
}

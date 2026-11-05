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
                    suggestion: Some(
                        "Use um índice dentro da faixa declarada para esse vetor ou matriz."
                            .to_owned(),
                    ),
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
    let (start, number) = condition.char_indices().find_map(|(index, character)| {
        if character.is_ascii_digit() {
            let end = condition[index..]
                .find(|next: char| !next.is_ascii_digit())
                .map_or(condition.len(), |offset| index + offset);
            condition[index..end]
                .parse::<i64>()
                .ok()
                .map(|number| (index, number))
        } else {
            None
        }
    })?;
    let end = condition[start..]
        .find(|next: char| !next.is_ascii_digit())
        .map_or(condition.len(), |offset| start + offset);
    let replacement = number.saturating_sub(2).to_string();
    Some(format!(
        "{}{}{}",
        &condition[..start],
        replacement,
        &condition[end..]
    ))
}

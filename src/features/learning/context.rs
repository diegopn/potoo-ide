use super::model::{LearningSymbol, LearningTopic};
use crate::languages::model::{Language, SourceCall};

pub(crate) fn assignment_operator(language: Language) -> &'static str {
    match language {
        Language::Portugol | Language::Calango => "=",
        Language::Visualg | Language::Mapler => "<-",
        Language::Potigol | Language::GPortugol => ":=",
    }
}

pub(super) fn expression_for_line(
    line: &str,
    topic: LearningTopic,
    language: Language,
    command: Option<&str>,
    call: Option<&SourceCall>,
) -> Option<String> {
    match topic {
        LearningTopic::Output => output_expression(line, command, call),
        LearningTopic::Assignment => assignment_expression(line, language),
        LearningTopic::Return => return_expression(line),
        LearningTopic::Program => nonempty(line),
        _ => None,
    }
}

pub(super) fn input_target(
    line: &str,
    language: Language,
    call: Option<&SourceCall>,
    variables: &[LearningSymbol],
    command: Option<&str>,
) -> Option<String> {
    call.and_then(|call| nonempty_list(&call.arguments))
        .or_else(|| {
            nonempty_list(
                &variables
                    .iter()
                    .map(|variable| variable.name.clone())
                    .collect::<Vec<_>>(),
            )
        })
        .or_else(|| assignment_target(line, language))
        .or_else(|| command.and_then(|command| instruction_payload(line, command)))
}

pub(super) fn loop_condition(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let keyword = "enquanto";
    if !trimmed.to_lowercase().starts_with(keyword) {
        return None;
    }

    let mut condition = trimmed[keyword.len()..].trim_start();
    let lowercase = condition.to_lowercase();
    let boundary = [" faça", " faca", " {"]
        .iter()
        .filter_map(|marker| lowercase.find(marker))
        .min();
    if let Some(boundary) = boundary {
        condition = condition[..boundary].trim_end();
    }
    condition = condition.trim_end_matches(['{', ';']).trim();
    if condition.starts_with('(') && condition.ends_with(')') {
        condition = &condition[1..condition.len() - 1];
    }
    nonempty(condition)
}

fn output_expression(
    line: &str,
    command: Option<&str>,
    call: Option<&SourceCall>,
) -> Option<String> {
    command
        .and_then(|command| instruction_payload(line, command))
        .or_else(|| call.and_then(|call| nonempty_list(&call.arguments)))
}

fn assignment_expression(line: &str, language: Language) -> Option<String> {
    let (_, expression) = line.split_once(assignment_operator(language))?;
    nonempty(trim_statement_end(expression)?)
}

fn assignment_target(line: &str, language: Language) -> Option<String> {
    let (target, _) = line.split_once(assignment_operator(language))?;
    let target = target.trim();
    let target = target
        .strip_prefix("var ")
        .or_else(|| target.strip_prefix("var\t"))
        .unwrap_or(target)
        .trim();
    nonempty(target)
}

fn return_expression(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let lowercase = trimmed.to_lowercase();
    ["retorne", "retorna", "return"]
        .iter()
        .find_map(|keyword| {
            lowercase
                .starts_with(keyword)
                .then(|| trimmed[keyword.len()..].trim_start())
        })
        .and_then(trim_statement_end)
        .and_then(nonempty)
}

fn instruction_payload(line: &str, command: &str) -> Option<String> {
    let trimmed = line.trim();
    let command_end = command.len();
    if !trimmed
        .get(..command_end)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(command))
    {
        return None;
    }
    let mut payload = trim_statement_end(&trimmed[command_end..])?;
    if payload.starts_with('(') && payload.ends_with(')') {
        payload = &payload[1..payload.len() - 1];
    }
    nonempty(payload)
}

fn nonempty_list(values: &[String]) -> Option<String> {
    let values = values
        .iter()
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.join(", "))
}

fn trim_statement_end(value: &str) -> Option<&str> {
    let value = value.trim();
    Some(value.trim_end_matches([';', '.']).trim_end())
}

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

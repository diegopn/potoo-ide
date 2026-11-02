use std::collections::{HashMap, HashSet};

use crate::{
    components::content_panel::Language,
    execution::{DiagnosticKind as ExecutionDiagnosticKind, ExecutionDiagnostic},
};

mod portugol;
mod visualg;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LearningTopic {
    Function,
    Procedure,
    Decision,
    Loop,
    Input,
    Output,
    Variable,
    Assignment,
    Return,
    Program,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LearningProblemKind {
    Syntax,
    Type,
    Runtime,
    Logic,
    PossibleLogic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LearningProblemText {
    InterpreterDiagnostic,
    ConstantCondition {
        always_true: bool,
    },
    ConstantIndex {
        index: i64,
        name: String,
        lower_bound: i64,
        upper_bound: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LearningSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearningProblem {
    pub(crate) kind: LearningProblemKind,
    pub(crate) severity: LearningSeverity,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) text: LearningProblemText,
    pub(crate) message: String,
    pub(crate) suggestion: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LearningDimension {
    pub(crate) lower_bound: Option<i64>,
    pub(crate) upper_bound: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearningSymbol {
    pub(crate) name: String,
    pub(crate) type_name: String,
    pub(crate) is_constant: bool,
    pub(crate) by_reference: bool,
    pub(crate) dimensions: Vec<LearningDimension>,
    pub(crate) initializer: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearningHint {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) language: Language,
    pub(crate) topic: LearningTopic,
    pub(crate) name: Option<String>,
    pub(crate) return_type: Option<String>,
    pub(crate) parameters: Vec<LearningSymbol>,
    pub(crate) variables: Vec<LearningSymbol>,
    pub(crate) expression: Option<String>,
    pub(crate) experiment_expression: Option<String>,
    pub(crate) problems: Vec<LearningProblem>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SourceConstruct {
    Program,
    Function,
    Procedure,
    VariableDeclaration,
    Assignment,
    Input,
    Output,
    Decision,
    Loop,
    Return,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct SourceDimension {
    pub(super) lower_bound: Option<i64>,
    pub(super) upper_bound: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceSymbol {
    pub(super) name: String,
    pub(super) type_name: String,
    pub(super) is_constant: bool,
    pub(super) by_reference: bool,
    pub(super) dimensions: Vec<SourceDimension>,
    pub(super) initializer: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SourceLineAnalysis {
    pub(super) line: usize,
    pub(super) column: usize,
    pub(super) construct: SourceConstruct,
    pub(super) name: Option<String>,
    pub(super) return_type: Option<String>,
    pub(super) parameters: Vec<SourceSymbol>,
    pub(super) variables: Vec<SourceSymbol>,
}

struct LearningFacts {
    topic: LearningTopic,
    name: Option<String>,
    return_type: Option<String>,
    parameters: Vec<LearningSymbol>,
    variables: Vec<LearningSymbol>,
}

#[cfg(test)]
pub(crate) fn learning_hint(
    source: &str,
    one_based_line: usize,
    language: Language,
) -> LearningHint {
    learning_hint_with_runtime(source, one_based_line, language, &[])
}

#[cfg(test)]
pub(crate) fn learning_hint_with_runtime(
    source: &str,
    one_based_line: usize,
    language: Language,
    runtime_diagnostics: &[ExecutionDiagnostic],
) -> LearningHint {
    learning_hint_with_runtime_cancellable(
        source,
        one_based_line,
        language,
        runtime_diagnostics,
        || false,
    )
    .expect("an uncancellable analysis always returns a hint")
}

pub(crate) fn learning_hint_with_runtime_cancellable<F>(
    source: &str,
    one_based_line: usize,
    language: Language,
    runtime_diagnostics: &[ExecutionDiagnostic],
    is_cancelled: F,
) -> Option<LearningHint>
where
    F: Fn() -> bool,
{
    if is_cancelled() {
        return None;
    }
    let line = source
        .lines()
        .nth(one_based_line.saturating_sub(1))
        .unwrap_or("");
    let line = match language {
        Language::Potigol => line.split_once('#').map_or(line, |(before, _)| before),
        _ => line.split_once("//").map_or(line, |(before, _)| before),
    }
    .trim();
    let (facts, column, mut problems) = match language {
        Language::Portugol => {
            match portugol::analyze_source_line_cancellable(source, one_based_line, &is_cancelled) {
                Ok(Some(analysis)) => {
                    let column = analysis.column;
                    (Some(portugol_facts(analysis)), column, Vec::new())
                }
                Ok(None) => (None, 1, Vec::new()),
                Err(diagnostics) => {
                    let problems = diagnostics
                        .into_iter()
                        .filter(|diagnostic| diagnostic.line == one_based_line)
                        .map(|diagnostic| LearningProblem {
                            kind: LearningProblemKind::Syntax,
                            severity: LearningSeverity::Error,
                            line: diagnostic.line,
                            column: diagnostic.column,
                            text: LearningProblemText::InterpreterDiagnostic,
                            message: diagnostic.message,
                            suggestion: None,
                        })
                        .collect();
                    (None, 1, problems)
                }
            }
        }
        Language::Visualg => {
            match visualg::analyze_source_line_cancellable(source, one_based_line, &is_cancelled) {
                Ok(Some(analysis)) => {
                    let column = analysis.column;
                    (Some(visualg_facts(analysis)), column, Vec::new())
                }
                Ok(None) => (None, 1, Vec::new()),
                Err(diagnostics) => {
                    let problems = diagnostics
                        .into_iter()
                        .filter(|diagnostic| diagnostic.line == one_based_line)
                        .map(|diagnostic| LearningProblem {
                            kind: LearningProblemKind::Syntax,
                            severity: LearningSeverity::Error,
                            line: diagnostic.line,
                            column: diagnostic.column,
                            text: LearningProblemText::InterpreterDiagnostic,
                            message: diagnostic.message,
                            suggestion: None,
                        })
                        .collect();
                    (None, 1, problems)
                }
            }
        }
        Language::Potigol => (None, 1, Vec::new()),
    };
    if is_cancelled() {
        return None;
    }
    let facts = facts.unwrap_or_else(|| fallback_facts(line));
    let expression = (facts.topic == LearningTopic::Decision).then(|| extract_condition(line));
    let experiment_expression = expression.as_deref().and_then(suggest_condition_change);
    for diagnostic in runtime_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.line == one_based_line)
    {
        let kind = match diagnostic.kind {
            ExecutionDiagnosticKind::Lexical | ExecutionDiagnosticKind::Syntax => {
                LearningProblemKind::Syntax
            }
            ExecutionDiagnosticKind::Type => LearningProblemKind::Type,
            ExecutionDiagnosticKind::Runtime => LearningProblemKind::Runtime,
        };
        if problems.iter().any(|problem: &LearningProblem| {
            problem.kind == kind
                && problem.line == diagnostic.line
                && problem.column == diagnostic.column
                && problem.message == diagnostic.message
        }) {
            continue;
        }
        problems.push(LearningProblem {
            kind,
            severity: LearningSeverity::Error,
            line: diagnostic.line,
            column: diagnostic.column,
            text: LearningProblemText::InterpreterDiagnostic,
            message: diagnostic.message.clone(),
            suggestion: None,
        });
    }
    if let Some(problem) = constant_condition_problem(line, one_based_line) {
        problems.push(problem);
    }
    if facts.topic != LearningTopic::Variable {
        let index_problems =
            constant_index_problems(source, line, one_based_line, language, &is_cancelled)?;
        problems.extend(index_problems);
    }
    if is_cancelled() {
        return None;
    }
    Some(LearningHint {
        line: one_based_line.max(1),
        column,
        language,
        topic: facts.topic,
        name: facts.name,
        return_type: facts.return_type,
        parameters: facts.parameters,
        variables: facts.variables,
        expression,
        experiment_expression,
        problems,
    })
}

fn portugol_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    let topic = match analysis.construct {
        SourceConstruct::Function => LearningTopic::Function,
        SourceConstruct::Procedure => LearningTopic::Procedure,
        SourceConstruct::VariableDeclaration => LearningTopic::Variable,
        SourceConstruct::Assignment => LearningTopic::Assignment,
        SourceConstruct::Input => LearningTopic::Input,
        SourceConstruct::Output => LearningTopic::Output,
        SourceConstruct::Decision => LearningTopic::Decision,
        SourceConstruct::Loop => LearningTopic::Loop,
        SourceConstruct::Return => LearningTopic::Return,
        SourceConstruct::Program | SourceConstruct::Other => LearningTopic::Program,
    };
    LearningFacts {
        topic,
        name: analysis.name,
        return_type: analysis.return_type,
        parameters: analysis
            .parameters
            .into_iter()
            .map(portugol_symbol)
            .collect(),
        variables: analysis
            .variables
            .into_iter()
            .map(portugol_symbol)
            .collect(),
    }
}

fn visualg_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    let topic = match analysis.construct {
        SourceConstruct::Function => LearningTopic::Function,
        SourceConstruct::Procedure => LearningTopic::Procedure,
        SourceConstruct::VariableDeclaration => LearningTopic::Variable,
        SourceConstruct::Assignment => LearningTopic::Assignment,
        SourceConstruct::Input => LearningTopic::Input,
        SourceConstruct::Output => LearningTopic::Output,
        SourceConstruct::Decision => LearningTopic::Decision,
        SourceConstruct::Loop => LearningTopic::Loop,
        SourceConstruct::Return => LearningTopic::Return,
        SourceConstruct::Program | SourceConstruct::Other => LearningTopic::Program,
    };
    LearningFacts {
        topic,
        name: analysis.name,
        return_type: analysis.return_type,
        parameters: analysis
            .parameters
            .into_iter()
            .map(visualg_symbol)
            .collect(),
        variables: analysis.variables.into_iter().map(visualg_symbol).collect(),
    }
}

fn portugol_symbol(symbol: SourceSymbol) -> LearningSymbol {
    LearningSymbol {
        name: symbol.name,
        type_name: symbol.type_name,
        is_constant: symbol.is_constant,
        by_reference: symbol.by_reference,
        dimensions: symbol
            .dimensions
            .into_iter()
            .map(|dimension| LearningDimension {
                lower_bound: dimension.lower_bound,
                upper_bound: dimension.upper_bound,
            })
            .collect(),
        initializer: symbol.initializer,
    }
}

fn visualg_symbol(symbol: SourceSymbol) -> LearningSymbol {
    LearningSymbol {
        name: symbol.name,
        type_name: symbol.type_name,
        is_constant: symbol.is_constant,
        by_reference: symbol.by_reference,
        dimensions: symbol
            .dimensions
            .into_iter()
            .map(|dimension| LearningDimension {
                lower_bound: dimension.lower_bound,
                upper_bound: dimension.upper_bound,
            })
            .collect(),
        initializer: symbol.initializer,
    }
}

fn fallback_facts(line: &str) -> LearningFacts {
    let normalized = line.to_lowercase();
    let topic = if starts_with_any(&normalized, &["funcao ", "procedimento "]) {
        if normalized.starts_with("procedimento ") {
            LearningTopic::Procedure
        } else {
            LearningTopic::Function
        }
    } else if starts_with_any(&normalized, &["se ", "senao", "caso ", "escolha "]) {
        LearningTopic::Decision
    } else if starts_with_any(
        &normalized,
        &[
            "para ",
            "enquanto ",
            "repita",
            "fimenquanto",
            "fimpara",
            "fimrepita",
        ],
    ) {
        LearningTopic::Loop
    } else if starts_with_any(&normalized, &["leia", "input("]) {
        LearningTopic::Input
    } else if starts_with_any(&normalized, &["escreva", "escreval", "print("]) {
        LearningTopic::Output
    } else if normalized.contains(": inteiro")
        || normalized.contains(": real")
        || normalized.contains(": caractere")
        || normalized.contains(": logico")
        || starts_with_any(
            &normalized,
            &[
                "inteiro ",
                "real ",
                "caracter ",
                "caractere ",
                "logico ",
                "cadeia ",
            ],
        )
    {
        LearningTopic::Variable
    } else if normalized.contains("<-") || normalized.contains(" = ") {
        LearningTopic::Assignment
    } else if normalized.starts_with("retorne") {
        LearningTopic::Return
    } else {
        LearningTopic::Program
    };
    LearningFacts {
        topic,
        name: None,
        return_type: None,
        parameters: Vec::new(),
        variables: Vec::new(),
    }
}

fn constant_condition_problem(line: &str, line_number: usize) -> Option<LearningProblem> {
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

fn constant_index_problems(
    source: &str,
    selected_line: &str,
    line_number: usize,
    language: Language,
    is_cancelled: &impl Fn() -> bool,
) -> Option<Vec<LearningProblem>> {
    let arrays = declared_array_bounds(source, language, is_cancelled)?;
    let code = without_literals_and_comments(selected_line);
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
) -> Option<HashMap<String, Vec<LearningDimension>>> {
    let mut arrays = HashMap::new();
    let mut ambiguous = HashSet::new();
    for source_line in source.lines() {
        if is_cancelled() {
            return None;
        }
        let code = without_literals_and_comments(source_line);
        let characters = code.trim().chars().collect::<Vec<_>>();
        let mut cursor = 0;
        match language {
            Language::Portugol => {
                let Some(type_name) = read_identifier(&characters, &mut cursor) else {
                    continue;
                };
                if !matches!(
                    type_name.to_lowercase().as_str(),
                    "inteiro" | "real" | "logico" | "caracter" | "caractere" | "cadeia"
                ) {
                    continue;
                }
                read_portugol_array_declarations(&characters, cursor, &mut arrays, &mut ambiguous);
            }
            Language::Visualg => {
                let mut variables = Vec::new();
                loop {
                    skip_spaces(&characters, &mut cursor);
                    let Some(name) = read_identifier(&characters, &mut cursor) else {
                        break;
                    };
                    variables.push(name);
                    skip_spaces(&characters, &mut cursor);
                    if characters.get(cursor) != Some(&',') {
                        break;
                    }
                    cursor += 1;
                }
                skip_spaces(&characters, &mut cursor);
                if characters.get(cursor) != Some(&':') {
                    continue;
                }
                cursor += 1;
                skip_spaces(&characters, &mut cursor);
                let Some(container) = read_identifier(&characters, &mut cursor) else {
                    continue;
                };
                if !container.eq_ignore_ascii_case("vetor") {
                    continue;
                }
                let Some(dimensions) = read_visualg_dimensions(&characters, &mut cursor) else {
                    continue;
                };
                if dimensions.is_empty() {
                    continue;
                }
                for name in variables {
                    insert_unique_array_bounds(
                        &mut arrays,
                        &mut ambiguous,
                        &name,
                        dimensions.clone(),
                    );
                }
            }
            Language::Potigol => continue,
        }
    }
    Some(arrays)
}

fn character_column(line: &str, fragment: &str) -> usize {
    line.find(fragment)
        .map(|byte_offset| line[..byte_offset].chars().count() + 1)
        .unwrap_or(1)
}

fn read_portugol_array_declarations(
    characters: &[char],
    mut cursor: usize,
    arrays: &mut HashMap<String, Vec<LearningDimension>>,
    ambiguous: &mut HashSet<String>,
) {
    loop {
        skip_spaces(characters, &mut cursor);
        let Some(name) = read_identifier(characters, &mut cursor) else {
            break;
        };
        skip_spaces(characters, &mut cursor);
        let mut dimensions = Vec::new();
        while characters.get(cursor) == Some(&'[') {
            cursor += 1;
            skip_spaces(characters, &mut cursor);
            let Some(size) = read_integer(characters, &mut cursor) else {
                break;
            };
            skip_spaces(characters, &mut cursor);
            if characters.get(cursor) != Some(&']') {
                break;
            }
            cursor += 1;
            dimensions.push(LearningDimension {
                lower_bound: (size > 0).then_some(0),
                upper_bound: (size > 0).then_some(size - 1),
            });
            skip_spaces(characters, &mut cursor);
        }
        if !dimensions.is_empty()
            && dimensions
                .iter()
                .all(|dimension| dimension.upper_bound.is_some())
        {
            insert_unique_array_bounds(arrays, ambiguous, &name, dimensions);
        }
        cursor = skip_to_top_level_comma(characters, cursor);
        if characters.get(cursor) != Some(&',') {
            break;
        }
        cursor += 1;
    }
}

fn insert_unique_array_bounds(
    arrays: &mut HashMap<String, Vec<LearningDimension>>,
    ambiguous: &mut HashSet<String>,
    name: &str,
    dimensions: Vec<LearningDimension>,
) {
    let name = name.to_lowercase();
    if ambiguous.contains(&name) {
        return;
    }
    if arrays.remove(&name).is_some() {
        ambiguous.insert(name);
    } else {
        arrays.insert(name, dimensions);
    }
}

fn read_visualg_dimensions(
    characters: &[char],
    cursor: &mut usize,
) -> Option<Vec<LearningDimension>> {
    skip_spaces(characters, cursor);
    if characters.get(*cursor) != Some(&'[') {
        return None;
    }
    *cursor += 1;
    let mut dimensions = Vec::new();
    loop {
        skip_spaces(characters, cursor);
        let lower = read_integer(characters, cursor)?;
        skip_spaces(characters, cursor);
        if characters.get(*cursor) != Some(&'.') || characters.get(*cursor + 1) != Some(&'.') {
            return None;
        }
        *cursor += 2;
        skip_spaces(characters, cursor);
        let upper = read_integer(characters, cursor)?;
        dimensions.push(LearningDimension {
            lower_bound: Some(lower),
            upper_bound: Some(upper),
        });
        skip_spaces(characters, cursor);
        match characters.get(*cursor) {
            Some(',') => *cursor += 1,
            Some(']') => {
                *cursor += 1;
                return Some(dimensions);
            }
            _ => return None,
        }
    }
}

fn read_identifier(characters: &[char], cursor: &mut usize) -> Option<String> {
    skip_spaces(characters, cursor);
    let start = *cursor;
    if !characters
        .get(*cursor)
        .is_some_and(|character| character.is_alphabetic() || *character == '_')
    {
        return None;
    }
    *cursor += 1;
    while characters
        .get(*cursor)
        .is_some_and(|character| character.is_alphanumeric() || *character == '_')
    {
        *cursor += 1;
    }
    Some(characters[start..*cursor].iter().collect())
}

fn read_integer(characters: &[char], cursor: &mut usize) -> Option<i64> {
    let start = *cursor;
    if matches!(characters.get(*cursor), Some('-' | '+')) {
        *cursor += 1;
    }
    while characters
        .get(*cursor)
        .is_some_and(|character| character.is_ascii_digit())
    {
        *cursor += 1;
    }
    if *cursor == start || (*cursor == start + 1 && matches!(characters[start], '-' | '+')) {
        *cursor = start;
        return None;
    }
    characters[start..*cursor]
        .iter()
        .collect::<String>()
        .parse()
        .ok()
}

fn skip_spaces(characters: &[char], cursor: &mut usize) {
    while characters
        .get(*cursor)
        .is_some_and(|character| character.is_whitespace())
    {
        *cursor += 1;
    }
}

fn skip_to_top_level_comma(characters: &[char], mut cursor: usize) -> usize {
    let mut square_depth = 0usize;
    let mut brace_depth = 0usize;
    let mut paren_depth = 0usize;
    while let Some(character) = characters.get(cursor) {
        match character {
            '[' => square_depth += 1,
            ']' => square_depth = square_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            ',' if square_depth == 0 && brace_depth == 0 && paren_depth == 0 => return cursor,
            _ => {}
        }
        cursor += 1;
    }
    cursor
}

fn without_literals_and_comments(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut characters = line.chars().peekable();
    let mut quote = None;
    while let Some(character) = characters.next() {
        if let Some(quote_character) = quote {
            if character == '\\' {
                result.push(' ');
                if characters.next().is_some() {
                    result.push(' ');
                }
            } else if character == quote_character {
                quote = None;
                result.push(' ');
            } else {
                result.push(' ');
            }
        } else if character == '/' && characters.peek() == Some(&'/') {
            break;
        } else if matches!(character, '"' | '\'') {
            quote = Some(character);
            result.push(' ');
        } else {
            result.push(character);
        }
    }
    result
}

fn starts_with_any(line: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| line.starts_with(prefix))
}

fn extract_condition(line: &str) -> String {
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
    if let Some(end) = condition.to_lowercase().find(" entao") {
        condition = condition[..end].trim();
    }
    condition = condition.trim_end_matches(['{', ';']).trim();
    condition.trim_matches(['(', ')', ' ']).to_owned()
}

fn suggest_condition_change(condition: &str) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::{
        LearningProblemKind, LearningSeverity, LearningTopic, learning_hint,
        learning_hint_with_runtime,
    };
    use crate::components::content_panel::Language;
    use crate::execution::{DiagnosticKind as ExecutionDiagnosticKind, ExecutionDiagnostic};

    #[test]
    fn learning_hint_tracks_active_line_and_updated_condition() {
        let source = "algoritmo \"votacao\"\nvar\nidade: inteiro\ninicio\nescreva(\"idade?\")\nleia(idade)\nse idade >= 18 entao\nescreval(\"sim\")\nfimse\nfimalgoritmo";
        let decision = learning_hint(source, 7, Language::Visualg);
        assert_eq!(decision.topic, LearningTopic::Decision);
        assert_eq!(decision.expression.as_deref(), Some("idade >= 18"));
        assert_eq!(
            decision.experiment_expression.as_deref(),
            Some("idade >= 16")
        );
        let input = learning_hint(source, 6, Language::Visualg);
        assert_eq!(input.topic, LearningTopic::Input);

        let edited = source.replace("18", "16");
        let changed = learning_hint(&edited, 7, Language::Visualg);
        assert_eq!(changed.expression.as_deref(), Some("idade >= 16"));
        assert_eq!(
            changed.experiment_expression.as_deref(),
            Some("idade >= 14")
        );
    }

    #[test]
    fn learning_hint_recognizes_output_loops_and_unclassified_lines() {
        assert_eq!(
            learning_hint("escreval(1)", 1, Language::Visualg).topic,
            LearningTopic::Output
        );
        assert_eq!(
            learning_hint("enquanto x < 3 faca", 1, Language::Visualg).topic,
            LearningTopic::Loop
        );
        assert_eq!(
            learning_hint("escreva(1)", 1, Language::Portugol).topic,
            LearningTopic::Output
        );
        assert_eq!(
            learning_hint("fimse", 1, Language::Visualg).topic,
            LearningTopic::Program
        );
    }

    #[test]
    fn learning_hint_maps_dialect_specific_routine_and_array_facts() {
        let portugol = "programa {\n funcao vazio registrar(inteiro valor) {}\n funcao inicio() {\n  inteiro valores[3]\n }\n}";
        let routine = learning_hint(portugol, 2, Language::Portugol);
        assert_eq!(routine.topic, LearningTopic::Function);
        assert_eq!(routine.name.as_deref(), Some("registrar"));
        assert_eq!(routine.return_type.as_deref(), Some("vazio"));
        assert_eq!(routine.parameters[0].name, "valor");
        assert_eq!(routine.parameters[0].type_name, "inteiro");

        let visualg =
            "algoritmo \"analise\"\nvar\ndados: vetor[2..5] de inteiro\ninicio\nfimalgoritmo\n";
        let vector = learning_hint(visualg, 3, Language::Visualg);
        assert_eq!(vector.topic, LearningTopic::Variable);
        assert_eq!(vector.variables[0].name, "dados");
        assert_eq!(vector.variables[0].type_name, "inteiro");
        assert_eq!(vector.variables[0].dimensions[0].lower_bound, Some(2));
        assert_eq!(vector.variables[0].dimensions[0].upper_bound, Some(5));
    }

    #[test]
    fn learning_hint_reports_line_specific_syntax_and_possible_logic_problems() {
        let source =
            "programa {\n funcao inicio() {\n  se verdadeiro {\n   escreva(\"ok\")\n  }\n }\n}";
        let hint = learning_hint(source, 3, Language::Portugol);
        assert!(hint.problems.iter().any(|problem| {
            problem.kind == LearningProblemKind::PossibleLogic
                && problem.severity == LearningSeverity::Warning
                && problem.line == 3
        }));
        let incomplete = learning_hint("programa {\n funcao inicio( {\n}", 2, Language::Portugol);
        assert_eq!(incomplete.topic, LearningTopic::Function);
        assert_eq!(incomplete.name, None);
        assert!(incomplete.parameters.is_empty());
        assert!(
            incomplete
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Syntax)
        );
    }

    #[test]
    fn runtime_diagnostics_are_shown_only_on_their_source_line() {
        let diagnostics = [ExecutionDiagnostic {
            kind: ExecutionDiagnosticKind::Runtime,
            message: "índice fora dos limites".to_owned(),
            line: 2,
            column: 4,
        }];
        let source = "programa {\n inteiro x = 1\n}";
        let selected = learning_hint_with_runtime(source, 2, Language::Portugol, &diagnostics);
        let other_line = learning_hint_with_runtime(source, 3, Language::Portugol, &diagnostics);
        assert!(selected.problems.iter().any(|problem| {
            problem.kind == LearningProblemKind::Runtime
                && problem.message == "índice fora dos limites"
        }));
        assert!(
            !other_line
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Runtime)
        );
    }

    #[test]
    fn reports_only_proven_constant_indexes_outside_declared_bounds() {
        let portugol = "programa {\n funcao inicio() {\n  inteiro valores[3]\n  escreva(valores[3])\n  escreva(valores[2])\n }\n}";
        let out_of_range = learning_hint(portugol, 4, Language::Portugol);
        let declaration = learning_hint(portugol, 3, Language::Portugol);
        let valid = learning_hint(portugol, 5, Language::Portugol);
        assert!(out_of_range.problems.iter().any(|problem| {
            problem.kind == LearningProblemKind::Logic
                && problem.severity == LearningSeverity::Error
        }));
        assert!(
            !valid
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Logic)
        );
        assert!(
            !declaration
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Logic)
        );

        let visualg = "algoritmo \"analise\"\nvar\ndados: vetor[2..5] de inteiro\ninicio\n  escreva(dados[1])\nfimalgoritmo\n";
        let visualg_index = learning_hint(visualg, 5, Language::Visualg);
        assert!(
            visualg_index
                .problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Logic)
        );
    }

    #[test]
    fn type_diagnostics_keep_their_own_learning_category() {
        let diagnostics = [ExecutionDiagnostic {
            kind: ExecutionDiagnosticKind::Type,
            message: "tipos incompatíveis: esperava Inteiro, recebeu Lógico".to_owned(),
            line: 1,
            column: 5,
        }];
        let hint = learning_hint_with_runtime(
            "valor = 1 + verdadeiro",
            1,
            Language::Potigol,
            &diagnostics,
        );
        assert!(
            hint.problems
                .iter()
                .any(|problem| problem.kind == LearningProblemKind::Type)
        );
    }
}

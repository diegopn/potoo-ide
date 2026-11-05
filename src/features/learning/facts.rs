use super::*;

pub(super) struct LearningFacts {
    pub(super) topic: LearningTopic,
    pub(super) name: Option<String>,
    pub(super) return_type: Option<String>,
    pub(super) parameters: Vec<LearningSymbol>,
    pub(super) variables: Vec<LearningSymbol>,
    pub(super) call: Option<SourceCall>,
}

pub(crate) fn source_call(
    source: &str,
    line: usize,
    column: usize,
    receiver: Option<String>,
    member: impl Into<String>,
    library: Option<String>,
) -> SourceCall {
    SourceCall {
        receiver,
        member: member.into(),
        arguments: call_arguments(source, line, column),
        library,
    }
}

fn call_arguments(source: &str, line: usize, column: usize) -> Vec<String> {
    let line_start = source
        .split_inclusive('\n')
        .take(line.saturating_sub(1))
        .map(str::len)
        .sum::<usize>();
    let Some(source_line) = source.lines().nth(line.saturating_sub(1)) else {
        return Vec::new();
    };
    let char_offset = column.saturating_sub(1);
    let byte_offset = source_line
        .char_indices()
        .nth(char_offset)
        .map_or(source_line.len(), |(offset, _)| offset);
    let Some(open_offset) = source.get(line_start + byte_offset..).and_then(|tail| {
        tail.char_indices()
            .find(|(_, character)| *character == '(')
            .map(|(offset, _)| line_start + byte_offset + offset)
    }) else {
        return Vec::new();
    };

    let mut arguments = Vec::new();
    let mut argument_start = open_offset + 1;
    let mut delimiter_depth = 1usize;
    let mut quote = None;
    let mut escaped = false;
    for (relative_offset, character) in source[open_offset + 1..].char_indices() {
        let offset = open_offset + 1 + relative_offset;
        if let Some(quote_character) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == quote_character {
                quote = None;
            }
            continue;
        }
        if matches!(character, '"' | '\'') {
            quote = Some(character);
            continue;
        }
        match character {
            '(' | '[' | '{' => delimiter_depth += 1,
            ')' if delimiter_depth == 1 => {
                let argument = source[argument_start..offset].trim();
                if !argument.is_empty() {
                    arguments.push(argument.to_owned());
                }
                return arguments;
            }
            ')' | ']' | '}' => delimiter_depth = delimiter_depth.saturating_sub(1),
            ',' if delimiter_depth == 1 => {
                arguments.push(source[argument_start..offset].trim().to_owned());
                argument_start = offset + character.len_utf8();
            }
            '#' | '\n' if delimiter_depth == 1 => return arguments,
            _ => {}
        }
    }
    arguments
}

pub(super) fn learning_facts(analysis: SourceLineAnalysis) -> LearningFacts {
    let topic = match analysis.construct {
        SourceConstruct::Program => LearningTopic::Program,
        SourceConstruct::Function => LearningTopic::Function,
        SourceConstruct::Procedure => LearningTopic::Procedure,
        SourceConstruct::VariableDeclaration => LearningTopic::Variable,
        SourceConstruct::Assignment => LearningTopic::Assignment,
        SourceConstruct::Input => LearningTopic::Input,
        SourceConstruct::Output => LearningTopic::Output,
        SourceConstruct::Decision => LearningTopic::Decision,
        SourceConstruct::Loop => LearningTopic::Loop,
        SourceConstruct::Return => LearningTopic::Return,
        SourceConstruct::Call | SourceConstruct::MethodCall => LearningTopic::Call,
        SourceConstruct::Import => LearningTopic::Import,
        SourceConstruct::Unrecognized | SourceConstruct::Other => LearningTopic::Unrecognized,
    };
    LearningFacts {
        topic,
        name: analysis.name,
        return_type: analysis.return_type,
        parameters: analysis
            .parameters
            .into_iter()
            .map(learning_symbol)
            .collect(),
        variables: analysis
            .variables
            .into_iter()
            .map(learning_symbol)
            .collect(),
        call: analysis.call,
    }
}

fn learning_symbol(symbol: SourceSymbol) -> LearningSymbol {
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

pub(super) fn unrecognized_facts() -> LearningFacts {
    LearningFacts {
        topic: LearningTopic::Unrecognized,
        name: None,
        return_type: None,
        parameters: Vec::new(),
        variables: Vec::new(),
        call: None,
    }
}

pub(super) fn call_from_source(source: &str, line: usize, column: usize) -> Option<SourceCall> {
    let source_line = source.lines().nth(line.saturating_sub(1))?;
    let start = source_line
        .char_indices()
        .nth(column.saturating_sub(1))
        .map_or(source_line.len(), |(offset, _)| offset);
    let remaining = source_line.get(start..)?;
    let open = remaining.find('(')?;
    let callee = remaining[..open].split_whitespace().last()?.trim();
    let (receiver, member) = callee
        .rsplit_once('.')
        .map_or((None, callee), |(receiver, member)| {
            (Some(receiver.to_owned()), member)
        });
    Some(source_call(source, line, column, receiver, member, None))
}

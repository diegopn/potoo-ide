use std::ops::Range;

use super::lexer;
use crate::languages::source::{
    SourceIndex, SourceToken, SourceTokenKind, StatementRules, close_open_blocks,
    mask_range_preserving_positions, statement_range as shared_statement_range, token_text,
};

const RULES: StatementRules<'static> = StatementRules {
    // Portugol Studio uses semicolons inside `para` headers, not between statements.
    semicolon_separator: false,
    block_pairs: &[],
    root_pairs: &[],
    multiline_continuations: &[],
    brace_delimited_blocks: true,
    case_sensitive_keywords: true,
};

pub(super) fn statement_range(source: &str, line: usize, column: usize) -> Option<Range<usize>> {
    if has_unsupported_line_ending(source) {
        return None;
    }
    let tokens = lexer::shared_tokens(source);
    shared_statement_range(source, &tokens, line, column, &RULES)
}

pub(super) fn mask_diagnostic_statement(
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    let range = statement_range(source, line, column)?;
    mask_range_preserving_positions(source, range)
}

pub(super) fn debugger_prefix(source: &str, line: usize, column: usize) -> Option<String> {
    // The statement scanner rejects ambiguous statements. Cutting at the original diagnostic
    // coordinate allows earlier same-line statements, while refusing synthetic closure of a loop
    // or initializer could change control flow or execute an incomplete expression. The worker
    // validates the remaining prefix before execution.
    statement_range(source, line, column)?;
    let target = SourceIndex::new(source).line_column_to_byte(line, column)?;
    let prefix = source.get(..target)?;
    let tokens = lexer::shared_tokens(prefix);
    if !unmatched_braces_are_safe_to_close(prefix, &tokens) {
        return None;
    }
    close_open_blocks(prefix, &tokens, &RULES)
}

fn unmatched_braces_are_safe_to_close(source: &str, tokens: &[SourceToken]) -> bool {
    let mut stack = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            SourceTokenKind::Symbol('{') => {
                stack.push(brace_opener_is_safe(source, tokens, index));
            }
            SourceTokenKind::Symbol('}') => {
                if stack.pop().is_none() {
                    return false;
                }
            }
            _ => {}
        }
    }
    stack.into_iter().all(|safe| safe)
}

fn brace_opener_is_safe(source: &str, tokens: &[SourceToken], index: usize) -> bool {
    let Some(previous) = previous_significant_token(tokens, index) else {
        return false;
    };
    let Some(previous_text) = token_text(source, &tokens[previous]) else {
        return false;
    };

    match tokens[previous].kind {
        SourceTokenKind::Word => matches!(previous_text, "programa" | "senao"),
        SourceTokenKind::Symbol(')') => {
            let Some(open_paren) = matching_paren_open(tokens, previous) else {
                return false;
            };
            let Some(header) = previous_significant_token(tokens, open_paren) else {
                return false;
            };
            let Some(header_text) = token_text(source, &tokens[header]) else {
                return false;
            };
            match header_text {
                "se" => true,
                "para" | "enquanto" | "escolha" => false,
                _ => function_header_before(source, tokens, header),
            }
        }
        _ => false,
    }
}

fn function_header_before(source: &str, tokens: &[SourceToken], name_index: usize) -> bool {
    for token in tokens[..name_index].iter().rev() {
        match token.kind {
            SourceTokenKind::Symbol('{' | '}' | ';') => return false,
            SourceTokenKind::Word => {
                if matches!(token_text(source, token), Some("funcao" | "função")) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}

fn previous_significant_token(tokens: &[SourceToken], before: usize) -> Option<usize> {
    (0..before)
        .rev()
        .find(|index| tokens[*index].kind != SourceTokenKind::Newline)
}

fn matching_paren_open(tokens: &[SourceToken], close_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for index in (0..close_index).rev() {
        match tokens[index].kind {
            SourceTokenKind::Symbol(')') => depth += 1,
            SourceTokenKind::Symbol('(') if depth == 0 => return Some(index),
            SourceTokenKind::Symbol('(') => depth -= 1,
            _ => {}
        }
    }
    None
}

fn has_unsupported_line_ending(source: &str) -> bool {
    source.char_indices().any(|(offset, character)| {
        character == '\r' && source.as_bytes().get(offset + 1) != Some(&b'\n')
    })
}

#[cfg(test)]
mod tests {
    use super::{debugger_prefix, mask_diagnostic_statement, statement_range};
    use potoo_portugol_interpreter::parse;

    #[test]
    fn shared_rules_ignore_braces_and_keywords_inside_strings_and_comments() {
        let source = r#"programa {
 funcao inicio() {
  escreva("} senao { // /*")
  /* } funcao { se */
  @
  escreva("DEPOIS")
 }
}"#;

        let prefix = debugger_prefix(source, 5, 3).expect("the source range is safe");
        assert!(prefix.ends_with("}\n}"), "{prefix:?}");
        assert!(parse(&prefix).is_ok(), "{prefix:?}");

        let masked = mask_diagnostic_statement(source, 5, 3).expect("the line can be masked");
        assert!(!masked.contains("\n  @\n"));
        assert!(masked.contains("escreva(\"DEPOIS\")"));
        assert!(parse(&masked).is_ok(), "{masked:?}");
    }

    #[test]
    fn shared_rules_reject_unbalanced_expressions_and_unsupported_bare_carriage_returns() {
        let source = "programa {\n funcao inicio() {\n  escreva(\n  @\n }\n}";
        assert!(statement_range(source, 4, 3).is_none());
        assert!(debugger_prefix(source, 4, 3).is_none());
        assert!(mask_diagnostic_statement(source, 4, 3).is_none());

        let bare_cr = "programa {\r funcao inicio() {\r @\r }\r}";
        assert!(statement_range(bare_cr, 3, 2).is_none());
    }
}

use std::ops::Range;

use super::lexer;
use crate::languages::source::{
    SourceIndex, SourceToken, SourceTokenKind, StatementRules, mask_range_preserving_positions,
    statement_range_with_next_line_continuations, token_text,
};
use potoo_potigol_interpreter::parse;

const NEXT_LINE_CONTINUATIONS: &[&str] = &[
    "+", "-", "*", "/", "^", "=", "<", ">", ":", ".", "(", "[", "e", "ou", "div", "mod", "formato",
];

const STATEMENT_RULES: StatementRules<'static> = StatementRules {
    semicolon_separator: false,
    block_pairs: &[],
    root_pairs: &[],
    multiline_continuations: &[],
    brace_delimited_blocks: false,
    case_sensitive_keywords: true,
};

pub(super) fn statement_range(source: &str, line: usize, column: usize) -> Option<Range<usize>> {
    let tokens = lexer::tokens(source);
    let index = SourceIndex::new(source);
    statement_range_with_next_line_continuations(
        source,
        &tokens,
        line,
        column,
        &STATEMENT_RULES,
        NEXT_LINE_CONTINUATIONS,
        &index,
    )
}

pub(super) fn mask_diagnostic_statement(
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    let range = statement_range(source, line, column)?;
    mask_range_preserving_positions(source, range)
}

pub(crate) fn debugger_prefix(source: &str, line: usize, column: usize) -> Option<String> {
    statement_range(source, line, column)?;
    let index = SourceIndex::new(source);
    let target = index.line_column_to_byte(line, column)?;
    let prefix = source.get(..target)?;
    let tokens = lexer::tokens(prefix);
    let safe_closures = unmatched_safe_conditionals(prefix, &tokens)?;
    let mut executable = prefix.to_owned();
    for _ in 0..safe_closures {
        if !executable.is_empty() && !executable.ends_with('\n') && !executable.ends_with('\r') {
            executable.push('\n');
        }
        executable.push_str("fim\n");
    }
    parse(&executable).ok()?;
    Some(executable)
}

fn unmatched_safe_conditionals(source: &str, tokens: &[SourceToken]) -> Option<usize> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Scope {
        Conditional,
        Unsafe,
    }

    let mut stack = Vec::<Scope>::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.kind != SourceTokenKind::Word {
            continue;
        }
        let Some(word) = token_text(source, token) else {
            return None;
        };
        match word {
            "se" => stack.push(Scope::Conditional),
            "enquanto" | "para" | "escolha" => stack.push(Scope::Unsafe),
            "def" | "tipo" if !has_same_line_equals(source, tokens, index) => {
                stack.push(Scope::Unsafe);
            }
            "fim" => {
                stack.pop();
            }
            _ => {}
        }
    }

    stack
        .iter()
        .all(|scope| *scope == Scope::Conditional)
        .then(|| stack.len())
}

fn has_same_line_equals(source: &str, tokens: &[SourceToken], start: usize) -> bool {
    let line = tokens[start].line;
    tokens[start + 1..]
        .iter()
        .take_while(|token| token.line == line && token.kind != SourceTokenKind::Newline)
        .any(|token| token_text(source, token) == Some("="))
}

#[cfg(test)]
mod tests {
    use super::{debugger_prefix, mask_diagnostic_statement, statement_range};
    use crate::languages::source::SourceIndex;
    use potoo_potigol_interpreter::parse;

    #[test]
    fn statement_range_selects_a_line_statement_and_preserves_unicode_positions() {
        let source = "escreva \"ação\"\n@\nescreva 2";
        let diagnostic = parse(source)
            .expect_err("the middle statement contains an invalid token")
            .remove(0);
        let range = statement_range(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("the invalid statement has a precise range");
        assert_eq!(&source[range], "@", "{diagnostic:?}");

        let masked = mask_diagnostic_statement(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("the invalid statement can be masked");
        assert_eq!(masked.len(), source.len());
        assert!(parse(&masked).is_ok(), "{masked}");
        assert_eq!(
            SourceIndex::new(&masked).byte_to_line_column(source.find("escreva 2").unwrap()),
            Some((3, 1))
        );
    }

    #[test]
    fn debugger_prefix_closes_nested_conditionals_but_rejects_open_loops() {
        let source = "se verdadeiro entao\n  se falso entao\n    escreva 1\n    @\n  fim\nfim";
        let diagnostic = parse(source)
            .expect_err("the selected statement contains an invalid token")
            .remove(0);
        let prefix = debugger_prefix(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("nested conditional blocks can be closed safely");
        assert!(prefix.ends_with("fim\nfim\n"), "{prefix:?}");
        assert!(parse(&prefix).is_ok(), "{prefix:?}");

        let source = "para i de 1 ate 2 faca\n  escreva i\n  @\nfim";
        let diagnostic = parse(source)
            .expect_err("the loop body contains an invalid token")
            .remove(0);
        assert!(
            debugger_prefix(
                source,
                diagnostic.span.start.line,
                diagnostic.span.start.column,
            )
            .is_none()
        );
    }

    #[test]
    fn debugger_prefix_preserves_empty_and_ambiguous_outcomes() {
        let source = "@\nescreva 1";
        let diagnostic = parse(source)
            .expect_err("the first statement contains an invalid token")
            .remove(0);
        assert_eq!(
            debugger_prefix(
                source,
                diagnostic.span.start.line,
                diagnostic.span.start.column,
            ),
            Some(String::new())
        );

        assert!(statement_range("escreva (\n  1", 2, 3).is_none());
    }

    #[test]
    fn statement_range_keeps_parser_supported_leading_operator_continuations_together() {
        let source = "escreva 1\n+ @\nescreva 2";
        let diagnostic = parse(source)
            .expect_err("the continued expression contains an invalid token")
            .remove(0);
        let range = statement_range(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("the continued expression has a safe statement boundary");
        assert_eq!(&source[range.clone()], "escreva 1\n+ @");

        let masked = mask_diagnostic_statement(
            source,
            diagnostic.span.start.line,
            diagnostic.span.start.column,
        )
        .expect("the complete continued expression can be masked");
        assert!(parse(&masked).is_ok(), "{masked:?}");
    }
}

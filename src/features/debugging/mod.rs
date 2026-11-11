use crate::features::execution::{DiagnosticKind, ExecutionDiagnostic};
use crate::languages::{model::Language, profile::DebuggerPrefixAdapter};

pub(crate) fn is_available(language: Language) -> bool {
    language.profile().debugger_prefix.is_some()
}

pub(crate) fn executable_prefix(
    language: Language,
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    resolve_prefix(language.profile().debugger_prefix, source, line, column)
}

pub(crate) fn diagnostic_location(diagnostics: &[ExecutionDiagnostic]) -> Option<(usize, usize)> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.kind != DiagnosticKind::Runtime)
        .map(|diagnostic| (diagnostic.line.max(1), diagnostic.column.max(1)))
        .min()
}

fn resolve_prefix(
    adapter: Option<DebuggerPrefixAdapter>,
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    adapter.and_then(|adapter| adapter(source, line, column))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safe_none(_: &str, _: usize, _: usize) -> Option<String> {
        None
    }

    fn safe_empty(_: &str, _: usize, _: usize) -> Option<String> {
        Some(String::new())
    }

    fn safe_prefix(_: &str, _: usize, _: usize) -> Option<String> {
        Some("prefix".to_owned())
    }

    #[test]
    fn prefix_resolution_preserves_unsupported_empty_and_nonempty_states() {
        assert_eq!(resolve_prefix(None, "source", 1, 1), None);

        assert_eq!(
            resolve_prefix(Some(safe_empty), "source", 1, 1),
            Some(String::new())
        );

        assert_eq!(resolve_prefix(Some(safe_none), "source", 1, 1), None);

        assert_eq!(
            resolve_prefix(Some(safe_prefix), "source", 1, 1),
            Some("prefix".to_owned())
        );
    }

    #[test]
    fn executable_prefix_takes_a_source_coordinate_and_returns_the_safe_option() {
        assert_eq!(
            executable_prefix(Language::Potigol, "@\nescreva 1", 1, 1),
            Some(String::new())
        );
    }

    #[test]
    fn prefix_location_uses_the_earliest_static_diagnostic_and_ignores_runtime_errors() {
        let diagnostics = [
            ExecutionDiagnostic {
                kind: DiagnosticKind::Syntax,
                message: "second".to_owned(),
                line: 4,
                column: 2,
            },
            ExecutionDiagnostic {
                kind: DiagnosticKind::Runtime,
                message: "runtime".to_owned(),
                line: 1,
                column: 1,
            },
            ExecutionDiagnostic {
                kind: DiagnosticKind::Lexical,
                message: "first".to_owned(),
                line: 2,
                column: 7,
            },
        ];

        assert_eq!(diagnostic_location(&diagnostics), Some((2, 7)));
        assert_eq!(diagnostic_location(&[]), None);
    }
}

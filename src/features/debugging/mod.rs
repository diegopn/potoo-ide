use crate::features::execution::{DiagnosticKind, ExecutionDiagnostic};
use crate::languages::{
    model::Language,
    profile::{DebuggerPrefixAdapter, LanguageProfile},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DebuggerPrefixResolution {
    Unsupported,
    Legacy(String),
    Safe(Option<String>),
}

pub(crate) fn is_available(language: Language) -> bool {
    language.profile().debugger_prefix.is_some()
}

pub(crate) fn executable_prefix(
    language: Language,
    source: &str,
    diagnostics: &[ExecutionDiagnostic],
) -> (DebuggerPrefixResolution, (usize, usize)) {
    let profile = language.profile();
    let location = diagnostic_location(profile.debugger_prefix, diagnostics).unwrap_or((1, 1));
    (
        resolve_prefix(profile, source, location.0, location.1),
        location,
    )
}

fn diagnostic_location(
    adapter: Option<DebuggerPrefixAdapter>,
    diagnostics: &[ExecutionDiagnostic],
) -> Option<(usize, usize)> {
    match adapter {
        Some(DebuggerPrefixAdapter::Legacy(_)) => diagnostics
            .iter()
            .find(|diagnostic| diagnostic.kind != DiagnosticKind::Runtime)
            .map(|diagnostic| (diagnostic.line.max(1), diagnostic.column.max(1))),
        Some(DebuggerPrefixAdapter::Safe(_)) => diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.kind != DiagnosticKind::Runtime)
            .map(|diagnostic| (diagnostic.line.max(1), diagnostic.column.max(1)))
            .min(),
        None => None,
    }
}

fn resolve_prefix(
    profile: &LanguageProfile,
    source: &str,
    line: usize,
    column: usize,
) -> DebuggerPrefixResolution {
    match profile.debugger_prefix {
        None => DebuggerPrefixResolution::Unsupported,
        Some(DebuggerPrefixAdapter::Legacy(adapter)) => {
            DebuggerPrefixResolution::Legacy(adapter(source, line, column))
        }
        Some(DebuggerPrefixAdapter::Safe(adapter)) => {
            DebuggerPrefixResolution::Safe(adapter(source, line, column))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::execution::DiagnosticKind;
    use crate::languages::profile::{DebuggerPrefixAdapter, LanguageProfile};

    fn safe_none(_: &str, _: usize, _: usize) -> Option<String> {
        None
    }

    fn safe_empty(_: &str, _: usize, _: usize) -> Option<String> {
        Some(String::new())
    }

    fn safe_prefix(_: &str, _: usize, _: usize) -> Option<String> {
        Some("prefix".to_owned())
    }

    fn legacy_prefix(_: &str, _: usize, _: usize) -> String {
        "prefix".to_owned()
    }

    fn test_profile(adapter: DebuggerPrefixAdapter) -> LanguageProfile {
        let mut profile = Language::Portugol.profile().clone();
        profile.debugger_prefix = Some(adapter);
        profile
    }

    #[test]
    fn prefix_resolution_preserves_unsupported_empty_and_nonempty_states() {
        let unsupported = Language::Portugol.profile().clone();
        let mut unsupported = unsupported;
        unsupported.debugger_prefix = None;
        assert_eq!(
            resolve_prefix(&unsupported, "source", 1, 1),
            DebuggerPrefixResolution::Unsupported
        );

        let empty = test_profile(DebuggerPrefixAdapter::Safe(safe_empty));
        assert_eq!(
            resolve_prefix(&empty, "source", 1, 1),
            DebuggerPrefixResolution::Safe(Some(String::new()))
        );

        let ambiguous = test_profile(DebuggerPrefixAdapter::Safe(safe_none));
        assert_eq!(
            resolve_prefix(&ambiguous, "source", 1, 1),
            DebuggerPrefixResolution::Safe(None)
        );

        let prefix = test_profile(DebuggerPrefixAdapter::Safe(safe_prefix));
        assert_eq!(
            resolve_prefix(&prefix, "source", 1, 1),
            DebuggerPrefixResolution::Safe(Some("prefix".to_owned()))
        );
    }

    #[test]
    fn legacy_keeps_reported_order_while_safe_uses_earliest_error_position() {
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

        assert_eq!(
            diagnostic_location(
                Some(DebuggerPrefixAdapter::Legacy(legacy_prefix)),
                &diagnostics,
            ),
            Some((4, 2))
        );
        assert_eq!(
            diagnostic_location(Some(DebuggerPrefixAdapter::Safe(safe_prefix)), &diagnostics,),
            Some((2, 7))
        );
    }
}

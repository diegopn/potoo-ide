use std::collections::HashMap;

use crate::features::execution::{ExecutionDiagnostic, WorkerIo};
use crate::languages::completion::CompletionAdapter;
use crate::languages::model::{
    Language, ProjectTemplate, SourceDiagnostic, SourceDimension, SourceLineAnalysis,
    SourceLineAnalysisOutcome,
};

pub(crate) type DiagnosticAdapter = fn(&str) -> Vec<SourceDiagnostic>;
pub(crate) type SourceLineCodeAdapter = fn(&str, usize) -> String;
pub(crate) type BlankCommentAdapter = fn(&str, usize) -> bool;
pub(crate) type LiteralMaskAdapter = fn(&str) -> String;
pub(crate) type ArrayBoundsAdapter =
    fn(&str, &dyn Fn() -> bool) -> Option<HashMap<String, Vec<SourceDimension>>>;
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub(crate) enum DebuggerPrefixAdapter {
    Legacy(fn(&str, usize, usize) -> String),
    Safe(fn(&str, usize, usize) -> Option<String>),
}
pub(crate) type ExecutionAdapter = fn(&str, &mut WorkerIo) -> Result<(), Vec<ExecutionDiagnostic>>;
#[derive(Clone, Copy, Debug)]
pub(crate) enum LearningAdapter {
    Legacy(
        fn(
            &str,
            usize,
            &dyn Fn() -> bool,
        ) -> Result<Option<SourceLineAnalysis>, Vec<SourceDiagnostic>>,
    ),
    #[allow(dead_code)]
    Recovering(fn(&str, usize, &dyn Fn() -> bool) -> SourceLineAnalysisOutcome),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct LanguageProfile {
    pub(crate) language: Language,
    pub(crate) key: &'static str,
    pub(crate) display_name: &'static str,
    pub(crate) file_name: &'static str,
    pub(crate) extension: &'static str,
    pub(crate) sample: &'static str,
    pub(crate) keywords: &'static [&'static str],
    pub(crate) diagnostics: DiagnosticAdapter,
    pub(crate) source_line_code: SourceLineCodeAdapter,
    pub(crate) is_blank_or_line_comment: BlankCommentAdapter,
    pub(crate) without_literals_and_comments: LiteralMaskAdapter,
    pub(crate) declared_array_bounds: Option<ArrayBoundsAdapter>,
    pub(crate) debugger_prefix: Option<DebuggerPrefixAdapter>,
    pub(crate) completion: Option<&'static CompletionAdapter>,
    pub(crate) execution: ExecutionAdapter,
    pub(crate) learning: Option<LearningAdapter>,
    pub(crate) default_template: ProjectTemplate,
}

static PROFILES: [LanguageProfile; 6] = [
    LanguageProfile {
        language: Language::Portugol,
        key: "portugol",
        display_name: "Portugol Studio 2.0",
        file_name: "principal.por",
        extension: "por",
        sample: crate::languages::portugol::studio_20::examples::SAMPLE,
        keywords: crate::languages::portugol::studio_20::completion::LANGUAGE_KEYWORDS,
        diagnostics: crate::languages::portugol::studio_20::diagnostics::analyze,
        source_line_code:
            crate::languages::portugol::studio_20::syntax_analysis::views::source_line_code,
        is_blank_or_line_comment:
            crate::languages::portugol::studio_20::syntax_analysis::views::is_blank_or_line_comment,
        without_literals_and_comments:
            crate::languages::portugol::studio_20::syntax_analysis::views::without_literals_and_comments,
        declared_array_bounds: Some(
            crate::languages::portugol::studio_20::learning::profile_declared_array_bounds,
        ),
        debugger_prefix: Some(DebuggerPrefixAdapter::Legacy(
            crate::languages::portugol::studio_20::syntax_analysis::views::debugger_prefix,
        )),
        completion: Some(&crate::languages::portugol::studio_20::completion::ADAPTER),
        execution: crate::languages::portugol::studio_20::runtime::execute,
        learning: Some(LearningAdapter::Legacy(
            crate::languages::portugol::studio_20::learning::profile_adapter,
        )),
        default_template: ProjectTemplate::GPortugol,
    },
    LanguageProfile {
        language: Language::Visualg,
        key: "visualg",
        display_name: "VisuAlg",
        file_name: "principal.alg",
        extension: "alg",
        sample: crate::languages::portugol::visualg::examples::SAMPLE,
        keywords: crate::languages::portugol::visualg::completion::LANGUAGE_KEYWORDS,
        diagnostics: crate::languages::portugol::visualg::diagnostics::analyze,
        source_line_code:
            crate::languages::portugol::visualg::syntax_analysis::views::source_line_code,
        is_blank_or_line_comment:
            crate::languages::portugol::visualg::syntax_analysis::views::is_blank_or_line_comment,
        without_literals_and_comments:
            crate::languages::portugol::visualg::syntax_analysis::views::without_literals_and_comments,
        declared_array_bounds: Some(
            crate::languages::portugol::visualg::learning::profile_declared_array_bounds,
        ),
        debugger_prefix: Some(DebuggerPrefixAdapter::Legacy(
            crate::languages::portugol::visualg::syntax_analysis::views::debugger_prefix,
        )),
        completion: Some(&crate::languages::portugol::visualg::completion::ADAPTER),
        execution: crate::languages::portugol::visualg::runtime::execute,
        learning: Some(LearningAdapter::Legacy(
            crate::languages::portugol::visualg::learning::profile_adapter,
        )),
        default_template: ProjectTemplate::Visualg,
    },
    LanguageProfile {
        language: Language::Potigol,
        key: "potigol",
        display_name: "Potigol",
        file_name: "principal.poti",
        extension: "poti",
        sample: crate::languages::potigol::classic::examples::SAMPLE,
        keywords: crate::languages::potigol::completion::LANGUAGE_KEYWORDS,
        diagnostics: crate::languages::potigol::diagnostics::analyze,
        source_line_code: crate::languages::potigol::syntax_analysis::views::source_line_code,
        is_blank_or_line_comment:
            crate::languages::potigol::syntax_analysis::views::is_blank_or_line_comment,
        without_literals_and_comments:
            crate::languages::potigol::syntax_analysis::views::without_literals_and_comments,
        declared_array_bounds: None,
        debugger_prefix: Some(DebuggerPrefixAdapter::Legacy(
            crate::languages::potigol::syntax_analysis::views::debugger_prefix,
        )),
        completion: Some(&crate::languages::potigol::completion::ADAPTER),
        execution: crate::languages::potigol::runtime::execute,
        learning: Some(LearningAdapter::Legacy(
            crate::languages::potigol::learning::profile_adapter,
        )),
        default_template: ProjectTemplate::PotigolClassic,
    },
    LanguageProfile {
        language: Language::GPortugol,
        key: "gportugol",
        display_name: "G-Portugol",
        file_name: "principal.gpt",
        extension: "gpt",
        sample: crate::languages::portugol::g_portugol::examples::SAMPLE,
        keywords: crate::languages::portugol::g_portugol::completion::LANGUAGE_KEYWORDS,
        diagnostics: crate::languages::portugol::g_portugol::diagnostics::analyze,
        source_line_code:
            crate::languages::portugol::g_portugol::syntax_analysis::views::source_line_code,
        is_blank_or_line_comment:
            crate::languages::portugol::g_portugol::syntax_analysis::views::is_blank_or_line_comment,
        without_literals_and_comments:
            crate::languages::portugol::g_portugol::syntax_analysis::views::without_literals_and_comments,
        declared_array_bounds: None,
        debugger_prefix: Some(DebuggerPrefixAdapter::Legacy(
            crate::languages::portugol::g_portugol::syntax_analysis::views::debugger_prefix,
        )),
        completion: Some(&crate::languages::portugol::g_portugol::completion::ADAPTER),
        execution: crate::languages::portugol::g_portugol::runtime::execute,
        learning: Some(LearningAdapter::Legacy(
            crate::languages::portugol::g_portugol::learning::profile_adapter,
        )),
        default_template: ProjectTemplate::GPortugol,
    },
    LanguageProfile {
        language: Language::Calango,
        key: "calango",
        display_name: "Calango",
        file_name: "principal.clg",
        extension: "clg",
        sample: crate::languages::calango::examples::SAMPLE,
        keywords: crate::languages::calango::completion::LANGUAGE_KEYWORDS,
        diagnostics: crate::languages::calango::diagnostics::analyze,
        source_line_code: crate::languages::calango::syntax_analysis::views::source_line_code,
        is_blank_or_line_comment:
            crate::languages::calango::syntax_analysis::views::is_blank_or_line_comment,
        without_literals_and_comments:
            crate::languages::calango::syntax_analysis::views::without_literals_and_comments,
        declared_array_bounds: Some(crate::languages::calango::learning::profile_declared_array_bounds),
        debugger_prefix: Some(DebuggerPrefixAdapter::Legacy(
            crate::languages::calango::syntax_analysis::views::debugger_prefix,
        )),
        completion: Some(&crate::languages::calango::completion::ADAPTER),
        execution: crate::languages::calango::runtime::execute,
        learning: Some(LearningAdapter::Legacy(
            crate::languages::calango::learning::profile_adapter,
        )),
        default_template: ProjectTemplate::Calango,
    },
    LanguageProfile {
        language: Language::Mapler,
        key: "mapler",
        display_name: "Mapler",
        file_name: "principal.mapler",
        extension: "mapler",
        sample: crate::languages::portugol::mapler::examples::SAMPLE,
        keywords: crate::languages::portugol::mapler::completion::LANGUAGE_KEYWORDS,
        diagnostics: crate::languages::portugol::mapler::diagnostics::analyze,
        source_line_code:
            crate::languages::portugol::mapler::syntax_analysis::views::source_line_code,
        is_blank_or_line_comment:
            crate::languages::portugol::mapler::syntax_analysis::views::is_blank_or_line_comment,
        without_literals_and_comments:
            crate::languages::portugol::mapler::syntax_analysis::views::without_literals_and_comments,
        declared_array_bounds: None,
        debugger_prefix: Some(DebuggerPrefixAdapter::Legacy(
            crate::languages::portugol::mapler::syntax_analysis::views::debugger_prefix,
        )),
        completion: Some(&crate::languages::portugol::mapler::completion::ADAPTER),
        execution: crate::languages::portugol::mapler::runtime::execute,
        learning: Some(LearningAdapter::Legacy(
            crate::languages::portugol::mapler::learning::profile_adapter,
        )),
        default_template: ProjectTemplate::Mapler,
    },
];

pub(crate) fn for_language(language: Language) -> &'static LanguageProfile {
    let profile = match language {
        Language::Portugol => &PROFILES[0],
        Language::Visualg => &PROFILES[1],
        Language::Potigol => &PROFILES[2],
        Language::GPortugol => &PROFILES[3],
        Language::Calango => &PROFILES[4],
        Language::Mapler => &PROFILES[5],
    };
    debug_assert_eq!(profile.language, language);
    profile
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::execution::WorkerIo;
    use crate::languages::model::SourceDiagnostic;

    type Diagnostics = fn(&str) -> Vec<SourceDiagnostic>;
    type SourceLineCode = fn(&str, usize) -> String;
    type BlankComment = fn(&str, usize) -> bool;
    type LiteralMask = fn(&str) -> String;
    type ArrayBounds = fn(&str, &dyn Fn() -> bool) -> Option<HashMap<String, Vec<SourceDimension>>>;
    type DebuggerPrefix = fn(&str, usize, usize) -> String;
    type Execution =
        fn(&str, &mut WorkerIo) -> Result<(), Vec<crate::features::execution::ExecutionDiagnostic>>;
    type Learning =
        fn(
            &str,
            usize,
            &dyn Fn() -> bool,
        )
            -> Result<Option<crate::languages::model::SourceLineAnalysis>, Vec<SourceDiagnostic>>;

    #[test]
    fn language_adapters_point_to_their_dialect_implementations() {
        let expected = [
            (
                Language::Portugol,
                crate::languages::portugol::studio_20::diagnostics::analyze as Diagnostics,
                crate::languages::portugol::studio_20::syntax_analysis::views::source_line_code
                    as SourceLineCode,
                crate::languages::portugol::studio_20::syntax_analysis::views::is_blank_or_line_comment
                    as BlankComment,
                crate::languages::portugol::studio_20::syntax_analysis::views::without_literals_and_comments
                    as LiteralMask,
                Some(
                    crate::languages::portugol::studio_20::learning::profile_declared_array_bounds
                        as ArrayBounds,
                ),
                crate::languages::portugol::studio_20::syntax_analysis::views::debugger_prefix
                    as DebuggerPrefix,
                crate::languages::portugol::studio_20::runtime::execute as Execution,
                &crate::languages::portugol::studio_20::completion::ADAPTER,
                crate::languages::portugol::studio_20::learning::profile_adapter as Learning,
            ),
            (
                Language::Visualg,
                crate::languages::portugol::visualg::diagnostics::analyze,
                crate::languages::portugol::visualg::syntax_analysis::views::source_line_code,
                crate::languages::portugol::visualg::syntax_analysis::views::is_blank_or_line_comment,
                crate::languages::portugol::visualg::syntax_analysis::views::without_literals_and_comments,
                Some(crate::languages::portugol::visualg::learning::profile_declared_array_bounds),
                crate::languages::portugol::visualg::syntax_analysis::views::debugger_prefix,
                crate::languages::portugol::visualg::runtime::execute,
                &crate::languages::portugol::visualg::completion::ADAPTER,
                crate::languages::portugol::visualg::learning::profile_adapter,
            ),
            (
                Language::Potigol,
                crate::languages::potigol::diagnostics::analyze,
                crate::languages::potigol::syntax_analysis::views::source_line_code,
                crate::languages::potigol::syntax_analysis::views::is_blank_or_line_comment,
                crate::languages::potigol::syntax_analysis::views::without_literals_and_comments,
                None,
                crate::languages::potigol::syntax_analysis::views::debugger_prefix,
                crate::languages::potigol::runtime::execute,
                &crate::languages::potigol::completion::ADAPTER,
                crate::languages::potigol::learning::profile_adapter,
            ),
            (
                Language::GPortugol,
                crate::languages::portugol::g_portugol::diagnostics::analyze,
                crate::languages::portugol::g_portugol::syntax_analysis::views::source_line_code,
                crate::languages::portugol::g_portugol::syntax_analysis::views::is_blank_or_line_comment,
                crate::languages::portugol::g_portugol::syntax_analysis::views::without_literals_and_comments,
                None,
                crate::languages::portugol::g_portugol::syntax_analysis::views::debugger_prefix,
                crate::languages::portugol::g_portugol::runtime::execute,
                &crate::languages::portugol::g_portugol::completion::ADAPTER,
                crate::languages::portugol::g_portugol::learning::profile_adapter,
            ),
            (
                Language::Calango,
                crate::languages::calango::diagnostics::analyze,
                crate::languages::calango::syntax_analysis::views::source_line_code,
                crate::languages::calango::syntax_analysis::views::is_blank_or_line_comment,
                crate::languages::calango::syntax_analysis::views::without_literals_and_comments,
                Some(crate::languages::calango::learning::profile_declared_array_bounds),
                crate::languages::calango::syntax_analysis::views::debugger_prefix,
                crate::languages::calango::runtime::execute,
                &crate::languages::calango::completion::ADAPTER,
                crate::languages::calango::learning::profile_adapter,
            ),
            (
                Language::Mapler,
                crate::languages::portugol::mapler::diagnostics::analyze,
                crate::languages::portugol::mapler::syntax_analysis::views::source_line_code,
                crate::languages::portugol::mapler::syntax_analysis::views::is_blank_or_line_comment,
                crate::languages::portugol::mapler::syntax_analysis::views::without_literals_and_comments,
                None,
                crate::languages::portugol::mapler::syntax_analysis::views::debugger_prefix,
                crate::languages::portugol::mapler::runtime::execute,
                &crate::languages::portugol::mapler::completion::ADAPTER,
                crate::languages::portugol::mapler::learning::profile_adapter,
            ),
        ];

        assert_eq!(expected.map(|entry| entry.0), Language::ALL);
        for (
            language,
            diagnostics,
            source_line_code,
            is_blank_or_line_comment,
            without_literals_and_comments,
            declared_array_bounds,
            debugger_prefix,
            execution,
            completion,
            learning,
        ) in expected
        {
            let profile = language.profile();
            assert!(
                std::ptr::fn_addr_eq(profile.diagnostics, diagnostics),
                "{language:?}"
            );
            assert!(
                std::ptr::fn_addr_eq(profile.source_line_code, source_line_code),
                "{language:?}"
            );
            assert!(
                std::ptr::fn_addr_eq(profile.is_blank_or_line_comment, is_blank_or_line_comment),
                "{language:?}"
            );
            assert!(
                std::ptr::fn_addr_eq(
                    profile.without_literals_and_comments,
                    without_literals_and_comments
                ),
                "{language:?}"
            );
            match (profile.declared_array_bounds, declared_array_bounds) {
                (Some(actual), Some(expected)) => {
                    assert!(std::ptr::fn_addr_eq(actual, expected), "{language:?}");
                }
                (None, None) => {}
                _ => panic!("array-bound adapter mismatch for {language:?}"),
            }
            assert!(
                profile
                    .debugger_prefix
                    .is_some_and(|adapter| match adapter {
                        DebuggerPrefixAdapter::Legacy(actual) => {
                            std::ptr::fn_addr_eq(actual, debugger_prefix)
                        }
                        DebuggerPrefixAdapter::Safe(_) => false,
                    }),
                "{language:?}"
            );
            assert!(
                std::ptr::fn_addr_eq(profile.execution, execution),
                "{language:?}"
            );
            assert!(std::ptr::eq(
                profile.completion.expect("completion adapter"),
                completion
            ));
            assert!(profile.learning.is_some_and(|adapter| match adapter {
                LearningAdapter::Legacy(actual) => {
                    std::ptr::fn_addr_eq(actual, learning)
                }
                LearningAdapter::Recovering(_) => false,
            }));
        }

        for (language, has_library_support) in [
            (Language::Portugol, true),
            (Language::Visualg, false),
            (Language::Potigol, true),
            (Language::GPortugol, false),
            (Language::Calango, false),
            (Language::Mapler, false),
        ] {
            let completion = language.profile().completion.expect("completion adapter");
            assert_eq!(
                completion.library_import_context.is_some(),
                has_library_support
            );
            assert_eq!(completion.libraries.is_some(), has_library_support);
            assert_eq!(completion.library_members.is_some(), has_library_support);
        }
    }

    #[test]
    fn every_current_profile_starts_on_the_legacy_adapter_path() {
        let fixtures = [
            (Language::Portugol, "Portugol Studio 2.0"),
            (Language::GPortugol, "G-Portugol"),
            (Language::Mapler, "Mapler"),
            (Language::Visualg, "VisuAlg"),
            (Language::Calango, "Calango"),
            (Language::Potigol, "Potigol Classic"),
            (Language::Potigol, "Potigol Jerimum"),
        ];

        for (language, fixture) in fixtures {
            let profile = language.profile();
            assert!(
                matches!(
                    profile.debugger_prefix,
                    Some(DebuggerPrefixAdapter::Legacy(_))
                ),
                "{fixture} debugger should keep its current adapter until migration"
            );
            assert!(
                matches!(profile.learning, Some(LearningAdapter::Legacy(_))),
                "{fixture} learning should keep its current adapter until migration"
            );
        }
    }
}

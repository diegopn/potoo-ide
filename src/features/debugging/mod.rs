use crate::languages::model::Language;

pub(crate) fn executable_prefix(
    language: Language,
    source: &str,
    line: usize,
    column: usize,
) -> String {
    match language {
        Language::Portugol => {
            crate::languages::portugol::studio_20::syntax_analysis::views::debugger_prefix(
                source, line, column,
            )
        }
        Language::Visualg => {
            crate::languages::portugol::visualg::syntax_analysis::views::debugger_prefix(
                source, line, column,
            )
        }
        Language::GPortugol => {
            crate::languages::portugol::g_portugol::syntax_analysis::views::debugger_prefix(
                source, line, column,
            )
        }
        Language::Calango => {
            crate::languages::calango::syntax_analysis::views::debugger_prefix(source, line, column)
        }
        Language::Mapler => {
            crate::languages::portugol::mapler::syntax_analysis::views::debugger_prefix(
                source, line, column,
            )
        }
        Language::Potigol => {
            crate::languages::potigol::syntax_analysis::views::debugger_prefix(source, line, column)
        }
    }
}

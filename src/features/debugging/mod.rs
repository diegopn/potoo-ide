use crate::languages::model::Language;

pub(crate) fn is_available(language: Language) -> bool {
    language.profile().debugger_prefix.is_some()
}

pub(crate) fn executable_prefix(
    language: Language,
    source: &str,
    line: usize,
    column: usize,
) -> String {
    language
        .profile()
        .debugger_prefix
        .map_or_else(String::new, |prefix| prefix(source, line, column))
}

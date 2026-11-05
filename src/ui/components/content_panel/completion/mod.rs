mod provider;
mod suggestions;

use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompletionKind {
    Keyword,
    Function,
    Library,
    Member,
    Snippet,
    Identifier,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompletionItem {
    pub(crate) label: String,
    pub(crate) insert_text: String,
    pub(crate) detail: String,
    pub(crate) kind: CompletionKind,
    pub(crate) cursor_offset: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CompletionResult {
    pub(crate) items: Vec<CompletionItem>,
    pub(crate) replace_range: Range<usize>,
    pub(crate) append_closing_quote: bool,
}

pub(crate) use provider::Provider;
pub(crate) use suggestions::suggestions;

#[cfg(test)]
mod tests {
    use super::{CompletionKind, suggestions};
    use crate::languages::model::Language;

    #[test]
    fn suggests_jerimum_in_potigol_import_and_uses_character_offsets() {
        let source = "escreva \"Olá\"\nuse \"jer";
        let cursor = source.chars().count();
        let result = suggestions(Language::Potigol, source, cursor, false)
            .expect("an open Potigol import should show library suggestions");

        assert!(result.items.iter().any(|item| item.label == "jerimum"));
        assert_eq!(result.replace_range, cursor - 3..cursor);
    }

    #[test]
    fn suggests_portugol_libraries_after_the_include_keyword() {
        let source = "inclua biblioteca Mat";
        let result = suggestions(Language::Portugol, source, source.chars().count(), false)
            .expect("the Portugol library context should show suggestions");

        assert!(result.items.iter().any(|item| item.label == "Matematica"));
        assert!(
            result
                .items
                .iter()
                .all(|item| item.kind == CompletionKind::Library)
        );
    }

    #[test]
    fn suggests_library_members_after_a_declared_alias() {
        let source = "inclua biblioteca Matematica --> m\nm.po";
        let result = suggestions(Language::Portugol, source, source.chars().count(), false)
            .expect("an imported library alias should show its members");

        assert!(result.items.iter().any(|item| item.label == "potencia"));
        assert!(
            result
                .items
                .iter()
                .all(|item| item.kind == CompletionKind::Member)
        );
    }

    #[test]
    fn suggests_runtime_members_only_after_the_potigol_library_import() {
        let source = "use \"jerimum\"\njogo.in";
        let result = suggestions(Language::Potigol, source, source.chars().count(), false)
            .expect("an imported Jerimum module should show its members");

        assert!(result.items.iter().any(|item| item.label == "iniciar"));
        assert!(
            result
                .items
                .iter()
                .all(|item| item.kind == CompletionKind::Member)
        );
    }

    #[test]
    fn suggests_default_functions_without_an_import_directive() {
        let source = "ab";
        let result = suggestions(Language::GPortugol, source, source.chars().count(), false)
            .expect("G-Portugol base.gpt functions are available by default");

        assert!(result.items.iter().any(|item| item.label == "absoluto"));
        assert!(
            result
                .items
                .iter()
                .all(|item| item.kind == CompletionKind::Function)
        );
    }

    #[test]
    fn does_not_suggest_an_unsupported_visualg_library_import() {
        let source = "inclua biblioteca ";
        assert!(suggestions(Language::Visualg, source, source.chars().count(), false).is_none());
    }

    #[test]
    fn does_not_suggest_inside_a_potigol_comment() {
        let source = "# escreva";
        assert!(suggestions(Language::Potigol, source, source.chars().count(), true).is_none());
    }

    #[test]
    fn does_not_suggest_inside_an_ordinary_string() {
        let source = "escreva \"escreva\"";
        let cursor = source.chars().count() - 1;
        assert!(suggestions(Language::Potigol, source, cursor, true).is_none());
    }

    #[test]
    fn offers_a_language_specific_snippet_when_opened_manually() {
        let result = suggestions(Language::Portugol, "", 0, true)
            .expect("manual completion should offer snippets without a prefix");

        assert!(
            result
                .items
                .iter()
                .any(|item| item.label == "programa · estrutura inicial")
        );
    }
}

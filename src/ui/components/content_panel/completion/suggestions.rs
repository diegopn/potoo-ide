use super::*;
use crate::languages::{completion as language_completion, model::Language};

const SNIPPET_CURSOR: &str = "⟪cursor⟫";

pub(crate) fn suggestions(
    language: Language,
    source: &str,
    cursor_offset: usize,
    manual: bool,
) -> Option<CompletionResult> {
    let chars: Vec<char> = source.chars().collect();
    let cursor = cursor_offset.min(chars.len());
    let lexical_context = language_completion::cursor_lexical_context(language, source, cursor);
    if lexical_context.in_comment {
        return None;
    }

    let (word_start, prefix) = language_completion::current_word(&chars, cursor);
    let mut replace_range = word_start..cursor;
    let mut candidates = Vec::new();

    if let Some(context) = language_completion::library_import_context(language, &chars, cursor) {
        replace_range = context.start..cursor;
        let detail = match language {
            Language::Potigol => "Biblioteca Potigol",
            _ => "Biblioteca Portugol 2.0",
        };
        for library in language_completion::libraries(language) {
            push_item(
                &mut candidates,
                library,
                library,
                CompletionKind::Library,
                detail,
                None,
            );
        }
        return filtered_result(
            candidates,
            context.prefix,
            replace_range,
            context.needs_quote,
            true,
        );
    }

    if lexical_context.in_string {
        return None;
    }

    if let Some((owner, member_prefix)) = language_completion::dot_context(&chars, cursor) {
        let members = language_completion::library_members(language, source, cursor, &owner);
        if !members.is_empty() {
            for member in members {
                push_item(
                    &mut candidates,
                    member,
                    member,
                    CompletionKind::Member,
                    &format!("Membro de {owner}"),
                    None,
                );
            }
            return filtered_result(candidates, member_prefix, replace_range, false, true);
        }
    }

    if !manual && prefix.chars().count() < 2 {
        return None;
    }

    let catalog = language_completion::catalog(language);
    for keyword in language.keywords().iter().chain(catalog.keywords.iter()) {
        if catalog
            .snippets
            .iter()
            .any(|snippet| snippet.label.eq_ignore_ascii_case(*keyword))
        {
            continue;
        }
        let kind = if catalog.functions.contains(keyword) {
            CompletionKind::Function
        } else {
            CompletionKind::Keyword
        };
        push_item(
            &mut candidates,
            keyword,
            keyword,
            kind,
            description_for_kind(kind),
            None,
        );
    }
    for function in catalog.functions {
        if catalog
            .snippets
            .iter()
            .any(|snippet| snippet.label.eq_ignore_ascii_case(*function))
        {
            continue;
        }
        push_item(
            &mut candidates,
            function,
            function,
            CompletionKind::Function,
            "Função disponível nesta linguagem",
            None,
        );
    }
    for snippet in catalog.snippets {
        let Some(marker) = snippet.text.find(SNIPPET_CURSOR) else {
            continue;
        };
        let insertion = snippet.text.replace(SNIPPET_CURSOR, "");
        push_item(
            &mut candidates,
            snippet.label,
            &insertion,
            CompletionKind::Snippet,
            "Snippet de código",
            Some(snippet.text[..marker].chars().count()),
        );
    }
    for identifier in language_completion::declared_identifiers(language, source) {
        push_item(
            &mut candidates,
            &identifier,
            &identifier,
            CompletionKind::Identifier,
            "Declarado neste arquivo",
            None,
        );
    }

    filtered_result(candidates, prefix, replace_range, false, manual)
}

fn filtered_result(
    candidates: Vec<CompletionItem>,
    prefix: String,
    replace_range: Range<usize>,
    append_closing_quote: bool,
    contextual: bool,
) -> Option<CompletionResult> {
    let folded_prefix = prefix.to_lowercase();
    let items: Vec<_> = candidates
        .into_iter()
        .filter(|item| item.label.to_lowercase().starts_with(&folded_prefix))
        .collect();
    if items.is_empty() || (!contextual && folded_prefix.chars().count() < 2) {
        return None;
    }
    Some(CompletionResult {
        items,
        replace_range,
        append_closing_quote,
    })
}

fn push_item(
    items: &mut Vec<CompletionItem>,
    label: &str,
    insert_text: &str,
    kind: CompletionKind,
    detail: &str,
    cursor_offset: Option<usize>,
) {
    if items.iter().any(|item| item.label == label) {
        return;
    }
    items.push(CompletionItem {
        label: label.to_owned(),
        insert_text: insert_text.to_owned(),
        detail: detail.to_owned(),
        kind,
        cursor_offset,
    });
}

fn description_for_kind(kind: CompletionKind) -> &'static str {
    match kind {
        CompletionKind::Keyword => "Palavra-chave",
        CompletionKind::Function => "Função disponível nesta linguagem",
        CompletionKind::Library => "Biblioteca disponível nesta linguagem",
        CompletionKind::Member => "Membro de biblioteca",
        CompletionKind::Snippet => "Snippet de código",
        CompletionKind::Identifier => "Declarado neste arquivo",
    }
}

use crate::languages::completion::{
    CompletionAdapter, CompletionCatalog, CompletionSyntax, CursorLexicalContext,
    cursor_lexical_context_for, identifier_at_start,
};

pub(crate) const LANGUAGE_KEYWORDS: &[&str] = &[
    "variaveis",
    "inicio",
    "fim",
    "inteiro",
    "real",
    "logico",
    "cadeia",
    "caractere",
    "vetor",
    "de",
    "modulo",
    "escrever",
    "ler",
    "se",
    "entao",
    "senao",
    "enquanto",
    "para",
    "ate",
    "passo",
    "faca",
    "repita",
    "verdadeiro",
    "falso",
    "e",
    "ou",
    "nao",
];

pub(crate) fn catalog() -> CompletionCatalog {
    let (keywords, functions, snippets) = (&[], &[], &[]);
    CompletionCatalog {
        keywords,
        functions,
        snippets,
    }
}

pub(crate) fn cursor_lexical_context(source: &str, cursor: usize) -> CursorLexicalContext {
    cursor_lexical_context_for(source, cursor, CompletionSyntax::default())
}

pub(crate) fn declared_identifiers(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in source.lines() {
        let line = line
            .split("//")
            .next()
            .unwrap_or_default()
            .split('#')
            .next()
            .unwrap_or_default()
            .trim();
        if line.is_empty() {
            continue;
        }
        let declaration = line.split(':').next();
        let Some(declaration) = declaration else {
            continue;
        };
        let name = identifier_at_start(declaration);
        if !name.is_empty() && !names.iter().any(|existing| existing == name) {
            names.push(name.to_owned());
        }
    }
    names
}

pub(crate) static ADAPTER: CompletionAdapter = CompletionAdapter {
    catalog,
    cursor_lexical_context,
    library_import_context: None,
    libraries: None,
    library_members: None,
    declared_identifiers,
};

use crate::languages::completion::{
    CompletionAdapter, CompletionCatalog, CompletionSnippet, CompletionSyntax,
    CursorLexicalContext, cursor_lexical_context_for, identifier_at_start,
};

pub(crate) const LANGUAGE_KEYWORDS: &[&str] = &[
    "algoritmo",
    "variáveis",
    "fim-variáveis",
    "início",
    "fim",
    "função",
    "retorne",
    "se",
    "então",
    "senão",
    "fim-se",
    "enquanto",
    "faça",
    "fim-enquanto",
    "repita",
    "até",
    "para",
    "de",
    "passo",
    "imprima",
    "leia",
    "inteiro",
    "reais",
    "caractere",
    "literais",
    "lógico",
    "verdadeiro",
    "falso",
];

pub(crate) fn catalog() -> CompletionCatalog {
    let (keywords, functions, snippets) = (
        &[
            "variaveis",
            "variáveis",
            "fim-variaveis",
            "fim-variáveis",
            "início",
            "fim",
            "função",
            "inteiro",
            "reais",
            "caractere",
            "literais",
            "lógico",
            "enquanto",
            "repita",
            "até",
            "para",
            "de",
            "passo",
        ],
        &[
            "imprima",
            "leia",
            "absoluto",
            "arredonda",
            "potencia",
            "raiz_quadrada",
        ],
        GPORTUGOL_SNIPPETS,
    );
    CompletionCatalog {
        keywords,
        functions,
        snippets,
    }
}

const GPORTUGOL_SNIPPETS: &[CompletionSnippet] = &[
    CompletionSnippet {
        label: "algoritmo · estrutura inicial",
        text: "algoritmo exemplo;\ninício\n  imprima(\"⟪cursor⟫\");\nfim",
    },
    CompletionSnippet {
        label: "se · condição",
        text: "se ⟪cursor⟫ então\n  \nsenão\n  \nfim-se",
    },
    CompletionSnippet {
        label: "enquanto · laço",
        text: "enquanto ⟪cursor⟫ faça\n  \nfim-enquanto",
    },
    CompletionSnippet {
        label: "imprima",
        text: "imprima(\"⟪cursor⟫\");",
    },
];

pub(crate) fn cursor_lexical_context(source: &str, cursor: usize) -> CursorLexicalContext {
    cursor_lexical_context_for(
        source,
        cursor,
        CompletionSyntax {
            block_comments: true,
            hash_comments: false,
            brace_comments: false,
        },
    )
}

pub(crate) fn declared_identifiers(source: &str) -> Vec<String> {
    let types = &[
        "inteiro",
        "reais",
        "caractere",
        "literais",
        "lógico",
        "logico",
    ][..];
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
        let mut parts = line.split_whitespace();
        let first = parts.next().unwrap_or_default();
        let declaration = if first == "funcao" || first == "função" {
            parts.next()
        } else if types.contains(&first) {
            parts.next()
        } else {
            None
        };
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

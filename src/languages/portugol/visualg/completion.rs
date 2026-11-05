use crate::languages::completion::{
    CompletionCatalog, CompletionSnippet, CompletionSyntax, CursorLexicalContext,
    cursor_lexical_context_for, identifier_at_start,
};

pub(crate) fn catalog() -> CompletionCatalog {
    let (keywords, functions, snippets) = (
        &[
            "algoritmo",
            "var",
            "inicio",
            "fimalgoritmo",
            "inteiro",
            "real",
            "logico",
            "caractere",
            "literal",
            "entao",
            "fimse",
            "enquanto",
            "fimenquanto",
            "faca",
            "para",
            "fimpara",
            "repita",
            "ate",
            "escolha",
            "caso",
            "outrocaso",
            "fimescolha",
            "fimfuncao",
            "fimprocedimento",
            "procedimento",
            "funcao",
            "arquivo",
            "interrompa",
            "aleatorio",
            "timer",
            "pausa",
            "debug",
            "eco",
            "cronometro",
            "limpatela",
            "retorne",
        ],
        &["escreva", "escreval", "leia"],
        VISUALG_SNIPPETS,
    );
    CompletionCatalog {
        keywords,
        functions,
        snippets,
    }
}

const VISUALG_SNIPPETS: &[CompletionSnippet] = &[
    CompletionSnippet {
        label: "algoritmo · estrutura inicial",
        text: "algoritmo \"exemplo\"\nvar\n  ⟪cursor⟫: inteiro\ninicio\n  escreval(\"Olá!\")\nfimalgoritmo",
    },
    CompletionSnippet {
        label: "se · condição",
        text: "se ⟪cursor⟫ entao\n  \nsenao\n  \nfimse",
    },
    CompletionSnippet {
        label: "enquanto · laço",
        text: "enquanto ⟪cursor⟫ faca\n  \nfimenquanto",
    },
    CompletionSnippet {
        label: "escreval",
        text: "escreval(\"⟪cursor⟫\")",
    },
];

pub(crate) fn cursor_lexical_context(source: &str, cursor: usize) -> CursorLexicalContext {
    cursor_lexical_context_for(
        source,
        cursor,
        CompletionSyntax {
            block_comments: false,
            hash_comments: false,
            brace_comments: true,
        },
    )
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
        let declaration = if line.contains(':') {
            line.split(':').next()
        } else {
            line.strip_prefix("funcao ")
                .or_else(|| line.strip_prefix("procedimento "))
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

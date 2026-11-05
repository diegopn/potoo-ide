use crate::languages::completion::{
    CompletionCatalog, CompletionSyntax, CursorLexicalContext, cursor_lexical_context_for,
    identifier_at_start,
};

pub(crate) fn catalog() -> CompletionCatalog {
    let (keywords, functions, snippets) = (
        &[
            "algoritmo",
            "principal",
            "fimPrincipal",
            "inteiro",
            "real",
            "logico",
            "caracter",
            "texto",
            "funcao",
            "procedimento",
            "retorna",
            "se",
            "entao",
            "senaoSe",
            "senao",
            "fimSe",
            "escolha",
            "caso",
            "outroCaso",
            "fimEscolha",
            "enquanto",
            "faca",
            "fimEnquanto",
            "para",
            "de",
            "ate",
            "passo",
            "fimPara",
            "interrompa",
            "verdadeiro",
            "falso",
            "e",
            "ou",
            "nao",
        ],
        &[
            "escreva",
            "escreval",
            "leia",
            "leiaCaracter",
            "limpaTela",
            "abs",
            "aleatorio",
            "asciiCaracter",
            "caracterAscii",
            "caracterTexto",
            "comparaTexto",
            "convCaractTexto",
            "convTextoCaract",
            "copia",
            "exp",
            "maiusculo",
            "maiusculoCaracter",
            "minusculo",
            "minusculoCaracter",
            "pi",
            "raizQuadrada",
            "tamanhoTexto",
        ],
        &[],
    );
    CompletionCatalog {
        keywords,
        functions,
        snippets,
    }
}

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
    let types = &["inteiro", "real", "logico", "caracter", "texto"][..];
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
        let declaration = if types.contains(&first) {
            parts.next()
        } else if first == "funcao" || first == "procedimento" {
            parts.find(|part| !types.contains(part))
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

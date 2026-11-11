use crate::languages::completion::{
    CompletionAdapter, CompletionCatalog, CompletionSnippet, CompletionSyntax,
    CursorLexicalContext, LibraryImportContext, cursor_lexical_context_for, identifier_at_start,
    line_bounds,
};

pub(crate) const LANGUAGE_KEYWORDS: &[&str] = &[
    "def",
    "var",
    "tipo",
    "escreva",
    "imprima",
    "retorne",
    "enquanto",
    "para",
    "faça",
    "faca",
    "de",
    "até",
    "ate",
    "passo",
    "em",
    "se",
    "então",
    "entao",
    "senãose",
    "senaose",
    "senão",
    "senao",
    "escolha",
    "caso",
    "gere",
    "fim",
    "verdadeiro",
    "falso",
    "e",
    "ou",
    "não",
    "nao",
    "div",
    "mod",
];

pub(crate) fn catalog() -> CompletionCatalog {
    let (keywords, functions, snippets) = (
        &[
            "def", "const", "Nada", "Inteiro", "Real", "Texto", "Lógico", "Logico", "Lista",
            "Matriz", "Cubo", "faça", "até", "senãose", "abstrato",
        ],
        &[
            "escreva",
            "imprima",
            "leia_texto",
            "leia_inteiro",
            "leia_real",
            "leia_logico",
            "leia_caracter",
            "abs",
            "sqrt",
            "sen",
            "cos",
            "tan",
            "tg",
            "arcsen",
            "arccos",
            "arctg",
            "raiz",
            "leia_numero",
            "leia_número",
            "leia_inteiros",
            "leia_reais",
            "leia_numeros",
            "leia_números",
            "leia_textos",
            "eof",
            "PI",
            "min",
            "menor",
            "max",
            "maior",
            "log",
            "log10",
        ],
        POTIGOL_SNIPPETS,
    );
    CompletionCatalog {
        keywords,
        functions,
        snippets,
    }
}

const POTIGOL_LIBRARIES: &[&str] = &["jerimum"];

const POTIGOL_SNIPPETS: &[CompletionSnippet] = &[
    CompletionSnippet {
        label: "escreva",
        text: "escreva \"⟪cursor⟫\"",
    },
    CompletionSnippet {
        label: "se · condição",
        text: "se ⟪cursor⟫ então\n  escreva \"sim\"\nsenão\n  escreva \"não\"\nfim",
    },
    CompletionSnippet {
        label: "def · função",
        text: "def nome: Nada\n  ⟪cursor⟫\nfim",
    },
    CompletionSnippet {
        label: "use · biblioteca",
        text: "use \"⟪cursor⟫\"",
    },
];

pub(crate) fn cursor_lexical_context(source: &str, cursor: usize) -> CursorLexicalContext {
    cursor_lexical_context_for(
        source,
        cursor,
        CompletionSyntax {
            block_comments: false,
            hash_comments: true,
            brace_comments: false,
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
        let declaration = line
            .strip_prefix("var ")
            .or_else(|| line.strip_prefix("const "))
            .or_else(|| line.strip_prefix("def "));
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

pub(crate) fn library_import_context(
    chars: &[char],
    cursor: usize,
) -> Option<LibraryImportContext> {
    let (start, prefix, needs_quote) = potigol_library_context(chars, cursor)?;
    Some(LibraryImportContext {
        start,
        prefix,
        needs_quote,
    })
}

pub(crate) fn library_members(source: &str, cursor: usize, owner: &str) -> &'static [&'static str] {
    if potigol_jerimum_imported(source, cursor) {
        potigol_members(owner)
    } else {
        &[]
    }
}

pub(crate) fn libraries() -> &'static [&'static str] {
    POTIGOL_LIBRARIES
}

fn potigol_library_context(chars: &[char], cursor: usize) -> Option<(usize, String, bool)> {
    let (line_start, line_end) = line_bounds(chars, cursor);
    let before: String = chars[line_start..cursor].iter().collect();
    let after: String = chars[cursor..line_end].iter().collect();
    let trimmed = before.trim_start();
    let rest = trimmed.strip_prefix("use")?;
    if !rest.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    let whitespace = rest
        .chars()
        .take_while(|character| character.is_whitespace())
        .count();
    let after_use: String = rest.chars().skip(whitespace).collect();
    if !after_use.starts_with('"') || after_use[1..].contains('"') {
        return None;
    }
    let leading = before
        .chars()
        .take_while(|character| character.is_whitespace())
        .count();
    let quote = line_start + leading + 3 + whitespace;
    let start = quote + 1;
    let prefix: String = chars[start..cursor].iter().collect();
    Some((start, prefix, !after.contains('"')))
}

fn potigol_jerimum_imported(source: &str, cursor: usize) -> bool {
    source
        .chars()
        .take(cursor)
        .collect::<String>()
        .lines()
        .any(|line| line.trim() == "use \"jerimum\"")
}

fn potigol_members(owner: &str) -> &'static [&'static str] {
    match owner.to_lowercase().as_str() {
        "jogo" => &[
            "titulo",
            "largura",
            "altura",
            "fps",
            "iniciar",
            "distância",
            "distancia",
            "projeção_X",
            "projeção_Y",
        ],
        "teclado" => &[
            "TECLA_ESPACO",
            "TECLA_ENTER",
            "TECLA_PARA_CIMA",
            "TECLA_PARA_BAIXO",
            "TECLA_PARA_ESQUERDA",
            "TECLA_PARA_DIREITA",
            "TECLA_W",
            "TECLA_A",
            "TECLA_S",
            "TECLA_D",
        ],
        "mouse" => &["BOTAO_ESQUERDO", "BOTAO_DIREITO", "x", "y"],
        "relógio" | "relogio" => &["agora", "milisegundos"],
        "desenho" => &["incluir", "desenhe"],
        "cor" => &["vermelho", "verde", "azul", "color"],
        "imagem" => &["largura", "altura", "caminho", "buffer", "fatie"],
        "animacao" | "animação" => &["velocidade", "imagens", "imagem"],
        "fonte" => &["tamanho"],
        "som" => &["arquivo", "tocando", "pare"],
        "musica" | "música" => &[
            "arquivo", "tocando", "pausado", "pare", "pause", "continue", "feche",
        ],
        "tela" => &["frame", "canvas"],
        "graphics2d" => &[
            "setColor",
            "getColor",
            "drawLine",
            "drawRect",
            "fillRect",
            "drawOval",
            "fillOval",
            "drawString",
            "drawImage",
        ],
        _ => &[],
    }
}

pub(crate) static ADAPTER: CompletionAdapter = CompletionAdapter {
    catalog,
    cursor_lexical_context,
    library_import_context: Some(library_import_context),
    libraries: Some(libraries),
    library_members: Some(library_members),
    declared_identifiers,
};

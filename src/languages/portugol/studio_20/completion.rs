use crate::languages::completion::{
    CompletionCatalog, CompletionSnippet, CompletionSyntax, CursorLexicalContext,
    LibraryImportContext, current_word, cursor_lexical_context_for, identifier_at_start,
    line_bounds,
};

pub(crate) fn catalog() -> CompletionCatalog {
    let (keywords, functions, snippets) = (
        &[
            "inclua",
            "biblioteca",
            "cadeia",
            "caracter",
            "logico",
            "real",
            "vazio",
            "const",
            "enquanto",
            "faca",
            "para",
            "escolha",
            "caso",
            "pare",
            "retorne",
        ],
        &["escreva", "leia", "limpa"],
        PORTUGOL_SNIPPETS,
    );
    CompletionCatalog {
        keywords,
        functions,
        snippets,
    }
}

const PORTUGOL_LIBRARIES: &[&str] = &[
    "Arquivos",
    "Calendario",
    "Graficos",
    "Internet",
    "Matematica",
    "Mouse",
    "Objetos",
    "ServicosWeb",
    "Sons",
    "Teclado",
    "Texto",
    "Tipos",
    "Util",
];

const PORTUGOL_SNIPPETS: &[CompletionSnippet] = &[
    CompletionSnippet {
        label: "programa · estrutura inicial",
        text: "programa {\n  funcao inicio() {\n    escreva(\"⟪cursor⟫\")\n  }\n}",
    },
    CompletionSnippet {
        label: "se · condição",
        text: "se (⟪cursor⟫) {\n  \n} senao {\n  \n}",
    },
    CompletionSnippet {
        label: "enquanto · laço",
        text: "enquanto (⟪cursor⟫) {\n  \n}",
    },
    CompletionSnippet {
        label: "escreva",
        text: "escreva(\"⟪cursor⟫\")",
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
    let types = &["inteiro", "real", "logico", "caracter", "cadeia"][..];
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

pub(crate) fn library_import_context(
    chars: &[char],
    cursor: usize,
) -> Option<LibraryImportContext> {
    let (start, prefix) = portugol_library_context(chars, cursor)?;
    Some(LibraryImportContext {
        start,
        prefix,
        needs_quote: false,
    })
}

pub(crate) fn library_members(source: &str, cursor: usize, owner: &str) -> &'static [&'static str] {
    portugol_imports(source, cursor)
        .into_iter()
        .find(|(_, alias)| alias.eq_ignore_ascii_case(owner))
        .map(|(library, _)| portugol_library_members(&library))
        .unwrap_or(&[])
}

pub(crate) fn libraries() -> &'static [&'static str] {
    PORTUGOL_LIBRARIES
}

fn portugol_library_context(chars: &[char], cursor: usize) -> Option<(usize, String)> {
    let (line_start, _) = line_bounds(chars, cursor);
    let before: String = chars[line_start..cursor].iter().collect();
    let trimmed = before.trim_start();
    let rest = trimmed.strip_prefix("inclua biblioteca")?;
    if !rest.chars().next().is_some_and(char::is_whitespace) || rest.contains("-->") {
        return None;
    }
    let (word_start, prefix) = current_word(chars, cursor);
    if prefix.is_empty() && !rest.trim().is_empty() {
        return None;
    }
    Some((word_start, prefix))
}

fn portugol_imports(source: &str, cursor: usize) -> Vec<(String, String)> {
    let prefix: String = source.chars().take(cursor).collect();
    prefix
        .lines()
        .filter_map(|line| {
            let line = line.split("//").next().unwrap_or_default().trim();
            let declaration = line.strip_prefix("inclua biblioteca ")?;
            let mut parts = declaration.split_whitespace();
            let library = parts.next()?.trim_end_matches(';');
            if !PORTUGOL_LIBRARIES.contains(&library) {
                return None;
            }
            let remaining: Vec<_> = parts.collect();
            let alias = remaining
                .windows(2)
                .find(|parts| parts[0] == "-->")
                .and_then(|parts| parts.get(1).copied())
                .unwrap_or(library);
            Some((library.to_owned(), alias.trim_end_matches(';').to_owned()))
        })
        .collect()
}

fn portugol_library_members(library: &str) -> &'static [&'static str] {
    match library {
        "Matematica" => &[
            "PI",
            "arredondar",
            "potencia",
            "raiz",
            "logaritmo",
            "maior_numero",
            "menor_numero",
            "seno",
            "cosseno",
            "tangente",
            "valor_absoluto",
        ],
        "Texto" => &[
            "numero_caracteres",
            "caixa_alta",
            "caixa_baixa",
            "substituir",
            "preencher_a_esquerda",
            "obter_caracter",
            "posicao_texto",
            "extrair_subtexto",
        ],
        "Tipos" => &[
            "cadeia_e_real",
            "cadeia_e_logico",
            "cadeia_e_caracter",
            "cadeia_para_caracter",
            "cadeia_para_real",
            "cadeia_para_logico",
            "cadeia_e_inteiro",
            "cadeia_para_inteiro",
            "inteiro_e_caracter",
            "inteiro_para_caracter",
            "inteiro_para_logico",
            "inteiro_para_real",
            "inteiro_para_cadeia",
            "caracter_e_inteiro",
            "caracter_e_logico",
            "caracter_para_cadeia",
            "caracter_para_inteiro",
            "caracter_para_logico",
            "logico_para_cadeia",
            "logico_para_inteiro",
            "logico_para_caracter",
            "real_para_cadeia",
            "real_para_inteiro",
        ],
        "Util" => &[
            "numero_elementos",
            "numero_linhas",
            "numero_colunas",
            "sorteia",
            "aguarde",
            "tempo_decorrido",
            "obter_diretorio_usuario",
        ],
        "Calendario" => &[
            "dia_mes_atual",
            "dia_semana_atual",
            "mes_atual",
            "ano_atual",
            "hora_atual",
            "minuto_atual",
            "segundo_atual",
            "milisegundo_atual",
            "dia_semana_completo",
            "dia_semana_curto",
            "dia_semana_abreviado",
        ],
        "Arquivos" => &[
            "abrir_arquivo",
            "escrever_linha",
            "fechar_arquivo",
            "fim_arquivo",
            "ler_linha",
            "substituir_texto",
            "arquivo_existe",
            "apagar_arquivo",
            "criar_pasta",
            "selecionar_arquivo",
            "listar_pastas",
            "listar_arquivos",
            "listar_arquivos_por_tipo",
        ],
        "Objetos" => &[
            "criar_objeto",
            "criar_objeto_via_json",
            "criar_objeto_via_xml",
            "liberar",
            "liberar_objeto",
            "obter_json",
            "atribuir_propriedade",
            "obter_propriedade_tipo_inteiro",
            "obter_propriedade_tipo_real",
            "obter_propriedade_tipo_logico",
            "obter_propriedade_tipo_caracter",
            "obter_propriedade_tipo_cadeia",
            "contem_propriedade",
            "tipo_propriedade",
        ],
        "Internet" => &[
            "definir_tempo_limite",
            "obter_texto",
            "endereco_disponivel",
            "baixar_imagem",
        ],
        "ServicosWeb" => &[
            "obter_dados",
            "excluir_dados",
            "abrir_conexao",
            "adicionar_parametros",
            "publicar_dados",
            "atualizar_dados",
            "adicionar_cabecalho",
            "fazer_requisicao",
        ],
        "Mouse" => &[
            "botao_pressionado",
            "algum_botao_pressionado",
            "ler_botao",
            "posicao_x",
            "posicao_y",
            "ocultar_cursor",
            "exibir_cursor",
        ],
        "Teclado" => &[
            "tecla_pressionada",
            "alguma_tecla_pressionada",
            "ler_tecla",
            "caracter_tecla",
        ],
        "Sons" => &[
            "carregar_som",
            "liberar_som",
            "pausar_som",
            "interromper_som",
            "definir_volume",
            "reproduzir_som",
            "obter_volume",
        ],
        "Graficos" => &[
            "iniciar_modo_grafico",
            "definir_dimensoes_janela",
            "definir_titulo_janela",
            "fechar_janela",
            "desenhar_texto",
            "desenhar_linha",
            "desenhar_retangulo",
            "desenhar_elipse",
            "desenhar_ponto",
            "desenhar_imagem",
        ],
        _ => &[],
    }
}

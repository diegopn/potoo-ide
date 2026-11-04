use crate::debugging::{before_line, close_blocks};

pub(crate) fn prefix(source: &str, line: usize) -> String {
    close_blocks(
        &before_line(source, line),
        &[
            ("se", "fimse"),
            ("escolha", "fimescolha"),
            ("enquanto", "fimenquanto"),
            ("para", "fimpara"),
            ("repita", "ate verdadeiro"),
            ("procedimento", "fimprocedimento"),
            ("funcao", "fimfuncao"),
        ],
        Some(("algoritmo", "fimalgoritmo")),
    )
}

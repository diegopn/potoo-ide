use crate::debugging::{before_line, close_blocks};

pub(crate) fn prefix(source: &str, line: usize) -> String {
    close_blocks(
        &before_line(source, line),
        &[
            ("se", "fimSe"),
            ("enquanto", "fimEnquanto"),
            ("para", "fimPara"),
            ("procedimento", "fimProcedimento"),
            ("funcao", "fimFuncao"),
        ],
        Some(("principal", "fimPrincipal")),
    )
}

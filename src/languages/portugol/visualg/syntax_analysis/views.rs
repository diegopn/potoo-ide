use super::structure;

pub(crate) fn mask_diagnostic_statement(
    source: &str,
    line: usize,
    column: usize,
) -> Option<String> {
    structure::mask_diagnostic_statement(source, line, column)
}

pub(crate) fn source_line_code(source: &str, line: usize) -> String {
    super::lexer::source_line_code(source, line)
}

pub(crate) fn without_literals_and_comments(source: &str) -> String {
    super::lexer::without_literals_and_comments(source)
}

pub(crate) fn is_blank_or_line_comment(source: &str, line_number: usize) -> bool {
    source_line_code(source, line_number).trim().is_empty()
}

pub(crate) fn debugger_prefix(source: &str, line: usize, column: usize) -> Option<String> {
    structure::debugger_prefix(source, line, column)
}

#[cfg(test)]
mod tests {
    use super::{is_blank_or_line_comment, source_line_code, without_literals_and_comments};

    #[test]
    fn visualg_views_keep_unsupported_block_comments_and_single_quotes_as_code() {
        let source = "algoritmo \"aula\"\ninicio\n  escreval(\"fimse \"\"// literal\"\"\") /* não é comentário */ // fimse\n'fimse'\nfimalgoritmo";

        let output = source_line_code(source, 3);
        assert!(
            output.contains("\"fimse \"\"// literal\"\"\""),
            "{output:?}"
        );
        assert!(output.contains("/* não é comentário */"), "{output:?}");
        assert_eq!(source_line_code(source, 4), "'fimse'");

        let masked = without_literals_and_comments(source);
        assert!(masked.contains("/* não é comentário */"));
        assert!(
            masked.contains("fimse"),
            "single-quoted text is not a literal here"
        );
    }

    #[test]
    fn visualg_line_views_follow_crlf_and_bare_cr_boundaries() {
        let source = "algoritmo \"aula\"\r\ninicio\r  @ // comentário\rfimalgoritmo";

        assert_eq!(source_line_code(source, 1), "algoritmo \"aula\"");
        assert_eq!(source_line_code(source, 2), "inicio");
        let error_line = source_line_code(source, 3);
        assert!(error_line.starts_with("  @ "), "{error_line:?}");
        assert!(!error_line.contains("comentário"), "{error_line:?}");
        assert_eq!(source_line_code(source, 4), "fimalgoritmo");
        assert!(is_blank_or_line_comment(source, 2) == false);
        assert!(is_blank_or_line_comment("inicio\r// comentário\r", 2));
    }
}

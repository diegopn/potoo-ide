use potoo_gportugol_interpreter::{DiagnosticKind, TokenKind, lex};

#[test]
fn lexes_accented_keywords_and_ascii_identifiers() {
    let tokens = lex("algoritmo teste;\nvariáveis\n x: inteiro;\nfim-variáveis\ninício fim")
        .expect("the documented accented keywords should lex");
    let kinds = tokens.iter().map(|token| token.kind).collect::<Vec<_>>();

    assert_eq!(
        kinds,
        [
            TokenKind::Algorithm,
            TokenKind::Identifier,
            TokenKind::Semicolon,
            TokenKind::Variables,
            TokenKind::Identifier,
            TokenKind::Colon,
            TokenKind::Integer,
            TokenKind::Semicolon,
            TokenKind::EndVariables,
            TokenKind::Start,
            TokenKind::End,
            TokenKind::EndOfFile,
        ]
    );
}

#[test]
fn rejects_accented_identifiers() {
    let diagnostics = lex("algoritmo ação;").expect_err("identifiers must be ASCII");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Lexical);
    assert_eq!(diagnostics[0].span.start.line, 1);
    assert_eq!(diagnostics[0].span.start.column, 11);
}

#[test]
fn reports_unterminated_literals_with_spans() {
    let diagnostics = lex("algoritmo teste;\ninício imprima(\"x)")
        .expect_err("an unterminated string must be rejected");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Lexical);
    assert_eq!(diagnostics[0].span.start.line, 2);
    assert_eq!(diagnostics[0].span.start.column, 16);
}

#[test]
fn tracks_unicode_source_columns() {
    let diagnostics = lex("algoritmo teste;\ninício\n  imprima(\"Olá\"); @\nfim")
        .expect_err("unknown punctuation must have a source location");

    assert_eq!(diagnostics[0].span.start.line, 3);
    assert_eq!(diagnostics[0].span.start.column, 19);
    assert_eq!(diagnostics[0].span.start.byte_offset, 44);
}

#[test]
fn accepts_a_utf8_bom_only_at_the_start_of_a_source() {
    let tokens =
        lex("\u{feff}algoritmo teste; início fim").expect("a leading UTF-8 BOM should be ignored");
    assert_eq!(tokens[0].kind, TokenKind::Algorithm);
    assert_eq!(tokens[0].span.start.column, 1);

    let diagnostics = lex("algoritmo teste; \u{feff}início fim")
        .expect_err("an interior BOM should remain invalid");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Lexical);
}

#[test]
fn matches_gpt_whitespace_rules() {
    for whitespace in [' ', '\t', '\n', '\r', '\u{00a0}'] {
        let source = format!("algoritmo{whitespace}teste; início fim");
        lex(&source).expect("GPT whitespace and ignored NBSP should be accepted");
    }

    for whitespace in ['\u{000b}', '\u{000c}'] {
        let source = format!("algoritmo{whitespace}teste; início fim");
        let diagnostics = lex(&source).expect_err("VT and FF are not GPT whitespace");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Lexical);
    }
}

#[test]
fn rejects_escaped_non_bmp_characters_in_literals() {
    for source in [
        r#"algoritmo teste; início imprima("\😀"); fim"#,
        r#"algoritmo teste; início imprima('\😀'); fim"#,
    ] {
        let diagnostics = lex(source).expect_err("escaped non-BMP characters must be rejected");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Lexical);
    }
}

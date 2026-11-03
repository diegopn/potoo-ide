use potoo_portugol_interpreter::parse;

fn accepts(source: &str) {
    parse(source).unwrap_or_else(|diagnostics| panic!("source should parse: {diagnostics:?}"));
}

fn rejects(source: &str) {
    assert!(
        parse(source).is_err(),
        "source should be rejected: {source}"
    );
}

#[test]
fn whitespace_separates_adjacent_commands_and_operators() {
    accepts("programa { funcao inicio() { inteiro x = 1 escreva(x) } }");
    accepts("programa { funcao inicio() { inteiro x = 2\n+\n3 escreva(x) } }");
}

#[test]
fn function_bodies_and_control_conditions_follow_the_grammar() {
    rejects("programa { funcao inicio() escreva(1) }");
    rejects("programa { funcao inicio() { se verdadeiro {} } }");
    rejects("programa { funcao inicio() { para(; verdadeiro; ) {} } }");
    accepts("programa { funcao inicio() { para(; verdadeiro; 1) {} } }");
}

#[test]
fn declarations_and_includes_are_restricted_to_program_sections() {
    rejects("programa { funcao inicio() {} inclua biblioteca Texto }");
    rejects("programa { escreva(1) funcao inicio() {} }");
}

#[test]
fn statement_semicolons_and_non_ascii_identifiers_are_not_tokens_in_the_language() {
    rejects("programa { funcao inicio() { escreva(1); escreva(2) } }");
    rejects("programa { funcao inicio() { inteiro café = 1 } }");
}

#[test]
fn non_reference_operators_and_uppercase_program_keyword_are_rejected() {
    rejects("programa { funcao inicio() { escreva(1 <> 2) } }");
    rejects("programa { funcao inicio() { escreva(!falso) } }");
    rejects("programa { funcao inicio() { escreva(4 \\ 2) } }");
    rejects("PROGRAMA { funcao inicio() {} }");
}

#[test]
fn lexer_preserves_the_reference_literal_and_number_rules() {
    accepts(r#"programa { funcao inicio() { escreva("escape desconhecido: \q") } }"#);
    accepts("programa { funcao inicio() { escreva(0xFFFFFF, .5, 1.) } }");
    rejects("programa { funcao inicio() { escreva(0xFFFFFFF) } }");
    rejects("programa { funcao inicio() { escreva(2147483648) } }");
}

use potoo_portugol_interpreter::{DiagnosticKind, RuntimeIo, execute, parse};

#[derive(Default)]
struct Output;

impl RuntimeIo for Output {
    fn write(&mut self, _text: &str) {}

    fn read_line(&mut self) -> Result<String, String> {
        Err("unexpected input".to_owned())
    }
}

fn rejects_semantically(source: &str) {
    let mut io = Output;
    let diagnostics = execute(source, &mut io).expect_err("program must fail semantic analysis");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type, "{diagnostics:?}");
}

#[test]
fn parse_remains_syntax_only_while_execution_checks_break_context() {
    let source = "programa { funcao inicio() { pare } }";
    assert!(parse(source).is_ok());
    rejects_semantically(source);
}

#[test]
fn catches_unknown_function_inside_an_unreachable_branch() {
    rejects_semantically("programa { funcao inicio() { se (falso) { funcao_inexistente() } } }");
}

#[test]
fn catches_constant_writes_inside_an_unreachable_branch_before_output() {
    rejects_semantically(
        "programa { const inteiro resposta = 42 funcao inicio() { escreva(resposta) se (falso) { resposta = 43 } } }",
    );
}

#[test]
fn catches_uninitialized_local_reads() {
    rejects_semantically("programa { funcao inicio() { inteiro resposta escreva(resposta) } }");
}

#[test]
fn catches_non_void_functions_with_a_missing_return_path() {
    rejects_semantically(
        "programa { funcao inteiro valor() { se (verdadeiro) { retorne 1 } } funcao inicio() {} }",
    );
}

#[test]
fn catches_duplicate_local_variables_and_invalid_dimensions() {
    rejects_semantically("programa { funcao inicio() { inteiro valor inteiro valor } }");
    rejects_semantically("programa { funcao inicio() { inteiro valores[0] } }");
}

#[test]
fn catches_unknown_libraries_and_builtin_functions_in_dead_code() {
    rejects_semantically("programa { inclua biblioteca Inexistente funcao inicio() {} }");
    rejects_semantically(
        "programa { inclua biblioteca Texto funcao inicio() { se (falso) { Texto.funcao_inexistente() } } }",
    );
}

#[test]
fn rejects_incompatible_reference_arguments_statically() {
    rejects_semantically(
        "programa { funcao vazio alterar(inteiro &valor) { valor = 1 } funcao inicio() { real valor = 0 alterar(valor) } }",
    );
}

#[test]
fn uppercase_type_spelling_is_an_undeclared_identifier() {
    rejects_semantically("programa { funcao inicio() { Inteiro valor = 1 } }");
}

#[test]
fn void_variables_and_parameters_are_rejected_by_semantic_analysis() {
    let variable = "programa { funcao inicio() { vazio valor } }";
    assert!(parse(variable).is_ok());
    rejects_semantically(variable);
    rejects_semantically("programa { funcao alterar(vazio valor) {} funcao inicio() {} }");
}

#[test]
fn library_modules_functions_constants_and_builtins_are_case_sensitive() {
    rejects_semantically("programa { inclua biblioteca texto funcao inicio() {} }");
    rejects_semantically(
        "programa { inclua biblioteca Texto --> t funcao inicio() { escreva(T.numero_caracteres(\"x\")) } }",
    );
    rejects_semantically(
        "programa { inclua biblioteca Texto funcao inicio() { escreva(Texto.NUMERO_CARACTERES(\"x\")) } }",
    );
    rejects_semantically(
        "programa { inclua biblioteca Matematica funcao inicio() { escreva(Matematica.pi) } }",
    );
    rejects_semantically(
        "programa { inclua biblioteca Teclado funcao inicio() { escreva(Teclado.tecla_enter) } }",
    );
    rejects_semantically(
        "programa { inclua biblioteca Graficos funcao inicio() { escreva(Graficos.cor_preto) } }",
    );
    rejects_semantically(
        "programa { inclua biblioteca Graficos funcao inicio() { escreva(Graficos.obter_rgb(0, 0)) } }",
    );
    rejects_semantically("programa { funcao inicio() { ESCREVA(1) } }");
}

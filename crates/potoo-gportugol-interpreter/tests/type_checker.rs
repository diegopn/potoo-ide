use potoo_gportugol_interpreter::{DiagnosticKind, check, parse};

fn check_source(source: &str) -> Result<(), Vec<potoo_gportugol_interpreter::Diagnostic>> {
    let program = parse(source).expect("test program should parse");
    check(&program)
}

#[test]
fn accepts_documented_numeric_compatibility() {
    check_source(
        "algoritmo tipos;\n\
         variáveis\n\
           n: inteiro; r: real; c: caractere; l: lógico;\n\
         fim-variáveis\n\
         início\n\
           n := r; c := n; l := c; n := l;\n\
         fim",
    )
    .expect("integer, real, character, and logical are numerically compatible");
}

#[test]
fn accepts_mixed_numeric_operands() {
    check_source(
        "algoritmo misto;\n\
         variáveis\n\
           numero: inteiro; decimal: real;\n\
         fim-variáveis\n\
         início\n\
           decimal := numero + 1.5;\n\
           decimal := 5 / 2;\n\
           numero := 7 % 2;\n\
         fim",
    )
    .expect("mixed numeric expressions should type check");
}

#[test]
fn checks_assignment_conversion_and_leia_target() {
    check_source(
        "algoritmo entrada;\n\
         variáveis\n\
           n: inteiro; flag: lógico; valores: matriz[2] de inteiros;\n\
         fim-variáveis\n\
         início\n\
           n := leia();\n\
           valores[leia()] := leia();\n\
           se leia() então n := 1; fim-se\n\
         fim",
    )
    .expect("leia should inherit its scalar target type");
}

#[test]
fn rejects_incompatible_function_arguments_and_returns() {
    let argument_diagnostics = check_source(
        "algoritmo tipos_invalidos;\n\
         início\n\
           imprima(converte(\"texto\"));\n\
         fim\n\
         função converte(valor: inteiro): inteiro\n\
         início\n\
           retorne valor;\n\
         fim",
    )
    .expect_err("incompatible function arguments must be rejected");

    let return_diagnostics = check_source(
        "algoritmo retorno_invalido; início fim\n\
         função converte(valor: inteiro): inteiro\n\
         início retorne \"texto\"; fim",
    )
    .expect_err("incompatible function returns must be rejected");

    assert_eq!(argument_diagnostics[0].kind, DiagnosticKind::Type);
    assert_eq!(return_diagnostics[0].kind, DiagnosticKind::Type);
}

#[test]
fn allows_void_calls_as_statements_but_not_as_values() {
    check_source(
        "algoritmo chamadas; início exibir(); fim\n\
         função exibir() início imprima(\"ok\"); fim",
    )
    .expect("a procedure can be called as a statement");

    let diagnostics = check_source(
        "algoritmo chamada_sem_valor;\n\
         variáveis\n\
           valor: inteiro;\n\
         fim-variáveis\n\
         início valor := exibir(); fim\n\
         função exibir() início retorne; fim",
    )
    .expect_err("a procedure cannot be used where a value is required");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
}

#[test]
fn logical_operators_require_compatible_operands() {
    for source in [
        "algoritmo incompativel; início imprima(\"texto\" e 1); fim",
        "algoritmo incompativel; início imprima(0 ou \"texto\"); fim",
    ] {
        let diagnostics = check_source(source)
            .expect_err("logical operators must reject mixed literal and numeric operands");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    }

    check_source("algoritmo compativel; início imprima(\"a\" e \"b\", 1 ou 0); fim")
        .expect("same-category logical operands remain valid");
}

#[test]
fn bitwise_not_rejects_literal_operands() {
    let diagnostics = check_source("algoritmo invalido; início imprima(~\"texto\"); fim")
        .expect_err("bitwise not requires a numeric operand");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    check_source("algoritmo valido; início imprima(~1); fim")
        .expect("integer bitwise not remains valid");
}

#[test]
fn matrix_indexes_accept_character_and_logical_but_reject_real_and_literal() {
    check_source(
        "algoritmo indices; variáveis a: matriz[2] de inteiros; flag: lógico; c: caractere; fim-variáveis início imprima(a[flag], a[c]); fim",
    )
    .expect("character and logical values are valid integer-compatible indexes");

    for source in [
        "algoritmo real_idx; variáveis a: matriz[2] de inteiros; r: real; fim-variáveis início imprima(a[r]); fim",
        "algoritmo literal_idx; variáveis a: matriz[2] de inteiros; fim-variáveis início imprima(a[\"texto\"]); fim",
    ] {
        let diagnostics = check_source(source).expect_err("real and literal indexes are invalid");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    }
}

#[test]
fn main_return_requires_an_exact_integer_value() {
    for value in ["1.5", "verdadeiro", "'a'", "leia()"] {
        let source = format!("algoritmo retorno; início retorne {value}; fim");
        let diagnostics =
            check_source(&source).expect_err("the main block return type must be exactly integer");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    }
}

#[test]
fn leia_cannot_be_an_unary_or_binary_operand() {
    for source in [
        "algoritmo unary; início imprima(não leia()); fim",
        "algoritmo binary; início imprima(leia() = \"texto\"); fim",
        "algoritmo binary; início imprima(leia() = leia()); fim",
    ] {
        let diagnostics = check_source(source)
            .expect_err("leia's untyped result cannot be used as an operator operand");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    }

    check_source("algoritmo contextual; variáveis valor: inteiro; fim-variáveis início valor := leia(); imprima(leia()); fim")
        .expect("leia remains valid as a contextual value and a standalone argument");
}

#[test]
fn static_numeric_promotion_preserves_character_and_logical_types() {
    for expression in [
        "verdadeiro + falso",
        "'a' + 'b'",
        "verdadeiro % falso",
        "'a' & 'b'",
        "~verdadeiro",
    ] {
        let source = format!(
            "algoritmo promocao; variáveis i: inteiro; fim-variáveis início para i de {expression} até 2 faça fim-para fim"
        );
        let diagnostics = check_source(&source)
            .expect_err("the loop bounds must retain the expression's static primitive type");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    }
}

#[test]
fn array_parameters_accept_numeric_elements_with_matching_dimensions() {
    check_source(
        "algoritmo arrays_compat; variáveis valores: matriz[2] de inteiros; fim-variáveis início recebe(valores); fim função recebe(itens: matriz[2] de reais) início imprima(itens[0]); fim",
    )
    .expect("array parameters may differ by compatible numeric element type");

    let diagnostics = check_source(
        "algoritmo arrays_dims; variáveis valores: matriz[2] de inteiros; fim-variáveis início recebe(valores); fim função recebe(itens: matriz[3] de reais) início fim",
    )
    .expect_err("array parameter dimensions must still match exactly");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
}

#[test]
fn checked_array_access_requires_every_dimension() {
    let diagnostics = check_source(
        "algoritmo parcial; variáveis grade: matriz[2][2] de inteiros; fim-variáveis início recebe(grade[0]); fim função recebe(linha: matriz[2] de inteiros) início fim",
    )
    .expect_err("partial-rank array access must be rejected");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    check_source(
        "algoritmo completo; variáveis grade: matriz[2][2] de inteiros; fim-variáveis início imprima(grade[0][0]); fim",
    )
    .expect("fully indexed array access remains valid");
}

#[test]
fn globals_cannot_collide_with_builtin_names_but_locals_can() {
    for name in [
        "leia",
        "imprima",
        "absoluto",
        "arredonda",
        "potencia",
        "raiz_quadrada",
    ] {
        let source =
            format!("algoritmo colisao; variáveis {name}: inteiro; fim-variáveis início fim");
        let diagnostics =
            check_source(&source).expect_err("a global must not replace a built-in function name");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    }

    check_source(
        "algoritmo local; início usa(1); fim função usa(leia: inteiro) início leia := leia + 1; fim",
    )
    .expect("a local variable may still use a built-in function's name");
}

#[test]
fn collects_diagnostics_from_independent_main_and_function_statements() {
    let diagnostics = check_source(
        "algoritmo diagnosticos; início ausente := 1; fim função retorna_inteiro(): inteiro início retorne \"texto\"; fim",
    )
    .expect_err("independent errors in main and function bodies should both be reported");

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics[0].message.contains("is not declared"));
    assert!(
        diagnostics[1]
            .message
            .contains("return value must be compatible")
    );
}

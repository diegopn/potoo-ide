use potoo_gportugol_interpreter::{
    ExecutionOptions, InputPrompt, RuntimeIo, execute, execute_with_sources,
    execute_with_sources_and_options,
};

#[derive(Default)]
struct Io {
    output: String,
}

impl RuntimeIo for Io {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, _prompt: &InputPrompt) -> Result<String, String> {
        Err("unexpected input".to_owned())
    }
}

#[test]
fn default_math_library_matches_gpt_1_4_examples() {
    let mut io = Io::default();
    let source = "algoritmo biblioteca;\n\
      variáveis abs_result, arredondado, arredondado_negativo, power_result, raiz: real; fim-variáveis\n\
      início\n\
        abs_result := absoluto(-3.5);\n\
        arredondado := arredonda(2.5);\n\
        arredondado_negativo := arredonda(-2.5);\n\
        power_result := potencia(2.0, 3.0);\n\
        raiz := raiz_quadrada(81.0);\n\
        imprima(abs_result, \"|\", arredondado, \"|\", arredondado_negativo, \"|\", power_result, \"|\", raiz);\n\
      fim";

    execute(source, &mut io).expect("the bundled default math library should be available");

    assert_eq!(io.output, "3.50|3.50|3.50|8.00|9.00\n");
}

#[test]
fn math_library_preserves_base_gpt_edge_cases_and_rounds_each_power_product() {
    let mut io = Io::default();
    let source = "algoritmo math_edges;\n\
      variáveis negative_fraction, rounded_power, fractional_root: real; fim-variáveis\n\
      início\n\
        negative_fraction := potencia(2.0, -1.5);\n\
        rounded_power := potencia(3.14159, 6.0);\n\
        fractional_root := raiz_quadrada(1.5);\n\
        imprima(negative_fraction, \"|\", rounded_power, \"|\", fractional_root);\n\
      fim";

    execute(source, &mut io).expect("the bundled math functions should follow base.gpt");

    assert_eq!(io.output, "1.00|961.39|0.00\n");
}

#[test]
fn typed_function_fallthrough_reuses_the_previous_return_value() {
    let mut io = Io::default();
    let source = "algoritmo retorno_residual; início imprima(previous(), \"|\", falls_through()); fim\n\
      função previous(): inteiro início retorne 7; fim\n\
      função falls_through(): inteiro início fim";

    execute(source, &mut io).expect("typed functions may fall through");

    assert_eq!(io.output, "7|7\n");
}

#[test]
fn user_functions_cannot_shadow_the_bundled_default_math_library() {
    let source = "algoritmo nomes; início fim\n\
      função absoluto(valor: real): real início retorne valor; fim";

    let diagnostics = execute(source, &mut Io::default())
        .expect_err("the bundled standard function name is already taken");

    assert!(diagnostics[0].message.contains("already declared"));
}

#[test]
fn executes_a_main_file_with_functions_from_additional_source_files() {
    let main = "algoritmo projeto; início imprima(soma(20, produto(3, 4))); fim";
    let libraries = [
        "função soma(a: inteiro, b: inteiro): inteiro início retorne a + b; fim",
        "função produto(a: inteiro, b: inteiro): inteiro início retorne a * b; fim",
    ];
    let mut io = Io::default();

    execute_with_sources(main, &libraries, &mut io)
        .expect("the runtime should compose function-only source files with the main file");

    assert_eq!(io.output, "32\n");
}

#[test]
fn rejects_an_additional_source_that_declares_another_algorithm() {
    let diagnostics = execute_with_sources(
        "algoritmo principal; início fim",
        &["algoritmo outro; início fim"],
        &mut Io::default(),
    )
    .expect_err("additional GPT source files must contain declarations, not another main block");

    assert_eq!(
        diagnostics[0].kind,
        potoo_gportugol_interpreter::DiagnosticKind::Syntax
    );
}

#[test]
fn applies_execution_options_to_composed_sources() {
    let diagnostics = execute_with_sources_and_options(
        "algoritmo limitado; início imprima(valor()); fim",
        &["função valor(): inteiro início retorne 1; fim"],
        &mut Io::default(),
        ExecutionOptions {
            max_steps: 0,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("composed programs must honor the caller's resource limits");

    assert_eq!(
        diagnostics[0].kind,
        potoo_gportugol_interpreter::DiagnosticKind::Runtime
    );
    assert!(diagnostics[0].message.contains("step limit"));
}

#[test]
fn returns_the_integer_from_the_main_block() {
    let code = execute(
        "algoritmo retorno; início retorne 37; fim",
        &mut Io::default(),
    )
    .expect("the main block should return its integer status");

    assert_eq!(code, Some(37));
}

use std::collections::VecDeque;

use potoo_gportugol_interpreter::{
    DataType, DiagnosticKind, ExecutionOptions, InputPrompt, RuntimeIo, execute,
    execute_with_options,
};

#[derive(Default)]
struct TestIo {
    inputs: VecDeque<String>,
    output: String,
    prompt_types: Vec<DataType>,
    stop_after_checks: Option<usize>,
    stop_checks: usize,
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String> {
        self.prompt_types.push(prompt.data_type);
        self.inputs
            .pop_front()
            .ok_or_else(|| "no test input".to_owned())
    }

    fn should_stop(&mut self) -> bool {
        self.stop_checks += 1;
        self.stop_after_checks
            .is_some_and(|limit| self.stop_checks >= limit)
    }
}

#[test]
fn runs_main_with_imprima_and_leia() {
    let mut io = TestIo {
        inputs: ["42".to_owned()].into(),
        ..TestIo::default()
    };
    let source = "algoritmo entrada; variáveis idade: inteiro; fim-variáveis\n\
                  início imprima(\"Idade?\"); idade := leia(); imprima(\"valor=\", idade); fim";

    execute(source, &mut io).expect("program should execute");

    assert_eq!(io.output, "Idade?\nvalor=42\n");
    assert_eq!(io.prompt_types, [DataType::Integer]);
}

#[test]
fn imprima_appends_a_newline() {
    let mut io = TestIo::default();
    execute("algoritmo saida; início imprima(1, \"x\"); fim", &mut io)
        .expect("program should execute");

    assert_eq!(io.output, "1x\n");
}

#[test]
fn executes_se_enquanto_repita_and_para() {
    let mut io = TestIo::default();
    let source = "algoritmo controles;\n\
      variáveis i, total: inteiro; fim-variáveis\n\
      início\n\
        total := 0;\n\
        para i de 1 até 3 faça total := total + i; fim-para\n\
        enquanto i < 4 faça i := i + 1; fim-enquanto\n\
        repita total := total + 1; até total > 7;\n\
        se total = 8 então imprima(total); senão imprima(0); fim-se\n\
      fim";

    execute(source, &mut io).expect("control flow should execute");

    assert_eq!(io.output, "8\n");
}

#[test]
fn for_loop_compares_the_counter_after_evaluating_the_dynamic_bound() {
    let source = "algoritmo limite; variáveis i: inteiro; fim-variáveis início para i de 1 até limite() faça imprima(i); fim-para imprima(i); fim função limite(): inteiro início i := i + 1; retorne 3; fim";
    let mut io = TestIo::default();

    execute(source, &mut io).expect("the loop should observe counter changes from its bound");

    assert_eq!(io.output, "2\n3\n");
}

#[test]
fn reports_runtime_errors_at_the_source_location() {
    let mut io = TestIo::default();
    let diagnostics = execute(
        "algoritmo erro; variáveis x: inteiro; fim-variáveis\n\
         início\n\
           x := 1 / 0;\n\
         fim",
        &mut io,
    )
    .expect_err("division by zero should be a runtime error");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
    assert_eq!(diagnostics[0].span.start.line, 3);
}

#[test]
fn preserves_integer_division_and_mixed_numeric_promotion() {
    let mut io = TestIo::default();
    let source = "algoritmo divisao;\n\
      variáveis whole: inteiro; decimal: real; fim-variáveis\n\
      início\n\
        whole := 5 / 2; decimal := 5.0 / 2; imprima(whole, \" \", decimal);\n\
      fim";

    execute(source, &mut io).expect("numeric conversions should execute");

    assert_eq!(io.output, "2 2.50\n");
}

#[test]
fn real_ordering_matches_numeric_comparisons_for_signed_zero_and_nan() {
    let mut io = TestIo {
        inputs: ["1e999".to_owned()].into(),
        ..TestIo::default()
    };
    let source = "algoritmo comparacao; variáveis negative_zero, nan_value: real; fim-variáveis\n\
      início\n\
        negative_zero := -0.0; nan_value := leia(); nan_value := nan_value - nan_value;\n\
        imprima(negative_zero < 0.0, \"|\", negative_zero <= 0.0, \"|\", negative_zero > 0.0, \"|\", negative_zero >= 0.0, \"|\", negative_zero = 0.0, \"|\", nan_value < 0.0, \"|\", nan_value <= 0.0, \"|\", nan_value > 0.0, \"|\", nan_value >= 0.0, \"|\", nan_value = 0.0, \"|\", nan_value <> 0.0);\n\
      fim";

    execute(source, &mut io).expect("real comparisons should use floating point operators");

    assert_eq!(
        io.output,
        "falso|verdadeiro|falso|verdadeiro|verdadeiro|falso|falso|falso|falso|falso|verdadeiro\n"
    );
}

#[test]
fn literal_values_use_the_full_string_for_output_comparison_and_length() {
    let mut io = TestIo {
        inputs: ["a\0bc".to_owned(), "a\0bc".to_owned()].into(),
        ..TestIo::default()
    };
    let source = "algoritmo teste_nul; variáveis text, same: literal; empty: literal; fim-variáveis\n\
      início\n\
        text := leia(); same := leia();\n\
        imprima(text, \"|\", text = \"a\", \"|\", text = same, \"|\", text > \"abc\", \"|\", text < \"abcde\", \"|\", empty = \"\");\n\
      fim";

    execute(source, &mut io).expect("literal values should preserve embedded NUL bytes");

    assert_eq!(
        io.output,
        "a|falso|verdadeiro|verdadeiro|verdadeiro|verdadeiro\n"
    );
}

#[test]
fn unary_not_treats_untyped_literals_as_numeric_prefixes() {
    let mut io = TestIo::default();
    let source = "algoritmo nao_literal; início imprima(não \"abc\", \"|\", não \"1abc\", \"|\", não \"\"); fim";

    execute(source, &mut io).expect("unary not should use the legacy numeric literal conversion");

    assert_eq!(io.output, "verdadeiro|falso|verdadeiro\n");
}

#[test]
fn unary_minus_and_complement_preserve_logical_and_character_values() {
    let mut io = TestIo::default();
    let source = "algoritmo unary_values; início\n\
      imprima(-verdadeiro, \"|\", ~falso, \"|\", -verdadeiro = -1, \"|\", ~verdadeiro = -2);\n\
      imprima(-'A', \"|\", ~'A', \"|\", -'A' = -65, \"|\", ~'A' = -66);\n\
    fim";

    execute(source, &mut io).expect("unary operations should preserve their source value types");

    assert_eq!(
        io.output,
        "verdadeiro|verdadeiro|verdadeiro|verdadeiro\n\\xbf|\\xbe|verdadeiro|verdadeiro\n"
    );
}

#[test]
fn converts_assignment_and_leia_by_target_type() {
    let mut io = TestIo {
        inputs: ["12.9 trailing", "2.75", "Z", "falso"]
            .map(str::to_owned)
            .into(),
        ..TestIo::default()
    };
    let source = "algoritmo conversoes;\n\
      variáveis n: inteiro; r: real; c: caractere; l: lógico; fim-variáveis\n\
      início\n\
        n := leia(); r := leia(); c := leia(); l := leia();\n\
        imprima(n, \"|\", r, \"|\", c, \"|\", l);\n\
      fim";

    execute(source, &mut io).expect("typed input should execute");

    assert_eq!(
        io.prompt_types,
        [
            DataType::Integer,
            DataType::Real,
            DataType::Character,
            DataType::Logical
        ]
    );
    assert_eq!(io.output, "12|2.75|Z|falso\n");
}

#[test]
fn stops_at_execution_output_and_string_limits() {
    let source = "algoritmo limites; início imprima(\"abc\"); fim";
    let mut step_io = TestIo::default();
    let diagnostics = execute_with_options(
        "algoritmo passos; início imprima(1); imprima(2); imprima(3); fim",
        &mut step_io,
        ExecutionOptions {
            max_steps: 2,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("step limit should stop execution");
    assert!(diagnostics[0].message.contains("step limit"));

    let mut output_io = TestIo::default();
    let diagnostics = execute_with_options(
        source,
        &mut output_io,
        ExecutionOptions {
            max_output_bytes: 2,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("output limit should stop execution");
    assert!(diagnostics[0].message.contains("output limit"));
    assert!(output_io.output.is_empty());

    let mut string_io = TestIo::default();
    let diagnostics = execute_with_options(
        source,
        &mut string_io,
        ExecutionOptions {
            max_string_bytes: 2,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("string limit should stop execution");
    assert!(diagnostics[0].message.contains("string limit"));
}

#[test]
fn honors_host_cancellation_during_a_loop() {
    let mut io = TestIo {
        stop_after_checks: Some(4),
        ..TestIo::default()
    };

    let diagnostics = execute(
        "algoritmo cancela; início enquanto verdadeiro faça imprima(1); fim-enquanto fim",
        &mut io,
    )
    .expect_err("the host cancellation signal should stop the loop");

    assert!(diagnostics[0].message.contains("cancelled"));
}

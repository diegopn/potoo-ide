use std::collections::VecDeque;

use potoo_gportugol_interpreter::{
    DataType, DiagnosticKind, ExecutionOptions, InputPrompt, RuntimeIo, execute,
    execute_with_options, execute_with_sources,
};

#[derive(Default)]
struct Io {
    inputs: VecDeque<String>,
    output: String,
    prompts: Vec<DataType>,
}

impl RuntimeIo for Io {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String> {
        self.prompts.push(prompt.data_type);
        self.inputs
            .pop_front()
            .ok_or_else(|| "unexpected input".to_owned())
    }
}

#[test]
fn gp_001_control_flow_bodies_execute_every_statement() {
    let source = "algoritmo teste; variáveis i: inteiro; fim-variáveis início se verdadeiro então imprima(1); imprima(2); fim-se enquanto i < 1 faça imprima(i); i := i + 1; fim-enquanto para i de 1 até 1 faça imprima(i); imprima(2); fim-para repita imprima(3); imprima(4); até verdadeiro; fim";
    let mut io = Io::default();

    execute(source, &mut io).expect("all statements in each body should execute");

    assert_eq!(io.output, "1\n2\n0\n1\n2\n3\n4\n");
}

#[test]
fn gp_001_empty_loop_bodies_still_obey_step_limits() {
    let mut io = Io {
        inputs: ["verdadeiro".to_owned(), "verdadeiro".to_owned()].into(),
        ..Io::default()
    };
    let diagnostics = execute_with_options(
        "algoritmo teste; início enquanto leia() faça fim-enquanto fim",
        &mut io,
        ExecutionOptions {
            max_steps: 2,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("empty loop bodies should still consume steps");

    assert!(diagnostics[0].message.contains("step limit"));
}

#[test]
fn gp_003_logical_operators_evaluate_the_right_operand() {
    let source = "algoritmo teste; variáveis x: inteiro; fim-variáveis início x := 0; imprima(falso e f()); imprima(x); x := 0; imprima(verdadeiro ou f()); imprima(x); fim função f(): lógico início x := x + 1; retorne verdadeiro; fim";
    let mut io = Io::default();

    execute(source, &mut io).expect("logical operands should execute in source order");

    assert_eq!(io.output, "falso\n1\nverdadeiro\n1\n");
}

#[test]
fn gp_003_logical_operands_preserve_order_and_right_side_errors() {
    let mut io = Io::default();
    execute(
        "algoritmo teste; início imprima(f() e g()); fim função f(): lógico início imprima(\"esquerda\"); retorne falso; fim função g(): lógico início imprima(\"direita\"); retorne verdadeiro; fim",
        &mut io,
    )
    .expect("both logical operands should execute");
    assert_eq!(io.output, "esquerda\ndireita\nfalso\n");

    let diagnostics = execute(
        "algoritmo teste; início imprima(falso e (1 / 0 = 0)); fim",
        &mut Io::default(),
    )
    .expect_err("an error in the right operand should remain observable");
    assert!(diagnostics[0].message.contains("division by zero"));
}

#[test]
fn gp_004_functions_see_only_their_own_locals_and_globals() {
    let source = "algoritmo teste; variáveis x: inteiro; fim-variáveis início x := 1; imprima(g()); fim função f(): inteiro início retorne x; fim função g(): inteiro x: inteiro; início x := 2; retorne f(); fim";
    let mut io = Io::default();
    execute(source, &mut io).expect("the global x should remain visible");
    assert_eq!(io.output, "1\n");

    let source = "algoritmo teste; início imprima(g()); fim função f(): inteiro início retorne x; fim função g(): inteiro x: inteiro; início x := 2; retorne f(); fim";
    let diagnostics = execute(source, &mut Io::default())
        .expect_err("a function must not see its caller's local x");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
}

#[test]
fn gp_005_assignment_resolves_the_target_before_evaluating_the_value() {
    let source = "algoritmo teste; variáveis v: matriz[1] de inteiros; fim-variáveis início v[f()] := g(); imprima(v[0]); fim função f(): inteiro início imprima(\"indice\"); retorne 0; fim função g(): inteiro início imprima(\"valor\"); retorne 9; fim";
    let mut io = Io::default();

    execute(source, &mut io).expect("valid indexed assignment should execute");

    assert_eq!(io.output, "indice\nvalor\n9\n");
}

#[test]
fn gp_005_invalid_assignment_target_prevents_value_side_effects() {
    let source = "algoritmo teste; variáveis v: matriz[1] de inteiros; fim-variáveis início v[1] := valor(); fim função valor(): inteiro início imprima(\"valor\"); retorne 9; fim";
    let mut io = Io::default();

    let diagnostics = execute(source, &mut io)
        .expect_err("an invalid assignment target should fail before its value runs");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
    assert!(io.output.is_empty());
}

#[test]
fn gp_006_array_reads_observe_mutations_made_by_index_expressions() {
    let source = "algoritmo teste; variáveis v: matriz[1] de inteiros; fim-variáveis início v[0] := 1; imprima(v[f()]); fim função f(): inteiro início v[0] := 7; retorne 0; fim";
    let mut io = Io::default();

    execute(source, &mut io).expect("the index expression should mutate the array");

    assert_eq!(io.output, "7\n");
}

#[test]
fn gp_007_for_loop_end_expression_is_rechecked() {
    let source = "algoritmo teste; variáveis i, lim: inteiro; fim-variáveis início lim := 3; para i de 1 até lim faça lim := lim - 1; fim-para imprima(i, lim); fim";
    let mut io = Io::default();

    execute(source, &mut io).expect("the loop should reevaluate its end expression");

    assert_eq!(io.output, "11\n");

    let mut io = Io::default();
    execute(
        "algoritmo teste; variáveis i, c: inteiro; fim-variáveis início para i de 1 até limite() faça fim-para imprima(c); fim função limite(): inteiro início c := c + 1; retorne 2; fim",
        &mut io,
    )
    .expect("the end function should be called for every bound check");
    assert_eq!(io.output, "3\n");
}

#[test]
fn gp_008_for_loop_resolves_its_target_once_and_sets_the_final_bound() {
    let source = "algoritmo teste; variáveis v: matriz[1] de inteiros; c: inteiro; fim-variáveis início para v[f()] de 1 até 2 faça imprima(v[0]); fim-para imprima(c); fim função f(): inteiro início c := c + 1; retorne 0; fim";
    let mut io = Io::default();
    execute(source, &mut io).expect("the array counter should be resolved once");
    assert_eq!(io.output, "1\n2\n1\n");

    let source = "algoritmo teste; variáveis i: inteiro; fim-variáveis início para i de 5 até 3 faça imprima(\"nunca\"); fim-para imprima(i); fim";
    let mut io = Io::default();
    execute(source, &mut io).expect("an empty loop should still set its final counter");
    assert_eq!(io.output, "3\n");

    let mut io = Io::default();
    let code = execute(
        "algoritmo teste; variáveis i: inteiro; fim-variáveis início para i de 3 até 1 passo -1 faça imprima(i); fim-para imprima(i); fim",
        &mut io,
    )
    .expect("negative steps should work");
    assert_eq!(code, None);
    assert_eq!(io.output, "3\n2\n1\n1\n");

    let mut io = Io::default();
    let code = execute(
        "algoritmo teste; variáveis i: inteiro; fim-variáveis início para i de 5 até 8 faça imprima(i); retorne 7; fim-para fim",
        &mut io,
    )
    .expect("return should leave the counter at its current value");
    assert_eq!(code, Some(7));
    assert_eq!(io.output, "5\n");
}

#[test]
fn gp_009_arithmetic_on_characters_and_logicals_returns_integers() {
    let mut io = Io::default();

    execute(
        "algoritmo teste; início imprima('a' + 'b'); imprima(verdadeiro + verdadeiro); imprima('a' - 'b', 'a' * 'b', verdadeiro * verdadeiro); fim",
        &mut io,
    )
    .expect("non-real arithmetic should produce integer values");

    assert_eq!(io.output, "195\n2\n-195061\n");
}

#[test]
fn gp_010_literal_order_uses_utf8_byte_length() {
    let mut io = Io::default();

    execute(
        "algoritmo teste; início imprima(\"á\" > \"b\", \"á\" = \"b\"); fim",
        &mut io,
    )
    .expect("literal comparison should execute");

    assert_eq!(io.output, "verdadeirofalso\n");
}

#[test]
fn gp_011_zero_literal_and_zero_escape_match_gpt_representation() {
    let source = "algoritmo teste; variáveis s: literal; fim-variáveis início imprima(\"[\", s, \"]\"); imprima(\"a\\0b\"); imprima(\"\"); imprima(\"[]\"); fim";
    let mut io = Io::default();

    execute(source, &mut io).expect("literal output should execute");

    assert_eq!(io.output, "[(nulo)]\na0b\n(nulo)\n[]\n");
}

#[test]
fn gp_011_nul_in_input_is_preserved_as_part_of_the_literal() {
    let mut io = Io {
        inputs: ["\0abc\n".to_owned()].into(),
        ..Io::default()
    };

    execute(
        "algoritmo teste; variáveis s: literal; fim-variáveis início s := leia(); imprima(s); imprima(\"fim\"); fim",
        &mut io,
    )
    .expect("a literal input line can contain a C NUL byte");

    assert_eq!(io.output, "\nfim\n");
}

#[test]
fn gp_011_empty_input_literal_uses_the_original_null_display() {
    let mut io = Io {
        inputs: ["\n".to_owned()].into(),
        ..Io::default()
    };

    execute(
        "algoritmo teste; variáveis s: literal; fim-variáveis início s := leia(); imprima(s); imprima(\"fim\"); fim",
        &mut io,
    )
    .expect("an empty literal input line is valid");

    assert_eq!(io.output, "(nulo)\nfim\n");
}

#[test]
fn gp_012_leia_without_a_conversion_target_reads_a_literal_line() {
    let mut io = Io {
        inputs: ["discarded\n".to_owned(), "hello\n".to_owned()].into(),
        ..Io::default()
    };

    execute(
        "algoritmo teste; início leia(); imprima(leia()); fim",
        &mut io,
    )
    .expect("untyped leia should consume and return a literal line");

    assert_eq!(io.output, "hello\n");
    assert_eq!(io.prompts, [DataType::Literal, DataType::Literal]);
}

#[test]
fn gp_013_read_logical_uses_exact_false_spellings() {
    for (input, expected) in [
        ("FALSO\n", "verdadeiro\n"),
        (" falso \n", "verdadeiro\n"),
        ("0.0\n", "verdadeiro\n"),
        ("falso\n", "falso\n"),
        ("0\n", "falso\n"),
    ] {
        let mut io = Io {
            inputs: [input.to_owned()].into(),
            ..Io::default()
        };
        execute(
            "algoritmo teste; variáveis b: lógico; fim-variáveis início b := leia(); imprima(b); fim",
            &mut io,
        )
        .expect("logical input should convert");
        assert_eq!(io.output, expected, "input was {input:?}");
    }
}

#[test]
fn gp_014_typed_function_without_return_uses_its_zero_value() {
    let mut io = Io::default();

    execute(
        "algoritmo teste; início imprima(f()); fim função f(): inteiro início fim",
        &mut io,
    )
    .expect("a completed typed function should return its initial value");

    assert_eq!(io.output, "0\n");
}

#[test]
fn gp_015_main_return_without_a_value_fails_before_side_effects() {
    let mut io = Io::default();
    let diagnostics = execute(
        "algoritmo teste; início imprima(\"a\"); retorne; fim",
        &mut io,
    )
    .expect_err("the main block return value must be an integer");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    assert!(io.output.is_empty());
}

#[test]
fn gp_016_for_loop_requires_integer_counter_and_bounds() {
    for source in [
        "algoritmo teste; variáveis b: lógico; fim-variáveis início para b de 1 até 0 faça fim-para fim",
        "algoritmo teste; variáveis r: real; fim-variáveis início para r de 1.0 até 2.0 faça fim-para fim",
        "algoritmo teste; variáveis i: inteiro; fim-variáveis início para i de 1.0 até 2 faça fim-para fim",
    ] {
        let diagnostics = execute(source, &mut Io::default())
            .expect_err("for-loop counter and bounds should be integers");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    }
}

#[test]
fn gp_017_remainder_rejects_real_operands() {
    let diagnostics = execute(
        "algoritmo teste; início imprima(5.5 % 2); fim",
        &mut Io::default(),
    )
    .expect_err("remainder is defined only for integer-compatible operands");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);

    let mut io = Io::default();
    execute("algoritmo teste; início imprima(-5 % 2); fim", &mut io)
        .expect("integer remainder should remain valid");
    assert_eq!(io.output, "-1\n");
}

#[test]
fn gp_018_globals_and_functions_share_a_declaration_namespace() {
    let mut io = Io::default();
    let diagnostics = execute(
        "algoritmo teste; variáveis x: inteiro; fim-variáveis início x := 1; imprima(x()); fim função x(): inteiro início retorne 2; fim",
        &mut io,
    )
    .expect_err("a global variable and function cannot share a name");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    assert!(io.output.is_empty());
}

#[test]
fn gp_020_hexadecimal_literals_use_signed_int32_before_real_conversion() {
    let mut io = Io::default();

    execute(
        "algoritmo teste; variáveis r: real; fim-variáveis início r := 0xFFFFFFFF; imprima(r); r := 0x80000000; imprima(r); r := 2147483648; imprima(r); fim",
        &mut io,
    )
    .expect("hexadecimal and decimal integer literals should convert");

    assert_eq!(io.output, "-1.00\n-2147483648.00\n2147483648.00\n");
}

#[test]
fn gp_021_composed_sources_share_lexing_and_keep_local_diagnostics() {
    let mut io = Io::default();
    execute_with_sources(
        "algoritmo teste; início imprima(1); /* unterminated",
        &["*/ fim"],
        &mut io,
    )
    .expect("a block comment may close in the following source");
    assert_eq!(io.output, "1\n");

    let diagnostics = execute_with_sources("algoritmo teste; início fim", &["@"], &mut io)
        .expect_err("lexical diagnostics should identify the source they came from");
    assert_eq!(diagnostics[0].span.start.line, 1);
    assert!(diagnostics[0].message.contains("source 2"));

    let diagnostics = execute_with_sources(
        "algoritmo teste; início imprima(extra()); fim",
        &["função extra(): inteiro início retorne missing; fim"],
        &mut io,
    )
    .expect_err("errors in added sources should use local source positions");
    assert_eq!(diagnostics[0].span.start.line, 1);
    assert!(diagnostics[0].message.contains("source 2"));
}

#[test]
fn gp_002_composed_sources_ignore_a_bom_at_each_file_start() {
    let mut io = Io::default();
    execute_with_sources(
        "algoritmo teste; início imprima(extra()); fim",
        &["\u{feff}função extra(): inteiro início retorne 7; fim"],
        &mut io,
    )
    .expect("each source file should ignore its own leading BOM");

    assert_eq!(io.output, "7\n");
}

#[test]
fn strict_gpt_library_round_preserves_the_missing_return_default() {
    let mut io = Io::default();
    execute(
        "algoritmo teste; início imprima(arredonda(2.6)); fim",
        &mut io,
    )
    .expect("arredonda should complete normally");
    assert_eq!(io.output, "0.00\n");
}

#[test]
fn strict_gpt_library_power_truncates_fractional_exponents() {
    let mut io = Io::default();
    execute(
        "algoritmo teste; início imprima(potencia(9.0, 0.5)); fim",
        &mut io,
    )
    .expect("potencia should complete normally");
    assert_eq!(io.output, "1.00\n");
}

#[test]
fn strict_gpt_library_power_does_not_reciprocate_other_negative_exponents() {
    let mut io = Io::default();
    execute(
        "algoritmo teste; início imprima(potencia(2.0, -2.0)); fim",
        &mut io,
    )
    .expect("potencia should complete normally");
    assert_eq!(io.output, "1.00\n");
}

#[test]
fn strict_gpt_library_sqrt_truncates_the_fractional_radicand() {
    let mut io = Io::default();
    execute(
        "algoritmo teste; início imprima(raiz_quadrada(2.5)); fim",
        &mut io,
    )
    .expect("raiz_quadrada should complete normally");
    assert_eq!(io.output, "1.41\n");
}

#[test]
fn strict_gpt_library_sqrt_returns_zero_for_negative_input() {
    let mut io = Io::default();
    execute(
        "algoritmo teste; início imprima(raiz_quadrada(-1)); fim",
        &mut io,
    )
    .expect("negative raiz_quadrada should complete normally");
    assert_eq!(io.output, "0.00\n");
}

#[test]
fn strict_gpt_real_print_converts_through_f32() {
    let mut io = Io::default();
    execute("algoritmo teste; início imprima(12345678.9); fim", &mut io)
        .expect("real output should complete normally");
    assert_eq!(io.output, "12345679.00\n");
}

#[test]
fn strict_gpt_real_arithmetic_keeps_six_significant_digits() {
    let mut io = Io::default();
    execute(
        "algoritmo teste; início imprima((1.23456789 + 0.0) * 1000000.0); fim",
        &mut io,
    )
    .expect("real arithmetic should complete normally");
    assert_eq!(io.output, "1234570.00\n");
}

#[test]
fn strict_gpt_unary_minus_on_character_uses_the_legacy_byte() {
    let mut io = Io::default();
    execute("algoritmo teste; início imprima(-'a'); fim", &mut io)
        .expect("character unary minus should complete normally");
    assert_eq!(io.output, "\\x9f\n");
}

#[test]
fn strict_gpt_character_input_uses_the_first_utf8_byte_as_signed() {
    let mut io = Io {
        inputs: ["é\n".to_owned()].into(),
        ..Io::default()
    };
    execute(
        "algoritmo teste; variáveis c: caractere; fim-variáveis início c := leia(); imprima(c + 0); fim",
        &mut io,
    )
    .expect("character input should convert");
    assert_eq!(io.output, "-61\n");
}

#[test]
fn strict_gpt_absolute_preserves_negative_zero() {
    let mut io = Io::default();
    execute(
        "algoritmo teste; início imprima(absoluto(-0.0)); fim",
        &mut io,
    )
    .expect("absoluto should complete normally");
    assert_eq!(io.output, "-0.00\n");
}

#[test]
fn gp_014_typed_function_fallthrough_reuses_the_previous_return_value() {
    let mut io = Io::default();
    execute(
        "algoritmo teste; início imprima(fi(), \"|\", fr(), \"|\", fl(), \"|\", fc(), \"|\", fs()); fim função fi(): inteiro início fim função fr(): real início fim função fl(): lógico início fim função fc(): caractere início fim função fs(): literal início fim",
        &mut io,
    )
    .expect("fallthrough should retain and retag the previous return value");
    assert_eq!(io.output, "0|0.00|falso|\0|0\n");
}

#[test]
fn non_bmp_literals_and_characters_are_rejected_like_the_reference_lexer() {
    let mut io = Io::default();
    assert!(execute("algoritmo teste; início imprima(\"😀\"); fim", &mut io).is_err());
    assert!(execute("algoritmo teste; início imprima('😀'); fim", &mut io).is_err());
}

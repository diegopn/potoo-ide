use potoo_portugol_interpreter::{RuntimeIo, execute, parse};

#[derive(Default)]
struct Output(String);

impl RuntimeIo for Output {
    fn write(&mut self, text: &str) {
        self.0.push_str(text);
    }

    fn read_line(&mut self) -> Result<String, String> {
        Err("unexpected input".to_owned())
    }
}

fn run(source: &str) -> String {
    let mut io = Output::default();
    execute(source, &mut io).expect("program should execute");
    io.0
}

#[test]
fn function_lookup_uses_global_scope_not_the_callers_local_scope() {
    assert_eq!(
        run(r#"programa {
  inteiro x = 1
  funcao f() { escreva(x) }
  funcao g() { inteiro x = 2 f() }
  funcao inicio() { g() }
}"#),
        "1"
    );
}

#[test]
fn arrays_passed_by_value_share_the_reference_backing_array() {
    assert_eq!(
        run(r#"programa {
  funcao alterar(inteiro valores[]) { valores[0] = 9 }
  funcao inicio() {
    inteiro valores[1] = {1}
    alterar(valores)
    escreva(valores[0])
  }
}"#),
        "9"
    );
}

#[test]
fn integer_arithmetic_wraps_at_thirty_two_bits() {
    assert_eq!(
        run(r#"programa {
  funcao inicio() { escreva(2147483647 + 1) }
}"#),
        "-2147483648"
    );
}

#[test]
fn shift_count_uses_java_int_masking() {
    assert_eq!(
        run(r#"programa {
  funcao inicio() { escreva(1 << 32) }
}"#),
        "1"
    );
}

#[test]
fn multiplication_has_higher_precedence_than_division_in_the_reference() {
    assert_eq!(
        run(r#"programa {
  funcao inicio() { escreva(24 / 3 * 2) }
}"#),
        "4"
    );
}

#[test]
fn grammar_operator_alternatives_keep_their_reference_precedence() {
    assert_eq!(
        run(r#"programa {
  funcao inicio() {
    escreva(8 % 3 * 2, "|", 10 - 3 + 2, "|", 8 ^ 3 << 1, "|", 8 << 1 & 3)
  }
}"#),
        "2|5|22|0"
    );
}

#[test]
fn default_case_keeps_its_source_position_during_fallthrough() {
    assert_eq!(
        run(r#"programa {
  funcao inicio() {
    escolha (1) {
      caso 1: escreva("A")
      caso contrario: escreva("D")
      caso 2: escreva("B")
    }
  }
}"#),
        "ADB"
    );
}

#[test]
fn assignment_evaluates_an_index_once_before_its_value() {
    assert_eq!(
        run(r#"programa {
  inteiro chamadas = 0
  funcao inteiro indice() { chamadas++ retorne 0 }
  funcao inicio() {
    inteiro valores[1] = {0}
    valores[indice()] = 7
    escreva(chamadas, "|", valores[0])
  }
}"#),
        "1|7"
    );
}

#[test]
fn indexed_assignment_and_increment_evaluate_the_index_once_before_the_value() {
    assert_eq!(
        run(r#"programa {
  cadeia ordem = ""
  funcao inteiro indice() { ordem = ordem + "I" retorne 0 }
  funcao inteiro valor() { ordem = ordem + "V" retorne 7 }
  funcao inicio() {
    inteiro valores[1] = {30}
    valores[indice()] = valor()
    escreva(ordem, "|", valores[0], "|")
    inteiro cursor = 0
    escreva(valores[cursor++]++, "|", cursor, "|", valores[0], "|")
    cursor = 0
    escreva(++valores[cursor++], "|", cursor, "|", valores[0])
  }
}"#),
        "IV|7|7|1|8|9|1|9"
    );
}

#[test]
fn keyboard_character_conversion_rejects_a_lone_surrogate_instead_of_returning_nul() {
    let source = r#"programa {
  inclua biblioteca Teclado
  funcao inicio() { escreva(Teclado.caracter_tecla(55296)) }
}"#;
    let mut io = Output::default();
    let errors = potoo_portugol_interpreter::execute(source, &mut io)
        .expect_err("a surrogate has no Rust char representation");
    assert!(errors[0].message.contains("surrogate UTF-16"));
}

#[test]
fn long_flat_expression_is_rejected_before_it_can_overflow_ast_drop() {
    let chain = vec!["1"; 1000].join("+");
    let source = format!("programa {{ funcao inicio() {{ escreva({chain}) }} }}");
    assert!(parse(&source).is_err());
}

#[test]
fn calendar_exports_all_reference_day_and_month_constants() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Calendario --> c
  funcao inicio() {
    escreva(
      c.DIA_DOMINGO, c.DIA_SEGUNDA_FEIRA, c.DIA_TERCA_FEIRA,
      c.DIA_QUARTA_FEIRA, c.DIA_QUINTA_FEIRA, c.DIA_SEXTA_FEIRA, c.DIA_SABADO,
      c.MES_JANEIRO, c.MES_FEVEREIRO, c.MES_MARCO, c.MES_ABRIL, c.MES_MAIO,
      c.MES_JUNHO, c.MES_JULHO, c.MES_AGOSTO, c.MES_SETEMBRO, c.MES_OUTUBRO,
      c.MES_NOVEMBRO, c.MES_DEZEMBRO
    )
  }
}"#),
        "1234567123456789101112"
    );
}

#[test]
fn real_conversion_uses_java_format_parse_and_int_saturation() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Tipos --> t
  funcao inicio() {
    escreva(
      t.real_para_cadeia(1.0), "|",
      t.real_para_cadeia(-0.0), "|",
      t.real_para_cadeia(0.0000001), "|",
      t.real_para_cadeia(t.cadeia_para_real(" 0x1.8p1 ")), "|",
      t.real_para_inteiro(3000000000.0)
    )
  }
}"#),
        "1.0|-0.0|1.0E-7|3.0|2147483647"
    );
}

#[test]
fn hexadecimal_real_parsing_saturates_extreme_binary_exponents() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Tipos --> t
  funcao inicio() {
    escreva(
      t.real_para_cadeia(t.cadeia_para_real("0x1p999999999999999999")), "|",
      t.real_para_cadeia(t.cadeia_para_real("0x0p999999999999999999")), "|",
      t.real_para_cadeia(t.cadeia_para_real("0x1p-999999999999999999"))
    )
  }
}"#),
        "Infinity|0.0|0.0"
    );
}

#[test]
fn radix_conversion_removes_the_first_prefix_even_inside_the_text() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Tipos --> t
  funcao inicio() {
    escreva(
      t.cadeia_para_inteiro("10xF", 16), "|",
      t.cadeia_para_inteiro("0x10X", 16), "|",
      t.cadeia_para_inteiro("0b10B", 2)
    )
  }
}"#),
        "31|1|1"
    );
}

#[test]
fn text_indices_lengths_and_search_positions_use_utf16_units() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Texto --> t
  inclua biblioteca Tipos --> c
  funcao inicio() {
    escreva(
      t.numero_caracteres("😀"), "|",
      t.extrair_subtexto("😀", 0, 2), "|",
      t.posicao_texto("😀", "x😀", -4), "|",
      t.preencher_a_esquerda('0', -1, "a"), "|",
      c.cadeia_e_caracter("😀")
    )
  }
}"#),
        "2|😀|1|a|falso"
    );
}

#[test]
fn text_lowercase_and_logical_regex_match_reference_edge_cases() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Texto --> t
  inclua biblioteca Tipos --> c
  funcao inicio() {
    escreva(t.caixa_baixa("ΟΣ ΟΣ.Α"), "|", c.cadeia_e_logico("verdadeirox"), "|", c.cadeia_e_logico("xfalso"))
  }
}"#),
        "ος οσ.α|verdadeiro|verdadeiro"
    );
}

#[test]
fn replacing_an_empty_target_inserts_between_utf16_units() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Texto --> t
  funcao inicio() { escreva(t.substituir("abc", "", "-"), "|", t.substituir("", "", "-")) }
}"#),
        "-a-b-c-|-"
    );
}

#[test]
fn empty_target_replacement_rejects_unrepresentable_surrogate_splitting() {
    let mut io = Output::default();
    let error = execute(
        r#"programa {
  inclua biblioteca Texto --> t
  funcao inicio() { escreva(t.substituir("😀", "", "-")) }
}"#,
        &mut io,
    )
    .expect_err("UTF-16 insertion between surrogate units cannot be represented as Rust text");

    assert!(error[0].message.contains("surrogate UTF-16"));
    assert!(io.0.is_empty());
}

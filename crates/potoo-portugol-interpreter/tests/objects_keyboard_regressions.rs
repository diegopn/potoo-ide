use potoo_portugol_interpreter::{RuntimeIo, execute};

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

fn fails(source: &str) -> bool {
    execute(source, &mut Output::default()).is_err()
}

#[test]
fn object_release_shifts_handles_and_release_all_restarts_at_zero() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro primeiro = o.criar_objeto()
    inteiro segundo = o.criar_objeto()
    o.atribuir_propriedade(segundo, "nome", "segundo")
    o.liberar_objeto(primeiro)
    escreva(o.obter_propriedade_tipo_cadeia(0, "nome"), "|")
    inteiro terceiro = o.criar_objeto()
    escreva(terceiro, "|")
    o.liberar()
    inteiro reiniciado = o.criar_objeto()
    escreva(reiniciado)
  }
}"#),
        "segundo|1|0"
    );
}

#[test]
fn object_json_accepts_trailing_data_and_uses_jackson_formatting() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro dados = o.criar_objeto_via_json("{\"a\":1.0,\"flags\":[true,false],\"nums\":[1,2],\"nested\":{\"x\":2.0}} trailing")
    escreva(o.contem_propriedade(dados, "a"), "|", o.obter_json(dados))
  }
}"#),
        "verdadeiro|{\n  \"a\" : 1.0,\n  \"flags\" : [ true, false ],\n  \"nums\" : [ 1, 2 ],\n  \"nested\" : {\n    \"x\" : 2.0\n  }\n}"
    );
}

#[test]
fn object_json_integer_categories_distinguish_int_long_and_big_integer() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro dados = o.criar_objeto_via_json("{\"small\":2147483647,\"long\":2147483648,\"big\":9223372036854775808,\"values\":[1,2147483648,9223372036854775808],\"duplicate\":2147483648,\"duplicate\":2}")
    escreva(o.tipo_propriedade(dados, "small"), "|", o.tipo_propriedade(dados, "long"), "|", o.tipo_propriedade(dados, "big"), "|", o.obter_propriedade_tipo_inteiro(dados, "small"), "|", o.tipo_propriedade(dados, "duplicate"), "|", o.obter_propriedade_tipo_inteiro(dados, "duplicate"), "|", o.obter_json(dados))
  }
}"#),
        "1|0|0|2147483647|1|2|{\n  \"small\" : 2147483647,\n  \"long\" : 2147483648,\n  \"big\" : 9223372036854775808,\n  \"values\" : [ 1, 2147483648, 9223372036854775808 ],\n  \"duplicate\" : 2\n}"
    );

    assert!(fails(
        r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro dados = o.criar_objeto_via_json("{\"long\":2147483648}")
    escreva(o.obter_propriedade_tipo_inteiro(dados, "long"))
  }
}"#
    ));
    assert!(fails(
        r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro dados = o.criar_objeto_via_json("{\"values\":[1,2147483648]}")
    escreva(o.obter_propriedade_tipo_inteiro_em_vetor(dados, "values", 1))
  }
}"#
    ));
}

#[test]
fn assigned_character_has_no_official_object_type_or_character_getter() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro dados = o.criar_objeto()
    o.atribuir_propriedade(dados, "letra", 'X')
    escreva(o.tipo_propriedade(dados, "letra"))
  }
}"#),
        "0"
    );

    assert!(fails(
        r#"programa {
  inclua biblioteca Objetos --> o
  funcao inicio() {
    inteiro dados = o.criar_objeto()
    o.atribuir_propriedade(dados, "letra", 'X')
    escreva(o.obter_propriedade_tipo_caracter(dados, "letra"))
  }
}"#
    ));
}

#[test]
fn keyboard_exposes_active_reference_constants_only() {
    assert_eq!(
        run(r#"programa {
  inclua biblioteca Teclado --> t
  funcao inicio() {
    escreva(t.TECLA_ENTER, "|", t.TECLA_F24, "|", t.TECLA_WINDOWS, "|", t.TECLA_MENU_CONTEXTO)
  }
}"#),
        "10|61451|524|525"
    );

    for name in [
        "TECLA_META",
        "TECLA_BACK_QUOTE",
        "TECLA_QUOTE",
        "TECLA_AMPERSAND",
        "TECLA_ASTERISK",
        "TECLA_QUOTEDBL",
        "TECLA_LESS",
        "TECLA_GREATER",
        "TECLA_BRACELEFT",
        "TECLA_BRACERIGHT",
        "TECLA_AT",
        "TECLA_COLON",
        "TECLA_CIRCUMFLEX",
        "TECLA_DOLLAR",
        "TECLA_EURO_SIGN",
        "TECLA_EXCLAMATION_MARK",
        "TECLA_INVERTED_EXCLAMATION_MARK",
        "TECLA_LEFT_PARENTHESIS",
        "TECLA_NUMBER_SIGN",
        "TECLA_PLUS",
        "TECLA_RIGHT_PARENTHESIS",
        "TECLA_UNDERSCORE",
    ] {
        let source = format!(
            "programa {{ inclua biblioteca Teclado --> t funcao inicio() {{ escreva(t.{name}) }} }}"
        );
        assert!(fails(&source), "{name} must stay unavailable");
    }
}

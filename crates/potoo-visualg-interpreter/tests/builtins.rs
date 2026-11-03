use potoo_visualg_interpreter::{DiagnosticKind, RuntimeIo, execute};

#[derive(Default)]
struct TestIo {
    output: String,
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(
        &mut self,
        _prompt: &potoo_visualg_interpreter::InputPrompt,
    ) -> Result<String, String> {
        Err("unexpected input".to_owned())
    }
}

#[test]
fn evaluates_every_numeric_builtin() {
    let source = r#"algoritmo "numericas"
inicio
  escreval(Abs(-2), ArcCos(1), ArcSen(0), ArcTan(0), Cos(0), CoTan(Pi / 4))
  escreval(Exp(2.0, 3.0), GrauPrad(180), Int(-1.8), Log(100), LogN(1), Pi)
  escreval(Quad(3), RadpGrau(Pi), RaizQ(9), Sen(0), Tan(0))
  escreval(Rand(), RandI(10))
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("all numeric built-ins should execute");

    let values = io
        .output
        .lines()
        .map(|line| {
            line.split_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    assert_eq!(values[0][0], "2");
    assert!((values[0][1].parse::<f64>().unwrap()).abs() < 1e-12);
    assert!((values[0][2].parse::<f64>().unwrap()).abs() < 1e-12);
    assert!((values[0][3].parse::<f64>().unwrap()).abs() < 1e-12);
    assert!((values[0][4].parse::<f64>().unwrap() - 1.0).abs() < 1e-12);
    assert!((values[0][5].parse::<f64>().unwrap() - 1.00000065).abs() < 1e-6);
    assert!((values[1][0].parse::<f64>().unwrap() - 8.0).abs() < 1e-12);
    assert!((values[1][1].parse::<f64>().unwrap() - std::f64::consts::PI).abs() < 1e-12);
    assert_eq!(values[1][2], "-1");
    assert!((values[1][3].parse::<f64>().unwrap() - 2.0).abs() < 1e-12);
    assert!((values[1][4].parse::<f64>().unwrap()).abs() < 1e-12);
    assert!((values[1][5].parse::<f64>().unwrap() - 3.141592).abs() < 1e-12);
    assert_eq!(values[2][0], "9");
    assert!((values[2][1].parse::<f64>().unwrap() - 179.99996).abs() < 1e-5);
    assert!((values[2][2].parse::<f64>().unwrap() - 3.0).abs() < 1e-12);
    assert!((values[2][3].parse::<f64>().unwrap()).abs() < 1e-12);
    assert!((values[2][4].parse::<f64>().unwrap()).abs() < 1e-12);
    let random = values[3][0].parse::<f64>().unwrap();
    let integer = values[3][1].parse::<i64>().unwrap();
    assert!((0.0..1.0).contains(&random));
    assert!((0..10).contains(&integer));
}

#[test]
fn evaluates_every_text_builtin() {
    let source = r#"algoritmo "texto"
inicio
  escreval(Asc("B"), Carac(65), Caracpnum("42"), Caracpnum("2.5"))
  escreval(Compr("abc"), Copia("abcdef", 2, 3), Maiusc("abc"), Minusc("ABC"))
  escreval(Numpcarac(7), Pos("cd", "abcd"), Pos("xy", "abcd"))
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("all text built-ins should execute");

    assert_eq!(io.output, " 66A 42 2.5\n 3bcdABCabc\n7 3 0\n");
}

#[test]
fn reports_invalid_builtin_domains_and_limits() {
    for expression in ["ArcCos(2)", "Log(0)", "LogN(-1)", "RaizQ(-1)", "RandI(0)"] {
        let source =
            format!("algoritmo \"dominio\"\ninicio\nescreva({expression})\nfimalgoritmo\n");
        let error =
            execute(&source, &mut TestIo::default()).expect_err("invalid domains should fail");
        assert_eq!(error[0].kind, DiagnosticKind::Runtime, "{expression}");
    }
}

#[test]
fn rejects_character_codes_and_integer_conversions_outside_visu_alg_limits() {
    for expression in ["Carac(-1)", "Carac(256)", "Int(9223372036854775808.0)"] {
        let source =
            format!("algoritmo \"limites\"\ninicio\nescreva({expression})\nfimalgoritmo\n");
        let error = execute(&source, &mut TestIo::default())
            .expect_err("out-of-range conversion should report a runtime diagnostic");
        assert_eq!(error[0].kind, DiagnosticKind::Runtime, "{expression}");
    }

    let source =
        "algoritmo \"limite\"\ninicio\nescreva(Int(-9223372036854775808.0))\nfimalgoritmo\n";
    let mut io = TestIo::default();
    execute(source, &mut io).expect("the minimum integer is representable");
    assert_eq!(io.output, " -9223372036854775808");
}

use std::collections::VecDeque;

use potoo_visualg_interpreter::{
    DataType, DiagnosticKind, ExecutionOptions, InputPrompt, RuntimeIo, execute,
    execute_with_options,
};

#[derive(Default)]
struct TestIo {
    output: String,
    inputs: VecDeque<String>,
    prompts: Vec<InputPrompt>,
}

impl TestIo {
    fn with_inputs(inputs: impl IntoIterator<Item = &'static str>) -> Self {
        Self {
            inputs: inputs.into_iter().map(str::to_owned).collect(),
            ..Self::default()
        }
    }
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String> {
        self.prompts.push(prompt.clone());
        self.inputs
            .pop_front()
            .ok_or_else(|| "no test input is available".to_owned())
    }
}

#[test]
fn executes_scalar_assignment_and_operator_precedence() {
    let source = r#"algoritmo "precedencia"
var
  resultado: inteiro
inicio
  resultado <- 1 + 2 * 4
  escreva(resultado, "|", verdadeiro, "|", "A" + "BC")
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("the program should execute");

    assert_eq!(io.output, " 9| VERDADEIRO|ABC");
}

#[test]
fn executes_documented_arithmetic_and_logical_operators() {
    let source = r#"algoritmo "operadores"
var
  quociente: inteiro
  resto: inteiro
  potencia: inteiro
inicio
  quociente <- 5 \ 2
  resto <- 8 mod 3
  potencia <- 5 ^ 2
  escreval(quociente, "|", resto, "|", potencia, "|", falso e (1 / 0 = 0), "|", verdadeiro ou (1 / 0 = 0), "|", falso < verdadeiro, "|", nao falso, "|", verdadeiro xou falso)
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("the documented operators should execute");

    assert_eq!(
        io.output,
        " 2| 2| 25| FALSO| VERDADEIRO| VERDADEIRO| VERDADEIRO| VERDADEIRO\n"
    );
}

#[test]
fn compares_strings_without_case_sensitivity() {
    let source = "algoritmo \"comparacao\"\ninicio\nescreval(\"ABC\" = \"abc\")\nfimalgoritmo\n";
    let mut io = TestIo::default();

    execute(source, &mut io).expect("the comparison should execute");

    assert_eq!(io.output, " VERDADEIRO\n");
}

#[test]
fn indexes_arrays_using_declared_lower_bounds() {
    let source = r#"algoritmo "indices"
var
  dados: vetor[2..4] de inteiro
  matriz: vetor[0..1, 8..9] de inteiro
inicio
  dados[2] <- 7
  dados[4] <- 9
  matriz[1, 9] <- 6
  escreva(dados[2], ",", dados[4], ",", matriz[1, 9])
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("declared lower bounds should be valid");

    assert_eq!(io.output, " 7, 9, 6");
}

#[test]
fn reports_invalid_array_indexes_at_source_location() {
    let source = "algoritmo \"limite\"\nvar\na: vetor[2..4] de inteiro\ninicio\nescreva(a[5])\nfimalgoritmo\n";
    let mut io = TestIo::default();

    let diagnostics = execute(source, &mut io).expect_err("index 5 is outside the vector");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (5, 11));
}

#[test]
fn reports_the_specific_out_of_range_matrix_index() {
    let source = "algoritmo \"matriz\"\nvar\nm: vetor[0..1, 8..9] de inteiro\ninicio\nescreva(m[1, 10])\nfimalgoritmo\n";
    let mut io = TestIo::default();

    let diagnostics = execute(source, &mut io).expect_err("second index 10 is out of range");

    assert_eq!((diagnostics[0].line, diagnostics[0].column), (5, 14));
}

#[test]
fn formats_write_items_and_reads_each_scalar_type() {
    let source = r#"algoritmo "entrada"
var
  idade: inteiro
  altura: real
  nome: caractere
  matriculada: logico
inicio
  leia(idade)
  leia(altura)
  leia(nome)
  leia(matriculada)
  escreva(idade:4, "|", altura:6:2, "|", nome, "|", matriculada)
fimalgoritmo
"#;
    let mut io = TestIo::with_inputs(["12", "2,5", "Maria", "verdadeiro"]);

    execute(source, &mut io).expect("all four scalar values should parse");

    assert_eq!(io.output, "   12|   2.50|Maria| VERDADEIRO");
    assert_eq!(
        io.prompts,
        [
            InputPrompt {
                variable_name: "idade".to_owned(),
                data_type: DataType::Integer,
            },
            InputPrompt {
                variable_name: "altura".to_owned(),
                data_type: DataType::Real,
            },
            InputPrompt {
                variable_name: "nome".to_owned(),
                data_type: DataType::Character,
            },
            InputPrompt {
                variable_name: "matriculada".to_owned(),
                data_type: DataType::Logical,
            },
        ]
    );
}

#[test]
fn output_width_counts_accented_characters_instead_of_utf8_bytes() {
    let source = "algoritmo \"largura\"\ninicio\nescreva(\"é\":2)\nfimalgoritmo\n";
    let mut io = TestIo::default();

    execute(source, &mut io).expect("the text should be formatted");

    assert_eq!(io.output, " é");
}

#[test]
fn reports_invalid_typed_input_at_the_target_location() {
    let source =
        "algoritmo \"entrada\"\nvar\nidade: inteiro\ninicio\n  leia(idade)\nfimalgoritmo\n";
    let mut io = TestIo::with_inputs(["doze"]);

    let diagnostics = execute(source, &mut io).expect_err("text is not a valid integer input");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (5, 8));
    assert!(diagnostics[0].message.contains("inteiro"));
}

#[test]
fn enforces_variable_array_output_and_string_limits() {
    let source = "algoritmo \"limites\"\nvar\na: inteiro\nb: inteiro\nc: vetor[1..1000000000] de inteiro\ninicio\nescreva(\"longo\")\nfimalgoritmo\n";
    let mut options = ExecutionOptions::default();
    options.max_variable_slots = 3;
    let mut io = TestIo::default();
    let variable_error = execute_with_options(source, &mut io, options)
        .expect_err("the declarations use four slots");
    assert_eq!(variable_error[0].kind, DiagnosticKind::Runtime);
    assert!(variable_error[0].message.contains("limite"));

    let mut options = ExecutionOptions::default();
    options.max_output_bytes = 4;
    let mut io = TestIo::default();
    let output_error = execute_with_options(
        "algoritmo \"saida\"\ninicio\nescreva(\"abcde\")\nfimalgoritmo\n",
        &mut io,
        options,
    )
    .expect_err("the output limit should be enforced");
    assert_eq!(output_error[0].kind, DiagnosticKind::Runtime);
    assert!(output_error[0].message.contains("saída"));

    let mut options = ExecutionOptions::default();
    options.max_string_bytes = 3;
    let mut io = TestIo::default();
    let string_error = execute_with_options(
        "algoritmo \"texto\"\ninicio\nescreva(\"longo\")\nfimalgoritmo\n",
        &mut io,
        options,
    )
    .expect_err("the string limit should be enforced");
    assert_eq!(string_error[0].kind, DiagnosticKind::Runtime);
    assert!(string_error[0].message.contains("texto"));

    let mut io = TestIo::default();
    let concatenation_error = execute_with_options(
        "algoritmo \"texto\"\ninicio\nescreva(\"ab\" + \"cd\")\nfimalgoritmo\n",
        &mut io,
        options,
    )
    .expect_err("concatenation must respect the string limit");
    assert_eq!(concatenation_error[0].kind, DiagnosticKind::Runtime);
    assert!(concatenation_error[0].message.contains("texto"));
}

#[test]
fn enforces_string_limit_for_interactive_character_input() {
    let source = "algoritmo \"entrada\"\nvar\nnome: caractere\ninicio\nleia(nome)\nfimalgoritmo\n";
    let mut options = ExecutionOptions::default();
    options.max_string_bytes = 2;
    let mut io = TestIo::with_inputs(["Ana"]);

    let diagnostics = execute_with_options(source, &mut io, options)
        .expect_err("interactive character input must respect the string limit");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
    assert!(diagnostics[0].message.contains("texto"));
}

#[test]
fn rejects_integer_only_operators_on_real_values() {
    let source = "algoritmo \"divisao\"\ninicio\nescreva(5.0 \\ 2)\nfimalgoritmo\n";
    let mut io = TestIo::default();

    let diagnostics =
        execute(source, &mut io).expect_err("integer division must reject real operands");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
    assert!(diagnostics[0].message.contains("inteiro"));
}

use potoo_gportugol_interpreter::{
    DiagnosticKind, ExecutionOptions, InputPrompt, RuntimeIo, execute, execute_with_options,
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
fn indexes_zero_based_vectors_and_matrices() {
    let mut io = Io::default();
    let source = "algoritmo indices;\n\
      variáveis vetor: matriz[3] de inteiros; grade: matriz[2][2] de inteiros; fim-variáveis\n\
      início\n\
        vetor[0] := 7; vetor[2] := 9; grade[1][0] := 4; grade[1][1] := 5;\n\
        imprima(vetor[0], \"|\", vetor[2], \"|\", grade[1][0] + grade[1][1]);\n\
      fim";

    execute(source, &mut io).expect("indexed assignments should execute");

    assert_eq!(io.output, "7|9|9\n");
}

#[test]
fn zero_initializes_vectors_and_matrices() {
    let mut io = Io::default();
    let source = "algoritmo inicializacao;\n\
      variáveis numeros: matriz[1] de inteiros; decimais: matriz[1] de reais;\n\
        logicos: matriz[1] de lógicos; textos: matriz[1] de literais; fim-variáveis\n\
      início imprima(numeros[0], \"|\", decimais[0], \"|\", logicos[0], \"|\", textos[0]); fim";

    execute(source, &mut io).expect("arrays should start with the zero value");

    assert_eq!(io.output, "0|0.00|falso|(nulo)\n");
}

#[test]
fn reports_each_out_of_bounds_dimension() {
    for (source, dimension) in [
        (
            "algoritmo vetor; variáveis a: matriz[2] de inteiros; fim-variáveis início imprima(a[2]); fim",
            1,
        ),
        (
            "algoritmo grade; variáveis a: matriz[2][3] de inteiros; fim-variáveis início imprima(a[1][3]); fim",
            2,
        ),
    ] {
        let mut io = Io::default();
        let diagnostics = execute(source, &mut io).expect_err("out-of-bounds index must fail");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
        assert!(
            diagnostics[0]
                .message
                .contains(&format!("dimension {dimension}"))
        );
    }
}

#[test]
fn rejects_array_element_limit() {
    for source in [
        "algoritmo limite; variáveis a: matriz[2][2] de inteiros; fim-variáveis início fim",
        "algoritmo limite; variáveis a,b: matriz[2] de inteiros; fim-variáveis início fim",
        "algoritmo limite; início f(); fim função f() a,b: matriz[2] de inteiros; início fim",
    ] {
        let mut io = Io::default();
        let diagnostics = execute_with_options(
            source,
            &mut io,
            ExecutionOptions {
                max_array_elements: 3,
                ..ExecutionOptions::default()
            },
        )
        .expect_err("all array allocations must obey the configured limit");

        assert!(diagnostics[0].message.contains("array element limit"));
    }
}

#[test]
fn rejects_whole_matrix_assignment_and_output() {
    let mut io = Io::default();
    for source in [
        "algoritmo atribuicao; variáveis a,b: matriz[2] de inteiros; fim-variáveis início a := b; fim",
        "algoritmo saida; variáveis a: matriz[2] de inteiros; fim-variáveis início imprima(a); fim",
    ] {
        let diagnostics = execute(source, &mut io).expect_err("whole matrix use must be rejected");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    }
}

#[test]
fn passes_arrays_to_functions_by_value() {
    let mut io = Io::default();
    let source = "algoritmo valor;\n\
      variáveis dados: matriz[2] de inteiros; fim-variáveis\n\
      início\n\
        dados[0] := 10; altera(dados); imprima(dados[0]);\n\
      fim\n\
      função altera(copia: matriz[2] de inteiros)\n\
      início copia[0] := 99; fim";

    execute(source, &mut io).expect("array arguments should be copied");

    assert_eq!(io.output, "10\n");
}

#[test]
fn compatible_array_parameters_use_the_declared_element_type() {
    let mut io = Io::default();
    let source = "algoritmo elemento_compativel;\n\
      variáveis integers: matriz[1] de inteiros; fim-variáveis\n\
      início integers[0] := 5; imprima(halve(integers)); fim\n\
      função halve(values: matriz[1] de reais): real início retorne values[0] / 2; fim";

    execute(source, &mut io).expect("compatible numeric arrays should reach function parameters");

    assert_eq!(io.output, "2.50\n");
}

#[test]
fn creates_local_function_scopes_and_allows_recursion() {
    let mut io = Io::default();
    let source = "algoritmo recursao;\n\
      variáveis n, resultado: inteiro; fim-variáveis\n\
      início n := 5; resultado := fatorial(n); imprima(n, \"|\", resultado); fim\n\
      função fatorial(n: inteiro): inteiro\n\
        parcial: inteiro;\n\
      início\n\
        parcial := n;\n\
        se n <= 1 então retorne 1; senão retorne parcial * fatorial(n - 1); fim-se\n\
      fim";

    execute(source, &mut io).expect("recursive functions should use local scopes");

    assert_eq!(io.output, "5|120\n");
}

#[test]
fn rejects_nested_functions_and_reference_parameters() {
    for source in [
        "algoritmo aninhada; início fim função externa() função interna() início fim início fim",
        "algoritmo referencia; início fim função muda(&valor: inteiro) início retorne; fim",
    ] {
        let diagnostics = potoo_gportugol_interpreter::parse(source)
            .expect_err("nested functions and reference parameters are not in the grammar");
        assert_eq!(diagnostics[0].kind, DiagnosticKind::Syntax);
    }
}

#[test]
fn enforces_call_depth_limit() {
    let mut io = Io::default();
    let source = "algoritmo profundidade; início imprima(repete(1)); fim\n\
      função repete(n: inteiro): inteiro início retorne repete(n + 1); fim";
    let diagnostics = execute_with_options(
        source,
        &mut io,
        ExecutionOptions {
            max_call_depth: 3,
            ..ExecutionOptions::default()
        },
    )
    .expect_err("recursive calls must respect the configured maximum");

    assert!(diagnostics[0].message.contains("call depth limit"));
}

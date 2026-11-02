use potoo_visualg_interpreter::{
    DiagnosticKind, ExecutionOptions, RuntimeIo, execute, execute_with_options,
};

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
        Err("unexpected input request".to_owned())
    }
}

#[test]
fn executes_nested_conditionals_and_choice_groups() {
    let selected = r#"algoritmo "escolha"
var
  valor: inteiro
inicio
  valor <- 2
  se valor = 2 entao
    se verdadeiro entao
      escreval("COND")
    fimse
  senao
    escreval("OUTRA COND")
  fimse
  escolha valor
  caso 1, 2
    escreval("CASO")
  caso 3
    escreval("TRES")
  outrocaso
    escreval("PADRAO")
  fimescolha
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(selected, &mut io).expect("nested if and grouped cases should execute");

    assert_eq!(io.output, "COND\nCASO\n");

    let fallback = "algoritmo \"padrao\"\nvar\nx: inteiro\ninicio\nx <- 8\nescolha x\ncaso 1\nescreval(\"UM\")\noutrocaso\nescreval(\"OUTRO\")\nfimescolha\nfimalgoritmo\n";
    let mut io = TestIo::default();
    execute(fallback, &mut io).expect("the default choice branch should execute");
    assert_eq!(io.output, "OUTRO\n");
}

#[test]
fn executes_while_and_both_repeat_forms() {
    let source = r#"algoritmo "repeticao"
var
  valor: inteiro
inicio
  valor <- 0
  enquanto valor < 3 faca
    valor <- valor + 1
  fimenquanto
  escreva(valor)
  repita
    valor <- valor - 1
  ate valor = 0
  escreva(valor)
  repita
    valor <- valor + 1
    interrompa
  fimrepita
  escreval(valor)
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("while and repeat forms should execute");

    assert_eq!(io.output, " 3 0 1\n");
}

#[test]
fn executes_counted_loops_in_both_directions_and_breaks_only_the_inner_loop() {
    let source = r#"algoritmo "para"
var
  i: inteiro
  j: inteiro
  total: inteiro
inicio
  para i de 1 ate 3 faca
    escreva(i)
  fimpara
  escreva("|")
  para i de 3 ate 1 passo -1 faca
    escreva(i)
  fimpara
  total <- 0
  para i de 1 ate 3 faca
    para j de 1 ate 4 faca
      total <- total + 1
      interrompa
    fimpara
  fimpara
  escreva("|", total)
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("counted loops should execute in both directions");

    assert_eq!(io.output, " 1 2 3| 3 2 1| 3");
}

#[test]
fn calls_procedures_and_functions_with_value_and_reference_parameters() {
    let source = r#"algoritmo "rotinas"
var
  original: inteiro
  destino: inteiro
procedimento alterar(valor: inteiro; var saida: inteiro)
var
  temporario: inteiro
inicio
  temporario <- valor
  valor <- 99
  saida <- temporario + 1
fimprocedimento
funcao somar(a, b: inteiro): inteiro
inicio
  retorne a + b
fimfuncao
inicio
  original <- 1
  destino <- 7
  alterar(original, destino)
  escreva(original, "|", destino, "|", somar(original, destino))
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("procedure and function calls should execute");

    assert_eq!(io.output, " 1| 2| 3");
}

#[test]
fn supports_recursive_factorial() {
    let source = r#"algoritmo "fatorial"
funcao fatorial(n: inteiro): inteiro
inicio
  se n <= 1 entao
    retorne 1
  senao
    retorne n * fatorial(n - 1)
  fimse
fimfuncao
inicio
  escreva(fatorial(5))
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("recursive calls should return a value");

    assert_eq!(io.output, " 120");
}

#[test]
fn evaluates_all_routine_arguments_in_the_callers_scope() {
    let source = r#"algoritmo "argumentos"
var
  x: inteiro
procedimento mostrar(x, y: inteiro)
inicio
  escreva(x, "|", y)
fimprocedimento
inicio
  x <- 8
  mostrar(1, x)
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("the procedure call should execute");

    assert_eq!(io.output, " 1| 8");
}

#[test]
fn reports_zero_step_invalid_return_interrompa_outside_loop_and_call_depth() {
    let invalid_step = "algoritmo \"passo\"\nvar\ni: inteiro\ninicio\npara i de 1 ate 2 passo 0 faca\nescreval(i)\nfimpara\nfimalgoritmo\n";
    assert_runtime_error(invalid_step, 5, 1);

    let invalid_return = "algoritmo \"retorno\"\nprocedimento p()\ninicio\n  retorne 1\nfimprocedimento\ninicio\n  p()\nfimalgoritmo\n";
    assert_runtime_error(invalid_return, 4, 3);

    let invalid_break = "algoritmo \"interrompa\"\ninicio\n  interrompa\nfimalgoritmo\n";
    assert_runtime_error(invalid_break, 3, 3);

    let too_deep = r#"algoritmo "profundidade"
funcao contar(n: inteiro): inteiro
inicio
  retorne contar(n + 1)
fimfuncao
inicio
  escreva(contar(0))
fimalgoritmo
"#;
    let diagnostics = execute(too_deep, &mut TestIo::default())
        .expect_err("calls should stop at the configured depth");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
    assert!(diagnostics[0].message.contains("profundidade"));
}

fn assert_runtime_error(source: &str, line: usize, column: usize) {
    let diagnostics = execute(source, &mut TestIo::default())
        .expect_err("the invalid program should return a runtime diagnostic");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (line, column));
}

#[test]
fn passes_arrays_by_value_and_reference_to_routines() {
    let source = r#"algoritmo "vetores em rotinas"
var
  dados: vetor[1..2] de inteiro
procedimento alterar_copia(valores: vetor[1..2] de inteiro)
inicio
  valores[1] <- 7
fimprocedimento
procedimento alterar_referencia(var valores: vetor[1..2] de inteiro)
inicio
  valores[2] <- 9
fimprocedimento
inicio
  alterar_copia(dados)
  alterar_referencia(dados)
  escreva(dados[1], "|", dados[2])
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("array parameters should support value and reference modes");

    assert_eq!(io.output, " 0| 9");
}

#[test]
fn reference_parameters_do_not_allocate_duplicate_array_storage() {
    let source = r#"algoritmo "parametro var"
var
  valores: vetor[1..2] de inteiro
procedimento alterar(var itens: vetor[1..2] de inteiro)
inicio
  itens[1] <- 7
fimprocedimento
inicio
  alterar(valores)
  escreva(valores[1])
fimalgoritmo
"#;
    let mut options = ExecutionOptions::default();
    options.max_variable_slots = 2;
    let mut io = TestIo::default();

    execute_with_options(source, &mut io, options)
        .expect("a reference parameter should reuse the caller's array slots");

    assert_eq!(io.output, " 7");
}

#[test]
fn counted_loops_finish_at_integer_boundaries_without_overflowing() {
    let source = r#"algoritmo "limites"
var
  i: inteiro
inicio
  para i de 9223372036854775807 ate 9223372036854775807 faca
    escreva(i)
  fimpara
  escreva("|")
  para i de -9223372036854775807 - 1 ate -9223372036854775807 - 1 passo -1 faca
    escreva(i)
  fimpara
fimalgoritmo
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("counted loops should stop after their inclusive bound");

    assert_eq!(io.output, " 9223372036854775807| -9223372036854775808");
}

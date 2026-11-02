use std::collections::VecDeque;

use potoo_potigol_interpreter::{
    ControlAction, DataType, DiagnosticKind, ExecutionEvent, ExecutionOptions, InputPrompt,
    RuntimeIo, check, execute, execute_with_options, parse,
};

#[derive(Debug, Default)]
struct TestIo {
    input: VecDeque<String>,
    output: String,
    events: Vec<ExecutionEvent>,
    input_types: Vec<DataType>,
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String> {
        self.input_types.push(prompt.data_type);
        self.input
            .pop_front()
            .ok_or_else(|| "unexpected input request".to_owned())
    }

    fn on_execution_event(&mut self, event: &ExecutionEvent) -> Result<ControlAction, String> {
        self.events.push(event.clone());
        Ok(ControlAction::Continue)
    }
}

fn run(source: &str, input: &[&str]) -> Result<TestIo, Vec<potoo_potigol_interpreter::Diagnostic>> {
    let mut io = TestIo {
        input: input.iter().map(|line| (*line).to_owned()).collect(),
        ..TestIo::default()
    };
    execute(source, &mut io)?;
    Ok(io)
}

#[test]
fn official_imperative_input_and_interpolated_output_example() {
    let io = run(
        "a, b = leia_inteiro\nescreva \"{a} + {b} = {a + b}\"",
        &["2", "3"],
    )
    .unwrap();

    assert_eq!(io.output, "2 + 3 = 5\n");
}

#[test]
fn official_higher_order_list_operations_and_currying() {
    let source = r#"
dobro(n: Inteiro) = n * 2
soma(a: Inteiro) = ((b: Inteiro) => a + b)
numeros = [1, 2, 3, 4, 5, 6]
pares = numeros.selecione(n => n mod 2 == 0)
dobrados = pares.mapeie(dobro)
total = dobrados.injete(0)((a, b) => a + b)
escreva "{total} {soma(2)(3)}"
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "24 5\n");
}

#[test]
fn official_pattern_matching_and_list_comprehension() {
    let source = r#"
fatorial(n: Inteiro): Inteiro = escolha n
  caso 0 => 1
  caso valor => valor * fatorial(valor - 1)
fim
quadrados = para i de 1 até 4 gere i * i fim
escreva "{fatorial(5)} {quadrados}"
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "120 [1, 4, 9, 16]\n");
}

#[test]
fn official_class_example_constructs_objects_and_calls_methods() {
    let source = r#"
tipo Quadrado
  lado: Inteiro
  area() = lado * lado
fim
q = Quadrado(10)
escreva q.area
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "100\n");
}

#[test]
fn rc1_abstract_type_inheritance_example() {
    let source = r#"
tipo abstrato Pessoa
  nome: Texto
  sexo: Caractere
  var email: Texto
fim

tipo Homem: Pessoa
  sexo = 'M'
fim

h1 = Homem("Joao", "Joao@gmail.com")
escreva "{h1.nome} {h1.sexo} {h1.email}"
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "Joao M Joao@gmail.com\n");
}

#[test]
fn inherited_methods_and_subtypes_work_through_base_type_parameters() {
    let source = r#"
tipo abstrato Figura
  descrição() = "figura"
fim

tipo Quadrado: Figura
  lado: Real
  descrição() = "quadrado"
fim

descrever(figura: Figura) = figura.descrição
q = Quadrado(3.0)
escreva descrever(q)
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "quadrado\n");
}

#[test]
fn abstract_types_cannot_be_instantiated() {
    let errors = run(
        "tipo abstrato Forma\n  area: Inteiro\nfim\nf = Forma(1)",
        &[],
    )
    .unwrap_err();
    assert!(errors[0].message.contains("não pode ser instanciado"));
}

#[test]
fn list_cons_pattern_and_placeholder_quicksort_work() {
    let source = r#"
quicksort(numeros: Lista[Inteiro]): Lista[Inteiro] = escolha numeros
  caso [] => []
  caso pivo::resto =>
    menores = resto.selecione( _ <= pivo )
    maiores = resto.selecione( _ > pivo )
    quicksort(menores) + pivo::quicksort(maiores)
fim
escreva quicksort([4, 1, 3, 2, 1])
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "[1, 1, 2, 3, 4]\n");
}

#[test]
fn official_loop_and_mutable_list_examples_work() {
    let source = r#"
var total := 0
para i de 1 até 5 faça
  total := total + i
fim
var valores := Lista.mutável(3, 0)
valores[2] := total
escreva valores
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "[0, 15, 0]\n");
}

#[test]
fn while_and_iterable_loops_work() {
    let source = r#"
var total := 0
var i := 1
enquanto i <= 3 faça
  total := total + i
  i := i + 1
fim
para item em [4, 5] faça
  total := total + item
fim
escreva total
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "15\n");
}

#[test]
fn static_diagnostics_keep_source_locations() {
    let errors = check(&parse("valor = ausente + 1").unwrap()).unwrap_err();
    assert_eq!(errors[0].span.start.line, 1);
    assert!(errors[0].span.start.column > 1);
}

#[test]
fn typed_input_reports_the_requested_type_to_host() {
    let mut io = TestIo {
        input: VecDeque::from(["42".to_owned()]),
        ..TestIo::default()
    };
    execute("idade = leia_inteiro\nescreva idade", &mut io).unwrap();
    assert_eq!(io.output, "42\n");
    assert_eq!(io.input_types, [DataType::Integer]);
}

#[test]
fn official_function_blocks_nested_functions_and_branch_expressions_work() {
    let source = r#"
soma(a, b: Inteiro): Inteiro
  dobro(n: Inteiro) = n * 2
  total = a + b
  retorne dobro(total)
fim
fatorial(n: Inteiro): Inteiro = escolha n
  caso 0 => 1
  caso valor => valor * fatorial(valor - 1)
fim
resultado = se 2 < 3 então "sim" senão "não" fim
escreva "{soma(2, 3)} {fatorial(5)} {resultado}"
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "10 120 sim\n");
}

#[test]
fn official_list_and_text_methods_work() {
    let source = r#"
a = [2, 4, 6, 8, 10]
partes = [2, 2, 3, 3, 3, 6, 5, 6].divida_quando((x, y) => x <> y)
escreva "{a.tamanho} {a.cabeça} {a.cauda} {a.último} {a.pegue(2)} {a.descarte(2)}"
escreva "{a.ordene(- _)} {a.junte(", ")} {a.posição(6)} {a.contém(12)}"
escreva "{partes} {a.pegue_enquanto(n => n < 6)} {a.descarte_enquanto(n => n < 6)}"
texto = "Olá mundo"
escreva "{texto.tamanho} {texto.pegue(3)} {texto.descarte(4)} {texto[1]} {texto.maiúsculo}"
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(
        io.output,
        "5 2 [4, 6, 8, 10] 10 [2, 4] [6, 8, 10]\n[10, 8, 6, 4, 2] 2, 4, 6, 8, 10 3 falso\n[[2, 2], [3, 3, 3], [6], [5], [6]] [2, 4] [6, 8, 10]\n9 Olá mundo O OLÁ MUNDO\n"
    );
}

#[test]
fn official_matrix_cube_empty_list_and_tuple_features_work() {
    let source = r#"
vazia = Lista.vazia[Inteiro]
matriz = Matriz.imutável(2, 2, 0)
cubo = Cubo.imutável(2, 2, 2, "-")
t = (2015, "potigol", 1.0)
a, b = (4, "ok")
escreva "{vazia} {matriz[2][1]} {cubo[1][2][1]} {t.primeiro} {t.segundo} {t.terceiro} {a} {b}"
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "[] 0 - 2015 potigol 1 4 ok\n");
}

#[test]
fn official_list_input_reading_by_count_and_delimiter_works() {
    let io = run(
        "numeros = leia_inteiros(3)\nreais = leia_reais(\",\")\nescreva \"{numeros} {reais}\"",
        &["2", "4", "6", "3.5,2.7"],
    )
    .unwrap();
    assert_eq!(io.output, "[2, 4, 6] [3.5, 2.7]\n");
}

#[test]
fn official_multiple_generators_loop_steps_and_function_values_work() {
    let source = r#"
identidade_inteiro(valor: Inteiro) = valor
aplicar(funcao: Inteiro => Inteiro, valor: Inteiro) = funcao(valor)
sucessor(valor: Inteiro) = valor + 1
inteiro = identidade_inteiro(7)
texto = "ok"
pares = para i de 1 até 3, j de i + 1 até 3 gere i + j fim
var soma := 0
para i de 1 até 5 passo 2 faça
  soma := soma + i
fim
status = se soma == 9 então "certo" senãose soma < 9 então "baixo" senão "alto" fim
escreva "{inteiro} {texto} {pares} {status} {aplicar(sucessor, 5)}"
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "7 ok [3, 4, 5] certo 6\n");
}

#[test]
fn official_formatted_interpolation_multiline_text_and_print_work() {
    let source = r#"
preço = 49.9
imprima "Total: R$ "
escreva "{(preço * 3) formato "%8.2f"}"
texto = """
Olá,
{ "mundo".maiúsculo }
"""
escreva texto
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "Total: R$   149.70\n\nOlá,\nMUNDO\n\n");
}

#[test]
fn type_and_inheritance_errors_are_reported_at_the_source() {
    let errors = run("valor = 1 + verdadeiro", &[]).unwrap_err();
    assert_eq!(errors[0].kind, DiagnosticKind::Type);
    assert_eq!(errors[0].span.start.line, 1);

    let errors = run("tipo A: B\nfim\ntipo B: A\nfim", &[]).unwrap_err();
    assert!(errors[0].message.contains("herança circular"));
}

#[test]
fn execution_options_stop_unbounded_programs() {
    let mut io = TestIo::default();
    let options = ExecutionOptions {
        max_steps: 8,
        ..ExecutionOptions::default()
    };
    let errors = execute_with_options(
        "var i := 0\nenquanto verdadeiro faça\n  i := i + 1\nfim",
        &mut io,
        options,
    )
    .unwrap_err();
    assert!(errors[0].message.contains("limite de passos"));
}

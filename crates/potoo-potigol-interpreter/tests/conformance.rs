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
    stop_after_checks: Option<usize>,
    stop_checks: usize,
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

    fn should_stop(&mut self) -> bool {
        self.stop_checks += 1;
        self.stop_after_checks
            .is_some_and(|limit| self.stop_checks > limit)
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
fn official_basic_types_operators_and_math_examples_work() {
    let source = r#"
resultados = [
  5 + 3 == 8,
  1.5 + 2.3 == 3.8,
  'a' + 1 == 98,
  'a' + 'b' == 195,
  'b' - 1.5 == 96.5,
  'b' - 'a' == 1,
  5 div 3 == 1,
  -5 mod 3 == -2,
  5 ^ 2 == 25.0,
  "ab" + 12 == "ab12",
  "ab" + verdadeiro == "abverdadeiro",
  "abc" - "cde" == "ab",
  "abc" * 3 == "abcabcabc",
  falso < verdadeiro,
  "a" == 'a' == falso,
  5.qual_tipo == "Inteiro",
  5.0.qual_tipo == "Real",
  abs(-2.4) == 2.4,
  raiz(9) == 3.0,
  log10(100) == 2.0,
  PI > 3.14,
  (12345 formato "%8d") == "   12345",
  (123 formato "%04d") == "0123",
  ("abc" formato "%-5s") == "abc  ",
  (12345.678 formato "%.1f") == "12345.7"
]
escreva resultados
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(
        io.output,
        format!("[{}]\n", vec!["verdadeiro"; 25].join(", "))
    );
}

#[test]
fn official_numeric_conversions_and_math_functions_work() {
    let source = r#"
resultados = [
  12345.real == 12345.0,
  12345.texto == "12345",
  97.caractere == 'a',
  12345.678.inteiro == 12345,
  12345.678.texto == "12345.678",
  12345.678.arredonde == 12346,
  12345.678.arredonde(2) == 12345.68,
  12345.678.piso == 12345.0,
  12345.678.teto == 12346.0,
  sen(0) == 0.0,
  cos(0) == 1.0,
  tg(0) == 0.0,
  arcsen(0) == 0.0,
  arccos(1) == 0.0,
  arctg(0) == 0.0,
  abs(-2.4) == 2.4,
  raiz(9) == 3.0,
  log(1) == 0.0,
  log10(100) == 2.0
]
escreva resultados
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(
        io.output,
        format!("[{}]\n", vec!["verdadeiro"; 19].join(", "))
    );
}

#[test]
fn official_random_function_overloads_return_documented_values() {
    let source = r#"
real = aleatório()
inteiro = aleatório(10)
intervalo = aleatório(1, 6)
item = aleatório([2, 4, 6, 8, 10])
escreva real >= 0.0 e real < 1.0
escreva inteiro >= 1 e inteiro <= 10
escreva intervalo >= 1 e intervalo <= 6
escreva [2, 4, 6, 8, 10].contém(item)
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(
        io.output,
        "verdadeiro\nverdadeiro\nverdadeiro\nverdadeiro\n"
    );
}

#[test]
fn official_text_conversion_and_collection_methods_work() {
    let source = r#"
verificacoes = [
  "abc".qual_tipo == "Texto",
  "12abc3".inteiro == 12,
  "abc".inteiro == 0,
  "12a.3".real == 12.0,
  "abc".real == 0.0,
  "abc"[2] == 'b',
  "abc".tamanho == 3,
  "abc".posição('b') == 2,
  "abc".posição('d') == 0,
  "abc".contém('a'),
  não "abc".contém('d'),
  "Abc".maiúsculo == "ABC",
  "Abc".minúsculo == "abc",
  "Abc".inverta == "cbA",
  "cab".ordene == "abc",
  "abc".junte("-") == "a-b-c",
  "abc".junte("[", ", ", "]") == "[a, b, c]",
  "Um texto".divida == ["Um", "texto"],
  "Um texto".divida("t") == ["Um ", "ex", "o"],
  "abc".lista == ['a', 'b', 'c'],
  "abc".cabeça == 'a',
  "abc".cauda == "bc",
  "abc".último == 'c',
  "abcde".pegue(3) == "abc",
  "abcde".descarte(3) == "de",
  "abcb".selecione(letra => letra <> 'c') == "abb",
  "abc".injete(0)((x, y) => x + y) == 294,
  "abc".injete("")((x, y) => x + "-" + y) == "-a-b-c",
  "abcb".descarte_enquanto(letra => letra <> 'c') == "cb",
  "abcb".pegue_enquanto(letra => letra < 'c') == "ab",
  "abc".remova(2) == "ac",
  "abc".insira(3, 'd') == "abdc",
  [2, 4, 6].junte == "246",
  [6, 2, 4].ordene == [2, 4, 6]
]
escreva verificacoes
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(
        io.output,
        format!("[{}]\n", vec!["verdadeiro"; 34].join(", "))
    );
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
fn constructor_patterns_bind_fields_and_match_guards_fall_through() {
    let source = r#"
tipo Ponto
  x, y: Inteiro
fim
p = Ponto(0, 3)
resultado = escolha p
  caso Ponto(0, 0) => "origem"
  caso Ponto(x, y) se x < y => "acima: {x},{y}"
  caso Ponto(x, y) => "ponto: {x},{y}"
fim
escreva resultado
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "acima: 0,3\n");
}

#[test]
fn guarded_match_uses_the_next_case_when_guard_is_false() {
    let source = r#"
resultado = escolha -3
  caso n se n > 0 => "positivo"
  caso n se n < 0 => "negativo: {n}"
  caso _ => "zero"
fim
escreva resultado
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "negativo: -3\n");
}

#[test]
fn official_tail_recursion_example_runs_past_the_call_depth_limit() {
    let source = r#"
h(a, cont: Inteiro): Inteiro = escolha a
  caso 0 => cont
  caso n => h(a - 1, cont + 1)
fim
escreva h(1000, 0)
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "1000\n");
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
fn immutable_list_updates_and_mutable_list_snapshots_are_independent() {
    let source = r#"
original = [2, 4, 6]
atualizada = original.atualize(2, 5)
var valores := original.mutável
snapshot = valores.imutável
valores[1] := 9
escreva "{original} {atualizada} {valores} {snapshot}"
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "[2, 4, 6] [2, 5, 6] [9, 4, 6] [2, 4, 6]\n");
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
fn official_parallel_text_list_input_reads_a_list_for_each_binding() {
    let io = run(
        "nomes, emails = leia_textos(3)\nescreva \"{nomes} {emails}\"",
        &[
            "Ana",
            "Bia",
            "Caio",
            "ana@example.com",
            "bia@example.com",
            "caio@example.com",
        ],
    )
    .unwrap();

    assert_eq!(
        io.output,
        "[Ana, Bia, Caio] [ana@example.com, bia@example.com, caio@example.com]\n"
    );
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
decrescente = para i de 3 até 1 passo -1 gere i fim
status = se soma == 9 então "certo" senãose soma < 9 então "baixo" senão "alto" fim
escreva "{inteiro} {texto} {pares} {decrescente} {status} {aplicar(sucessor, 5)}"
"#;

    let io = run(source, &[]).unwrap();
    assert_eq!(io.output, "7 ok [3, 4, 5] [3, 2, 1] certo 6\n");
}

#[test]
fn official_scalar_input_supports_every_documented_basic_type() {
    let mut io = TestIo {
        input: VecDeque::from([
            "42".to_owned(),
            "3.5".to_owned(),
            "Olá".to_owned(),
            "ç".to_owned(),
            "verdadeiro".to_owned(),
        ]),
        ..TestIo::default()
    };
    execute(
        "i = leia_inteiro\nr = leia_real\nt = leia_texto\nc = leia_caractere\nl = leia_logico\nescreva \"{i} {r} {t} {c} {l}\"",
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "42 3.5 Olá ç verdadeiro\n");
    assert_eq!(
        io.input_types,
        [
            DataType::Integer,
            DataType::Real,
            DataType::Text,
            DataType::Character,
            DataType::Logical,
        ]
    );
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
fn documented_decimal_format_uses_the_potigol_rounding_result() {
    let io = run(
        r#"escreva 123.45 formato "%.1f"
escreva 123.55 formato "%.1f"
escreva 123.46 formato "%.1f"
escreva (-123.45) formato "%.1f"
escreva 2.5 formato "%.0f"
escreva 3.5 formato "%.0f"
escreva 9.95 formato "%.1f""#,
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "123.4\n123.6\n123.5\n-123.4\n2\n4\n10.0\n");
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

#[test]
fn collection_byte_limit_stops_repeated_large_values() {
    let mut io = TestIo::default();
    let options = ExecutionOptions {
        max_collection_bytes: 4096,
        ..ExecutionOptions::default()
    };

    let errors =
        execute_with_options("itens = Lista(2, \"x\" * 4096)", &mut io, options).unwrap_err();
    assert!(errors[0].message.contains("limite de memória de coleções"));

    let options = ExecutionOptions {
        max_collection_bytes: 4096,
        ..ExecutionOptions::default()
    };
    let errors = execute_with_options(
        "itens = para i de 1 até 5 gere \"x\" * 1000 fim",
        &mut TestIo::default(),
        options,
    )
    .unwrap_err();
    assert!(errors[0].message.contains("limite de memória de coleções"));

    let options = ExecutionOptions {
        max_collection_bytes: 4096,
        ..ExecutionOptions::default()
    };
    let errors = execute_with_options(
        "valores = [1, 2]\nordenados = valores.ordene(chave => \"x\" * 4096)",
        &mut TestIo::default(),
        options,
    )
    .unwrap_err();
    assert!(errors[0].message.contains("limite de memória de coleções"));

    let errors = execute_with_options(
        "Matriz.imutável(2, 2, \"x\" * 4096)",
        &mut TestIo::default(),
        options,
    )
    .unwrap_err();
    assert!(errors[0].message.contains("limite de memória de coleções"));

    let errors = execute_with_options(
        "valores = [1, 2]\nmap = valores.mapeie(x => \"x\" * 4096)",
        &mut TestIo::default(),
        options,
    )
    .unwrap_err();
    assert!(errors[0].message.contains("limite de memória de coleções"));

    let options = ExecutionOptions {
        max_collection_bytes: 1024,
        ..ExecutionOptions::default()
    };
    let errors = execute_with_options(
        "itens = (\"x\" * 2048).divida(\",\")",
        &mut TestIo::default(),
        options,
    )
    .unwrap_err();
    assert!(errors[0].message.contains("limite de memória de coleções"));
}

#[test]
fn cancellation_interrupts_default_list_sorting() {
    let mut io = TestIo {
        stop_after_checks: Some(6),
        ..TestIo::default()
    };
    let errors = execute("escreva Lista(1000, 0).ordene", &mut io).unwrap_err();

    assert!(errors[0].message.contains("execução interrompida"));
    assert!(io.output.is_empty());
}

#[test]
fn cancellation_interrupts_text_sorting() {
    let mut io = TestIo {
        stop_after_checks: Some(5),
        ..TestIo::default()
    };
    let errors = execute("escreva (\"z\" * 100000).ordene", &mut io).unwrap_err();

    assert!(errors[0].message.contains("execução interrompida"));
    assert!(io.output.is_empty());
}

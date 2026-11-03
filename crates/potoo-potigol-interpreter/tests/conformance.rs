use std::collections::VecDeque;

use potoo_potigol_interpreter::{
    ControlAction, DataType, DiagnosticKind, ExecutionEvent, ExecutionOptions, InputPrompt,
    RuntimeIo, check, execute, execute_with_options, lex, parse, parse_with_options,
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
  'a' + "b" == "ab",
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
    assert_eq!(io.output, format!("[{}]\n", vec!["true"; 26].join(", ")));
}

#[test]
fn integer_literals_arithmetic_and_input_match_scala_int32() {
    let io = run(
        "escreva 2147483647\nescreva 2147483647 + 1\nescreva 2147483647 * 2\nescreva -2147483648\nescreva -2147483648 - 1\nescreva -2147483648 div -1",
        &[],
    )
    .unwrap();
    assert_eq!(
        io.output,
        "2147483647\n-2147483648\n-2\n-2147483648\n2147483647\n-2147483648\n"
    );

    assert!(run("escreva 2147483648", &[]).is_err());
    assert!(run("x = leia_inteiro", &["2147483648"]).is_err());
    assert_eq!(
        run("escreva BigInt(\"2147483648\").inteiro", &[])
            .unwrap()
            .output,
        "-2147483648\n"
    );
}

#[test]
fn potigol_text_uses_utf16_code_units_for_size_iteration_and_order() {
    let io = run(
        "escreva \"😀\".tamanho\nescreva \"😀\".lista.tamanho\nescreva \"😀a\".posicao('a')\nvar total := 0\npara caractere em \"😀\" faça\n total := total + 1\nfim\nescreva total\nescreva \"😀\" < \"\"",
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "2\n2\n3\n2\nverdadeiro\n");
}

#[test]
fn conversions_and_case_class_values_match_the_reference() {
    let io = run(
        r#"
escreva " +12x".inteiro
escreva " 12".inteiro
escreva "+12".inteiro
escreva "1e2".real
escreva 65537.caractere.inteiro
escreva (-1).caractere.inteiro
tipo P
  x: Inteiro
fim
a = P(1)
b = P(1)
escreva a
escreva a == b
f(): Nada
  escreva 1
fim
escreva f()
"#,
        &[],
    )
    .unwrap();

    assert_eq!(
        io.output,
        "0\n0\n0\n1.0\n1\n65535\nP(1)\nverdadeiro\n1\n()\n"
    );
}

#[test]
fn real_rounding_matches_scala_math_round_at_negative_ties() {
    let io = run(
        "escreva (-1.5).arredonde\nescreva (-1.25).arredonde(1)\nescreva 1.25.arredonde(1)",
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "-1\n-1.2\n1.3\n");
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
    assert_eq!(io.output, format!("[{}]\n", vec!["true"; 19].join(", ")));
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
    assert_eq!(io.output, format!("[{}]\n", vec!["true"; 34].join(", ")));
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
    assert_eq!(io.output, "[2, 4, 6] [2, 4, 5] [9, 4, 6] [2, 4, 6]\n");
}

#[test]
fn collection_operations_and_factories_match_potigol() {
    let io = run(
        r#"
escreva [1, 2, 1] - [1]
escreva [1, 2].zip([3, 4]) == [(1, 3), (2, 4)]
escreva [1, 2].zip("ab") == [(1, 'a'), (2, 'b')]
escreva [1, 2].ache(x => x > 1)
escreva [1, 2].get(1)
escreva [1, 2].ache(x => x > 3)
escreva "ab".ache(x => x == 'a')
escreva [1, 2].reduza((x: Inteiro, y: Inteiro) => x + y)
escreva [1, 2].lista
escreva [1, 2].para_lista
escreva Lista(2, 2, 0)
escreva Matriz.mutável(2, 2, 0).qual_tipo
escreva raiz(8, 3)
"#
        .trim(),
        &[],
    )
    .unwrap();

    assert_eq!(
        io.output,
        "[2, 1]\nverdadeiro\nverdadeiro\nSome(2)\n1\nNone\nSome(a)\n3\n[1, 2]\n[1, 2]\n[[0, 0], [0, 0]]\nVetor\n2.0\n"
    );
}

#[test]
fn mutable_collection_transforms_copy_but_atualize_uses_zero_based_index() {
    let io = run(
        r#"
xs = Lista.mutável(2, 1)
removida = xs.remova(1)
inserida = xs.insira(1, 9)
mapa = xs.mapeie(x => x + 1)
cauda = xs.cauda
escreva xs
escreva removida
escreva inserida
escreva mapa.qual_tipo
escreva cauda.qual_tipo
escreva [10, 20].atualize(0, 99)
escreva [10, 20].atualize(0, x => x + 1)
var alteravel := Lista.mutável(2, 10)
alteravel.atualize(0, 99)
escreva alteravel
"#
        .trim(),
        &[],
    )
    .unwrap();

    assert_eq!(
        io.output,
        "[1, 1]\n[1]\n[9, 1, 1]\nVetor\nVetor\n[99, 20]\n[11, 20]\n[99, 10]\n"
    );
}

#[test]
fn list_factory_evaluates_its_initializer_once_per_element() {
    let io = run(
        r#"
var i := 0
proximo(): Inteiro
  i := i + 1
  retorne i
fim
escreva Lista(3, proximo())
escreva i
escreva Lista.mutável(2, proximo())
escreva Matriz.mutável(2, 2, proximo())
escreva i
escreva Lista(-1, 0)
"#
        .trim(),
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "[1, 2, 3]\n3\n[4, 5]\n[[6, 7], [8, 9]]\n9\n[]\n");
}

#[test]
fn counted_numeric_input_consumes_tokens_on_one_or_multiple_lines() {
    let one_line = run("numeros = leia_inteiros(3)\nescreva numeros", &["1 2 3"]).unwrap();
    assert_eq!(one_line.output, "[1, 2, 3]\n");

    let multiple_lines = run("numeros = leia_inteiros(3)\nescreva numeros", &["1 2", "3"]).unwrap();
    assert_eq!(multiple_lines.output, "[1, 2, 3]\n");

    let prefixes = run(
        "inteiro = leia_inteiro\nreal = leia_real\nescreva \"{inteiro} {real}\"",
        &["12abc", "1.5xyz"],
    )
    .unwrap();
    assert_eq!(prefixes.output, "12 1.5\n");
}

#[test]
fn text_split_uses_regex_and_reference_whitespace_rules() {
    let io = run(
        r#"
escreva "a1b2c".divida("[0-9]")
escreva " a  b ".divida()
escreva "a,b,".divida(",")
"#
        .trim(),
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "[a, b, c]\n[, a, b]\n[a, b]\n");

    let inputs = run("partes = leia_textos(\",;\")\nescreva partes", &["a,b;c"]).unwrap();
    assert_eq!(inputs.output, "[a, b, c]\n");
}

#[test]
fn contextual_collection_literals_widen_and_infer_nested_empty_lists() {
    let io = run(
        r#"
a: Lista[Inteiro] = []
b: Lista[Real] = [1, 2]
c: Lista[Lista[Inteiro]] = [[]]
tamanho(xs: Lista[Inteiro]): Inteiro = xs.tamanho
tamanho_zero = tamanho([])
lista_mista = 1.5 :: [1, 2]
escreva a
escreva b
escreva c
escreva tamanho_zero
escreva lista_mista
escreva [1, verdadeiro]
escreva se verdadeiro então 1 senão falso fim
"#
        .trim(),
        &[],
    )
    .unwrap();

    assert_eq!(
        io.output,
        "[]\n[1.0, 2.0]\n[[]]\n0\n[1.5, 1.0, 2.0]\n[1, true]\n1\n"
    );
}

#[test]
fn nested_output_matches_scala_real_tuple_and_case_class_display() {
    let io = run(
        r#"
escreva 1.0
escreva (1, "a")
tipo P
  x, y: Inteiro
fim
escreva P(1, 2)
"#
        .trim(),
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "1.0\n(1,a)\nP(1,2)\n");
}

#[test]
fn incompatible_function_parameter_variance_is_rejected() {
    let source = r#"
f(g: Real => Real): Real = g(1.5)
h(x: Inteiro): Inteiro = x + 1
escreva f(h)
"#;

    let errors = check(&parse(source).unwrap()).unwrap_err();
    assert!(errors[0].message.contains("tipos incompatíveis"));
}

#[test]
fn lexical_functions_shadow_builtins_and_builtins_are_first_class() {
    let io = run(
        r#"
raiz(x: Inteiro): Inteiro = x + 1
f(g: Real => Real): Real = g(0.0)
escreva raiz(9)
escreva f(cos)
"#
        .trim(),
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "10\n1.0\n");
}

#[test]
fn subtype_constructors_match_patterns_against_base_typed_values() {
    let io = run(
        r#"
tipo abstrato A
  x: Inteiro
fim
tipo B: A
  x: Inteiro
fim
a: A = B(1)
escreva escolha a
  caso B(x) => x
  caso _ => 0
fim
"#
        .trim(),
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "1\n");
}

#[test]
fn reference_type_aliases_cover_numbers_matrices_cubes_and_vectors() {
    let io = run(
        r#"
a: Número = 1.5
b: Numero = 2.5
c: InteiroGrande = BigInt(3)
matriz_valor: Matriz[Inteiro] = [[4]]
cubo_valor: Cubo[Inteiro] = [[[5]]]
f: Vetor[Inteiro] = Lista.mutável(1, 6)
escreva a
escreva b
escreva c
escreva matriz_valor
escreva cubo_valor
escreva f.qual_tipo
"#
        .trim(),
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "1.5\n2.5\n3\n[[4]]\n[[[5]]]\nVetor\n");
}

#[test]
fn typed_function_values_are_displayable_and_bad_reductions_are_rejected() {
    let io = run("f: Inteiro => Inteiro = x => x + 1\nescreva f", &[]).unwrap();
    assert_eq!(io.output, "<função>\n");

    let errors = check(&parse("escreva [1, 2].reduza((a: Texto, b: Inteiro) => \"12\")").unwrap())
        .unwrap_err();
    assert!(errors[0].message.contains("tipos incompatíveis"));
}

#[test]
fn default_sort_of_case_classes_is_rejected_by_the_checker() {
    let source = r#"
tipo A
  x: Inteiro
fim
escreva [A(1), A(2)].ordene
"#;

    let errors = check(&parse(source).unwrap()).unwrap_err();
    assert!(errors[0].message.contains("não podem ser ordenados"));

    let errors = check(&parse(&source.replace(".ordene", ".ordene()")).unwrap()).unwrap_err();
    assert!(errors[0].message.contains("não podem ser ordenados"));
}

#[test]
fn text_mapping_and_split_when_match_the_collection_signatures() {
    let io = run(
        "escreva \"abc\".mapeie(c => c)\nescreva \"aabbc\".divida_quando((a, b) => a <> b)\nescreva \"abc\".primeiro",
        &[],
    )
    .unwrap();

    assert_eq!(io.output, "abc\n[aa, bb, c]\na\n");
}

#[test]
fn one_argument_reduce_requires_the_collection_element_as_accumulator() {
    let error = check(&parse("escreva [1, 2].injete((a: Texto, b: Inteiro) => a + b)").unwrap())
        .unwrap_err();
    assert!(error[0].message.contains("tipos incompatíveis"));
}

#[test]
fn optional_val_and_type_alias_declarations_are_supported() {
    let io = run(
        "val inteiro = 3\ntipo Contagem = Inteiro\nvalor: Contagem = inteiro\nescreva valor",
        &[],
    )
    .unwrap();
    assert_eq!(io.output, "3\n");
}

#[test]
fn optional_entao_same_line_statements_and_standalone_blocks_work() {
    let io = run(
        "escreva se verdadeiro 1 senão 2 fim\nescreva 3 escreva 4\nfaça\nescreva 5\nfim",
        &[],
    )
    .unwrap();
    assert_eq!(io.output, "1\n3\n4\n5\n");
}

#[test]
fn lambda_and_comprehension_bodies_accept_instruction_lists() {
    let io = run(
        "f = (x: Inteiro) =>\n  escreva x\n  x + 1\nescreva f(2)\nvalores = para i de 1 até 2 gere\n  proximo = i + 1\n  proximo * 2\nfim\nescreva valores",
        &[],
    )
    .unwrap();
    assert_eq!(io.output, "2\n3\n[4, 6]\n");
}

#[test]
fn statement_initial_para_gere_is_a_comprehension() {
    let parsed = parse("para i de 1 até 2 gere i fim\nescreva 1").unwrap();
    assert_eq!(parsed.statements.len(), 2);
}

#[test]
fn reference_operator_precedence_and_newline_continuation_are_preserved() {
    let io = run(
        "identidade(x: Inteiro) = x\nescreva 1 + 2 formato \"%d\"\nescreva (1\n)\nescreva 1\n  .texto\nescreva identidade\n  (2)\nescreva [4, 5]\n  [1]\nescreva 1 + 2\n  formato \"%d\"",
        &[],
    )
    .unwrap();
    assert_eq!(io.output, "3\n1\n1\n2\n4\n3\n");

    assert!(run("escreva 2 * 3 :: [4]", &[]).is_err());
    assert!(run("escreva -1 :: [2]", &[]).is_err());
}

#[test]
fn typed_patterns_multiple_parents_and_abstract_methods_parse() {
    let io = run(
        "escreva escolha 1 caso n: Inteiro => n fim\ntipo A\n  x: Inteiro\nfim\ntipo B\n  y: Texto\nfim\ntipo C: A, B\nfim\nc = C(2, \"ok\")\nescreva \"{c.x} {c.y}\"",
        &[],
    )
    .unwrap();
    assert_eq!(io.output, "1\n2 ok\n");

    assert!(parse("tipo abstrato F\n  calcula(x: Inteiro): Inteiro\nfim").is_ok());
}

#[test]
fn lexer_matches_reference_number_comment_and_multiline_string_rules() {
    assert!(run("escreva 1e2", &[]).is_err());
    assert!(parse("escreva 1 // comentário").is_err());
    assert!(parse("escreva 1 # comentário").is_err());
    assert!(parse("escreva 1; escreva 2").is_err());
    assert!(parse("escreva \"\"\"texto\"\"\"").is_err());
    assert!(parse("use \"jerimum\"\nescreva 1").is_ok());
    assert!(parse("escreva 1 # comentário\n").is_ok());

    let io = run("escreva \"a\nb\"", &[]).unwrap();
    assert_eq!(io.output, "a\nb\n");

    let io = run("escreva \"a\r\nb\"", &[]).unwrap();
    assert_eq!(io.output, "a\r\nb\n");
    assert!(lex("'\\n'").is_err());
    assert!(lex("escreva \u{1f600}").is_err());
    assert!(lex("escreva a©\n").is_ok());
    assert!(parse("val então = 1").is_err());
}

#[test]
fn inferred_function_blocks_and_expression_control_forms_work() {
    let io = run(
        "soma(x: Inteiro)\nx + 1\nfim\nidentidade(x: Inteiro) x fim\nescreva soma(2)\nescreva identidade(4)\nvalor = faça\n  7\nfim\nescreva valor",
        &[],
    )
    .unwrap();
    assert_eq!(io.output, "3\n4\n7\n");

    let io = run(
        "para i de 1 até 2 gere escreva i fim\nescreva enquanto falso faça 1 fim",
        &[],
    )
    .unwrap();
    assert_eq!(io.output, "1\n2\n()\n");
}

#[test]
fn optional_val_works_in_fields_and_typed_parameters() {
    let io = run(
        "soma(val a, val b: Inteiro): Inteiro = a + b\ninc = (val x: Inteiro) => x + 1\ntipo Caixa\n  val conteudo: Texto\nfim\ncaixa = Caixa(\"ok\")\nescreva \"{soma(1, 2)} {inc(2)} {caixa.conteudo}\"",
        &[],
    )
    .unwrap();
    assert_eq!(io.output, "3 3 ok\n");
}

#[test]
fn multiple_inheritance_merges_fields_and_dispatches_methods() {
    let io = run(
        "tipo A\n  val x: Inteiro\n  dobro() = x * 2\nfim\ntipo B\n  val y: Texto\n  tamanho() = y.tamanho\nfim\ntipo abstrato C\n  calcula(valor: Inteiro): Inteiro\nfim\ntipo D: A, B, C\n  calcula(valor: Inteiro): Inteiro = valor + x\nfim\nd = D(4, \"abc\")\nescreva \"{d.dobro} {d.tamanho} {d.calcula(3)} {d.x} {d.y}\"",
        &[],
    )
    .unwrap();
    assert_eq!(io.output, "8 3 7 4 abc\n");
}

#[test]
fn interpolation_comments_nesting_guards_and_node_ids_are_consistent() {
    let io = run("escreva \"{1 # } is inside a comment\n + 2}\"", &[]).unwrap();
    assert_eq!(io.output, "3\n");

    let mut expression = "1".to_owned();
    for _ in 0..30 {
        expression = format!("\"{{{expression}}}\"");
    }
    let source = format!("escreva {expression}");
    assert!(
        parse_with_options(
            &source,
            ExecutionOptions {
                max_parse_depth: 8,
                ..ExecutionOptions::default()
            }
        )
        .is_err()
    );

    let program = parse("f(a, b: Lista[Inteiro], c: Real) = a").unwrap();
    let potoo_potigol_interpreter::ast::StmtKind::Function(function) = &program.statements[0].kind
    else {
        panic!("expected function declaration");
    };
    let mut ids = std::collections::HashSet::new();
    for annotation in function
        .parameters
        .iter()
        .filter_map(|parameter| parameter.annotation.as_ref())
    {
        collect_type_ids(annotation, &mut ids);
    }
    assert_eq!(ids.len(), 5);
}

fn collect_type_ids(
    ty: &potoo_potigol_interpreter::ast::TypeExpr,
    ids: &mut std::collections::HashSet<potoo_potigol_interpreter::ast::NodeId>,
) {
    assert!(ids.insert(ty.id));
    match &ty.kind {
        potoo_potigol_interpreter::ast::TypeExprKind::Named(_, args)
        | potoo_potigol_interpreter::ast::TypeExprKind::Tuple(args) => {
            for arg in args {
                collect_type_ids(arg, ids);
            }
        }
        potoo_potigol_interpreter::ast::TypeExprKind::Function(params, result) => {
            for param in params {
                collect_type_ids(param, ids);
            }
            collect_type_ids(result, ids);
        }
    }
}

#[test]
fn flat_operator_chains_obey_the_parse_depth_limit() {
    let source = format!("escreva {}", vec!["1"; 100].join(" + "));
    assert!(
        parse_with_options(
            &source,
            ExecutionOptions {
                max_parse_depth: 16,
                ..ExecutionOptions::default()
            }
        )
        .is_err()
    );
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
    assert_eq!(io.output, "[] 0 - 2015 potigol 1.0 4 ok\n");
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
texto = "
Olá,
{ "mundo".maiúsculo }
"
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
fn nested_identifier_clones_are_charged_to_collection_budget() {
    let mut source = String::from("a0 = [1]\n");
    for index in 1..=8 {
        source.push_str(&format!("a{index} = [a{}, a{}]\n", index - 1, index - 1));
    }
    source.push_str("escreva 1");

    let errors = execute_with_options(
        &source,
        &mut TestIo::default(),
        ExecutionOptions {
            max_collection_bytes: 1024,
            ..ExecutionOptions::default()
        },
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

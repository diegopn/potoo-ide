use std::collections::VecDeque;

use potoo_calango_interpreter::{
    ControlAction, DataType, DiagnosticKind, ExecutionEvent, ExecutionOptions, InputPrompt,
    RuntimeIo, execute, execute_with_options, parse,
};

#[derive(Default)]
struct TestIo {
    output: String,
    inputs: VecDeque<String>,
    input_requests: usize,
    input_types: Vec<DataType>,
    stop_at_line: Option<usize>,
    stop_sent: bool,
    stop_on_poll: Option<usize>,
    stop_polls: usize,
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String> {
        self.input_requests += 1;
        self.input_types.push(prompt.data_type);
        self.inputs
            .pop_front()
            .ok_or_else(|| "unexpected input request".to_owned())
    }

    fn is_debugging(&self) -> bool {
        self.stop_at_line.is_some()
    }

    fn should_stop(&mut self) -> bool {
        self.stop_polls += 1;
        self.stop_on_poll == Some(self.stop_polls)
    }

    fn on_execution_event(&mut self, event: &ExecutionEvent) -> Result<ControlAction, String> {
        if self.stop_at_line == Some(event.location.line) && !self.stop_sent {
            self.stop_sent = true;
            Ok(ControlAction::Stop)
        } else {
            Ok(ControlAction::Continue)
        }
    }
}

fn source(main: &str) -> String {
    format!("algoritmo teste;\nprincipal\n{main}\nfimPrincipal\n")
}

#[test]
fn pi_is_available_without_arguments() {
    let mut io = TestIo::default();

    execute(&source("escreval(pi());"), &mut io).unwrap();

    assert_eq!(io.output, format!("{}\n", std::f64::consts::PI));
}

#[test]
fn leia_caracter_accepts_one_character_target() {
    let mut io = TestIo {
        inputs: ["z".to_owned()].into(),
        ..TestIo::default()
    };

    execute(
        &source("caracter c;\nleiaCaracter(c);\nescreval(c);"),
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "z\n");
    assert_eq!(io.input_requests, 1);
}

#[test]
fn leia_caracter_rejects_multiple_targets() {
    let result = parse(&source("caracter a;\ncaracter b;\nleiaCaracter(a, b);"));

    let diagnostics = result.unwrap_err();
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Syntax);
}

#[test]
fn leia_rejects_character_before_requesting_input() {
    let mut io = TestIo::default();

    let diagnostics = execute(&source("caracter c;\nleia(c);"), &mut io).unwrap_err();

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    assert_eq!(io.input_requests, 0);
}

#[test]
fn leia_still_reads_scalar_values_from_a_line() {
    let mut io = TestIo {
        inputs: ["42".to_owned()].into(),
        ..TestIo::default()
    };

    execute(&source("inteiro n;\nleia(n);\nescreval(n);"), &mut io).unwrap();

    assert_eq!(io.output, "42\n");
    assert_eq!(io.input_types, [DataType::Integer]);
}

#[test]
fn leia_caracter_rejects_non_character_target_before_requesting_input() {
    let mut io = TestIo::default();

    let diagnostics = execute(&source("inteiro n;\nleiaCaracter(n);"), &mut io).unwrap_err();

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    assert_eq!(io.input_requests, 0);
}

#[test]
fn leia_caracter_rejects_character_array_as_a_whole() {
    let mut io = TestIo::default();

    let diagnostics = execute(
        &source("caracter letras[2];\nleiaCaracter(letras);"),
        &mut io,
    )
    .unwrap_err();

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    assert_eq!(io.input_requests, 0);
}

#[test]
fn stop_inside_nested_loop_ends_the_entire_program() {
    let program = source(
        "enquanto verdadeiro faca\nse verdadeiro entao\nescreva(\"x\");\nfimSe\nfimEnquanto\nescreva(\"depois\");",
    );
    let mut io = TestIo {
        stop_at_line: Some(5),
        ..TestIo::default()
    };

    execute_with_options(
        &program,
        &mut io,
        ExecutionOptions {
            max_steps: 20,
            ..ExecutionOptions::default()
        },
    )
    .unwrap();

    assert!(io.output.is_empty());
}

#[test]
fn stop_during_loop_tick_ends_the_entire_program() {
    let mut io = TestIo {
        stop_on_poll: Some(2),
        ..TestIo::default()
    };

    execute(
        &source("enquanto verdadeiro faca\nescreva(\"x\");\nfimEnquanto\nescreva(\"depois\");"),
        &mut io,
    )
    .unwrap();

    assert!(io.output.is_empty());
}

#[test]
fn stop_inside_do_while_and_for_bodies_propagates() {
    let programs = [
        source("faca\nescreva(\"x\");\nenquanto verdadeiro;\nescreva(\"depois\");"),
        source(
            "inteiro i;\npara i de 1 ate 2 passo 1 faca\nescreva(\"x\");\nfimPara\nescreva(\"depois\");",
        ),
    ];

    for program in programs {
        let stop_line = program
            .lines()
            .position(|line| line.contains("escreva(\"x\")"))
            .unwrap()
            + 1;
        let mut io = TestIo {
            stop_at_line: Some(stop_line),
            ..TestIo::default()
        };

        execute_with_options(
            &program,
            &mut io,
            ExecutionOptions {
                max_steps: 20,
                ..ExecutionOptions::default()
            },
        )
        .unwrap();

        assert!(io.output.is_empty());
    }
}

#[test]
fn stop_inside_routine_body_ends_the_calling_program() {
    let program = "algoritmo teste;\nprocedimento escrever()\nescreva(\"x\");\nfimProcedimento\nprincipal\nescrever();\nescreva(\"depois\");\nfimPrincipal\n";
    let mut io = TestIo {
        stop_at_line: Some(3),
        ..TestIo::default()
    };

    execute(program, &mut io).unwrap();

    assert!(io.output.is_empty());
}

#[test]
fn procedure_call_is_rejected_in_value_expression() {
    let program = "algoritmo teste;\nprocedimento executar()\nescreval(\"feito\");\nfimProcedimento\nprincipal\nescreval(executar());\nfimPrincipal\n";
    let mut io = TestIo::default();

    let diagnostics = execute(program, &mut io).unwrap_err();

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Type);
    assert!(io.output.is_empty());
}

#[test]
fn duplicate_parameter_names_are_rejected_at_the_second_name() {
    let program = "algoritmo teste;\nfuncao inteiro somar(inteiro x, inteiro x)\nretorna x;\nfimFuncao\nprincipal\nfimPrincipal\n";

    let diagnostics = parse(program).unwrap_err();

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Syntax);
    assert_eq!((diagnostics[0].line, diagnostics[0].column), (2, 41));
}

#[test]
fn non_finite_real_literal_is_rejected() {
    let diagnostics = parse(&source("escreval(1e999);")).unwrap_err();

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Syntax);
}

#[test]
fn logical_operators_short_circuit() {
    let mut io = TestIo::default();

    execute(
        &source("escreval(falso e (1 / 0 = 0));\nescreval(verdadeiro ou (1 / 0 = 0));"),
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "falso\nverdadeiro\n");
}

#[test]
fn typed_function_call_returns_a_value() {
    let program = "algoritmo teste;\nfuncao inteiro duplicar(inteiro n)\nretorna n * 2;\nfimFuncao\nprincipal\nescreval(duplicar(21));\nfimPrincipal\n";
    let mut io = TestIo::default();

    execute(program, &mut io).unwrap();

    assert_eq!(io.output, "42\n");
}

#[test]
fn arrays_can_be_declared_indexed_and_read() {
    let mut io = TestIo::default();

    execute(
        &source(
            "inteiro valores[2];\nvalores[0] = 3;\nvalores[1] = 4;\nescreval(valores[0] + valores[1]);",
        ),
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "7\n");
}

#[test]
fn choose_runs_the_matching_case() {
    let mut io = TestIo::default();

    execute(
        &source(
            "escolha 2\ncaso 2\nescreval(\"correto\");\noutroCaso\nescreval(\"errado\");\nfimEscolha",
        ),
        &mut io,
    )
    .unwrap();

    assert_eq!(io.output, "correto\n");
}

#[test]
fn character_read_prompt_uses_character_data_type() {
    let mut io = TestIo {
        inputs: ["a".to_owned()].into(),
        ..TestIo::default()
    };

    execute(&source("caracter c;\nleiaCaracter(c);"), &mut io).unwrap();

    assert_eq!(io.input_requests, 1);
    assert_eq!(io.input_types, [DataType::Character]);
}

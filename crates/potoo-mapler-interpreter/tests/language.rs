use std::collections::VecDeque;

use potoo_mapler_interpreter::{
    ControlAction, DiagnosticKind, ExecutionEvent, ExecutionOptions, InputPrompt, PauseReason,
    RuntimeIo, execute, execute_with_options,
};

#[derive(Default)]
struct TestIo {
    input: VecDeque<String>,
    output: String,
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, _prompt: &InputPrompt) -> Result<String, String> {
        self.input
            .pop_front()
            .ok_or_else(|| "no test input".to_owned())
    }
}

#[test]
fn executes_documented_mapler_vectors_loops_conditionals_and_modules() {
    let source = r#"
variaveis
    limite, soma, i: inteiro;
    valores: vetor [1..3] de inteiro;
    verificar: modulo;
inicio
    ler limite;
    soma <- 0;
    para i de 1 ate limite passo 1 faca
        valores[i] <- i;
        soma <- soma + valores[i];
    fim para;
    verificar;
fim.

modulo verificar;
    se soma = 6 entao
        escrever "Soma correta";
    senao
        escrever "Soma incorreta";
    fim se;
fim modulo;
"#;
    let mut io = TestIo {
        input: ["3".to_owned()].into(),
        ..TestIo::default()
    };

    execute(source, &mut io).expect("official Mapler constructs should execute");

    assert_eq!(io.output, "Soma correta\n");
}

#[test]
fn compares_chain_values_in_documented_conditionals() {
    let source = r#"
variaveis
    senha, senha_correta: cadeia;
inicio
    senha <- "12345";
    senha_correta <- "12345";
    se senha = senha_correta entao
        escrever "Senha correta!";
    senao
        escrever "Senha incorreta!";
    fim se;
fim.
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("Mapler compares chain values with '='");

    assert_eq!(io.output, "Senha correta!\n");
}

#[test]
fn rejects_string_literals_over_the_configured_limit() {
    let source = r#"
variaveis
    texto: cadeia;
inicio
    texto <- "ab";
fim
"#;
    let mut io = TestIo::default();
    let options = ExecutionOptions {
        max_string_bytes: 1,
        ..ExecutionOptions::default()
    };

    let errors = execute_with_options(source, &mut io, options).unwrap_err();

    assert_eq!(errors[0].kind, DiagnosticKind::Lexical);
}

#[test]
fn checks_output_limit_before_evaluating_later_values() {
    let source = r#"
variaveis
    nao_inicializada: inteiro;
inicio
    escrever "texto longo", nao_inicializada;
fim
"#;
    let mut io = TestIo::default();
    let options = ExecutionOptions {
        max_output_bytes: 1,
        ..ExecutionOptions::default()
    };

    let errors = execute_with_options(source, &mut io, options).unwrap_err();

    assert!(errors[0].message.contains("saída excedeu"));
    assert!(io.output.is_empty());
}

#[test]
fn reports_lexical_syntax_and_type_errors() {
    let mut io = TestIo::default();
    let lexical = execute("@", &mut io).unwrap_err();
    let syntax = execute("variaveis inicio escrever 1;", &mut io).unwrap_err();
    let type_error =
        execute("variaveis valor: real; inicio valor <- 2 + 2; fim", &mut io).unwrap_err();

    assert_eq!(lexical[0].kind, DiagnosticKind::Lexical);
    assert_eq!(syntax[0].kind, DiagnosticKind::Syntax);
    assert_eq!(type_error[0].kind, DiagnosticKind::Type);
}

#[test]
fn reports_vector_bounds_and_division_errors_at_runtime() {
    let mut io = TestIo::default();
    let bounds = execute(
        "variaveis valores: vetor [1..2] de inteiro; inicio escrever valores[3]; fim",
        &mut io,
    )
    .unwrap_err();
    let division = execute(
        "variaveis quociente: inteiro; inicio quociente <- 1 / 0; fim",
        &mut io,
    )
    .unwrap_err();

    assert_eq!(bounds[0].kind, DiagnosticKind::Runtime);
    assert!(bounds[0].message.contains("índice fora dos limites"));
    assert_eq!(division[0].kind, DiagnosticKind::Runtime);
    assert!(division[0].message.contains("divisão por zero"));
}

#[test]
fn executes_documented_while_and_repeat_conditions() {
    let source = r#"
variaveis
    i: inteiro;
inicio
    i <- 1;
    enquanto i <= 2 faca
        escrever i;
        i <- i + 1;
    fim enquanto;
    i <- 1;
    repita
        escrever i;
        i <- i + 1;
    ate (i <= 2);
fim
"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("documented loop forms should execute");

    assert_eq!(io.output, "1\n2\n1\n2\n");
}

#[test]
fn enforces_step_variable_and_parse_limits() {
    let mut io = TestIo::default();
    let step = execute_with_options(
        "variaveis inicio escrever 1; fim",
        &mut io,
        ExecutionOptions {
            max_steps: 0,
            ..ExecutionOptions::default()
        },
    )
    .unwrap_err();
    let variables = execute_with_options(
        "variaveis a, b: inteiro; inicio fim",
        &mut io,
        ExecutionOptions {
            max_variable_slots: 1,
            ..ExecutionOptions::default()
        },
    )
    .unwrap_err();
    let parse_depth = execute_with_options(
        "variaveis inicio se verdadeiro entao fim se; fim",
        &mut io,
        ExecutionOptions {
            max_parse_depth: 0,
            ..ExecutionOptions::default()
        },
    )
    .unwrap_err();

    assert_eq!(step[0].kind, DiagnosticKind::Runtime);
    assert_eq!(variables[0].kind, DiagnosticKind::Runtime);
    assert_eq!(parse_depth[0].kind, DiagnosticKind::Syntax);
}

#[derive(Default)]
struct DebugIo {
    output: String,
    events: Vec<ExecutionEvent>,
    actions: VecDeque<ControlAction>,
}

impl RuntimeIo for DebugIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, _prompt: &InputPrompt) -> Result<String, String> {
        Err("no test input".to_owned())
    }

    fn is_debugging(&self) -> bool {
        true
    }

    fn on_execution_event(&mut self, event: &ExecutionEvent) -> Result<ControlAction, String> {
        self.events.push(event.clone());
        Ok(self.actions.pop_front().unwrap_or(ControlAction::Continue))
    }
}

#[test]
fn debugger_receives_statement_snapshots_and_step_actions() {
    let source = "variaveis\n    x: inteiro;\ninicio\n    x <- 1;\n    escrever x;\nfim\n";
    let mut io = DebugIo {
        actions: [ControlAction::Step, ControlAction::Continue].into(),
        ..DebugIo::default()
    };

    execute(source, &mut io).expect("debugger actions should continue execution");

    assert_eq!(io.output, "1\n");
    assert_eq!(io.events.len(), 2);
    assert_eq!(io.events[0].location.line, 4);
    assert_eq!(io.events[0].pause_reason, Some(PauseReason::Debug));
    assert_eq!(io.events[1].location.line, 5);
    assert_eq!(io.events[1].pause_reason, Some(PauseReason::Step));
}

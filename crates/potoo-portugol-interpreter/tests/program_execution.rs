use std::collections::VecDeque;

use potoo_portugol_interpreter::{
    ControlAction, DiagnosticKind, ExecutionPoint, RuntimeIo, execute,
};

#[derive(Default)]
struct TestIo {
    inputs: VecDeque<String>,
    output: String,
    stop_after_checks: Option<usize>,
}

impl TestIo {
    fn with_inputs(inputs: &[&str]) -> Self {
        Self {
            inputs: inputs.iter().map(|value| (*value).to_owned()).collect(),
            output: String::new(),
            stop_after_checks: None,
        }
    }
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self) -> Result<String, String> {
        self.inputs
            .pop_front()
            .ok_or_else(|| "no test input available".to_owned())
    }

    fn should_stop(&mut self) -> bool {
        let Some(remaining) = &mut self.stop_after_checks else {
            return false;
        };
        if *remaining == 0 {
            true
        } else {
            *remaining -= 1;
            false
        }
    }
}

const SAMPLE: &str = r#"programa {
  funcao inicio() {
    inteiro idade

    escreva("Qual é sua idade? ")
    leia(idade)
    se (idade >= 18) {
      escreva("Pode votar.")
    } senao {
      escreva("Ainda não pode votar.")
    }
  }
}"#;

#[test]
fn executes_the_portugol_sample_with_interactive_input() {
    let mut io = TestIo::with_inputs(&["19"]);

    execute(SAMPLE, &mut io).expect("sample program should execute");

    assert_eq!(io.output, "Qual é sua idade? Pode votar.");
}

#[test]
fn executes_the_other_branch_when_the_input_is_under_eighteen() {
    let mut io = TestIo::with_inputs(&["16"]);

    execute(SAMPLE, &mut io).expect("sample program should execute");

    assert_eq!(io.output, "Qual é sua idade? Ainda não pode votar.");
}

#[test]
fn executes_void_functions_and_aliased_libraries() {
    let source = r#"programa {
  inclua biblioteca Matematica --> m

  funcao vazio inicio() {
    escreva(m.arredondar(m.potencia(2, 3), 1))
  }
}"#;
    let mut io = TestIo::default();

    execute(source, &mut io).expect("aliased library and explicit void return should execute");

    assert_eq!(io.output, "8");
}

#[test]
fn reports_the_line_of_a_syntax_error() {
    let mut io = TestIo::default();
    let error = execute("programa {\n funcao inicio( {\n }\n}", &mut io)
        .expect_err("malformed source should be rejected");

    assert_eq!(error[0].kind, DiagnosticKind::Syntax);
    assert_eq!(error[0].line, 2);
}

#[test]
fn debugger_reports_statement_snapshots_and_honors_step_and_stop() {
    #[derive(Default)]
    struct DebugIo {
        points: Vec<ExecutionPoint>,
        actions: VecDeque<ControlAction>,
        output: String,
    }

    impl RuntimeIo for DebugIo {
        fn write(&mut self, text: &str) {
            self.output.push_str(text);
        }
        fn read_line(&mut self) -> Result<String, String> {
            Err("unexpected input".into())
        }
        fn is_debugging(&self) -> bool {
            true
        }
        fn on_statement(&mut self, point: &ExecutionPoint) -> Result<ControlAction, String> {
            self.points.push(point.clone());
            Ok(self.actions.pop_front().unwrap_or(ControlAction::Continue))
        }
    }

    let source = r#"programa {
  funcao inicio() {
    inteiro valor
    valor = 1
    valor = 2
    escreva(valor)
  }
}"#;
    let mut io = DebugIo {
        actions: [
            ControlAction::Continue,
            ControlAction::Continue,
            ControlAction::Continue,
            ControlAction::Step,
            ControlAction::Stop,
        ]
        .into(),
        ..DebugIo::default()
    };

    execute(source, &mut io).expect("debug stop should finish without a runtime error");

    assert_eq!(io.output, "");
    assert_eq!(
        io.points[3].variables,
        [potoo_portugol_interpreter::VariableSnapshot {
            name: "valor".into(),
            value: "1".into()
        }]
    );
    assert_eq!(io.points[4].location.line, 6);
}

#[test]
fn host_can_stop_a_cpu_loop_outside_debugging_mode() {
    let source = r#"programa {
  funcao inicio() {
    enquanto (verdadeiro) {
    }
  }
}"#;
    let mut io = TestIo {
        stop_after_checks: Some(5),
        ..TestIo::default()
    };

    execute(source, &mut io).expect("the host can stop a CPU-bound loop");
}

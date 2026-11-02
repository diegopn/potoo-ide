use std::collections::VecDeque;
use std::time::{Duration, Instant};

use potoo_visualg_interpreter::{
    ControlAction, DiagnosticKind, ExecutionEvent, ExecutionOptions, InputPrompt, PauseReason,
    RuntimeIo, execute, execute_with_options,
};

#[derive(Default)]
struct TestIo {
    output: String,
    input: VecDeque<String>,
    prompts: Vec<InputPrompt>,
    data_file: Option<Vec<String>>,
    created_file: bool,
    recorded: Vec<String>,
    waits: Vec<Duration>,
    events: Vec<ExecutionEvent>,
    actions: VecDeque<ControlAction>,
    debugging: bool,
    clear_count: usize,
    wait_error: bool,
    event_error: bool,
    stop_after_checks: Option<usize>,
}

impl RuntimeIo for TestIo {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String> {
        self.prompts.push(prompt.clone());
        self.input
            .pop_front()
            .ok_or_else(|| "no input configured".to_owned())
    }

    fn read_data_file(&mut self, _path: &str) -> Result<Option<Vec<String>>, String> {
        Ok(self.data_file.clone())
    }

    fn create_data_file(&mut self, _path: &str) -> Result<(), String> {
        self.created_file = true;
        Ok(())
    }

    fn append_data_file_line(&mut self, _path: &str, line: &str) -> Result<(), String> {
        self.recorded.push(line.to_owned());
        Ok(())
    }

    fn wait(&mut self, duration: Duration) -> Result<(), String> {
        self.waits.push(duration);
        if self.wait_error {
            Err("wait failed".into())
        } else {
            Ok(())
        }
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(&mut self, event: &ExecutionEvent) -> Result<ControlAction, String> {
        self.events.push(event.clone());
        if self.event_error {
            Err("event failed".into())
        } else {
            Ok(self.actions.pop_front().unwrap_or(ControlAction::Continue))
        }
    }

    fn clear_output(&mut self) {
        self.clear_count += 1;
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

#[test]
fn aleatorio_generates_values_until_off_but_keeps_logical_input_interactive() {
    let source = r#"algoritmo "aleatorio"
var
  numero, digitado: inteiro
  texto: caractere
  ativo: logico
inicio
  aleatorio 3, 8
  leia(numero)
  leia(texto)
  leia(ativo)
  aleatorio off
  leia(digitado)
  escreva(numero, "|", texto, "|", ativo, "|", digitado)
fimalgoritmo
"#;
    let mut io = TestIo {
        input: ["verdadeiro".into(), "27".into()].into(),
        ..TestIo::default()
    };

    execute(source, &mut io).expect("random reads and host input should execute");

    let fields = io.output.split('|').collect::<Vec<_>>();
    let number = fields[0].trim().parse::<i64>().unwrap();
    assert!((3..=8).contains(&number));
    let text = fields[1];
    assert_eq!(text.len(), 5);
    assert!(text.chars().all(|character| character.is_ascii_uppercase()));
    assert_eq!(fields[2], " VERDADEIRO");
    assert_eq!(fields[3], " 27");
    assert_eq!(
        io.prompts.len(),
        2,
        "logical values still use interactive input"
    );
    assert_eq!(io.prompts[0].variable_name, "ativo");
    assert_eq!(io.prompts[1].variable_name, "digitado");
}

#[test]
fn arquivo_records_missing_file_inputs_and_replays_existing_data() {
    let source = "algoritmo \"dados\"\narquivo \"casos.txt\"\nvar\nx, y: inteiro\ninicio\nleia(x)\nleia(y)\nescreva(x, y)\nfimalgoritmo\n";
    let mut first = TestIo {
        input: ["12".into(), "13".into()].into(),
        ..TestIo::default()
    };

    execute(source, &mut first).expect("missing data file should record host input");

    assert!(first.created_file);
    assert_eq!(first.recorded, ["12", "13"]);
    assert_eq!(first.output, " 12 13");

    let mut second = TestIo {
        data_file: Some(vec!["4".into()]),
        input: ["9".into()].into(),
        ..TestIo::default()
    };
    execute(source, &mut second).expect("existing file should replay then fall back to input");
    assert_eq!(second.output, " 4 9");
    assert_eq!(second.prompts.len(), 1);
    assert!(second.recorded.is_empty());
}

#[test]
fn data_file_directive_reports_when_the_host_has_no_file_service() {
    struct MinimalIo;
    impl RuntimeIo for MinimalIo {
        fn write(&mut self, _text: &str) {}

        fn read_line(&mut self, _prompt: &InputPrompt) -> Result<String, String> {
            Err("unexpected input".to_owned())
        }
    }

    let source = "algoritmo \"dados\"\narquivo \"casos.txt\"\ninicio\nfimalgoritmo\n";
    let error = execute(source, &mut MinimalIo)
        .expect_err("arquivo should not silently succeed when the host has no file service");

    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert_eq!(error[0].line, 2);
    assert!(error[0].message.contains("arquivos de dados"));
}

#[test]
fn timer_uses_the_default_wait_service_when_the_host_does_not_override_it() {
    struct MinimalIo;
    impl RuntimeIo for MinimalIo {
        fn write(&mut self, _text: &str) {}

        fn read_line(&mut self, _prompt: &InputPrompt) -> Result<String, String> {
            Err("unexpected input".to_owned())
        }
    }

    let source = "algoritmo \"timer\"\ninicio\ntimer 10\nescreva(1)\nfimalgoritmo\n";
    let started = Instant::now();
    execute(source, &mut MinimalIo).expect("timer should use its standard host service");

    assert!(started.elapsed() >= Duration::from_millis(10));
}

#[test]
fn timer_pause_debug_echo_chronometer_and_clear_call_host_services() {
    let source = r#"algoritmo "servicos"
var
  valor: inteiro
inicio
  timer on
  escreva("A")
  timer 20000
  escreval("B")
  timer off
  debug falso
  debug verdadeiro
  pausa
  eco on
  leia(valor)
  cronometro on
  cronometro off
  limpatela
fimalgoritmo
"#;
    let mut io = TestIo {
        input: ["5".into()].into(),
        actions: [ControlAction::Continue, ControlAction::Continue].into(),
        ..TestIo::default()
    };

    execute(source, &mut io).expect("runtime services should be dispatched to the host");

    assert!(io.waits.contains(&Duration::from_millis(500)));
    assert!(
        io.waits.contains(&Duration::from_millis(10_000)),
        "timer values clamp to 10 seconds"
    );
    assert_eq!(
        io.events
            .iter()
            .filter_map(|event| event.pause_reason)
            .collect::<Vec<_>>(),
        [PauseReason::Debug, PauseReason::Pause]
    );
    assert_eq!(io.clear_count, 1);
    assert!(
        io.output
            .starts_with("AB\n 5\nCronômetro iniciado.\nCronômetro terminado. Tempo decorrido: ")
    );
    assert!(io.output.ends_with(" ms\n"));
}

#[test]
fn debugger_control_actions_step_continue_and_stop_with_visible_snapshots() {
    let source =
        "algoritmo \"passos\"\nvar\nx: inteiro\ninicio\nx <- 1\nx <- 2\nescreva(x)\nfimalgoritmo\n";
    let mut io = TestIo {
        debugging: true,
        actions: [
            ControlAction::Continue,
            ControlAction::Step,
            ControlAction::Stop,
        ]
        .into(),
        ..TestIo::default()
    };

    execute(source, &mut io).expect("stop should end execution cleanly");

    assert_eq!(io.output, "");
    assert_eq!(io.events.len(), 3);
    assert_eq!(
        io.events[1].variables,
        [potoo_visualg_interpreter::VariableSnapshot {
            name: "x".into(),
            value: "1".into()
        }]
    );
    assert_eq!(io.events[2].pause_reason, Some(PauseReason::Step));
}

#[test]
fn debugger_reports_each_loop_header_before_the_next_iteration() {
    let source = "algoritmo \"passos\"\nvar\ni: inteiro\ninicio\ni <- 0\nenquanto i < 2 faca\ni <- i + 1\nfimenquanto\nescreval(\"fim\")\nfimalgoritmo\n";
    let mut io = TestIo {
        debugging: true,
        ..TestIo::default()
    };

    execute(source, &mut io).expect("the loop should execute in debug mode");

    assert_eq!(
        io.events
            .iter()
            .map(|event| event.location.line)
            .collect::<Vec<_>>(),
        [5, 6, 7, 6, 7, 6, 9]
    );
    assert_eq!(
        io.events[3].variables,
        [potoo_visualg_interpreter::VariableSnapshot {
            name: "i".into(),
            value: "1".into(),
        }]
    );
}

#[test]
fn stop_is_polled_inside_cpu_loops_without_debugging() {
    let source = "algoritmo \"parar\"\ninicio\nenquanto verdadeiro faca\n  escreva(\"\")\nfimenquanto\nfimalgoritmo\n";
    let mut options = ExecutionOptions::default();
    options.max_steps = 12;
    let mut io = TestIo {
        stop_after_checks: Some(5),
        ..TestIo::default()
    };

    execute_with_options(source, &mut io, options)
        .expect("a host stop request should end a non-debugging loop cleanly");
}

#[test]
fn empty_cpu_loops_are_bounded_by_the_execution_step_limit() {
    let source =
        "algoritmo \"limite\"\ninicio\nenquanto verdadeiro faca\nfimenquanto\nfimalgoritmo\n";
    let mut options = ExecutionOptions::default();
    options.max_steps = 5;

    let error = execute_with_options(source, &mut TestIo::default(), options)
        .expect_err("empty loops should still count toward the execution limit");

    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert!(error[0].message.contains("limite de passos"));
}

#[test]
fn debugger_snapshots_expand_array_elements() {
    let source = "algoritmo \"vetor\"\nvar\ndados: vetor[1..2, 4..5] de inteiro\ninicio\ndados[1, 4] <- 8\nescreva(dados[1, 4])\nfimalgoritmo\n";
    let mut io = TestIo {
        debugging: true,
        ..TestIo::default()
    };

    execute(source, &mut io).expect("array program should execute in debug mode");

    assert_eq!(
        io.events[1].variables,
        [
            potoo_visualg_interpreter::VariableSnapshot {
                name: "dados[1,4]".into(),
                value: "8".into(),
            },
            potoo_visualg_interpreter::VariableSnapshot {
                name: "dados[1,5]".into(),
                value: "0".into(),
            },
            potoo_visualg_interpreter::VariableSnapshot {
                name: "dados[2,4]".into(),
                value: "0".into(),
            },
            potoo_visualg_interpreter::VariableSnapshot {
                name: "dados[2,5]".into(),
                value: "0".into(),
            },
        ]
    );
}

#[test]
fn enforces_total_timer_and_data_file_limits_and_reports_host_errors() {
    let timer_source =
        "algoritmo \"timer\"\ninicio\ntimer 800\nescreva(1)\nescreva(2)\nfimalgoritmo\n";
    let mut options = ExecutionOptions::default();
    options.max_wait_millis = 1000;
    let error = execute_with_options(timer_source, &mut TestIo::default(), options)
        .expect_err("total wait limit should be enforced");
    assert_eq!(error[0].kind, DiagnosticKind::Runtime);

    let oversized_file = "algoritmo \"arquivo\"\narquivo \"data.txt\"\ninicio\nfimalgoritmo\n";
    let mut options = ExecutionOptions::default();
    options.max_file_bytes = 2;
    let mut file_io = TestIo {
        data_file: Some(vec!["long".into()]),
        ..TestIo::default()
    };
    let error = execute_with_options(oversized_file, &mut file_io, options)
        .expect_err("oversized data should fail");
    assert!(error[0].message.contains("arquivo de dados"));
    assert_eq!((error[0].line, error[0].column), (2, 9));

    let mut wait_io = TestIo {
        wait_error: true,
        ..TestIo::default()
    };
    let error = execute(
        "algoritmo \"wait\"\ninicio\ntimer on\nescreva(1)\nfimalgoritmo\n",
        &mut wait_io,
    )
    .expect_err("wait callback errors should be located diagnostics");
    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert!(error[0].message.contains("wait failed"));

    let mut event_io = TestIo {
        event_error: true,
        ..TestIo::default()
    };
    let error = execute(
        "algoritmo \"event\"\ninicio\npausa\nfimalgoritmo\n",
        &mut event_io,
    )
    .expect_err("execution callback errors should be located diagnostics");
    assert_eq!(error[0].kind, DiagnosticKind::Runtime);
    assert!(error[0].message.contains("event failed"));
}

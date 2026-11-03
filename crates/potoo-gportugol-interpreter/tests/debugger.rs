use std::collections::VecDeque;

use potoo_gportugol_interpreter::{
    ControlAction, ExecutionEvent, InputPrompt, PauseReason, RuntimeIo, execute,
};

#[derive(Default)]
struct Io {
    output: String,
    debugging: bool,
    actions: VecDeque<ControlAction>,
    events: Vec<ExecutionEvent>,
}

impl RuntimeIo for Io {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, _prompt: &InputPrompt) -> Result<String, String> {
        Err("unexpected input".to_owned())
    }

    fn is_debugging(&self) -> bool {
        self.debugging
    }

    fn on_execution_event(&mut self, event: &ExecutionEvent) -> Result<ControlAction, String> {
        self.events.push(event.clone());
        Ok(self.actions.pop_front().unwrap_or(ControlAction::Continue))
    }
}

#[test]
fn debugger_reports_visible_variables() {
    let mut io = Io {
        debugging: true,
        ..Io::default()
    };
    let source = "algoritmo depura;\n\
      variáveis total: inteiro; fim-variáveis\n\
      início\n\
        total := 7;\n\
        imprima(total);\n\
      fim";

    execute(source, &mut io).expect("debugger callbacks should not change execution");

    assert_eq!(io.events.len(), 2);
    assert_eq!(io.events[0].location.line, 4);
    assert_eq!(io.events[0].variables[0].name, "total");
    assert_eq!(io.events[0].variables[0].value, "0");
    assert_eq!(io.events[1].location.line, 5);
    assert_eq!(io.events[1].variables[0].value, "7");
}

#[test]
fn debugger_reports_local_variables_and_shadows_globals() {
    let mut io = Io {
        debugging: true,
        ..Io::default()
    };
    let source = "algoritmo sombras;\n\
      variáveis valor: inteiro; fim-variáveis\n\
      início\n\
        valor := 1;\n\
        chama();\n\
      fim\n\
      função chama()\n\
        valor: inteiro;\n\
      início\n\
        valor := 9;\n\
        imprima(valor);\n\
      fim";

    execute(source, &mut io).expect("the debugger should include function-local values");

    assert_eq!(io.output, "9\n");
    assert_eq!(io.events[2].location.line, 10);
    assert_eq!(io.events[2].variables.len(), 1);
    assert_eq!(io.events[2].variables[0].name, "valor");
    assert_eq!(io.events[2].variables[0].value, "0");
    assert_eq!(io.events[3].variables[0].value, "9");
}

#[test]
fn debugger_reports_globals_and_only_the_active_local_frame() {
    let mut io = Io {
        debugging: true,
        ..Io::default()
    };
    let source = "algoritmo frames;\n\
      variáveis global_value: inteiro; fim-variáveis\n\
      início\n\
        chama_outer();\n\
      fim\n\
      função chama_outer(): inteiro\n\
        caller_value: inteiro;\n\
      início\n\
        caller_value := 1;\n\
        chama_inner();\n\
        retorne 0;\n\
      fim\n\
      função chama_inner(): inteiro\n\
        inner_value: inteiro;\n\
      início\n\
        inner_value := 2;\n\
        imprima(inner_value);\n\
        retorne inner_value;\n\
      fim";

    execute(source, &mut io).expect("nested calls should run under the debugger");

    let event = io
        .events
        .iter()
        .find(|event| event.location.line == 17)
        .expect("the inner print should produce a debugger event");
    let names = event
        .variables
        .iter()
        .map(|variable| variable.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(names, ["global_value", "inner_value"]);
}

#[test]
fn step_continue_and_stop_control_execution() {
    let mut io = Io {
        debugging: true,
        actions: [
            ControlAction::Step,
            ControlAction::Continue,
            ControlAction::Stop,
        ]
        .into(),
        ..Io::default()
    };
    let source = "algoritmo controle;\n\
      variáveis valor: inteiro; fim-variáveis\n\
      início\n\
        valor := 1;\n\
        imprima(valor);\n\
        valor := 2;\n\
        imprima(valor);\n\
      fim";

    execute(source, &mut io).expect("stopping from the debugger should end execution cleanly");

    assert_eq!(io.output, "1\n");
    assert_eq!(io.events.len(), 3);
    assert_eq!(io.events[0].pause_reason, Some(PauseReason::Debugger));
    assert_eq!(io.events[1].pause_reason, Some(PauseReason::Step));
    assert_eq!(io.events[2].pause_reason, Some(PauseReason::Debugger));
}

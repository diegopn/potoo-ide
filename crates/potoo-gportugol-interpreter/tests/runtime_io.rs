use std::collections::VecDeque;

use potoo_gportugol_interpreter::{DataType, DiagnosticKind, InputPrompt, RuntimeIo, execute};

#[derive(Default)]
struct Io {
    inputs: VecDeque<String>,
    prompts: Vec<InputPrompt>,
    output: String,
}

impl RuntimeIo for Io {
    fn write(&mut self, text: &str) {
        self.output.push_str(text);
    }

    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String> {
        self.prompts.push(prompt.clone());
        self.inputs
            .pop_front()
            .ok_or_else(|| "input closed".to_owned())
    }
}

#[test]
fn reads_using_the_assignment_type_and_source_location() {
    let mut io = Io {
        inputs: ["7".to_owned()].into(),
        ..Io::default()
    };

    execute(
        "algoritmo prompt; variáveis n: inteiro; fim-variáveis\n\
         início\n\
           n := leia();\n\
         fim",
        &mut io,
    )
    .expect("input should be read");

    assert_eq!(io.prompts[0].data_type, DataType::Integer);
    assert_eq!(io.prompts[0].source.line, 3);
}

#[test]
fn reads_real_decimal_exponents_from_the_numeric_prefix() {
    let mut io = Io {
        inputs: ["+1e2tail", "-2.5E-1junk", "1e+"].map(str::to_owned).into(),
        ..Io::default()
    };

    execute(
        "algoritmo expoentes; variáveis first, second, incomplete: real; fim-variáveis\n\
         início\n\
           first := leia(); second := leia(); incomplete := leia();\n\
           imprima(first, \"|\", second, \"|\", incomplete);\n\
         fim",
        &mut io,
    )
    .expect("real input should accept decimal exponent prefixes");

    assert_eq!(io.output, "100.00|-0.25|1.00\n");
}

#[test]
fn reports_host_input_errors_as_runtime_diagnostics() {
    let mut io = Io::default();
    let diagnostics = execute(
        "algoritmo sem_entrada; variáveis n: inteiro; fim-variáveis início n := leia(); fim",
        &mut io,
    )
    .expect_err("closed input should stop the program");

    assert_eq!(diagnostics[0].kind, DiagnosticKind::Runtime);
    assert!(diagnostics[0].message.contains("input closed"));
}

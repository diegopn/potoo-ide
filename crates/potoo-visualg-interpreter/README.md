# potoo-visualg-interpreter

A reusable Rust interpreter for the classic VisuAlg 3.0.5.2 pseudolanguage. The library has no third-party dependencies and can be embedded in Potoo IDE or another Rust application.

The crate targets the language and commands documented by the [VisuAlg 3.0.5.2 manual](https://documentation.help/VISUALG-Versao/linguagem5.htm) and its [built-in functions reference](https://documentation.help/VISUALG-Versao/funcoes.htm). Compatibility testing is ongoing; the crate does not invoke or bundle the original VisuAlg application.

## Use

Add the crate as a dependency:

~~~toml
[dependencies]
potoo-visualg-interpreter = { git = "https://github.com/diegopn/potoo-visualg-interpreter" }
~~~

Implement RuntimeIo to provide input and output, then call execute:

~~~rust
use potoo_visualg_interpreter::{InputPrompt, RuntimeIo, execute};

struct Console;

impl RuntimeIo for Console {
    fn write(&mut self, text: &str) {
        print!("{text}");
    }

    fn read_line(&mut self, prompt: &InputPrompt) -> Result<String, String> {
        eprint!("{} ({:?}): ", prompt.variable_name, prompt.data_type);
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|error| error.to_string())?;
        Ok(input.trim_end_matches(['\r', '\n']).to_owned())
    }
}

fn main() -> Result<(), Vec<potoo_visualg_interpreter::Diagnostic>> {
    let source = r#"algoritmo "exemplo"
inicio
  escreval("Olá, VisuAlg!")
fimalgoritmo
"#;
    execute(source, &mut Console)
}
~~~

Call `parse` instead of `execute` when a host needs to inspect the public syntax tree without running the program.

RuntimeIo::is_debugging enables statement events. RuntimeIo::on_execution_event receives the current source position, visible variable values, and pause reason; return Continue, Step, or Stop to control execution. Hosts can also implement should_stop to interrupt CPU-bound loops, wait for timer, and the data-file callbacks for arquivo. The default wait implementation sleeps for the requested duration; the default data-file callbacks return an error so a host without file support cannot silently ignore the directive.

## Supported language features

- algoritmo, declaration sections, procedures, functions, typed parameters passed by value or reference, recursion, and retorne
- inteiro, real, caractere, and logico, plus vectors and matrices with explicit lower and upper bounds
- Assignments, arithmetic and logical expressions, comparisons, text operations, and formatted escreva / escreval
- Typed leia, se / senao, escolha / caso / outrocaso, enquanto, repita / ate, para, and interrompa
- Numeric and text functions: Abs, ArcCos, ArcSen, ArcTan, Asc, Carac, Caracpnum, Compr, Copia, Cos, CoTan, Exp, GrauPrad, Int, Log, LogN, Maiusc, Minusc, Numpcarac, Pi, Pos, Quad, RaizQ, Rand, RandI, RadpGrau, Sen, and Tan
- Runtime commands aleatorio, arquivo, timer, pausa, debug, eco, cronometro, and limpatela
- Source-located lexical, syntax, and runtime diagnostics, and debugger snapshots that include array elements

The arquivo command delegates storage to RuntimeIo; the interpreter itself does not access the filesystem. The IDE host confines data-file paths to the open project. Numeric random ranges, character generation, timers, and debugger pauses follow the classic manual.

## Runtime limits

execute_with_options accepts explicit limits. Defaults for one run are 1,000,000 statement and loop steps, 1 MiB of output, 1 MiB per string, 500 variable and array-element slots, 1 MiB of data-file contents, 30 seconds of total timer wait, 32 nested routine calls, and 128 levels of parser nesting.

The parser reports the first lexical or syntax error. Runtime diagnostics include a one-based line and column. Matching the original application's behavior across the entire language remains an ongoing compatibility task.

Nested procedures and functions are not supported. The compatibility target is the classic 3.0.5.2 language described by the linked manual; other VisuAlg forks and versions have not been validated.

## Development

From the IDE repository root:

~~~sh
cargo test -p potoo-visualg-interpreter
~~~

## Project policies

- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [GPL-3.0-or-later license](LICENSE)

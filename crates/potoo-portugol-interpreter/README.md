# potoo-portugol-interpreter

A reusable Portugol interpreter written in Rust. The crate depends on `indexmap` and can be embedded in the Potoo IDE or another Rust application.

## Use

Add the crate as a dependency:

```toml
[dependencies]
potoo-portugol-interpreter = { git = "https://github.com/diegopn/potoo-ide", branch = "develop" }
```

Provide input and output through `RuntimeIo`, then call `execute`:

```rust
use potoo_portugol_interpreter::{RuntimeIo, execute};

struct Console;

impl RuntimeIo for Console {
    fn write(&mut self, text: &str) {
        print!("{text}");
    }

    fn read_line(&mut self) -> Result<String, String> {
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|error| error.to_string())?;
        Ok(input.trim_end_matches(['\r', '\n']).to_owned())
    }
}

fn main() -> Result<(), Vec<potoo_portugol_interpreter::Diagnostic>> {
    let source = r#"programa {
  funcao vazio inicio() {
    escreva("Olá, Portugol!")
  }
}"#;
    execute(source, &mut Console)
}
```

Call `parse` instead of `execute` when a host needs to inspect the public syntax tree without running the program.

`RuntimeIo::clear_output` can be implemented by hosts that want to handle `limpa()`; its default implementation does nothing. `execute_with_options` accepts limits for executed statements, output bytes, string values, total allocated array elements, and time spent waiting. The defaults are 1,000,000 steps, 1 MiB of output, 1 MiB per string, 100,000 array elements, and 30 seconds of waiting per run. Recursive calls are capped at 32 levels to protect the host stack, and the parser rejects statement or expression nesting beyond 128 levels.

## Supported language features

- `programa`, typed and `vazio` functions, parameters, return values, and global variables
- `inteiro`, `real`, `logico`, `caracter`, and `cadeia`
- Local variables, constants, vectors and matrices (including initializers and inferred vector sizes), and array parameters
- Value and reference parameters, expressions, assignments, and arithmetic, bitwise, comparison, and logical operators
- `se` / `senao`, `enquanto`, `faca` / `enquanto`, `para`, `escolha` / `caso`, `pare`, and `retorne`
- `escreva`, `escreva_linha`, `leia`, and `limpa`
- `Matematica`: `PI`, `potencia`, `raiz`, `arredondar`, `logaritmo`, trigonometric functions, `valor_absoluto`, `maior_numero`, and `menor_numero`
- `Texto`: `numero_caracteres`, `caixa_alta`, `caixa_baixa`, `substituir`, `preencher_a_esquerda`, `obter_caracter`, `posicao_texto`, and `extrair_subtexto`
- `Tipos`: common conversions between strings, integers, reals, logical values, and characters
- `Util`: vector and matrix sizes, `sorteia`, `aguarde`, time since first library use, and the user's home directory
- `Calendario`: current date and time fields, plus full, short, and abbreviated weekday names
- `Arquivos`: open, read, write, list, filter, create, delete, replace text with regular expressions, and select files. Hosts provide the file operations through `RuntimeIo`.
- `Objetos`: in-memory objects, typed scalar and vector properties, JSON and XML input, JSON output, and object release
- `Internet` and `ServicosWeb`: HTTP requests and image downloads through host callbacks
- `Graficos`: every function and constant exported by the reference library is dispatched through `RuntimeIo::portugol_graphics`
- `Mouse` and `Teclado`: all exported functions and constants; input events and cursor control use `RuntimeIo` callbacks
- `Sons`: all exported functions through `RuntimeIo::portugol_audio`
- Library aliases such as `inclua biblioteca Matematica --> m`

## Dialect reference

This crate targets **Portugol 2.0**. The [Potoo language reference](https://diegopn.github.io/potoo-site/en/manual/portugol/) identifies the upstream sources used for compatibility work. There is no single language-wide Portugol standard that covers all tools and dialects.

The interpreter does not claim full compatibility yet: its parser reports the first lexical or syntax error, and graphics, keyboard, mouse, audio, HTTP, file, and calendar behavior depends on the host implementing the corresponding `RuntimeIo` callbacks. The Potoo IDE now provides GTK host handlers for graphics, keyboard, mouse, and audio, but those interactive features still need manual validation on each supported desktop platform. Runtime diagnostics include a source line and column.

## Development

From the IDE repository root:

```sh
cargo test -p potoo-portugol-interpreter
```

## Project policies

- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [GPL-3.0-or-later license](LICENSE)

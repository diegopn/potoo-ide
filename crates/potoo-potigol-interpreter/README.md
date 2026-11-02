# potoo-potigol-interpreter

A Rust interpreter for the documented Potigol 1.0.0-RC2 language core. The crate has no third-party dependencies and can be embedded in the Potoo IDE or another Rust application. It is an independent implementation and does not run or bundle the official Scala/JVM interpreter.

## Use

Implement `RuntimeIo` to provide input and output, then call `execute`:

```rust
use potoo_potigol_interpreter::{InputPrompt, RuntimeIo, execute};

struct Console;

impl RuntimeIo for Console {
    fn write(&mut self, text: &str) {
        print!("{text}");
    }

    fn read_line(&mut self, _prompt: &InputPrompt) -> Result<String, String> {
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .map_err(|error| error.to_string())?;
        Ok(input.trim_end_matches(['\r', '\n']).to_owned())
    }
}

fn main() -> Result<(), Vec<potoo_potigol_interpreter::Diagnostic>> {
    execute(
        "nome = leia_texto\nescreva \"Olá, {nome}!\"",
        &mut Console,
    )
}
```

Call `parse` when a host needs to inspect the public syntax tree without executing the program. `check` exposes static type checking. `execute_with_options` accepts per-run resource limits.

## Supported language features

- Type inference, typed declarations and parameters, primitive values, lists, mutable lists, tuples, and user-defined types
- Functions, closures, recursion, lambdas, currying, and higher-order list operations
- Pattern matching, constructor patterns, guards, conditionals, loops, and list comprehensions
- Typed input, formatted and interpolated output, numeric and text operations, and built-in math functions
- Abstract types, single inheritance, source-located diagnostics, cancellation, and debugger events for host applications

The compatibility target is the documented Potigol 1.0.0-RC2 language core. External Scala/JVM libraries such as Jerimum are not included.

## Runtime limits

By default, one execution is limited to 1,000,000 steps, 1 MiB of output, 1 MiB per string, 100,000 collection elements, an estimated cumulative 64 MiB of collection payload, 30 seconds of waiting, 128 nested calls, and 128 levels of parser nesting. Hosts can change these limits with `ExecutionOptions`.

The collection byte limit estimates cumulative payload allocation; it is not a strict live-heap ceiling. This interpreter runs in the host process and is not an operating-system sandbox. Hosts running untrusted source should apply appropriate process isolation.

## Development

From the workspace root:

```sh
cargo fmt --all -- --check
cargo check --locked -p potoo-potigol-interpreter
cargo test --locked -p potoo-potigol-interpreter
```

The conformance tests cover representative examples from Potigol documentation and release notes.

## Project policies

- [Contributing](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [GPL-3.0-or-later license](LICENSE)

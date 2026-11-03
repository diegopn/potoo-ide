# Potoo G-Portugol Interpreter

An independent Rust interpreter for the G-Portugol 1.4.0 language, integrated with Potoo IDE. G-Portugol source files use the `.gpt` extension and remain separate from Portugol 2.0 (`.por`), VisuAlg (`.alg`), and Potigol (`.poti`).

## Language coverage

The implementation follows the G-Portugol 1.4.0 manual and grammar for declarations, primitive types, expressions and conversions, integer division, `imprima`, typed `leia()`, conditionals, loops, vectors, matrices, and functions. It provides lexer, parser, type checking, runtime diagnostics with source locations, execution limits, cancellation, host-controlled input/output, and debugger events. The language defines `imprima` and `leia()` as intrinsic functions; Potoo also embeds an independent Rust implementation of the default `base.gpt` math library (`absoluto`, `arredonda`, `potencia`, and `raiz_quadrada`).

G-Portugol keywords may contain accents, while identifiers are ASCII. Vector and matrix indexes start at zero; arrays are initialized with zero or a null value. Arrays passed to functions are copied by value. Nested functions and reference parameters are not supported by the language.

Integer storage and integer operations follow GPT 1.4.0's signed 32-bit behavior. Overflow wraps; `-2147483648 / -1` returns `-2147483648`, and its remainder is zero. Mixed expressions still promote operands to real before arithmetic when a real operand is present.

## Source example

```gpt
algoritmo votacao;
variáveis idade: inteiro; fim-variáveis
início
  imprima("Qual é sua idade?");
  idade := leia();
  se idade >= 18 então
    imprima("Pode votar.");
  senão
    imprima("Ainda não pode votar.");
  fim-se
fim
```

This is the executable IDE smoke fixture, [`gportugol-voting.gpt`](../../tests/fixtures/gportugol-voting.gpt), also used by the interpreter and Potoo execution tests.

## Rust API

The crate has no third-party dependencies. Add it to a workspace member with a path dependency:

```toml
[dependencies]
potoo-gportugol-interpreter = { path = "../potoo-gportugol-interpreter" }
```

The main APIs are:

- `lex(source)` returns tokens with source spans.
- `parse(source)` and `parse_with_options(source, options)` build the public AST.
- `check(&program)` performs static type checking.
- `execute(source, io)` parses, checks, and runs a program, returning `Some(code)` when the main block executes `retorne <inteiro>` and `None` when it ends normally.
- `execute_with_options(source, io, options)` returns the same optional main-block status with caller-configured limits.
- `execute_with_sources(main, modules, io)` composes a main file with function-only source files and returns its optional status.
- `execute_with_sources_and_options(main, modules, io, options)` does the same with caller-configured limits.

The host implements `RuntimeIo` to receive output, provide typed input, signal cancellation, and handle debugger events. This keeps terminal, desktop, or other host I/O outside the interpreter and lets the IDE run it on its existing execution worker.

Default resource limits are 1,000,000 execution steps, 1 MiB of output, 1 MiB per string, 100,000 array elements, 32 nested function calls, 128 parser nesting levels, and 30 seconds waiting for input. Hosts can override these through `ExecutionOptions`.

## Compatibility and platform status

The documented 1.4.0 language is the compatibility target. The upstream `gpt` executable was not available for a differential run, so compatibility has been checked with the crate's local lexer, parser, type, runtime, I/O, limits, and debugger tests rather than compared against the reference executable.

The crate API supports GPT's multi-file source composition when the host supplies the main file and its function-only source files. The IDE currently sends the active `.gpt` document as a single source; it does not yet provide project-level module selection or source-aware navigation for external functions.

Linux is the platform used for the current build and test run. Windows and macOS have not yet been built or exercised; the crate itself uses only the Rust standard library, but those platforms are not claimed as verified here.

## License

This crate is licensed under GPL-3.0-or-later. It is an independent implementation: it neither invokes nor bundles the upstream `gpt` executable and does not reuse upstream implementation code.

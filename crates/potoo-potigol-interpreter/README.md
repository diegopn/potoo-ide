# potoo-potigol-interpreter

A reusable Rust interpreter for the Potigol programming language. The compatibility target is the official Potigol 1.0.0-RC2 language core. This crate is independent of the Potoo IDE UI and its learning features.

The interpreter supports typed expressions and inference, immutable and mutable values, parallel binding, functions and closures, recursion, lambdas and currying, lists and higher-order operations, tuples, comprehensions, pattern matching, conditionals, loops, text interpolation and formatting, user-defined types, abstract types and single inheritance. The IDE connects typed input/output, cancellation, source-located diagnostics, resource limits, and debugger events through the `RuntimeIo` interface.

External JVM libraries such as Jerimum are not loaded by this crate. The interpreter is a fresh Rust implementation based on the published language documentation and examples; it does not include or invoke the upstream Scala/JVM interpreter.

## Development

From the IDE workspace root:

```sh
cargo check -p potoo-potigol-interpreter
cargo test -p potoo-potigol-interpreter
```

The conformance suite exercises representative examples from the official Potigol documentation and release notes.

The implementation targets the GPL-3.0-or-later license used by Potoo IDE.

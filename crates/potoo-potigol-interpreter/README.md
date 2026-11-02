# potoo-potigol-interpreter

A reusable Rust interpreter for the Potigol programming language. The compatibility target is the official Potigol 1.0.0-RC2 release. This crate is being developed independently of the Potoo IDE UI and its learning features.

The implementation is in progress and is not yet a complete Potigol interpreter. The intended public API will provide source-located diagnostics, resource limits, typed input/output, cancellation, and debugger events through a host interface.

The interpreter is a fresh Rust implementation based on the published language documentation and examples. It does not include or invoke the upstream Scala/JVM interpreter.

## Development

From the IDE workspace root:

```sh
cargo check -p potoo-potigol-interpreter
```

The implementation targets the GPL-3.0-or-later license used by Potoo IDE.

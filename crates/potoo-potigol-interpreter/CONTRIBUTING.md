# Contributing to potoo-potigol-interpreter

Contributions that improve the Potigol interpreter, its tests, or its documentation are welcome. This crate is part of the Potoo IDE workspace; changes to the desktop application belong in the workspace's application code.

## Report a bug

Open an [issue in the Potoo IDE repository](https://github.com/diegopn/potoo-ide/issues). Include:

- the crate version or commit and your Rust version;
- the operating system and host application, if relevant;
- a short Potigol program that reproduces the issue;
- the expected result, the actual result, and the steps you followed.

Remove credentials, personal information, and unrelated project data from examples.

## Development setup

The crate uses Rust edition 2024 and requires Rust 1.93 or later. From the workspace root, run:

```sh
cargo fmt --all -- --check
cargo check --locked -p potoo-potigol-interpreter
cargo test --locked -p potoo-potigol-interpreter
cargo test --release --locked -p potoo-potigol-interpreter
cargo package --allow-dirty --locked -p potoo-potigol-interpreter
```

The interpreter currently has no third-party dependencies. If a contribution proposes one, explain why the standard library is insufficient and include its maintenance, license, and security implications.

## Changes to language behavior

- Add tests for new syntax, type checking, runtime behavior, diagnostics, and resource limits.
- Keep the Potigol 1.0.0-RC2 compatibility target explicit. Do not claim support for external JVM libraries or unverified language extensions.
- Keep public documentation in English and update it when the public API or supported behavior changes.
- Keep changes focused and list the verification commands in the pull request.
- Preserve existing copyright and license notices. Identify copied third-party material and verify its license before submitting it.

## License for contributions

By submitting a contribution, you agree that it may be distributed under the GNU General Public License version 3 or, at your option, any later version. See [LICENSE](LICENSE) for the full license text.

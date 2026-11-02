# Contributing to potoo-portugol-interpreter

Contributions that improve the interpreter, its tests, or its documentation are welcome. This repository contains the Portugol interpreter crate; changes to the desktop IDE belong in the Potoo IDE repository.

## Report a bug

Open an issue in the repository that hosts this crate. Include:

- the crate version or commit and your Rust version;
- the operating system and host application, if relevant;
- a short Portugol program that reproduces the issue;
- the expected result, the actual result, and the steps you followed.

Remove credentials, personal information, and unrelated project data from examples.

## Development setup

The crate uses Rust edition 2024 and requires Rust 1.93 or later. From the crate directory, run:

```sh
cargo fmt --all -- --check
cargo test --locked
cargo test --release --locked
cargo package --allow-dirty --locked
```

The interpreter currently has no third-party dependencies. If a contribution proposes one, explain why the standard library is insufficient and include its maintenance, license, and security implications.

## Changes to language behavior

- Add tests for new syntax, runtime behavior, error cases, and resource limits.
- Keep dialect behavior explicit. The README describes the current compatibility reference and unsupported libraries; do not claim complete compatibility without evidence.
- Keep public documentation in English and update it when the public API or supported behavior changes.
- Keep changes focused and list the verification commands in the pull request.
- Preserve existing copyright and license notices. Identify any copied third-party material and verify its license before submitting it.

## License for contributions

By submitting a contribution, you agree that it may be distributed under the GNU General Public License version 3 or, at your option, any later version. See [LICENSE](LICENSE) for the full license text.

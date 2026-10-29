# Contributing

Contributions are welcome, including bug reports, application testing, documentation improvements, and code. The project is still at an early stage, so feedback on the experience for students and teachers is also valuable.

## Report a bug

Open an [issue](https://github.com/diegopn/potoo-ide/issues) with:

- your operating system and version;
- what you were trying to do;
- what you expected to happen and what happened;
- steps to reproduce the problem.

Use examples created for the report. Do not send personal documents, private data, or credentials.

## Set up the development environment

Install Rust 1.93 or later and the GTK 4.20 and libadwaita 1.8 development libraries available for your distribution. Then run:

```sh
cargo run --locked
```

Before opening a pull request, format the code and check that the project builds:

```sh
cargo fmt --all
cargo check --locked
```

## Pull requests

- Keep each pull request focused on one change.
- Describe the problem or improvement and how to review the result.
- List the verification commands you ran.
- Target `develop` for development changes; `main` is reserved for stable releases.
- When including third-party code or files, check license compatibility and preserve copyright and license notices.

## License

Contributions to Potoo IDE's code will be distributed under the GNU GPL version 3 or later, as described in [LICENSE](LICENSE).

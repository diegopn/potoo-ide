# Security Policy

## Supported versions

This crate is part of the Potoo IDE workspace. The root [Security Policy](../../SECURITY.md) defines which branches and releases are in scope.

## Report a vulnerability

Do not publish vulnerability details in issues, pull requests, discussions, or public comments. Use the repository's **Security → Report a vulnerability** private reporting form when it is available.

If private reporting is unavailable, use the [Potoo contact page](https://diegopn.github.io/#contato) only to request a private reporting channel. Do not include technical details or a proof of concept in that first message.

Include the affected version or commit, Rust version, operating system, a minimal `.alg` example, reproduction steps, and the impact. Remove credentials, personal information, and unrelated data.

## Security boundaries

This interpreter runs a program inside the host process; it is not an operating-system sandbox. Resource limits reduce accidental or abusive CPU, memory, output, file-data, and wait usage, but a host can change those limits. Hosts should keep appropriate bounds and apply operating-system isolation when executing programs from untrusted users in a privileged or shared service.

The interpreter delegates input, output, cancellation, waits, and data-file access to `RuntimeIo`. A host that implements `arquivo` must confine paths and enforce its own access policy; the Potoo IDE confines data files to the open project. Treat every host callback as the authority boundary and avoid giving untrusted programs unnecessary access. `Rand` and `RandI` are for programming exercises, not cryptographic secrets.

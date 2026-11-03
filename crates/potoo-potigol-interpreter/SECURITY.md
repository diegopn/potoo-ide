# Security Policy

## Supported versions

This crate is part of the Potoo IDE workspace. The root [Security Policy](../../SECURITY.md) defines which branches and releases are in scope.

## Report a vulnerability

Do not publish vulnerability details in issues, pull requests, discussions, or public comments. Use the Potoo IDE repository's **Security → Report a vulnerability** private reporting form when it is available.

If private reporting is unavailable, use the [Potoo contact page](https://diegopn.github.io/#contato) only to request a private reporting channel. Do not include technical details or a proof of concept in that first message.

Include the affected version or commit, Rust version, operating system, a minimal Potigol example, reproduction steps, and the impact. Remove credentials, personal information, and unrelated data.

## Security boundaries

This interpreter runs source code inside the host process; it is not an operating-system sandbox. Resource limits reduce excessive CPU, output, string, collection, and wait usage, but hosts can change those limits. The collection byte limit is an allocation estimate, not a strict live-heap ceiling. Hosts should keep suitable bounds and apply operating-system isolation when running untrusted programs in a privileged or shared service.

The host implements `RuntimeIo` and controls input, output, cancellation, waiting, and debugger behavior. Treat these callbacks as the authority boundary and avoid exposing unnecessary host capabilities. Potigol random functions are intended for programming exercises and must not be used to generate cryptographic secrets.

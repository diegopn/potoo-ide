# Security Policy

## Supported versions

Until a stable release is available, security reports for the latest published version and the repository's current default branch are in scope. Older `0.x` versions are reviewed case by case.

## Report a vulnerability

Do not publish vulnerability details in issues, pull requests, discussions, or public comments. Use the repository's **Security → Report a vulnerability** private reporting form when it is available.

If private reporting is unavailable, use the [Potoo contact page](https://diegopn.github.io/#contato) only to request a private reporting channel. Do not include technical details or a proof of concept in that first message.

Include the affected version or commit, Rust version, operating system, a minimal Portugol example, reproduction steps, and the impact. Remove credentials, personal information, and unrelated data.

## Security boundaries

This interpreter runs a program inside the host process; it is not an operating-system sandbox. Resource limits reduce accidental or abusive CPU, memory, output, and wait usage, but a host can change those limits. Hosts should keep appropriate bounds and apply operating-system isolation when executing programs from untrusted users in a privileged or shared service.

The host implements `RuntimeIo` and controls input, output, cancellation, and any additional capabilities it exposes. Treat those callbacks as the authority boundary and avoid giving untrusted programs unnecessary host access. The language's random functions are for programming exercises and must not be used to generate cryptographic secrets.

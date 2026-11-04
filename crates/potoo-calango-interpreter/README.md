# Potoo Calango Interpreter

An independent Rust interpreter for the Calango pseudolanguage. It reads the
`algoritmo`/`principal` structure and Calango's Portuguese keywords, including
typed variables, arrays, routines, expressions, control flow, and console I/O.

The interpreter exposes source diagnostics and a host I/O interface so Potoo
can run Calango programs and report input and debugger events.

The Calango built-in `pi()` returns Rust's IEEE-754 binary64 approximation
(`f64::consts::PI`) of π.

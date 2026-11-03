# Potoo Mapler Interpreter

Independent Rust interpreter for the Portugol Mapler syntax documented by the
[Projeto-Mapler Portugol repository](https://github.com/Projeto-Mapler/portugol)
and the [Mapler examples](https://portugol.sourceforge.io/exemplos/).

It implements the documented variable types and vectors, `escrever`/`ler`,
arithmetic and logical expressions, `se`/`senao`, `para`/`enquanto`/`repita`,
and declared modules. It exposes source-located diagnostics, type checking,
runtime limits, input/output hooks, and debugger events for host applications.

The original Mapler IDE stores source files as `.txt`. Potoo uses `.mapler` for
files it creates; the extension is not attributed to the original IDE. The
suffix follows [DesignLiquido's Mapler project](https://github.com/DesignLiquido/mapler)
and is referenced here only as the source of Potoo's file extension; syntax and
behavior follow Projeto-Mapler's documentation above.

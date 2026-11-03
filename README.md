<p align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.diegopn.potooide.svg" alt="Potoo IDE" width="112" />
</p>

# Potoo IDE

A desktop IDE for students and teachers to practice programming logic with Portugol, VisuAlg, Potigol, and G-Portugol.

The project aims to provide a simple, intuitive space for students to practice and for teachers to support the concepts taught through code. The interface is built with Rust, GTK 4, and Relm4, with Potoo-owned components and inspiration from GNOME Builder. Teachers choose which tools fit their classes; the IDE does not impose one learning path.

## Languages

**Runnable in the IDE; compatibility work is ongoing**

- **Portugol 2.0** (`.por`)
- **VisuAlg** (`.alg`; targets classic 3.0.5.2)
- **Potigol** (`.poti`; independent Rust implementation targeting the documented 1.0.0-RC2 language)
- **G-Portugol** (`.gpt`; independent Rust implementation targeting the documented 1.4.0 language)

The IDE includes a native compatibility layer for a supported subset of the Jerimum API through its GTK graphics and audio hosts. It provides native facades for selected `Color`, `BufferedImage`, `JFrame`, `Canvas`, and `Graphics2D` members without bundling the JVM or providing general Swing/AWT interop.

**Future**

- PSeInt
- Linotte
- C0 (Simplified C)
- Karel

The app runs Potigol through an independent Rust interpreter targeting the documented 1.0.0-RC2 language; it does not run or bundle the official Scala/JVM interpreter. See [crates/potoo-potigol-interpreter](crates/potoo-potigol-interpreter/README.md) for coverage and limits.

The app also runs Portugol programs with variables, functions, constants, conditionals, loops, vectors and matrices, interactive input, and the Matematica, Texto, Tipos, Util, Calendario, Arquivos, and Objetos libraries. Its reusable interpreter is in [crates/potoo-portugol-interpreter](crates/potoo-portugol-interpreter/README.md), which lists compatibility gaps.

The app can also run classic VisuAlg .alg programs through the reusable [potoo-visualg-interpreter](crates/potoo-visualg-interpreter/README.md) Rust library, which targets VisuAlg 3.0.5.2 and has no third-party dependencies. The interpreters provide interactive input, source-located diagnostics, resource limits, and debugger control. Compatibility details and current limits are documented in each crate README.

G-Portugol is a separate `.gpt` dialect, implemented in Rust by [potoo-gportugol-interpreter](crates/potoo-gportugol-interpreter/README.md). It has no third-party runtime dependencies and does not bundle or invoke the upstream `gpt` executable. The implementation targets the documented G-Portugol 1.4.0 syntax and behavior; native Windows and macOS builds have not yet been validated.

Project documentation is maintained in English. The app offers Portuguese, English, and Spanish UI languages. The project website is available in Brazilian Portuguese (default), English, and Spanish.

## Platforms

The goal is to offer Potoo IDE for Linux, Windows, and macOS. This base prepares the Linux Flatpak distribution first; support and installers for the other platforms still need to be developed and validated.

## Development

Requirements for running the native version: Rust 1.93 or later and GTK 4.20 with the platform development libraries.

```sh
cargo run --locked
```

To build and install a local Flatpak, with Flatpak and Flatpak Builder installed:

```sh
flatpak-builder --user --install --force-clean _build io.github.diegopn.potooide.json
```

## Contributing

Bug reports, suggestions, and contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request.

## License

Potoo IDE is distributed under the GNU General Public License, version 3 or (at your option) any later version. See [LICENSE](LICENSE).

## Links

- [Project website](https://diegopn.github.io/potoo-site/)
- [GitHub repository](https://github.com/diegopn/potoo-ide)
- [Sponsor development](https://github.com/sponsors/diegopn)

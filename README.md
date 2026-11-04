<p align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.diegopn.potooide.svg" alt="Potoo IDE" width="112" />
</p>

# Potoo IDE

A desktop IDE for students and teachers to practice programming logic with Portugol, VisuAlg, Potigol, and G-Portugol.

The project aims to provide a simple, intuitive space for students to practice and for teachers to support the concepts taught through code. The interface is built with Rust, GTK 4, and Relm4, with Potoo-owned components and inspiration from GNOME Builder. Teachers choose which tools fit their classes; the IDE does not impose one learning path.

## Languages

**Runnable in the IDE; compatibility work is ongoing**

- **Portugol Studio 2.0** (`.por`)
- **VisuAlg** (`.alg`; targets classic 3.0.5.2)
- **Potigol** (`.poti`; independent Rust implementation targeting the documented 1.0.0-RC2 language)
- **G-Portugol** (`.gpt`; independent Rust implementation targeting the documented 1.4.0 language)

**Integrations in development**

- **Calango** (`.clg`)
- **Portugol Mapler** (`.mapler`; Potoo-specific extension)

Each interpreter has a standalone repository so it can be installed and used outside Potoo IDE. Mapler is a separate Portugol dialect; its upstream manual describes structured Portuguese pseudocode and `.txt` files. See the [Mapler manual](https://portugol.sourceforge.io/manuais.html) and [syntax repository](https://github.com/Projeto-Mapler/portugol). Calango's upstream project is [available on GitHub](https://github.com/GeovanaRamos/calango).

## Independent interpreters

Each interpreter repository contains its Rust library, a standalone command-line runner, GPL-3.0-or-later license, platform-specific installation instructions, and CI checks for Linux, Windows, and macOS. The IDE pins each library to a specific Git commit and bundles it into the application at build time. The installed IDE does not need network access to GitHub. Building the IDE from source may need network access the first time to fetch these dependencies, unless they are already cached.

- [Calango](https://github.com/diegopn/potoo-calango-interpreter)
- [G-Portugol](https://github.com/diegopn/potoo-gportugol-interpreter)
- [Portugol Mapler](https://github.com/diegopn/potoo-portugol-mapler-interpreter)
- [Portugol Studio 2.0](https://github.com/diegopn/potoo-portugol-studio-2-interpreter)
- [Potigol](https://github.com/diegopn/potoo-potigol-interpreter)
- [Portugol VisuAlg](https://github.com/diegopn/potoo-portugol-visualg-interpreter)

The IDE includes a native compatibility layer for a supported subset of the Jerimum API through its GTK graphics and audio hosts. It provides native facades for selected `Color`, `BufferedImage`, `JFrame`, `Canvas`, and `Graphics2D` members without bundling the JVM or providing general Swing/AWT interop.

**Future**

- PSeInt
- Linotte
- C0 (Simplified C)
- Karel
- [Égua](https://github.com/eguadev/egua)
- Logic Basic

The app runs Potigol through an independent Rust interpreter targeting the documented 1.0.0-RC2 language; it does not run or bundle the official Scala/JVM interpreter. See the [Potigol interpreter README](https://github.com/diegopn/potoo-potigol-interpreter#readme) for coverage and limits.

The app also runs Portugol Studio 2.0 programs with variables, functions, constants, conditionals, loops, vectors and matrices, interactive input, and the Matematica, Texto, Tipos, Util, Calendario, Arquivos, and Objetos libraries. See the [Portugol Studio 2.0 interpreter README](https://github.com/diegopn/potoo-portugol-studio-2-interpreter#readme) for compatibility gaps.

The app can also run classic VisuAlg `.alg` programs through the reusable [Portugol VisuAlg Rust library](https://github.com/diegopn/potoo-portugol-visualg-interpreter#readme), which targets VisuAlg 3.0.5.2 and has no third-party dependencies. The interpreters provide interactive input, source-located diagnostics, resource limits, and debugger control. Compatibility details and current limits are documented in each repository README.

G-Portugol is a separate `.gpt` dialect, implemented in Rust by the [G-Portugol interpreter](https://github.com/diegopn/potoo-gportugol-interpreter#readme). It has no third-party runtime dependencies and does not bundle or invoke the upstream `gpt` executable. The implementation targets the documented G-Portugol 1.4.0 syntax and behavior.

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

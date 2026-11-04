<p align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.diegopn.potooide.svg" alt="Potoo IDE" width="112" />
</p>

# Potoo IDE

A desktop IDE for students and teachers to practice programming logic with Portugol dialects, Calango, and Potigol.

The project aims to provide a simple, intuitive space for students to practice and for teachers to support the concepts taught through code. The interface is built with Rust, GTK 4, and Relm4, with Potoo-owned components and inspiration from GNOME Builder. Teachers choose which tools fit their classes; the IDE does not impose one learning path.

## Languages

The IDE has execution integrations for these languages. Compatibility work is ongoing; each interpreter repository documents its supported syntax and current limits.

- **Portugol Studio 2.0** (`.por`)
- **VisuAlg** (`.alg`; compatibility target: classic 3.0.5.2)
- **Potigol** (`.poti`; independent Rust implementation targeting the documented 1.0.0-RC2 language core)
- **G-Portugol** (`.gpt`; independent Rust implementation targeting the documented 1.4.0 language)
- **Calango** (`.clg`)
- **Portugol Mapler** (`.mapler`)

Mapler is a separate Portugol dialect. The original Mapler IDE uses `.txt`; Potoo adopts `.mapler` from [Design Liquido's Mapler implementation](https://github.com/DesignLiquido/mapler), which began as part of Delégua. See the [Mapler manual](https://portugol.sourceforge.io/manuais.html) and [syntax repository](https://github.com/Projeto-Mapler/portugol). Calango's upstream project is [available on GitHub](https://github.com/GeovanaRamos/calango).

## Independent interpreters

The IDE uses independent Rust interpreters maintained in separate repositories. Each repository includes a reusable library and a standalone command-line runner, and documents its compatibility target and limits. The IDE pins each interpreter to a Git commit in `Cargo.toml` and builds it with the application. The installed app does not need GitHub access; building from source may need to fetch dependencies that are not already cached.

- [Calango](https://github.com/diegopn/potoo-calango-interpreter)
- [G-Portugol](https://github.com/diegopn/potoo-gportugol-interpreter)
- [Portugol Mapler](https://github.com/diegopn/potoo-portugol-mapler-interpreter)
- [Portugol Studio 2.0](https://github.com/diegopn/potoo-portugol-studio-2-interpreter)
- [Potigol](https://github.com/diegopn/potoo-potigol-interpreter)
- [Portugol VisuAlg](https://github.com/diegopn/potoo-portugol-visualg-interpreter)

The Potigol interpreter includes a compatibility layer for a supported subset of the Jerimum API. Potoo routes graphics and audio requests through GTK hosts and provides native facades for selected `Color`, `BufferedImage`, `JFrame`, `Canvas`, and `Graphics2D` members. It does not bundle the JVM or provide general Swing/AWT interoperability.

**Future**

- PSeInt
- Linotte
- C0 (Simplified C)
- Karel
- [Égua](https://github.com/eguadev/egua)
- Logic Basic

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

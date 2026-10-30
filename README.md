<p align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.diegopn.potooide.svg" alt="Potoo IDE" width="112" />
</p>

# Potoo IDE

A desktop IDE for students and teachers to practice programming logic with Portugol and VisuAlg.

The project aims to provide a simple, intuitive space for students to practice and for teachers to support the concepts taught through code. The interface is built with Rust, GTK 4, and Relm4, with Potoo-owned components and inspiration from GNOME Builder. Teachers choose which tools fit their classes; the IDE does not impose one learning path.

## Languages

**In development**

- Portugol
- VisuAlg

**Future**

- Potigol
- PSeInt
- Linotte
- C0 (Simplified C)
- Karel

The current native preview includes an editable code area and the main learning workspace. Portugol and VisuAlg execution engines are not connected yet; compatibility and integration work will be delivered in stages. Project documentation is maintained in English. Both the app and project website are planned to support English (default), Brazilian Portuguese, and Spanish; app localization has not been implemented yet.

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

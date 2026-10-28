<p align="center">
  <img src="data/icons/hicolor/scalable/apps/io.github.diegopn.potooide.svg" alt="Potoo IDE" width="112" />
</p>

# Potoo IDE

A desktop IDE for learning programming logic with educational languages, including Portugol, VisuAlg, and Potigol.

The project aims to provide a simple, intuitive space for students to practice and for teachers to support the concepts taught through code. The interface is built with Rust, GTK 4, and libadwaita, with inspiration from GNOME Builder. Teachers choose which tools fit their classes; the IDE does not impose one learning path.

## Languages

- **Portugol**, **VisuAlg**, and **Potigol** are planned for the first stage.
- Potigol is a beginner-friendly language with Portuguese keywords, strong static typing, and type inference ([official project](https://github.com/potigol/potigol)).
- **C0 (Simplified C)** is being considered as another optional language for a future stage. Carnegie Mellon University's [course repository](https://github.com/CMU-07-120/intro-to-software-construction) documents `coin`, the C0 interpreter, and `cc0`, the compiler. The course repository includes an [Unlicense](https://github.com/CMU-07-120/intro-to-software-construction/blob/main/LICENSE), while its [submodule definition](https://github.com/CMU-07-120/intro-to-software-construction/blob/main/.gitmodules) points C0 to a separate [Bitbucket project](https://bitbucket.org/c0-lang/c0/src/master/). We did not find an explicit redistribution license for the C0 source in the upstream locations reviewed, so permission to reuse or bundle `coin` remains unconfirmed. An independently written simplified-C language or interpreter is a separate option and would not automatically be compatible with official C0. The C0 tutorial identifies Frank Pfenning as a principal designer and maintainer; CMU lists [fp@cmu.edu](mailto:fp@cmu.edu) as his contact for permission requests. No permission has been granted. Confirm licensing and compatibility before reusing the official implementation or claiming official C0 compatibility.
- **PSeInt** and **Linotte** are possibilities for a later stage.
- [**Karel**](https://en.wikipedia.org/wiki/Karel_(programming_language)) remains a final candidate for evaluation; its fit for the project and the appropriate variant still need to be decided.

The current scaffold does not yet include a code editor or interpreters. Compatibility and integration work will be delivered in stages. Project documentation is maintained in English. Both the app and project website are planned to support English (default), Brazilian Portuguese, and Spanish; app localization has not been implemented yet.

## Platforms

The goal is to offer Potoo IDE for Linux, Windows, and macOS. This base prepares the Linux Flatpak distribution first; support and installers for the other platforms still need to be developed and validated.

## Development

Requirements for running the native version: Rust 1.92 or later, GTK 4.20, and libadwaita 1.8 with the platform development libraries.

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

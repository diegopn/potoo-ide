<p align="center">
  <a href="https://diegopn.github.io/potoo-site/">
    <img src="data/icons/hicolor/scalable/apps/io.github.diegopn.potooide.svg" alt="Potoo IDE icon" width="112" />
  </a>
</p>

<h1 align="center">Potoo IDE</h1>

<p align="center">A desktop IDE for students and teachers learning programming logic through Portugol dialects, VisuAlg, and other educational languages.</p>

<p align="center">
  <a href="https://github.com/diegopn/potoo-ide/releases/latest">Download</a> ·
  <a href="https://diegopn.github.io/potoo-site/">Website</a> ·
  <a href="https://diegopn.github.io/potoo-site/manual/">User manual</a>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img alt="Rust" src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white"></a>
  <a href="https://www.gtk.org/"><img alt="GTK 4" src="https://img.shields.io/badge/GTK-4A86CF?style=for-the-badge&logo=gnome&logoColor=white"></a>
  <a href="https://relm4.org/"><img alt="Relm4" src="https://img.shields.io/badge/Relm4-6A4C93?style=for-the-badge"></a>
</p>

Potoo helps students and teachers explore programming logic through Portugol and VisuAlg. Its language integrations are maintained as independent Rust interpreters.

## Supported languages

| Language | Files |
| --- | --- |
| [Portugol Studio 2.0](https://github.com/diegopn/potoo-portugol-studio-2-interpreter) | `.por` |
| [VisuAlg 3.0.5.2](https://github.com/diegopn/potoo-portugol-visualg-interpreter) | `.alg` |
| [Potigol 1.0.0-RC2 core](https://github.com/diegopn/potoo-potigol-interpreter) | `.poti` |
| [G-Portugol 1.4.0](https://github.com/diegopn/potoo-gportugol-interpreter) | `.gpt` |
| [Calango](https://github.com/diegopn/potoo-calango-interpreter) | `.clg` |
| [Portugol Mapler](https://github.com/diegopn/potoo-portugol-mapler-interpreter) | `.mapler` |

Compatibility is in progress; each language link opens its interpreter project, which documents supported syntax and current limits. The app interface is available in Portuguese, English, and Spanish.

## Install

| Platform | Package |
| --- | --- |
| Linux x86_64 | Flatpak bundle. If needed, [set up Flatpak](https://flatpak.org/setup) first. |
| Windows x64 | Setup installer with the GTK, GtkSourceView, and GStreamer runtime files. Windows may warn that the installer is unsigned. |
| macOS | Installers are not available yet. |

Download the [latest release](https://github.com/diegopn/potoo-ide/releases/latest). To install the Linux bundle:

```sh
flatpak install --user --bundle ./Potoo-IDE-<version>-linux-x86_64.flatpak
flatpak run io.github.diegopn.potooide
```

The first Flatpak installation may download the GNOME 51 runtime. Release downloads include `SHA256SUMS` for verification.

## Building from source on Linux

Requirements: Rust 1.93 or later, GTK 4.20 or later, and GtkSourceView 5 development files. Package versions vary by release; check that your distribution provides GTK 4.20 before building.

### Fedora and compatible RPM distributions

```sh
sudo dnf install gcc rustup pkgconf-pkg-config gtk4-devel gtksourceview5-devel
```

On Fedora, run `rustup-init` once after installing `rustup`.

### Debian, Ubuntu, and derivatives

```sh
sudo apt-get install build-essential rustup pkg-config libgtk-4-dev libgtksourceview-5-dev
```

### Arch-based distributions

```sh
sudo pacman -S --needed base-devel rustup pkgconf gtk4 gtksourceview5
```

### openSUSE Tumbleweed

```sh
sudo zypper install gcc rustup pkgconf-pkg-config gtk4-devel gtksourceview5-devel
```

### Build and run

```sh
cargo run --locked
```

To build and install a local Flatpak, with Flatpak and Flatpak Builder installed:

```sh
flatpak-builder --user --install --force-clean _build io.github.diegopn.potooide.json
```

## Contributing

Bug reports, suggestions, and contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidance.

## Support

If Potoo is useful in your classes or studies, you can support its continued development:

<p align="center">
  <a href="https://github.com/sponsors/diegopn"><img alt="Become a sponsor" src="https://img.shields.io/badge/Become_a_sponsor-EA4AAA?style=for-the-badge&logo=githubsponsors&logoColor=white"></a>
</p>

## License

Potoo IDE is distributed under the GNU General Public License, version 3 or (at your option) any later version. See [LICENSE](LICENSE).

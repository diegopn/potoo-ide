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

Built with Rust, GTK 4, and Relm4, Potoo lets teachers choose tools that fit their classes without imposing a single learning path.

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

Potoo IDE installers are currently available for Windows x64 and x86_64 Linux (Flatpak). macOS installers are not included yet; Apple Silicon builds are also unavailable.

## Install

Download the installers from the [latest GitHub release](https://github.com/diegopn/potoo-ide/releases/latest). The release also includes a ZIP containing the installers available for that release and a `SHA256SUMS` file.

### Windows

1. Download `Potoo-IDE-<version>-windows-x64-setup.exe`.
2. Run the installer and follow its prompts.
3. Start Potoo IDE from the Start menu.

The installer includes the GTK, GtkSourceView, and GStreamer runtime files needed by the application. The Windows installer is not code-signed, so Windows may show a publisher warning.

### macOS

The macOS installer is not included in this release while we work through packaging issues for Intel Macs. Apple Silicon builds are not available yet.

### Linux (Flatpak)

If Flatpak is not yet set up on your computer, [follow the instructions to prepare your system](https://flatpak.org/setup) before installing the app. Then install the downloaded bundle:

```sh
flatpak install --user --bundle ./Potoo-IDE-<version>-linux-x86_64.flatpak
flatpak run io.github.diegopn.potooide
```

The single-file Flatpak bundle contains the application, not its runtime. If the GNOME 51 runtime is not already installed, Flatpak downloads it from the configured Flathub remote during installation; internet access is required for that first install.

### Verify downloads

Download `SHA256SUMS` alongside the installers. On Linux or macOS, verify them with:

```sh
sha256sum --check SHA256SUMS
```

On Windows PowerShell, compare the output of `Get-FileHash <installer> -Algorithm SHA256` with the matching entry in `SHA256SUMS`.

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

Bug reports, suggestions, and contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) before opening an issue or pull request.

## License

Potoo IDE is distributed under the GNU General Public License, version 3 or (at your option) any later version. See [LICENSE](LICENSE).

## More links

- [Source repository](https://github.com/diegopn/potoo-ide)
- [Sponsor development](https://github.com/sponsors/diegopn)

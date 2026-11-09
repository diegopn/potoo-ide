# Building Potoo IDE

## Native Linux build

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

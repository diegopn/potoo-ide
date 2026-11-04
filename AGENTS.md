# Project Guidance

## Product and scope

- Potoo IDE is a desktop IDE for students learning programming logic and for the teachers who support them.
- The current language focus is Portugol and VisuAlg. Other languages listed as future candidates in `README.md` are not current implementation commitments.
- Do not describe an interpreter, debugger, or cross-platform installer as available unless it is implemented and verified.

## Technology and platforms

- The application is written in Rust and uses GTK 4.20 with Relm4 0.11. The interface does not use Libadwaita.
- Linux, Windows, and macOS are target platforms. Keep platform-specific behavior behind the existing Rust `cfg` boundaries and avoid GNOME-only assumptions in shared UI.
- Preserve the current dependency choices unless a change is needed for the requested work.

## Structure

- `src/main.rs` initializes GTK, registers bundled fonts and icons, loads the application stylesheet, and starts the Relm4 application.
- `src/app.rs` assembles the main window, application state, dialogs, and interactions.
- Reusable UI modules live in `src/components/`; keep component-specific view and model code with its component where practical.
- `data/potoo.css` contains the GTK styling and light/dark color tokens.

## Interface and visual design

- Treat `docs/mockups/potoo-interface-study.html` as the visual reference for layout, colors, typography, and component states.
- Keep the Potoo interface platform-neutral and visually consistent across Linux desktop environments, Windows, and macOS. Use GTK 4 widgets with Potoo-owned styling; do not adopt GNOME-specific layout conventions by default.
- Preserve both light and dark themes. Keep the corresponding color tokens and component states in `data/potoo.css` aligned with the mockup.
- Prefer scoped `potoo-*` CSS classes. Avoid broad widget selectors that unintentionally restyle unrelated controls.
- Preserve the established square-corner visual language and restrained green accent. Use neutral surfaces for large areas.

## Project files and documentation

- Keep public-facing Markdown documentation in English.
- `docs/` contains internal project notes and mockups. It is intentionally excluded from version control; do not force-add its contents.
- Preserve the Flatpak manifest, root-level public documents, and `.github/` configuration when making unrelated changes.

## Verification

- Format Rust changes with `cargo fmt --all` and check the Linux build with `cargo check --locked` when appropriate.
- Report which checks were run. Do not claim Windows or macOS validation without building or running on those platforms.

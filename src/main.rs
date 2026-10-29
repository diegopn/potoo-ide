mod app;
mod components;

use gtk::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

fn register_app_icons() {
    let Some(display) = gtk::gdk::Display::default() else {
        return;
    };

    let icon_theme = gtk::IconTheme::for_display(&display);
    let mut icon_paths = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("data/icons/hicolor/symbolic/actions"),
    ];
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        icon_paths.extend([
            directory.join("data/icons/hicolor/symbolic/actions"),
            directory.join("icons/hicolor/symbolic/actions"),
            directory.join("share/icons/hicolor/symbolic/actions"),
            directory.join("../share/icons/hicolor/symbolic/actions"),
            directory.join("../Resources/icons/hicolor/symbolic/actions"),
        ]);
    }

    for path in icon_paths {
        if path.is_dir() {
            icon_theme.add_search_path(&path);
        }
    }
}

fn main() {
    let application = adw::Application::builder()
        .application_id("io.github.diegopn.potooide")
        .build();

    application.connect_startup(|_| {
        register_app_icons();
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("../data/potoo.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });

    RelmApp::from_app(application).run::<app::MainWindow>(());
}

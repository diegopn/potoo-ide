mod app;
mod components;

use gtk::prelude::*;
use relm4::prelude::*;

fn main() {
    let application = adw::Application::builder()
        .application_id("io.github.diegopn.potooide")
        .build();

    application.connect_startup(|_| {
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

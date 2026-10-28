use adw::prelude::*;
use gtk::prelude::*;

fn main() {
    let application = adw::Application::builder()
        .application_id("io.github.diegopn.potooide")
        .build();

    application.connect_activate(build_window);
    let _ = application.run();
}

fn build_window(application: &adw::Application) {
    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Potoo IDE")
        .default_width(1100)
        .default_height(760)
        .build();

    window.set_titlebar(Some(&adw::HeaderBar::new()));

    let welcome = gtk::Label::builder()
        .label("Potoo IDE

Seu espaço para programar em pseudolinguagens.")
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .build();

    window.set_content(Some(&welcome));
    window.present();
}

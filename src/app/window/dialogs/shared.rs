use super::*;

pub(in crate::app::window) fn set_dialog_titlebar(
    window: &gtk::Window,
    language: UiLanguage,
    title: &'static str,
) {
    close_window_on_escape(window);

    let titlebar = gtk::HeaderBar::new();
    titlebar.add_css_class("potoo-dialog-titlebar");
    titlebar.set_show_title_buttons(false);
    let title_label = gtk::Label::new(Some(tr(language, title)));
    title_label.add_css_class("potoo-settings-window-title");
    titlebar.set_title_widget(Some(&title_label));

    let close_button = gtk::Button::new();
    close_button.add_css_class("potoo-dialog-close");
    close_button.set_child(Some(&icon("close")));
    close_button.set_tooltip_text(Some(tr(language, "Fechar")));
    titlebar.pack_end(&close_button);
    window.set_titlebar(Some(&titlebar));

    let window_for_close = window.clone();
    close_button.connect_clicked(move |_| window_for_close.close());
}

pub(in crate::app::window) fn close_window_on_escape(window: &gtk::Window) {
    let key_controller = gtk::EventControllerKey::new();
    let weak_window = window.downgrade();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gtk::gdk::Key::Escape {
            if let Some(window) = weak_window.upgrade() {
                window.close();
            }
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);
}

pub(in crate::app::window) fn language_option_button(name: &str, detail: &str) -> gtk::Button {
    let button = gtk::Button::new();
    let choice = gtk::Box::new(gtk::Orientation::Vertical, 3);
    choice.set_halign(gtk::Align::Start);
    let name_label = gtk::Label::new(Some(name));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-language-name");
    let detail_label = gtk::Label::new(Some(detail));
    detail_label.set_xalign(0.0);
    detail_label.add_css_class("potoo-project-language-detail");
    choice.append(&name_label);
    choice.append(&detail_label);
    button.set_child(Some(&choice));
    button.set_tooltip_text(Some(detail));
    button
}

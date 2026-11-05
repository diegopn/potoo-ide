use super::ThemePreference;
use gtk::prelude::*;

pub(super) fn system_theme_is_dark() -> bool {
    gtk::Settings::default().is_some_and(|settings| {
        settings.property::<gtk::InterfaceColorScheme>("gtk-interface-color-scheme")
            == gtk::InterfaceColorScheme::Dark
    })
}

pub(in crate::app::window) fn theme_is_dark(theme: ThemePreference) -> bool {
    match theme {
        ThemePreference::System => system_theme_is_dark(),
        ThemePreference::Light => false,
        ThemePreference::Dark => true,
    }
}

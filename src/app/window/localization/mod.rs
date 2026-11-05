mod english;
mod spanish;
mod theme;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) use tests::{UI_TRANSLATION_KEYS, translate_known_text};
pub(super) use theme::theme_is_dark;

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UiLanguage {
    Portuguese,
    English,
    Spanish,
}

impl UiLanguage {
    pub(super) fn key(self) -> &'static str {
        match self {
            Self::Portuguese => "pt-BR",
            Self::English => "en",
            Self::Spanish => "es",
        }
    }

    pub(super) fn from_key(key: &str) -> Option<Self> {
        match key {
            "pt" | "pt-BR" => Some(Self::Portuguese),
            "en" => Some(Self::English),
            "es" => Some(Self::Spanish),
            _ => None,
        }
    }

    pub(super) fn from_locale_name(locale: &str) -> Option<Self> {
        let language = locale
            .split(['_', '-', '.', '@'])
            .next()?
            .to_ascii_lowercase();
        match language.as_str() {
            "pt" => Some(Self::Portuguese),
            "en" => Some(Self::English),
            "es" => Some(Self::Spanish),
            _ => None,
        }
    }

    pub(super) fn from_language_names<I, S>(names: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        names
            .into_iter()
            .find_map(|name| Self::from_locale_name(name.as_ref()))
            .unwrap_or(Self::English)
    }

    pub(super) fn from_preference<I, S>(saved: Option<&str>, system_languages: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        saved
            .and_then(Self::from_key)
            .unwrap_or_else(|| Self::from_language_names(system_languages))
    }

    pub(super) fn from_system() -> Self {
        Self::from_language_names(gtk::glib::language_names())
    }

    pub(super) fn selected_index(self) -> u32 {
        match self {
            Self::Portuguese => 0,
            Self::English => 1,
            Self::Spanish => 2,
        }
    }

    pub(super) fn from_selected_index(index: u32) -> Self {
        match index {
            1 => Self::English,
            2 => Self::Spanish,
            _ => Self::Portuguese,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ThemePreference {
    System,
    Light,
    Dark,
}

impl ThemePreference {
    pub(super) fn selected_index(self) -> u32 {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }

    pub(super) fn from_selected_index(index: u32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }

    pub(super) fn key(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    pub(super) fn from_key(key: &str) -> Self {
        match key {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
}

pub(crate) fn tr(language: UiLanguage, portuguese: &'static str) -> &'static str {
    match language {
        UiLanguage::Portuguese => portuguese,
        UiLanguage::English => english::translate(portuguese),
        UiLanguage::Spanish => spanish::translate(portuguese),
    }
}

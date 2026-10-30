use crate::{
    app::{MainWindow, MainWindowMsg, UiLanguage, tr},
    components::{content_panel::Language, icon},
};
use gtk::prelude::*;
use relm4::prelude::*;
use std::path::{Path, PathBuf};

pub(crate) struct StartPageUi {
    pub(crate) widget: gtk::Widget,
    pub(crate) recent_list: gtk::Box,
    pub(crate) portugol_button: gtk::Button,
    pub(crate) visualg_button: gtk::Button,
    pub(crate) open_folder_button: gtk::Button,
    pub(crate) settings_button: gtk::Button,
}

pub(crate) fn build_start_page(language: UiLanguage) -> StartPageUi {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.add_css_class("potoo-start-page");

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .build();
    scroll.add_css_class("potoo-start-scroll");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-start-content");
    content.set_halign(gtk::Align::Center);
    content.set_size_request(720, -1);
    content.set_margin_top(48);
    content.set_margin_bottom(48);

    let intro = gtk::Box::new(gtk::Orientation::Vertical, 8);
    intro.add_css_class("potoo-start-intro");
    intro.set_margin_bottom(30);
    let title = gtk::Label::new(Some(tr(language, "Abra um programa")));
    title.set_xalign(0.0);
    title.add_css_class("potoo-start-title");
    let description = gtk::Label::new(Some(tr(
        language,
        "Continue um exercício recente ou comece um novo.",
    )));
    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("potoo-start-description");
    intro.append(&title);
    intro.append(&description);
    content.append(&intro);

    let recent_heading = gtk::Label::new(Some(tr(language, "Programas recentes")));
    recent_heading.set_xalign(0.0);
    recent_heading.add_css_class("potoo-start-section-title");
    recent_heading.set_margin_bottom(8);
    content.append(&recent_heading);

    let recent_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
    recent_list.add_css_class("potoo-start-recent-list");
    content.append(&recent_list);

    let create_heading = gtk::Label::new(Some(tr(language, "Comece um novo programa")));
    create_heading.set_xalign(0.0);
    create_heading.add_css_class("potoo-start-section-title");
    create_heading.set_margin_top(30);
    create_heading.set_margin_bottom(8);
    content.append(&create_heading);

    let create_options = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    create_options.add_css_class("potoo-start-create-options");
    let portugol_button = language_option(
        "Portugol",
        tr(language, "Projeto Portugol"),
        tr(language, "Comece com um arquivo .por"),
    );
    let visualg_button = language_option(
        "VisuAlg",
        tr(language, "Projeto VisuAlg"),
        tr(language, "Comece com um arquivo .alg"),
    );
    create_options.append(&portugol_button);
    create_options.append(&visualg_button);
    content.append(&create_options);

    let open_folder_button = gtk::Button::new();
    open_folder_button.add_css_class("potoo-start-open-folder");
    let folder_content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    folder_content.set_hexpand(true);
    folder_content.append(&icon("folder-open"));
    let folder_text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    folder_text.set_hexpand(true);
    let folder_title = gtk::Label::new(Some(tr(language, "Selecionar uma pasta")));
    folder_title.set_xalign(0.0);
    folder_title.add_css_class("potoo-start-action-title");
    let folder_description = gtk::Label::new(Some(tr(
        language,
        "Abra um projeto que não aparece na lista.",
    )));
    folder_description.set_xalign(0.0);
    folder_description.add_css_class("potoo-start-action-description");
    folder_text.append(&folder_title);
    folder_text.append(&folder_description);
    folder_content.append(&folder_text);
    folder_content.append(&icon("chevron-right"));
    open_folder_button.set_child(Some(&folder_content));
    open_folder_button.set_tooltip_text(Some(tr(language, "Selecionar uma pasta")));
    open_folder_button.set_margin_top(10);
    content.append(&open_folder_button);

    scroll.set_child(Some(&content));
    page.append(&scroll);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    footer.add_css_class("potoo-start-footer");
    let settings_button = gtk::Button::new();
    settings_button.add_css_class("potoo-start-settings");
    let settings_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    settings_content.append(&icon("settings"));
    settings_content.append(&gtk::Label::new(Some(tr(language, "Configurações"))));
    settings_button.set_child(Some(&settings_content));
    footer.append(&settings_button);
    page.append(&footer);

    StartPageUi {
        widget: page.upcast(),
        recent_list,
        portugol_button,
        visualg_button,
        open_folder_button,
        settings_button,
    }
}

fn language_option(label: &str, title: &str, description: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("potoo-start-language-option");
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.set_hexpand(true);
    content.append(&icon("new-document"));
    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_hexpand(true);
    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("potoo-start-action-title");
    let description_label = gtk::Label::new(Some(description));
    description_label.set_xalign(0.0);
    description_label.add_css_class("potoo-start-action-description");
    text.append(&title_label);
    text.append(&description_label);
    content.append(&text);
    let language_label = gtk::Label::new(Some(label));
    language_label.add_css_class("potoo-start-language-badge");
    content.append(&language_label);
    button.set_child(Some(&content));
    button.set_tooltip_text(Some(title));
    button.set_hexpand(true);
    button
}

pub(crate) fn rebuild_recent_items(
    container: &gtk::Box,
    paths: &[PathBuf],
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    if paths.is_empty() {
        let empty = gtk::Label::new(Some(tr(
            language,
            "Seus programas recentes aparecerão aqui.",
        )));
        empty.set_xalign(0.0);
        empty.add_css_class("potoo-start-recent-empty");
        container.append(&empty);
        return;
    }

    for (index, path) in paths.iter().enumerate() {
        if index > 0 {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.add_css_class("potoo-start-recent-separator");
            container.append(&separator);
        }

        let button = gtk::Button::new();
        button.add_css_class("potoo-start-recent-row");
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        content.set_hexpand(true);
        content.append(&icon(if path.is_dir() { "folder" } else { "document" }));

        let detail = gtk::Box::new(gtk::Orientation::Vertical, 3);
        detail.set_hexpand(true);
        let name = gtk::Label::new(Some(display_name(path)));
        name.set_xalign(0.0);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name.add_css_class("potoo-start-recent-name");
        let location = gtk::Label::new(Some(&path.display().to_string()));
        location.set_xalign(0.0);
        location.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        location.add_css_class("potoo-start-recent-location");
        detail.append(&name);
        detail.append(&location);
        content.append(&detail);

        let kind = if path.is_dir() {
            tr(language, "Pasta")
        } else {
            Language::from_path(path).map(Language::label).unwrap_or("")
        };
        let badge = gtk::Label::new(Some(kind));
        badge.add_css_class("potoo-start-recent-badge");
        content.append(&badge);
        content.append(&icon("chevron-right"));
        button.set_child(Some(&content));
        button.set_tooltip_text(Some(&path.display().to_string()));

        let selected_path = path.clone();
        let sender_for_recent = sender.clone();
        button.connect_clicked(move |_| {
            sender_for_recent.input(MainWindowMsg::RecentSelected(selected_path.clone()));
        });
        container.append(&button);
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

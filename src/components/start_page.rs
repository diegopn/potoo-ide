use crate::{
    app::{MainWindow, MainWindowMsg, UiLanguage, tr},
    components::icon,
};
use gtk::prelude::*;
use relm4::prelude::*;
use std::path::{Path, PathBuf};

pub(crate) struct StartPageUi {
    pub(crate) widget: gtk::Widget,
    pub(crate) recent_section: gtk::Box,
    pub(crate) recent_list: gtk::Box,
    pub(crate) portugol_button: gtk::Button,
    pub(crate) visualg_button: gtk::Button,
    pub(crate) potigol_button: gtk::Button,
    pub(crate) open_folder_button: gtk::Button,
    pub(crate) open_file_button: gtk::Button,
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
    content.set_margin_top(40);
    content.set_margin_bottom(40);

    let intro = gtk::Box::new(gtk::Orientation::Vertical, 8);
    intro.add_css_class("potoo-start-intro");
    intro.set_margin_bottom(30);
    let title = gtk::Label::new(Some(tr(language, "Vamos programar?")));
    title.set_xalign(0.0);
    title.add_css_class("potoo-start-title");
    let description = gtk::Label::new(Some(tr(
        language,
        "Continue de onde parou ou comece a transformar suas ideias em algoritmos.",
    )));
    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("potoo-start-description");
    intro.append(&title);
    intro.append(&description);
    content.append(&intro);

    let recent_section = gtk::Box::new(gtk::Orientation::Vertical, 0);
    recent_section.set_visible(false);

    let recent_heading = gtk::Label::new(Some(tr(language, "Programas recentes")));
    recent_heading.set_xalign(0.0);
    recent_heading.add_css_class("potoo-start-section-title");
    recent_heading.set_margin_bottom(8);
    recent_section.append(&recent_heading);

    let recent_list = gtk::Box::new(gtk::Orientation::Vertical, 0);
    recent_list.add_css_class("potoo-start-recent-list");
    recent_section.append(&recent_list);
    content.append(&recent_section);

    let create_heading = gtk::Label::new(Some(tr(language, "Comece um novo programa")));
    create_heading.set_xalign(0.0);
    create_heading.add_css_class("potoo-start-section-title");
    create_heading.set_margin_top(30);
    create_heading.set_margin_bottom(8);
    content.append(&create_heading);

    let create_options = gtk::Box::new(gtk::Orientation::Vertical, 8);
    create_options.add_css_class("potoo-start-create-options");
    let portugol_button =
        language_option("Portugol", tr(language, "Criar programa inicial (.por)"));
    let visualg_button = language_option("VisuAlg", tr(language, "Criar programa inicial (.alg)"));
    let potigol_button = language_option("Potigol", tr(language, "Criar programa inicial (.poti)"));
    create_options.append(&portugol_button);
    create_options.append(&visualg_button);
    create_options.append(&potigol_button);
    content.append(&create_options);

    let open_actions = gtk::Box::new(gtk::Orientation::Vertical, 8);
    open_actions.set_margin_top(12);
    let open_folder_button = open_option(
        language,
        "folder-open",
        "Selecionar uma pasta",
        "Abra um projeto que não aparece na lista.",
        "potoo-start-open-folder",
    );
    let open_file_button = open_option(
        language,
        "open-document",
        "Abrir arquivo",
        "Abra um arquivo .por, .alg ou .poti diretamente no editor.",
        "potoo-start-open-file",
    );
    open_actions.append(&open_folder_button);
    open_actions.append(&open_file_button);
    content.append(&open_actions);

    scroll.set_child(Some(&content));
    page.append(&scroll);

    StartPageUi {
        widget: page.upcast(),
        recent_section,
        recent_list,
        portugol_button,
        visualg_button,
        potigol_button,
        open_folder_button,
        open_file_button,
    }
}

fn open_option(
    language: UiLanguage,
    icon_name: &'static str,
    title: &'static str,
    description: &'static str,
    css_class: &'static str,
) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class(css_class);
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.set_hexpand(true);
    content.set_valign(gtk::Align::Center);
    content.append(&icon(icon_name));
    let text = gtk::Box::new(gtk::Orientation::Vertical, 3);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let title_label = gtk::Label::new(Some(tr(language, title)));
    title_label.set_xalign(0.0);
    title_label.add_css_class("potoo-start-action-title");
    let description_label = gtk::Label::new(Some(tr(language, description)));
    description_label.set_xalign(0.0);
    description_label.add_css_class("potoo-start-action-description");
    text.append(&title_label);
    text.append(&description_label);
    content.append(&text);
    content.append(&icon("chevron-right"));
    button.set_child(Some(&content));
    button.set_tooltip_text(Some(tr(language, title)));
    button.set_hexpand(true);
    button
}

fn language_option(language: &str, description: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.add_css_class("potoo-start-language-option");
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    content.set_hexpand(true);
    content.set_valign(gtk::Align::Center);
    content.append(&icon("new-document"));
    let text = gtk::Box::new(gtk::Orientation::Vertical, 4);
    text.set_hexpand(true);
    text.set_valign(gtk::Align::Center);
    let title_label = gtk::Label::new(Some(language));
    title_label.set_xalign(0.0);
    title_label.add_css_class("potoo-start-action-title");
    let description_label = gtk::Label::new(Some(description));
    description_label.set_xalign(0.0);
    description_label.add_css_class("potoo-start-action-description");
    text.append(&title_label);
    text.append(&description_label);
    content.append(&text);
    content.append(&icon("chevron-right"));
    button.set_child(Some(&content));
    button.set_tooltip_text(Some(language));
    button.set_hexpand(true);
    button
}

pub(crate) fn rebuild_recent_items(
    section: &gtk::Box,
    container: &gtk::Box,
    paths: &[PathBuf],
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    section.set_visible(!paths.is_empty());

    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    if paths.is_empty() {
        return;
    }

    for (index, path) in paths.iter().enumerate() {
        if index > 0 {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.add_css_class("potoo-start-recent-separator");
            container.append(&separator);
        }

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        row.add_css_class("potoo-start-recent-row");
        row.set_hexpand(true);

        let button = gtk::Button::new();
        button.add_css_class("potoo-start-recent-open");
        button.set_halign(gtk::Align::Fill);
        button.set_hexpand(true);
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        content.set_hexpand(true);
        content.set_valign(gtk::Align::Center);
        content.append(&icon(if path.is_dir() { "folder" } else { "document" }));

        let detail = gtk::Box::new(gtk::Orientation::Vertical, 3);
        detail.set_hexpand(true);
        let display_name = display_name(path);
        let name = gtk::Label::new(Some(&display_name));
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

        button.set_child(Some(&content));
        button.set_tooltip_text(Some(&path.display().to_string()));

        let selected_path = path.clone();
        let sender_for_recent = sender.clone();
        button.connect_clicked(move |_| {
            sender_for_recent.input(MainWindowMsg::RecentSelected(selected_path.clone()));
        });
        row.append(&button);

        let remove_button = gtk::Button::new();
        remove_button.add_css_class("potoo-start-recent-remove");
        remove_button.set_valign(gtk::Align::Center);
        let remove_icon = icon("close");
        remove_icon.set_pixel_size(13);
        remove_button.set_child(Some(&remove_icon));
        remove_button.set_tooltip_text(Some(tr(language, "Remover dos recentes")));

        let path_to_remove = path.clone();
        let sender_for_remove = sender.clone();
        remove_button.connect_clicked(move |_| {
            sender_for_remove.input(MainWindowMsg::RecentRemoved(path_to_remove.clone()));
        });
        row.append(&remove_button);

        let open_action_button = gtk::Button::new();
        open_action_button.add_css_class("potoo-start-recent-open-action");
        open_action_button.set_valign(gtk::Align::Center);
        open_action_button.set_child(Some(&icon("chevron-right")));
        open_action_button.set_tooltip_text(Some(tr(language, "Abrir um programa recente")));

        let selected_path = path.clone();
        let sender_for_open_action = sender.clone();
        open_action_button.connect_clicked(move |_| {
            sender_for_open_action.input(MainWindowMsg::RecentSelected(selected_path.clone()));
        });
        row.append(&open_action_button);
        container.append(&row);
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

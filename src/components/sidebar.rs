use crate::{
    app::{MainWindow, MainWindowMsg, UiLanguage, tr},
    components::{clear_box, content_panel::Language, icon_label_button},
};
use gtk::prelude::*;
use relm4::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectFile {
    pub(crate) path: PathBuf,
    pub(crate) relative_path: String,
}

pub(crate) struct SidebarUi {
    pub(crate) widget: gtk::Widget,
    pub(crate) project_label: gtk::Label,
    pub(crate) file_list: gtk::Box,
    pub(crate) new_file_button: gtk::Button,
    pub(crate) open_project_button: gtk::Button,
    pub(crate) open_file_button: gtk::Button,
    pub(crate) language_picker: gtk::DropDown,
}

pub(crate) fn build_file_sidebar(language: UiLanguage, default_language: Language) -> SidebarUi {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar.add_css_class("potoo-file-sidebar");
    sidebar.set_size_request(220, -1);

    let heading_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    heading_row.set_margin_start(16);
    heading_row.set_margin_end(10);
    heading_row.set_margin_top(12);
    heading_row.set_margin_bottom(8);
    let heading = gtk::Label::new(Some(tr(language, "Arquivos")));
    heading.set_xalign(0.0);
    heading.add_css_class("heading");
    heading.set_hexpand(true);
    let language_picker = gtk::DropDown::from_strings(&["Portugol", "VisuAlg"]);
    language_picker.add_css_class("potoo-sidebar-language-picker");
    language_picker.set_selected(default_language.selected_index());
    language_picker.set_tooltip_text(Some(tr(language, "Linguagem para novos arquivos")));
    let new_file_button = gtk::Button::new();
    new_file_button.set_child(Some(&gtk::Image::from_icon_name("list-add-symbolic")));
    new_file_button.set_tooltip_text(Some(tr(language, "Criar arquivo")));
    new_file_button.add_css_class("potoo-sidebar-action");
    new_file_button.set_sensitive(false);
    heading_row.append(&heading);
    heading_row.append(&language_picker);
    heading_row.append(&new_file_button);
    sidebar.append(&heading_row);

    let file_actions = gtk::Box::new(gtk::Orientation::Vertical, 4);
    file_actions.add_css_class("potoo-file-actions");
    let open_project_button = icon_label_button(
        "folder-open-symbolic",
        tr(language, "Abrir pasta"),
        tr(language, "Abrir pasta de projeto"),
        "potoo-file-action-button",
    );
    let open_file_button = icon_label_button(
        "document-open-symbolic",
        tr(language, "Abrir arquivo"),
        tr(language, "Abrir arquivo de código"),
        "potoo-file-action-button",
    );
    file_actions.append(&open_project_button);
    file_actions.append(&open_file_button);
    sidebar.append(&file_actions);

    let project = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    project.set_margin_start(12);
    project.set_margin_end(12);
    project.set_margin_top(4);
    project.set_margin_bottom(8);
    project.append(&gtk::Image::from_icon_name("folder-symbolic"));
    let project_label = gtk::Label::new(Some(tr(language, "Meu primeiro programa")));
    project_label.set_xalign(0.0);
    project_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    project_label.set_hexpand(true);
    project.append(&project_label);
    sidebar.append(&project);

    let file_list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let empty_label = gtk::Label::new(Some(tr(
        language,
        "Abra uma pasta de projeto para navegar pelos arquivos.",
    )));
    empty_label.set_wrap(true);
    empty_label.add_css_class("potoo-muted");
    empty_label.set_margin_start(14);
    empty_label.set_margin_end(14);
    empty_label.set_margin_top(12);
    file_list.append(&empty_label);
    let file_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&file_list)
        .build();
    sidebar.append(&file_scroll);

    SidebarUi {
        widget: sidebar.upcast(),
        project_label,
        file_list,
        new_file_button,
        open_project_button,
        open_file_button,
        language_picker,
    }
}

pub(crate) fn rebuild_file_list(
    file_list: &gtk::Box,
    files: &[ProjectFile],
    active_path: Option<&PathBuf>,
    project_open: bool,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    clear_box(file_list);
    if files.is_empty() {
        let message = if project_open {
            "Esta pasta ainda não tem arquivos .por ou .alg."
        } else {
            "Abra uma pasta de projeto para navegar pelos arquivos."
        };
        let empty_label = gtk::Label::new(Some(tr(language, message)));
        empty_label.set_wrap(true);
        empty_label.set_xalign(0.0);
        empty_label.add_css_class("potoo-muted");
        empty_label.set_margin_start(14);
        empty_label.set_margin_end(14);
        empty_label.set_margin_top(12);
        file_list.append(&empty_label);
        return;
    }

    for file in files {
        let button = gtk::Button::new();
        button.add_css_class("potoo-file-row");
        if active_path == Some(&file.path) {
            button.add_css_class("potoo-file-row-selected");
        }
        button.set_tooltip_text(Some(&file.path.display().to_string()));
        button.set_hexpand(true);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_hexpand(true);
        row.set_margin_start(10);
        row.set_margin_end(10);
        row.set_margin_top(7);
        row.set_margin_bottom(7);
        row.append(&gtk::Image::from_icon_name("text-x-generic-symbolic"));
        let label = gtk::Label::new(Some(&file.relative_path));
        label.set_xalign(0.0);
        label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        label.set_max_width_chars(28);
        label.set_hexpand(true);
        row.append(&label);
        button.set_child(Some(&row));
        let path = file.path.clone();
        let sender_for_file = sender.clone();
        button.connect_clicked(move |_| {
            sender_for_file.input(MainWindowMsg::OpenFile(path.clone()));
        });
        file_list.append(&button);
    }
}

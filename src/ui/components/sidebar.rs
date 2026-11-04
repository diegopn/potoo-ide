use crate::{
    app::{MainWindow, MainWindowMsg, UiLanguage, tr},
    languages::model::Language,
    ui::helpers::{clear_box, icon, icon_label_button},
};
use gtk::prelude::*;
use relm4::prelude::*;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    rc::Rc,
};

pub(crate) type ExpandedFolders = Rc<RefCell<HashSet<PathBuf>>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProjectFile {
    pub(crate) path: PathBuf,
    pub(crate) relative_path: String,
    pub(crate) is_directory: bool,
}

pub(crate) struct SidebarUi {
    pub(crate) widget: gtk::Widget,
    pub(crate) project_row: gtk::Box,
    pub(crate) project_label: gtk::Label,
    pub(crate) new_project_button: gtk::Button,
    pub(crate) file_list: gtk::Box,
    pub(crate) new_file_button: gtk::MenuButton,
    pub(crate) new_program_button: gtk::Button,
    pub(crate) open_file_menu_button: gtk::Button,
    pub(crate) new_folder_button: gtk::Button,
    pub(crate) new_item_popover: gtk::Popover,
    pub(crate) empty_state: gtk::Widget,
    pub(crate) empty_open_project_button: gtk::Button,
    pub(crate) empty_open_file_button: gtk::Button,
    pub(crate) settings_button: gtk::Button,
    pub(crate) expanded_folders: ExpandedFolders,
}

pub(crate) fn build_file_sidebar(language: UiLanguage) -> SidebarUi {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar.add_css_class("potoo-file-sidebar");
    sidebar.set_size_request(242, -1);

    let heading_row = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    heading_row.set_margin_start(12);
    heading_row.set_margin_end(8);
    heading_row.set_margin_top(12);
    heading_row.set_margin_bottom(8);
    let heading = gtk::Label::new(Some(tr(language, "Arquivos")));
    heading.set_xalign(0.0);
    heading.add_css_class("heading");
    heading.set_hexpand(true);
    heading.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let new_file_button = gtk::MenuButton::new();
    new_file_button.set_child(Some(&icon("add")));
    new_file_button.set_tooltip_text(Some(tr(language, "Criar item no projeto")));
    new_file_button.add_css_class("potoo-sidebar-action");
    new_file_button.set_sensitive(false);

    let new_item_popover = gtk::Popover::new();
    new_item_popover.add_css_class("potoo-sidebar-popover");
    let new_item_menu = gtk::Box::new(gtk::Orientation::Vertical, 2);
    new_item_menu.set_margin_top(4);
    new_item_menu.set_margin_bottom(4);
    new_item_menu.set_margin_start(4);
    new_item_menu.set_margin_end(4);
    let new_program_button = icon_label_button(
        "new-document",
        tr(language, "Novo arquivo"),
        tr(
            language,
            "Criar um arquivo Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler ou Potigol",
        ),
        "potoo-sidebar-menu-item",
    );
    let open_file_menu_button = icon_label_button(
        "open-document",
        tr(language, "Abrir arquivo"),
        tr(language, "Abrir arquivo de código"),
        "potoo-sidebar-menu-item",
    );
    let new_folder_button = icon_label_button(
        "new-folder",
        tr(language, "Nova pasta"),
        tr(language, "Criar uma pasta no projeto"),
        "potoo-sidebar-menu-item",
    );
    new_item_menu.append(&new_program_button);
    new_item_menu.append(&open_file_menu_button);
    new_item_menu.append(&new_folder_button);
    new_item_popover.set_child(Some(&new_item_menu));
    new_file_button.set_popover(Some(&new_item_popover));

    heading_row.append(&heading);
    heading_row.append(&new_file_button);
    sidebar.append(&heading_row);

    let new_project_button = icon_label_button(
        "new-folder",
        tr(language, "Novo projeto"),
        tr(
            language,
            "Criar um projeto Calango, G-Portugol, VisuAlg, Portugol Studio 2.0, Mapler ou Potigol",
        ),
        "potoo-sidebar-create-project",
    );
    new_project_button.set_margin_start(10);
    new_project_button.set_margin_end(10);
    new_project_button.set_margin_bottom(4);
    sidebar.append(&new_project_button);

    let project = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    project.add_css_class("potoo-project-row");
    project.set_margin_start(10);
    project.set_margin_end(10);
    project.set_margin_top(6);
    project.set_margin_bottom(4);
    project.set_visible(false);
    project.append(&icon("folder"));
    let project_label = gtk::Label::new(Some(tr(language, "Meu primeiro programa")));
    project_label.set_xalign(0.0);
    project_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    project_label.set_hexpand(true);
    project.append(&project_label);
    sidebar.append(&project);

    let file_list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let (empty_state, empty_open_project_button, empty_open_file_button) =
        empty_project_state(language);
    file_list.append(&empty_state);
    let file_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&file_list)
        .build();
    sidebar.append(&file_scroll);

    let settings_footer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    settings_footer.add_css_class("potoo-sidebar-footer");
    let settings_button = icon_label_button(
        "settings",
        tr(language, "Configurações"),
        tr(language, "Configurações"),
        "potoo-sidebar-settings-button",
    );
    settings_footer.append(&settings_button);
    sidebar.append(&settings_footer);

    SidebarUi {
        widget: sidebar.upcast(),
        project_row: project,
        project_label,
        new_project_button,
        file_list,
        new_file_button,
        new_program_button,
        open_file_menu_button,
        new_folder_button,
        new_item_popover,
        empty_state,
        empty_open_project_button,
        empty_open_file_button,
        settings_button,
        expanded_folders: Rc::new(RefCell::new(HashSet::new())),
    }
}

pub(crate) fn rebuild_file_list(
    file_list: &gtk::Box,
    files: &[ProjectFile],
    active_path: Option<&PathBuf>,
    project_open: bool,
    project_root: Option<&std::path::Path>,
    empty_state: &gtk::Widget,
    language: UiLanguage,
    expanded_folders: &Rc<RefCell<HashSet<PathBuf>>>,
    sender: ComponentSender<MainWindow>,
) {
    clear_box(file_list);
    if files.is_empty() {
        if project_open {
            let empty_label = gtk::Label::new(Some(tr(
                language,
                "Esta pasta não contém arquivos visíveis.",
            )));
            empty_label.set_wrap(true);
            empty_label.set_xalign(0.0);
            empty_label.add_css_class("potoo-muted");
            empty_label.set_margin_start(14);
            empty_label.set_margin_end(14);
            empty_label.set_margin_top(12);
            file_list.append(&empty_label);
        } else {
            file_list.append(empty_state);
        }
        return;
    }

    let mut folder_contents = HashMap::new();
    if let Some(project_root) = project_root {
        folder_contents.insert(project_root.to_path_buf(), file_list.clone());
    }
    let expanded_paths = expanded_folders.borrow().clone();
    if project_open
        && !files
            .iter()
            .any(|file| !file.is_directory && Language::from_path(&file.path).is_some())
    {
        let empty_label = gtk::Label::new(Some(tr(
            language,
            "Nenhum programa .por, .alg, .poti, .gpt, .clg ou .mapler nesta pasta. Os demais arquivos aparecem na lista.",
        )));
        empty_label.set_wrap(true);
        empty_label.set_xalign(0.0);
        empty_label.add_css_class("potoo-muted");
        empty_label.set_margin_start(14);
        empty_label.set_margin_end(14);
        empty_label.set_margin_top(12);
        empty_label.set_margin_bottom(8);
        file_list.append(&empty_label);
    }

    for file in files {
        if file.is_directory {
            let parent = file
                .path
                .parent()
                .and_then(|path| folder_contents.get(path))
                .cloned()
                .unwrap_or_else(|| file_list.clone());
            let expander = gtk::Expander::new(None);
            expander.add_css_class("potoo-folder-expander");
            expander.set_hexpand(true);
            let title = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            title.set_margin_start(8);
            title.set_margin_end(8);
            title.set_margin_top(4);
            title.set_margin_bottom(4);
            title.add_css_class("potoo-folder-title");
            let disclosure = icon(if expanded_paths.contains(&file.path) {
                "chevron-down"
            } else {
                "chevron-right"
            });
            disclosure.add_css_class("potoo-folder-disclosure");
            title.append(&disclosure);
            title.append(&icon("folder"));
            let folder_name = file
                .path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| file.relative_path.clone());
            let label = gtk::Label::new(Some(&folder_name));
            label.set_xalign(0.0);
            label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
            label.set_hexpand(true);
            title.append(&label);
            expander.set_label_widget(Some(&title));
            attach_project_item_context_menu(&title, &file.path, language, sender.clone());

            let children = gtk::Box::new(gtk::Orientation::Vertical, 1);
            children.set_margin_start(14);
            expander.set_child(Some(&children));
            expander.set_expanded(expanded_paths.contains(&file.path));
            let expanded_path = file.path.clone();
            let expanded_state = Rc::clone(expanded_folders);
            let disclosure_for_expander = disclosure.clone();
            expander.connect_expanded_notify(move |expander| {
                let mut paths = expanded_state.borrow_mut();
                if expander.is_expanded() {
                    paths.insert(expanded_path.clone());
                    disclosure_for_expander.set_icon_name(Some("potoo-chevron-down-symbolic"));
                } else {
                    paths.remove(&expanded_path);
                    disclosure_for_expander.set_icon_name(Some("potoo-chevron-right-symbolic"));
                }
            });
            parent.append(&expander);
            folder_contents.insert(file.path.clone(), children);
            continue;
        }

        let parent = file
            .path
            .parent()
            .and_then(|path| folder_contents.get(path))
            .cloned()
            .unwrap_or_else(|| file_list.clone());
        let button = gtk::Button::new();
        button.add_css_class("potoo-file-row");
        if active_path == Some(&file.path) {
            button.add_css_class("potoo-file-row-selected");
        }
        button.set_tooltip_text(Some(&file.relative_path));
        button.set_hexpand(true);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_hexpand(true);
        row.set_margin_start(10);
        row.set_margin_end(10);
        row.set_margin_top(7);
        row.set_margin_bottom(7);
        row.append(&icon("document"));
        let file_name = file
            .path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.relative_path.clone());
        let label = gtk::Label::new(Some(&file_name));
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
        attach_project_item_context_menu(&button, &file.path, language, sender.clone());
        parent.append(&button);
    }
}

fn attach_project_item_context_menu(
    target: &impl IsA<gtk::Widget>,
    path: &Path,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    let popover = gtk::Popover::new();
    popover.add_css_class("potoo-sidebar-popover");
    let menu = gtk::Box::new(gtk::Orientation::Vertical, 2);
    menu.set_margin_top(4);
    menu.set_margin_bottom(4);
    menu.set_margin_start(4);
    menu.set_margin_end(4);

    let rename = icon_label_button(
        "edit",
        tr(language, "Renomear"),
        tr(language, "Renomear"),
        "potoo-sidebar-menu-item",
    );
    let delete = icon_label_button(
        "delete",
        tr(language, "Excluir"),
        tr(language, "Excluir"),
        "potoo-sidebar-menu-item",
    );
    delete.add_css_class("potoo-sidebar-menu-danger");
    menu.append(&rename);
    menu.append(&delete);
    popover.set_child(Some(&menu));
    popover.set_parent(target);

    let rename_path = path.to_path_buf();
    let sender_for_rename = sender.clone();
    let popover_for_rename = popover.clone();
    rename.connect_clicked(move |_| {
        popover_for_rename.popdown();
        sender_for_rename.input(MainWindowMsg::RenameProjectItem(rename_path.clone()));
    });
    let delete_path = path.to_path_buf();
    let popover_for_delete = popover.clone();
    delete.connect_clicked(move |_| {
        popover_for_delete.popdown();
        sender.input(MainWindowMsg::DeleteProjectItem(delete_path.clone()));
    });

    let click = gtk::GestureClick::new();
    click.set_button(3);
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    let popover_for_click = popover;
    click.connect_pressed(move |gesture, _, x, y| {
        let point = gtk::gdk::Rectangle::new(x.round() as i32, y.round() as i32, 1, 1);
        popover_for_click.set_pointing_to(Some(&point));
        let _ = gesture.set_state(gtk::EventSequenceState::Claimed);
        popover_for_click.popup();
    });
    target.add_controller(click);
}

fn empty_project_state(language: UiLanguage) -> (gtk::Widget, gtk::Button, gtk::Button) {
    let state = gtk::Box::new(gtk::Orientation::Vertical, 4);
    state.add_css_class("potoo-sidebar-empty-state");
    state.set_margin_start(14);
    state.set_margin_end(14);
    state.set_margin_top(16);

    let title = gtk::Label::new(Some(tr(language, "Nenhum projeto aberto")));
    title.set_xalign(0.0);
    title.add_css_class("potoo-sidebar-empty-title");
    state.append(&title);

    let description = gtk::Label::new(Some(tr(
        language,
        "Abra uma pasta ou um programa para começar.",
    )));
    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("potoo-muted");
    state.append(&description);

    let open_project_button = icon_label_button(
        "folder-open",
        tr(language, "Abrir pasta"),
        tr(language, "Abrir pasta de projeto"),
        "potoo-sidebar-empty-action",
    );
    open_project_button.set_margin_top(6);
    let open_file_button = icon_label_button(
        "open-document",
        tr(language, "Abrir arquivo"),
        tr(language, "Abrir arquivo de código"),
        "potoo-sidebar-empty-action",
    );
    state.append(&open_project_button);
    state.append(&open_file_button);

    (state.upcast(), open_project_button, open_file_button)
}

use super::*;

pub(in crate::app::window) fn show_open_file_dialog(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title(tr(language, "Abrir arquivo de código"));
    let filter = gtk::FileFilter::new();
    filter.set_name(Some(tr(
        language,
        "Arquivos Mapler e outros dialetos (*.mapler, *.clg, *.gpt, *.alg, *.por, *.poti, *.txt para Mapler)",
    )));
    filter.add_suffix("por");
    filter.add_suffix("alg");
    filter.add_suffix("poti");
    filter.add_suffix("gpt");
    filter.add_suffix("clg");
    filter.add_suffix("mapler");
    filter.add_suffix("txt");
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    dialog.set_filters(Some(&filters));
    dialog.open(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(file) => match file.path() {
                Some(path) => sender.input(MainWindowMsg::OpenFile(path)),
                None => sender.input(MainWindowMsg::FileOperationFailed(
                    tr(language, "Selecione um arquivo local.").to_string(),
                )),
            },
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {}
            Err(error) => sender.input(MainWindowMsg::FileOperationFailed(format!(
                "{}: {error}",
                tr(language, "Não foi possível abrir o arquivo de código")
            ))),
        },
    );
}

pub(in crate::app::window) fn show_portugol_file_selection_dialog(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    execution_id: u64,
    project_root: Option<PathBuf>,
    formats: Vec<String>,
    allow_all_files: bool,
    cancellable: gtk::gio::Cancellable,
    sender: ComponentSender<MainWindow>,
) {
    let Some(project_root) = project_root else {
        sender.input(MainWindowMsg::FileSelectionResolved(
            execution_id,
            Err(tr(language, "Abra um projeto antes de selecionar um arquivo.").to_owned()),
        ));
        return;
    };

    let dialog = gtk::FileDialog::new();
    dialog.set_title(tr(language, "Selecionar arquivo"));
    let root_file = gtk::gio::File::for_path(&project_root);
    dialog.set_initial_folder(Some(&root_file));
    let filters = gtk::gio::ListStore::new::<gtk::FileFilter>();
    for format in formats {
        let (label, suffixes) = format
            .split_once('|')
            .map_or((format.as_str(), format.as_str()), |(label, suffixes)| {
                (label, suffixes)
            });
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(label));
        if let Some(normalized_suffixes) =
            crate::languages::portugol::studio_20::files::normalize_file_selection_suffixes(
                suffixes,
            )
        {
            for normalized_suffix in normalized_suffixes {
                filter.add_suffix(&normalized_suffix);
            }
        } else {
            filter.add_pattern("*");
        }
        filters.append(&filter);
    }
    if allow_all_files || filters.n_items() == 0 {
        let all_files = gtk::FileFilter::new();
        all_files.set_name(Some(tr(language, "Todos os arquivos")));
        all_files.add_pattern("*");
        filters.append(&all_files);
    }
    dialog.set_filters(Some(&filters));
    dialog.open(Some(parent), Some(&cancellable), move |result| {
        let selection = match result {
            Ok(file) => file
                .path()
                .map(|path| {
                    let canonical_root = fs::canonicalize(&project_root).unwrap_or(project_root);
                    let canonical_path = fs::canonicalize(&path).unwrap_or(path.clone());
                    canonical_path
                        .strip_prefix(&canonical_root)
                        .map(|relative| relative.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_else(|_| canonical_path.to_string_lossy().into_owned())
                })
                .ok_or_else(|| tr(language, "Selecione um arquivo local.").to_owned()),
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => Ok(String::new()),
            Err(error) => Err(format!(
                "{}: {error}",
                tr(language, "Não foi possível selecionar o arquivo")
            )),
        };
        sender.input(MainWindowMsg::FileSelectionResolved(
            execution_id,
            selection,
        ));
    });
}

pub(in crate::app::window) fn show_new_file_dialog(
    parent: &gtk::ApplicationWindow,
    default_language: Language,
    ui_language: UiLanguage,
    folder_options: Vec<(String, PathBuf)>,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, "Novo arquivo")));
    dialog.set_default_size(680, 510);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, ui_language, "Novo arquivo");

    let dialog_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dialog_content.add_css_class("potoo-dialog-surface");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-dialog-body");
    content.set_vexpand(true);
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    let description = gtk::Label::new(Some(tr(
        ui_language,
        "Crie um arquivo dentro do projeto atual.",
    )));
    description.set_xalign(0.0);
    description.add_css_class("potoo-dialog-intro");
    description.set_margin_bottom(18);
    content.append(&description);

    let name_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    name_field.add_css_class("potoo-dialog-field");
    name_field.set_margin_bottom(16);
    let name_label = gtk::Label::new(Some(tr(ui_language, "Nome do arquivo")));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-field-label");
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some(tr(
        ui_language,
        "Nome do arquivo, por exemplo: exercicio-1",
    )));
    name.set_text(tr(ui_language, "novo-arquivo"));
    name.add_css_class("potoo-project-name-entry");
    name_field.append(&name_label);
    name_field.append(&name);
    content.append(&name_field);

    let language_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    language_field.add_css_class("potoo-dialog-field");
    language_field.set_margin_bottom(16);
    let language_label = gtk::Label::new(Some(tr(ui_language, "Linguagem/pseudolinguagem")));
    language_label.set_xalign(0.0);
    language_label.add_css_class("potoo-project-field-label");
    let language_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    language_row.set_margin_top(1);
    let portugol_button = language_option_button("Portugol", tr(ui_language, "4 variantes"));
    let calango_button = language_option_button("Calango", ".clg");
    let potigol_button = language_option_button("Potigol", tr(ui_language, "2 variantes"));
    for button in [&portugol_button, &calango_button, &potigol_button] {
        button.add_css_class("potoo-project-language-choice");
        button.set_hexpand(true);
    }
    let default_template = ProjectTemplate::default_for_language(default_language);
    match default_template.language() {
        Language::Portugol | Language::Visualg | Language::GPortugol | Language::Mapler => {
            portugol_button.add_css_class("selected")
        }
        Language::Potigol => potigol_button.add_css_class("selected"),
        Language::Calango => calango_button.add_css_class("selected"),
    }
    language_row.append(&portugol_button);
    language_row.append(&calango_button);
    language_row.append(&potigol_button);
    language_field.append(&language_label);
    language_field.append(&language_row);
    content.append(&language_field);

    let selected_template = Rc::new(Cell::new(default_template));
    let variant_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    variant_field.add_css_class("potoo-dialog-field");
    variant_field.set_margin_bottom(16);
    let variant_label = gtk::Label::new(None);
    variant_label.set_xalign(0.0);
    variant_label.add_css_class("potoo-project-field-label");
    let portugol_variant_picker = gtk::DropDown::from_strings(&[
        "G-Portugol (.gpt)",
        "VisuAlg (.alg)",
        "Portugol Studio 2.0 (.por)",
        "Mapler (.mapler)",
    ]);
    portugol_variant_picker.add_css_class("potoo-settings-picker");
    portugol_variant_picker.set_hexpand(true);
    portugol_variant_picker.set_selected(project_template_variant_index(default_template));
    let potigol_variant_picker =
        gtk::DropDown::from_strings(&["Potigol Clássico", "Potigol Jerimum (Jogo 2D)"]);
    potigol_variant_picker.add_css_class("potoo-settings-picker");
    potigol_variant_picker.set_hexpand(true);
    potigol_variant_picker.set_selected(if default_template == ProjectTemplate::PotigolJerimum {
        1
    } else {
        0
    });
    variant_field.append(&variant_label);
    variant_field.append(&portugol_variant_picker);
    variant_field.append(&potigol_variant_picker);
    content.append(&variant_field);
    match default_template.language() {
        Language::Portugol | Language::Visualg | Language::GPortugol | Language::Mapler => {
            variant_label.set_text(tr(ui_language, "Variante do Portugol"));
            potigol_variant_picker.set_visible(false);
        }
        Language::Potigol => {
            variant_label.set_text(tr(ui_language, "Variante do Potigol"));
            portugol_variant_picker.set_visible(false);
        }
        Language::Calango => {
            variant_field.set_visible(false);
            portugol_variant_picker.set_visible(false);
            potigol_variant_picker.set_visible(false);
        }
    }
    portugol_variant_picker.connect_selected_notify({
        let selected_template = selected_template.clone();
        move |picker| {
            if let Some(template) =
                project_template_for_variant(Language::Portugol, picker.selected())
            {
                selected_template.set(template);
            }
        }
    });
    potigol_variant_picker.connect_selected_notify({
        let selected_template = selected_template.clone();
        move |picker| {
            if let Some(template) =
                project_template_for_variant(Language::Potigol, picker.selected())
            {
                selected_template.set(template);
            }
        }
    });

    let potigol_for_portugol = potigol_button.clone();
    let calango_for_portugol = calango_button.clone();
    let selected_for_portugol = selected_template.clone();
    let variant_for_portugol = variant_field.clone();
    let variant_label_for_portugol = variant_label.clone();
    let portugol_picker_for_portugol = portugol_variant_picker.clone();
    let potigol_picker_for_portugol = potigol_variant_picker.clone();
    portugol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        potigol_for_portugol.remove_css_class("selected");
        calango_for_portugol.remove_css_class("selected");
        let template = ProjectTemplate::Portugol20;
        selected_for_portugol.set(template);
        variant_for_portugol.set_visible(true);
        variant_label_for_portugol.set_text(tr(ui_language, "Variante do Portugol"));
        portugol_picker_for_portugol.set_visible(true);
        potigol_picker_for_portugol.set_visible(false);
        portugol_picker_for_portugol.set_selected(1);
    });
    let portugol_for_potigol = portugol_button.clone();
    let calango_for_potigol = calango_button.clone();
    let selected_for_potigol = selected_template.clone();
    let variant_for_potigol = variant_field.clone();
    let variant_label_for_potigol = variant_label.clone();
    let portugol_picker_for_potigol = portugol_variant_picker.clone();
    let potigol_picker_for_potigol = potigol_variant_picker.clone();
    potigol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        portugol_for_potigol.remove_css_class("selected");
        calango_for_potigol.remove_css_class("selected");
        let template = ProjectTemplate::PotigolClassic;
        selected_for_potigol.set(template);
        variant_for_potigol.set_visible(true);
        variant_label_for_potigol.set_text(tr(ui_language, "Variante do Potigol"));
        portugol_picker_for_potigol.set_visible(false);
        potigol_picker_for_potigol.set_visible(true);
        potigol_picker_for_potigol.set_selected(0);
    });
    let portugol_for_calango = portugol_button.clone();
    let potigol_for_calango = potigol_button.clone();
    let selected_for_calango = selected_template.clone();
    let variant_for_calango = variant_field.clone();
    let portugol_picker_for_calango = portugol_variant_picker.clone();
    let potigol_picker_for_calango = potigol_variant_picker.clone();
    calango_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        portugol_for_calango.remove_css_class("selected");
        potigol_for_calango.remove_css_class("selected");
        selected_for_calango.set(ProjectTemplate::Calango);
        variant_for_calango.set_visible(false);
        portugol_picker_for_calango.set_visible(false);
        potigol_picker_for_calango.set_visible(false);
    });

    let folder_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    folder_field.add_css_class("potoo-dialog-field");
    let folder_label = gtk::Label::new(Some(tr(ui_language, "Pasta")));
    folder_label.set_xalign(0.0);
    folder_label.add_css_class("potoo-project-field-label");
    let folder_names = folder_options
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    let folder_picker = gtk::DropDown::from_strings(&folder_names);
    folder_picker.add_css_class("potoo-settings-picker");
    folder_picker.set_hexpand(true);
    folder_picker.set_margin_top(1);
    folder_field.append(&folder_label);
    folder_field.append(&folder_picker);
    content.append(&folder_field);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("potoo-dialog-actions");
    actions.set_hexpand(true);
    let actions_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions_spacer.set_hexpand(true);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    cancel_button.add_css_class("potoo-project-cancel-button");
    let create_button = gtk::Button::with_label(tr(ui_language, "Criar arquivo"));
    create_button.add_css_class("potoo-run-button");
    actions.append(&actions_spacer);
    actions.append(&cancel_button);
    actions.append(&create_button);
    dialog_content.append(&content);
    dialog_content.append(&actions);
    dialog.set_child(Some(&dialog_content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());
    let dialog_for_create = dialog.clone();
    let name_for_create = name.clone();
    let selected_template_for_create = selected_template.clone();
    let folder_picker_for_create = folder_picker.clone();
    create_button.connect_clicked(move |_| {
        let relative_folder = folder_options
            .get(folder_picker_for_create.selected() as usize)
            .map(|(_, path)| path.clone())
            .unwrap_or_default();
        sender.input(MainWindowMsg::CreateFile(
            name_for_create.text().to_string(),
            selected_template_for_create.get(),
            relative_folder,
        ));
        dialog_for_create.close();
    });
    let create_button_for_entry = create_button.clone();
    name.connect_activate(move |_| create_button_for_entry.emit_clicked());
    dialog.present();
}

pub(in crate::app::window) fn show_rename_project_item_dialog(
    parent: &gtk::ApplicationWindow,
    path: PathBuf,
    is_directory: bool,
    ui_language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    let title = if is_directory {
        "Renomear pasta"
    } else {
        "Renomear arquivo"
    };
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, title)));
    dialog.set_default_size(520, 260);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, ui_language, title);

    let dialog_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dialog_content.add_css_class("potoo-dialog-surface");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-dialog-body");
    content.set_vexpand(true);
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(20);
    content.set_margin_bottom(20);

    let description = gtk::Label::new(Some(tr(
        ui_language,
        "Escolha um novo nome para este item do projeto.",
    )));
    description.set_xalign(0.0);
    description.add_css_class("potoo-dialog-intro");
    description.set_margin_bottom(18);
    content.append(&description);

    let name_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    name_field.add_css_class("potoo-dialog-field");
    let name_label = gtk::Label::new(Some(tr(ui_language, "Novo nome")));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-field-label");
    let name = gtk::Entry::new();
    if let Some(file_name) = path.file_name() {
        name.set_text(&file_name.to_string_lossy());
    }
    name.add_css_class("potoo-project-name-entry");
    name.select_region(0, -1);
    let error = gtk::Label::new(None);
    error.set_xalign(0.0);
    error.set_wrap(true);
    error.add_css_class("potoo-project-error");
    error.set_visible(false);
    name_field.append(&name_label);
    name_field.append(&name);
    name_field.append(&error);
    content.append(&name_field);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("potoo-dialog-actions");
    actions.set_hexpand(true);
    let actions_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions_spacer.set_hexpand(true);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    cancel_button.add_css_class("potoo-project-cancel-button");
    let rename_button = gtk::Button::with_label(tr(ui_language, "Renomear"));
    rename_button.add_css_class("potoo-run-button");
    actions.append(&actions_spacer);
    actions.append(&cancel_button);
    actions.append(&rename_button);
    dialog_content.append(&content);
    dialog_content.append(&actions);
    dialog.set_child(Some(&dialog_content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());

    let dialog_for_rename = dialog.clone();
    let name_for_rename = name.clone();
    let error_for_rename = error.clone();
    let path_for_rename = path.clone();
    rename_button.connect_clicked(move |_| {
        let new_name = match normalized_project_item_name(
            &path_for_rename,
            &name_for_rename.text(),
            is_directory,
            ui_language,
        ) {
            Ok(name) => name,
            Err(message) => {
                error_for_rename.set_text(&message);
                error_for_rename.set_visible(true);
                return;
            }
        };
        let Some(parent) = path_for_rename.parent() else {
            error_for_rename.set_text(tr(
                ui_language,
                "O item selecionado não está mais disponível.",
            ));
            error_for_rename.set_visible(true);
            return;
        };
        let new_path = parent.join(&new_name);
        if new_path.exists()
            && fs::canonicalize(&new_path).ok() != fs::canonicalize(&path_for_rename).ok()
        {
            error_for_rename.set_text(tr(ui_language, "Já existe um item com esse nome."));
            error_for_rename.set_visible(true);
            return;
        }
        sender.input(MainWindowMsg::RenameProjectItemConfirmed(
            path_for_rename.clone(),
            new_name,
        ));
        dialog_for_rename.close();
    });
    let rename_button_for_entry = rename_button.clone();
    name.connect_activate(move |_| rename_button_for_entry.emit_clicked());
    dialog.present();
}

pub(in crate::app::window) fn show_new_folder_dialog(
    parent: &gtk::ApplicationWindow,
    ui_language: UiLanguage,
    folder_options: Vec<(String, PathBuf)>,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, "Nova pasta")));
    dialog.set_default_size(520, 340);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, ui_language, "Nova pasta");

    let dialog_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    dialog_content.add_css_class("potoo-dialog-surface");
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-dialog-body");
    content.set_vexpand(true);
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(20);
    content.set_margin_bottom(20);
    let description = gtk::Label::new(Some(tr(
        ui_language,
        "Organize seus programas em uma pasta do projeto.",
    )));
    description.set_xalign(0.0);
    description.add_css_class("potoo-dialog-intro");
    description.set_margin_bottom(18);
    content.append(&description);

    let name_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    name_field.add_css_class("potoo-dialog-field");
    name_field.set_margin_bottom(16);
    let name_label = gtk::Label::new(Some(tr(ui_language, "Nome da pasta")));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-field-label");
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some(tr(
        ui_language,
        "Nome da pasta, por exemplo: exercicios",
    )));
    name.add_css_class("potoo-project-name-entry");
    name_field.append(&name_label);
    name_field.append(&name);
    content.append(&name_field);

    let parent_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    parent_field.add_css_class("potoo-dialog-field");
    let parent_label = gtk::Label::new(Some(tr(ui_language, "Criar dentro de")));
    parent_label.set_xalign(0.0);
    parent_label.add_css_class("potoo-project-field-label");
    let folder_names = folder_options
        .iter()
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    let folder_picker = gtk::DropDown::from_strings(&folder_names);
    folder_picker.add_css_class("potoo-settings-picker");
    folder_picker.set_hexpand(true);
    folder_picker.set_margin_top(1);
    parent_field.append(&parent_label);
    parent_field.append(&folder_picker);
    content.append(&parent_field);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("potoo-dialog-actions");
    actions.set_hexpand(true);
    let actions_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions_spacer.set_hexpand(true);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    cancel_button.add_css_class("potoo-project-cancel-button");
    let create_button = gtk::Button::with_label(tr(ui_language, "Criar pasta"));
    create_button.add_css_class("potoo-run-button");
    actions.append(&actions_spacer);
    actions.append(&cancel_button);
    actions.append(&create_button);
    dialog_content.append(&content);
    dialog_content.append(&actions);
    dialog.set_child(Some(&dialog_content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());
    let dialog_for_create = dialog.clone();
    let name_for_create = name.clone();
    let folder_picker_for_create = folder_picker.clone();
    create_button.connect_clicked(move |_| {
        let relative_folder = folder_options
            .get(folder_picker_for_create.selected() as usize)
            .map(|(_, path)| path.clone())
            .unwrap_or_default();
        sender.input(MainWindowMsg::CreateFolder(
            name_for_create.text().to_string(),
            relative_folder,
        ));
        dialog_for_create.close();
    });
    let create_button_for_entry = create_button.clone();
    name.connect_activate(move |_| create_button_for_entry.emit_clicked());
    dialog.present();
}

pub(in crate::app::window) fn show_save_dialog(
    parent: &gtk::ApplicationWindow,
    document: &SourceDocument,
    project_root: Option<&Path>,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title(tr(language, "Salvar arquivo"));
    dialog.set_modal(true);
    let initial_name = match document.path.as_deref() {
        Some(path)
            if document.language == Language::Mapler
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("txt")) =>
        {
            path.with_extension("mapler")
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| document.name.clone())
        }
        _ => document.name.clone(),
    };
    dialog.set_initial_name(Some(&initial_name));
    if let Some(folder) = document
        .path
        .as_deref()
        .and_then(Path::parent)
        .or(project_root)
    {
        dialog.set_initial_folder(Some(&gtk::gio::File::for_path(folder)));
    }
    let id = document.id;
    dialog.save(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(file) => match file.path() {
                Some(path) => sender.input(MainWindowMsg::SaveAsSelected(id, path)),
                None => {
                    sender.input(MainWindowMsg::SaveAsCanceled(id));
                    sender.input(MainWindowMsg::FileOperationFailed(
                        tr(language, "Escolha um caminho local para salvar o arquivo.").to_string(),
                    ));
                }
            },
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {
                sender.input(MainWindowMsg::SaveAsCanceled(id));
            }
            Err(error) => {
                sender.input(MainWindowMsg::SaveAsCanceled(id));
                sender.input(MainWindowMsg::FileOperationFailed(format!(
                    "{}: {error}",
                    tr(language, "Não foi possível abrir a janela de salvamento")
                )));
            }
        },
    );
}

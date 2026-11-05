use super::project::*;
use super::*;

pub(super) fn show_open_project_dialog(
    parent: &impl IsA<gtk::Window>,
    language: UiLanguage,
    sender: ComponentSender<MainWindow>,
    close_parent_on_success: Option<gtk::Window>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title(tr(language, "Abrir pasta de projeto"));
    dialog.set_modal(true);
    dialog.select_folder(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(folder) => match folder.path() {
                Some(path) => {
                    if let Some(window) = close_parent_on_success {
                        window.close();
                    }
                    sender.input(MainWindowMsg::ProjectSelected(path));
                }
                None => sender.input(MainWindowMsg::FileOperationFailed(
                    tr(language, "Selecione uma pasta local para abrir.").to_string(),
                )),
            },
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {}
            Err(error) => sender.input(MainWindowMsg::FileOperationFailed(format!(
                "{}: {error}",
                tr(language, "Não foi possível abrir a pasta")
            ))),
        },
    );
}

pub(super) fn show_open_file_dialog(
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

pub(super) fn show_portugol_file_selection_dialog(
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

pub(super) fn update_project_template_selector(
    template: ProjectTemplate,
    ui_language: UiLanguage,
    field: &gtk::Box,
    label: &gtk::Label,
    portugol_options: &gtk::Box,
    portugol_buttons: [&gtk::Button; 4],
    potigol_options: &gtk::Box,
    potigol_buttons: [&gtk::Button; 2],
) {
    match template {
        ProjectTemplate::GPortugol
        | ProjectTemplate::Portugol20
        | ProjectTemplate::Visualg
        | ProjectTemplate::Mapler => {
            label.set_text(tr(ui_language, "Variante do Portugol"));
            portugol_options.set_visible(true);
            potigol_options.set_visible(false);
            let selected = match template {
                ProjectTemplate::GPortugol => 0,
                ProjectTemplate::Visualg => 1,
                ProjectTemplate::Portugol20 => 2,
                ProjectTemplate::Mapler => 3,
                _ => unreachable!(),
            };
            set_selected_variant_button(&portugol_buttons, selected);
            for button in potigol_buttons {
                button.remove_css_class("selected");
            }
        }
        ProjectTemplate::PotigolClassic | ProjectTemplate::PotigolJerimum => {
            label.set_text(tr(ui_language, "Variante do Potigol"));
            portugol_options.set_visible(false);
            potigol_options.set_visible(true);
            let selected = if template == ProjectTemplate::PotigolJerimum {
                1
            } else {
                0
            };
            set_selected_variant_button(&potigol_buttons, selected);
            for button in portugol_buttons {
                button.remove_css_class("selected");
            }
        }
        ProjectTemplate::Calango => {
            portugol_options.set_visible(false);
            potigol_options.set_visible(false);
            for button in portugol_buttons.into_iter().chain(potigol_buttons) {
                button.remove_css_class("selected");
            }
            field.set_visible(false);
        }
    }
    if template != ProjectTemplate::Calango {
        field.set_visible(true);
    }
}

pub(super) fn project_template_variant_index(template: ProjectTemplate) -> u32 {
    match template {
        ProjectTemplate::GPortugol | ProjectTemplate::PotigolClassic => 0,
        ProjectTemplate::Visualg | ProjectTemplate::PotigolJerimum => 1,
        ProjectTemplate::Portugol20 => 2,
        ProjectTemplate::Mapler => 3,
        ProjectTemplate::Calango => 0,
    }
}

pub(super) fn project_template_for_variant(
    language: Language,
    selected: u32,
) -> Option<ProjectTemplate> {
    match (language, selected) {
        (Language::Potigol, 0) => Some(ProjectTemplate::PotigolClassic),
        (Language::Potigol, 1) => Some(ProjectTemplate::PotigolJerimum),
        (Language::Potigol, _) => None,
        (Language::Calango, _) => None,
        (_, 0) => Some(ProjectTemplate::GPortugol),
        (_, 1) => Some(ProjectTemplate::Visualg),
        (_, 2) => Some(ProjectTemplate::Portugol20),
        (_, 3) => Some(ProjectTemplate::Mapler),
        (_, _) => None,
    }
}

pub(super) fn set_selected_variant_button(buttons: &[&gtk::Button], selected: usize) {
    for (index, button) in buttons.iter().enumerate() {
        if index == selected {
            button.add_css_class("selected");
        } else {
            button.remove_css_class("selected");
        }
    }
}

pub(super) fn connect_project_template_choices(
    buttons: &[gtk::Button],
    templates: &[ProjectTemplate],
    selected_template: Rc<Cell<ProjectTemplate>>,
) {
    for (button, template) in buttons.iter().zip(templates) {
        let selected_button = (*button).clone();
        let button_for_connection = selected_button.clone();
        let buttons = buttons.to_vec();
        let selected_template = selected_template.clone();
        let template = *template;
        button_for_connection.connect_clicked(move |_| {
            selected_template.set(template);
            for choice in &buttons {
                choice.remove_css_class("selected");
            }
            selected_button.add_css_class("selected");
        });
    }
}

pub(super) fn show_new_project_dialog(
    parent: &gtk::ApplicationWindow,
    ui_language: UiLanguage,
    default_template: ProjectTemplate,
    show_language_selector: bool,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(ui_language, "Criar projeto")));
    let default_language = default_template.language();
    dialog.set_default_size(680, if show_language_selector { 520 } else { 430 });
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, ui_language, "Criar projeto");

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
        "Crie uma pasta para reunir seus programas e exercícios.",
    )));
    description.set_xalign(0.0);
    description.set_wrap(true);
    description.add_css_class("potoo-dialog-intro");
    description.set_margin_bottom(18);
    content.append(&description);

    let name_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    name_field.add_css_class("potoo-dialog-field");
    name_field.set_margin_bottom(16);
    let name_label = gtk::Label::new(Some(tr(ui_language, "Nome do projeto")));
    name_label.set_xalign(0.0);
    name_label.add_css_class("potoo-project-field-label");
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some(tr(ui_language, "Por exemplo: Meu primeiro projeto")));
    name.set_text(tr(ui_language, "Meu projeto"));
    name.add_css_class("potoo-project-name-entry");
    name_field.append(&name_label);
    name_field.append(&name);
    content.append(&name_field);

    let language_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    language_field.add_css_class("potoo-dialog-field");
    language_field.set_margin_bottom(16);
    language_field.set_visible(show_language_selector);
    let language_label = gtk::Label::new(Some(tr(
        ui_language,
        "Selecione a linguagem/pseudolinguagem do projeto",
    )));
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
    match default_language {
        Language::Portugol | Language::GPortugol | Language::Visualg | Language::Mapler => {
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
    let portugol_options = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    portugol_options.set_homogeneous(true);
    portugol_options.set_hexpand(true);
    let portugol_variant_buttons = [
        language_option_button("G-Portugol", ".gpt"),
        language_option_button("VisuAlg", ".alg"),
        language_option_button("Portugol Studio 2.0", ".por"),
        language_option_button("Mapler", ".mapler"),
    ];
    let potigol_options = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    potigol_options.set_homogeneous(true);
    potigol_options.set_hexpand(true);
    let potigol_variant_buttons = [
        language_option_button("Potigol Clássico", ".poti"),
        language_option_button("Potigol Jerimum (Jogo 2D)", ".poti"),
    ];
    for button in portugol_variant_buttons
        .iter()
        .chain(potigol_variant_buttons.iter())
    {
        button.add_css_class("potoo-project-language-choice");
        button.set_hexpand(true);
    }
    for button in &portugol_variant_buttons {
        portugol_options.append(button);
    }
    for button in &potigol_variant_buttons {
        potigol_options.append(button);
    }
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
    variant_field.append(&portugol_options);
    variant_field.append(&potigol_options);
    variant_field.append(&portugol_variant_picker);
    variant_field.append(&potigol_variant_picker);
    content.append(&variant_field);
    update_project_template_selector(
        default_template,
        ui_language,
        &variant_field,
        &variant_label,
        &portugol_options,
        [
            &portugol_variant_buttons[0],
            &portugol_variant_buttons[1],
            &portugol_variant_buttons[2],
            &portugol_variant_buttons[3],
        ],
        &potigol_options,
        [&potigol_variant_buttons[0], &potigol_variant_buttons[1]],
    );
    portugol_variant_picker.set_visible(false);
    potigol_variant_picker.set_visible(false);
    if show_language_selector {
        portugol_options.set_visible(false);
        potigol_options.set_visible(false);
        match default_language {
            Language::Portugol | Language::GPortugol | Language::Visualg | Language::Mapler => {
                portugol_variant_picker.set_visible(true)
            }
            Language::Potigol => potigol_variant_picker.set_visible(true),
            Language::Calango => variant_field.set_visible(false),
        }
    }
    connect_project_template_choices(
        &portugol_variant_buttons,
        &[
            ProjectTemplate::GPortugol,
            ProjectTemplate::Visualg,
            ProjectTemplate::Portugol20,
            ProjectTemplate::Mapler,
        ],
        selected_template.clone(),
    );
    connect_project_template_choices(
        &potigol_variant_buttons,
        &[
            ProjectTemplate::PotigolClassic,
            ProjectTemplate::PotigolJerimum,
        ],
        selected_template.clone(),
    );
    if show_language_selector {
        let selected_template_for_portugol = selected_template.clone();
        portugol_variant_picker.connect_selected_notify(move |picker| {
            if let Some(template) =
                project_template_for_variant(Language::Portugol, picker.selected())
            {
                selected_template_for_portugol.set(template);
            }
        });
        let selected_template_for_potigol = selected_template.clone();
        potigol_variant_picker.connect_selected_notify(move |picker| {
            if let Some(template) =
                project_template_for_variant(Language::Potigol, picker.selected())
            {
                selected_template_for_potigol.set(template);
            }
        });
    }

    let potigol_button_for_portugol = potigol_button.clone();
    let calango_button_for_portugol = calango_button.clone();
    let selected_template_for_portugol = selected_template.clone();
    let variant_field_for_portugol = variant_field.clone();
    let variant_label_for_portugol = variant_label.clone();
    let portugol_options_for_portugol = portugol_options.clone();
    let portugol_buttons_for_portugol = portugol_variant_buttons.clone();
    let potigol_options_for_portugol = potigol_options.clone();
    let potigol_buttons_for_portugol = potigol_variant_buttons.clone();
    let show_variant_picker_for_portugol = show_language_selector;
    let portugol_picker_for_portugol = portugol_variant_picker.clone();
    let potigol_picker_for_portugol = potigol_variant_picker.clone();
    portugol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        potigol_button_for_portugol.remove_css_class("selected");
        calango_button_for_portugol.remove_css_class("selected");
        let template = ProjectTemplate::Portugol20;
        selected_template_for_portugol.set(template);
        update_project_template_selector(
            template,
            ui_language,
            &variant_field_for_portugol,
            &variant_label_for_portugol,
            &portugol_options_for_portugol,
            [
                &portugol_buttons_for_portugol[0],
                &portugol_buttons_for_portugol[1],
                &portugol_buttons_for_portugol[2],
                &portugol_buttons_for_portugol[3],
            ],
            &potigol_options_for_portugol,
            [
                &potigol_buttons_for_portugol[0],
                &potigol_buttons_for_portugol[1],
            ],
        );
        if show_variant_picker_for_portugol {
            portugol_options_for_portugol.set_visible(false);
            potigol_options_for_portugol.set_visible(false);
            portugol_picker_for_portugol.set_visible(true);
            potigol_picker_for_portugol.set_visible(false);
            portugol_picker_for_portugol.set_selected(1);
        }
    });
    let portugol_button_for_potigol = portugol_button.clone();
    let calango_button_for_potigol = calango_button.clone();
    let selected_template_for_potigol = selected_template.clone();
    let variant_field_for_potigol = variant_field.clone();
    let variant_label_for_potigol = variant_label.clone();
    let portugol_options_for_potigol = portugol_options.clone();
    let portugol_buttons_for_potigol = portugol_variant_buttons.clone();
    let potigol_options_for_potigol = potigol_options.clone();
    let potigol_buttons_for_potigol = potigol_variant_buttons.clone();
    let show_variant_picker_for_potigol = show_language_selector;
    let portugol_picker_for_potigol = portugol_variant_picker.clone();
    let potigol_picker_for_potigol = potigol_variant_picker.clone();
    potigol_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        portugol_button_for_potigol.remove_css_class("selected");
        calango_button_for_potigol.remove_css_class("selected");
        let template = ProjectTemplate::PotigolClassic;
        selected_template_for_potigol.set(template);
        update_project_template_selector(
            template,
            ui_language,
            &variant_field_for_potigol,
            &variant_label_for_potigol,
            &portugol_options_for_potigol,
            [
                &portugol_buttons_for_potigol[0],
                &portugol_buttons_for_potigol[1],
                &portugol_buttons_for_potigol[2],
                &portugol_buttons_for_potigol[3],
            ],
            &potigol_options_for_potigol,
            [
                &potigol_buttons_for_potigol[0],
                &potigol_buttons_for_potigol[1],
            ],
        );
        if show_variant_picker_for_potigol {
            portugol_options_for_potigol.set_visible(false);
            potigol_options_for_potigol.set_visible(false);
            portugol_picker_for_potigol.set_visible(false);
            potigol_picker_for_potigol.set_visible(true);
            potigol_picker_for_potigol.set_selected(0);
        }
    });
    let portugol_button_for_calango = portugol_button.clone();
    let potigol_button_for_calango = potigol_button.clone();
    let selected_template_for_calango = selected_template.clone();
    let variant_field_for_calango = variant_field.clone();
    let portugol_options_for_calango = portugol_options.clone();
    let potigol_options_for_calango = potigol_options.clone();
    let portugol_picker_for_calango = portugol_variant_picker.clone();
    let potigol_picker_for_calango = potigol_variant_picker.clone();
    calango_button.connect_clicked(move |button| {
        button.add_css_class("selected");
        portugol_button_for_calango.remove_css_class("selected");
        potigol_button_for_calango.remove_css_class("selected");
        selected_template_for_calango.set(ProjectTemplate::Calango);
        variant_field_for_calango.set_visible(false);
        portugol_options_for_calango.set_visible(false);
        potigol_options_for_calango.set_visible(false);
        portugol_picker_for_calango.set_visible(false);
        potigol_picker_for_calango.set_visible(false);
    });

    let location_field = gtk::Box::new(gtk::Orientation::Vertical, 6);
    location_field.add_css_class("potoo-dialog-field");
    let location_label = gtk::Label::new(Some(tr(ui_language, "Local do projeto")));
    location_label.set_xalign(0.0);
    location_label.add_css_class("potoo-project-field-label");
    let initial_folder = project_default_directory();
    let selected_folder = Rc::new(RefCell::new(initial_folder.clone()));
    let location_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    location_row.set_margin_top(1);
    let location = gtk::Label::new(Some(&initial_folder.display().to_string()));
    location.set_xalign(0.0);
    location.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    location.set_hexpand(true);
    location.add_css_class("potoo-project-location");
    location.set_tooltip_text(Some(&initial_folder.display().to_string()));
    let choose_folder = gtk::Button::with_label(tr(ui_language, "Escolher pasta"));
    choose_folder.add_css_class("potoo-project-location-button");
    location_row.append(&location);
    location_row.append(&choose_folder);
    location_field.append(&location_label);
    location_field.append(&location_row);
    content.append(&location_field);

    let error_label = gtk::Label::new(Some(tr(
        ui_language,
        "Digite um nome válido para o projeto.",
    )));
    error_label.set_xalign(0.0);
    error_label.add_css_class("potoo-project-error");
    error_label.set_visible(false);
    content.append(&error_label);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.add_css_class("potoo-project-actions");
    actions.add_css_class("potoo-dialog-actions");
    actions.set_hexpand(true);
    let open_project_button = gtk::Button::new();
    open_project_button.add_css_class("potoo-project-open-button");
    open_project_button.set_visible(show_language_selector);
    open_project_button.set_tooltip_text(Some(tr(ui_language, "Abrir projeto")));
    let open_project_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    open_project_content.append(&icon("folder-open"));
    let open_project_label = gtk::Label::new(Some(tr(ui_language, "Abrir projeto")));
    open_project_content.append(&open_project_label);
    open_project_button.set_child(Some(&open_project_content));
    let actions_spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    actions_spacer.set_hexpand(true);
    let cancel_button = gtk::Button::with_label(tr(ui_language, "Cancelar"));
    cancel_button.add_css_class("potoo-project-cancel-button");
    let create_button = gtk::Button::with_label(tr(ui_language, "Criar projeto"));
    create_button.add_css_class("potoo-run-button");
    actions.append(&open_project_button);
    actions.append(&actions_spacer);
    actions.append(&cancel_button);
    actions.append(&create_button);
    dialog_content.append(&content);
    dialog_content.append(&actions);
    dialog.set_child(Some(&dialog_content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());

    let dialog_for_open_project = dialog.clone();
    let dialog_to_close_after_open = dialog.clone();
    let sender_for_open_project = sender.clone();
    open_project_button.connect_clicked(move |_| {
        show_open_project_dialog(
            &dialog_for_open_project,
            ui_language,
            sender_for_open_project.clone(),
            Some(dialog_to_close_after_open.clone()),
        );
    });

    let dialog_for_picker = dialog.clone();
    let selected_folder_for_picker = selected_folder.clone();
    let location_for_picker = location.clone();
    let sender_for_picker = sender.clone();
    choose_folder.connect_clicked(move |_| {
        let picker = gtk::FileDialog::new();
        picker.set_title(tr(ui_language, "Escolha a pasta de destino"));
        picker.set_modal(true);
        let current_folder = selected_folder_for_picker.borrow().clone();
        picker.set_initial_folder(Some(&gtk::gio::File::for_path(current_folder)));
        let selected_folder_for_result = selected_folder_for_picker.clone();
        let location_for_result = location_for_picker.clone();
        let sender_for_result = sender_for_picker.clone();
        picker.select_folder(
            Some(&dialog_for_picker),
            None::<&gtk::gio::Cancellable>,
            move |result| match result {
                Ok(folder) => match folder.path() {
                    Some(path) => {
                        selected_folder_for_result.replace(path.clone());
                        let path_text = path.display().to_string();
                        location_for_result.set_text(&path_text);
                        location_for_result.set_tooltip_text(Some(&path_text));
                    }
                    None => sender_for_result.input(MainWindowMsg::FileOperationFailed(
                        tr(
                            ui_language,
                            "Selecione uma pasta local para criar o projeto.",
                        )
                        .to_string(),
                    )),
                },
                Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {}
                Err(error) => sender_for_result.input(MainWindowMsg::FileOperationFailed(format!(
                    "{}: {error}",
                    tr(ui_language, "Não foi possível escolher a pasta do projeto")
                ))),
            },
        );
    });

    let dialog_for_create = dialog.clone();
    let name_for_create = name.clone();
    let error_for_create = error_label.clone();
    let selected_template_for_create = selected_template.clone();
    let selected_folder_for_create = selected_folder.clone();
    create_button.connect_clicked(move |_| {
        let project_name = name_for_create.text().to_string();
        if !is_valid_directory_name(project_name.trim()) {
            error_for_create.set_visible(true);
            return;
        }
        sender.input(MainWindowMsg::CreateProject(
            selected_folder_for_create.borrow().clone(),
            project_name,
            selected_template_for_create.get(),
        ));
        dialog_for_create.close();
    });
    let create_button_for_entry = create_button.clone();
    let error_for_name = error_label.clone();
    name.connect_changed(move |entry| {
        let name_is_valid = !entry.text().trim().is_empty();
        create_button_for_entry.set_sensitive(name_is_valid);
        if name_is_valid {
            error_for_name.set_visible(false);
        }
    });
    let create_button_for_entry = create_button.clone();
    name.connect_activate(move |_| create_button_for_entry.emit_clicked());
    dialog.present();
}

pub(super) fn project_default_directory() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(super) fn project_directory_options(
    root: &Path,
    language: UiLanguage,
    files: &[ProjectFile],
) -> Vec<(String, PathBuf)> {
    let root_label = root
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| tr(language, "Pasta do projeto").to_string());
    let mut folders = vec![(root_label, PathBuf::new())];
    folders.extend(files.iter().filter(|file| file.is_directory).map(|file| {
        (
            file.relative_path.clone(),
            PathBuf::from(&file.relative_path),
        )
    }));
    folders
}

pub(super) fn set_dialog_titlebar(window: &gtk::Window, language: UiLanguage, title: &'static str) {
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

pub(super) fn close_window_on_escape(window: &gtk::Window) {
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

pub(super) fn language_option_button(name: &str, detail: &str) -> gtk::Button {
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

pub(super) fn show_new_file_dialog(
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

pub(super) fn show_rename_project_item_dialog(
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

pub(super) fn show_new_folder_dialog(
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

pub(super) fn show_save_dialog(
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

pub(super) fn show_project_switch_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    show_potoo_confirmation(
        parent,
        language,
        initial_dark,
        "Alterações não salvas",
        "Alterações não salvas",
        "Salve os arquivos abertos antes de trocar de pasta ou descarte as alterações para continuar.",
        vec!["Cancelar", "Descartar e abrir"],
        Some(1),
        None,
        move |response| sender.input(MainWindowMsg::ProjectSwitchDecision(response == 1)),
    );
}

pub(super) fn show_close_document_confirmation(
    parent: &gtk::ApplicationWindow,
    id: u64,
    language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    show_potoo_confirmation(
        parent,
        language,
        initial_dark,
        "Alterações não salvas",
        "O arquivo aberto tem alterações que ainda não foram salvas. O que você gostaria de fazer?",
        "Salve o arquivo antes de fechar ou descarte as alterações para continuar.",
        vec![
            "Continuar editando",
            "Descartar alterações",
            "Salvar e fechar",
        ],
        Some(2),
        None,
        move |response| match response {
            1 => sender.input(MainWindowMsg::CloseDocumentDecision(id, true)),
            2 => sender.input(MainWindowMsg::SaveAndCloseDocument(id)),
            _ => sender.input(MainWindowMsg::CloseDocumentDecision(id, false)),
        },
    );
}

pub(super) fn show_close_app_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    initial_dark: bool,
    sender: ComponentSender<MainWindow>,
) {
    show_potoo_confirmation(
        parent,
        language,
        initial_dark,
        "Alterações não salvas",
        "Alterações não salvas",
        "Salve os arquivos antes de sair ou descarte as alterações para fechar o Potoo IDE.",
        vec!["Continuar editando", "Descartar e sair"],
        Some(1),
        None,
        move |response| sender.input(MainWindowMsg::CloseAppDecision(response == 1)),
    );
}

pub(super) fn show_potoo_confirmation(
    parent: &gtk::ApplicationWindow,
    language: UiLanguage,
    initial_dark: bool,
    title: &'static str,
    message: &'static str,
    detail: &'static str,
    actions: Vec<&'static str>,
    primary_action: Option<usize>,
    destructive_action: Option<usize>,
    on_response: impl Fn(usize) + 'static,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some(tr(language, title)));
    dialog.set_default_size(520, -1);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if initial_dark {
        dialog.add_css_class("potoo-dark");
    }
    set_dialog_titlebar(&dialog, language, title);

    let surface = gtk::Box::new(gtk::Orientation::Vertical, 0);
    surface.add_css_class("potoo-dialog-surface");

    let content = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    content.add_css_class("potoo-confirm-content");
    content.set_margin_start(22);
    content.set_margin_end(22);
    content.set_margin_top(22);
    content.set_margin_bottom(22);

    let mark = gtk::Label::new(Some("!"));
    mark.add_css_class("potoo-confirm-mark");
    mark.set_valign(gtk::Align::Start);
    content.append(&mark);

    let copy = gtk::Box::new(gtk::Orientation::Vertical, 6);
    copy.set_valign(gtk::Align::Center);
    let message_label = gtk::Label::new(Some(tr(language, message)));
    message_label.set_xalign(0.0);
    message_label.set_wrap(true);
    message_label.add_css_class("potoo-confirm-message");
    let detail_label = gtk::Label::new(Some(tr(language, detail)));
    detail_label.set_xalign(0.0);
    detail_label.set_wrap(true);
    detail_label.add_css_class("potoo-confirm-detail");
    copy.append(&message_label);
    copy.append(&detail_label);
    content.append(&copy);
    surface.append(&content);

    let actions_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions_row.add_css_class("potoo-dialog-actions");
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    actions_row.append(&spacer);

    let response_sent = Rc::new(Cell::new(false));
    let on_response: Rc<dyn Fn(usize)> = Rc::new(on_response);
    for (index, label) in actions.into_iter().enumerate() {
        let button = gtk::Button::with_label(tr(language, label));
        button.add_css_class(if Some(index) == destructive_action {
            "potoo-destructive-button"
        } else if Some(index) == primary_action {
            "potoo-run-button"
        } else {
            "potoo-project-cancel-button"
        });
        let dialog_for_button = dialog.clone();
        let response_sent_for_button = response_sent.clone();
        let on_response_for_button = on_response.clone();
        button.connect_clicked(move |_| {
            if !response_sent_for_button.replace(true) {
                on_response_for_button(index);
            }
            dialog_for_button.close();
        });
        actions_row.append(&button);
    }
    surface.append(&actions_row);
    dialog.set_child(Some(&surface));

    let response_sent_for_close = response_sent;
    dialog.connect_close_request(move |_| {
        if !response_sent_for_close.replace(true) {
            on_response(0);
        }
        gtk::glib::Propagation::Proceed
    });
    dialog.present();
}

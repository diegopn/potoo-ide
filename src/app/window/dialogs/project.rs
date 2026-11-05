use super::*;

pub(in crate::app::window) fn show_open_project_dialog(
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

pub(in crate::app::window) fn update_project_template_selector(
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

pub(in crate::app::window) fn project_template_variant_index(template: ProjectTemplate) -> u32 {
    match template {
        ProjectTemplate::GPortugol | ProjectTemplate::PotigolClassic => 0,
        ProjectTemplate::Visualg | ProjectTemplate::PotigolJerimum => 1,
        ProjectTemplate::Portugol20 => 2,
        ProjectTemplate::Mapler => 3,
        ProjectTemplate::Calango => 0,
    }
}

pub(in crate::app::window) fn project_template_for_variant(
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

pub(in crate::app::window) fn set_selected_variant_button(
    buttons: &[&gtk::Button],
    selected: usize,
) {
    for (index, button) in buttons.iter().enumerate() {
        if index == selected {
            button.add_css_class("selected");
        } else {
            button.remove_css_class("selected");
        }
    }
}

pub(in crate::app::window) fn connect_project_template_choices(
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

pub(in crate::app::window) fn show_new_project_dialog(
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

pub(in crate::app::window) fn project_default_directory() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(in crate::app::window) fn project_directory_options(
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

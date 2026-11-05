use super::persistence::*;
use super::*;

pub(super) struct SettingsUi {
    pub(super) window: gtk::Window,
    pub(super) theme_buttons: [gtk::Button; 3],
    pub(super) font_size_picker: gtk::DropDown,
    pub(super) line_numbers_switch: gtk::Switch,
    pub(super) word_wrap_switch: gtk::Switch,
    pub(super) indent_width_picker: gtk::DropDown,
    pub(super) ui_language_picker: gtk::DropDown,
}

pub(super) fn build_settings_window(
    parent: &gtk::ApplicationWindow,
    sender: ComponentSender<MainWindow>,
    preferences: Preferences,
    initial_dark: bool,
    ui_language_state: Rc<Cell<UiLanguage>>,
) -> SettingsUi {
    let language = preferences.ui_language;
    let window = gtk::Window::new();
    window.set_title(Some(tr(language, "Configurações")));
    window.set_default_size(740, 685);
    window.set_resizable(true);
    window.set_modal(true);
    close_window_on_escape(&window);
    window.set_transient_for(Some(parent));
    window.add_css_class("potoo-window");
    window.add_css_class("potoo-settings-window");
    if initial_dark {
        window.add_css_class("potoo-dark");
    }

    let titlebar = gtk::HeaderBar::new();
    titlebar.add_css_class("potoo-settings-titlebar");
    titlebar.set_show_title_buttons(false);
    let title_label = gtk::Label::new(Some(tr(language, "Configurações")));
    title_label.add_css_class("potoo-settings-window-title");
    titlebar.set_title_widget(Some(&title_label));
    let back_button = gtk::Button::new();
    back_button.add_css_class("potoo-settings-back");
    let back_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    back_content.append(&icon("chevron-left"));
    back_content.append(&gtk::Label::new(Some(tr(language, "Configurações"))));
    back_button.set_child(Some(&back_content));
    back_button.set_visible(false);
    titlebar.pack_start(&back_button);
    let close_button = gtk::Button::new();
    close_button.add_css_class("potoo-dialog-close");
    close_button.set_child(Some(&icon("close")));
    close_button.set_tooltip_text(Some(tr(language, "Fechar")));
    titlebar.pack_end(&close_button);
    window.set_titlebar(Some(&titlebar));
    let window_for_titlebar_close = window.clone();
    close_button.connect_clicked(move |_| window_for_titlebar_close.close());

    let page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    page.add_css_class("potoo-settings-page");
    page.set_margin_start(22);
    page.set_margin_end(22);
    page.set_margin_top(18);
    page.set_margin_bottom(20);

    let description = gtk::Label::new(Some(tr(language, "Personalize a aparência e o editor.")));
    description.set_xalign(0.0);
    description.add_css_class("potoo-settings-description");
    description.set_margin_bottom(14);
    page.append(&description);

    let appearance = settings_section(tr(language, "Aparência"));
    let theme_buttons = [
        gtk::Button::with_label(tr(language, "Sistema")),
        gtk::Button::with_label(tr(language, "Claro")),
        gtk::Button::with_label(tr(language, "Escuro")),
    ];
    for button in &theme_buttons {
        button.add_css_class("potoo-theme-choice");
    }
    select_theme_button(&theme_buttons, preferences.theme.selected_index());
    let theme_options = gtk::Box::new(gtk::Orientation::Horizontal, 3);
    theme_options.add_css_class("potoo-theme-options");
    theme_options.set_homogeneous(true);
    for button in &theme_buttons {
        theme_options.append(button);
    }
    appearance.append(&settings_row(
        tr(language, "Tema"),
        tr(language, "Escolha como o Potoo aparece nesta tela."),
        &theme_options,
    ));
    page.append(&appearance);

    let editor = settings_section(tr(language, "Editor"));
    editor.add_css_class("potoo-settings-section-separated");
    let font_size_picker = gtk::DropDown::from_strings(&["14 px", "15 px", "16 px", "18 px"]);
    font_size_picker.add_css_class("potoo-settings-picker");
    font_size_picker.set_selected(font_size_index(preferences.font_size));
    editor.append(&settings_row(
        tr(language, "Tamanho da fonte"),
        tr(language, "Ajuste a leitura do código."),
        &font_size_picker,
    ));

    let line_numbers_switch = gtk::Switch::new();
    line_numbers_switch.add_css_class("potoo-settings-switch");
    line_numbers_switch.set_active(preferences.line_numbers_visible);
    editor.append(&settings_row(
        tr(language, "Numeração de linhas"),
        tr(language, "Mostra o número de cada linha do código."),
        &line_numbers_switch,
    ));

    let word_wrap_switch = gtk::Switch::new();
    word_wrap_switch.add_css_class("potoo-settings-switch");
    word_wrap_switch.set_active(preferences.word_wrap);
    editor.append(&settings_row(
        tr(language, "Quebra automática"),
        tr(language, "Ajusta linhas longas à largura do editor."),
        &word_wrap_switch,
    ));

    let indent_width_picker =
        gtk::DropDown::from_strings(&[tr(language, "2 espaços"), tr(language, "4 espaços")]);
    indent_width_picker.add_css_class("potoo-settings-picker");
    indent_width_picker.set_selected(if preferences.indent_width == 2 { 0 } else { 1 });
    editor.append(&settings_row(
        tr(language, "Recuo"),
        tr(language, "Espaços por nível de recuo."),
        &indent_width_picker,
    ));

    page.append(&editor);

    let interface = settings_section(tr(language, "Interface"));
    interface.add_css_class("potoo-settings-section-separated");
    let ui_language_picker = gtk::DropDown::from_strings(&["Português", "English", "Español"]);
    ui_language_picker.add_css_class("potoo-settings-picker");
    ui_language_picker.set_selected(preferences.ui_language.selected_index());
    interface.append(&settings_row(
        tr(language, "Idioma da interface"),
        tr(
            language,
            "A alteração de idioma será aplicada após salvar e reiniciar o aplicativo.",
        ),
        &ui_language_picker,
    ));
    page.append(&interface);

    let about_section = settings_section(tr(language, "Sobre"));
    about_section.add_css_class("potoo-settings-section-separated");
    let about_button = gtk::Button::new();
    about_button.add_css_class("potoo-settings-about-entry");
    about_button.set_hexpand(true);
    let about_button_content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    about_button_content.set_hexpand(true);
    let about_button_copy = gtk::Box::new(gtk::Orientation::Vertical, 4);
    about_button_copy.set_hexpand(true);
    about_button_copy.set_valign(gtk::Align::Center);
    let about_button_title = gtk::Label::new(Some(tr(language, "Sobre")));
    about_button_title.set_xalign(0.0);
    about_button_title.add_css_class("potoo-settings-row-title");
    let about_button_description = gtk::Label::new(Some(tr(language, "Versão e links do projeto")));
    about_button_description.set_xalign(0.0);
    about_button_description.add_css_class("potoo-settings-row-description");
    about_button_copy.append(&about_button_title);
    about_button_copy.append(&about_button_description);
    about_button_content.append(&about_button_copy);
    let about_arrow = icon("chevron-right");
    about_arrow.add_css_class("potoo-settings-about-arrow");
    about_button_content.append(&about_arrow);
    about_button.set_child(Some(&about_button_content));
    about_section.append(&about_button);
    page.append(&about_section);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&page)
        .build();

    let about_page = gtk::Box::new(gtk::Orientation::Vertical, 0);
    about_page.add_css_class("potoo-about-page");
    about_page.set_hexpand(true);
    about_page.set_vexpand(true);

    let about_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    about_content.add_css_class("potoo-about-content");
    about_content.set_halign(gtk::Align::Center);
    about_content.set_valign(gtk::Align::Start);
    about_content.set_size_request(500, -1);

    let identity = gtk::Box::new(gtk::Orientation::Vertical, 6);
    identity.add_css_class("potoo-about-identity");
    identity.set_halign(gtk::Align::Center);
    let logo = header::app_icon();
    logo.set_pixel_size(66);
    logo.add_css_class("potoo-about-logo");
    let app_name = gtk::Label::new(Some("Potoo IDE"));
    app_name.add_css_class("potoo-about-title");
    let creator = gtk::Label::new(Some(tr(language, "Criado por Diego Pereira do Nascimento")));
    creator.add_css_class("potoo-about-creator");
    creator.set_wrap(true);
    creator.set_justify(gtk::Justification::Center);
    let version = gtk::Label::new(Some(&format!(
        "{} {}",
        tr(language, "Versão"),
        env!("CARGO_PKG_VERSION")
    )));
    version.add_css_class("potoo-about-version");
    identity.append(&logo);
    identity.append(&app_name);
    identity.append(&creator);
    identity.append(&version);
    about_content.append(&identity);

    let about_links = gtk::Box::new(gtk::Orientation::Vertical, 0);
    about_links.add_css_class("potoo-about-links");
    let link_items = [
        (
            tr(language, "Relatar um problema"),
            "https://github.com/diegopn/potoo-ide/issues",
        ),
        (
            tr(language, "Código do projeto"),
            "https://github.com/diegopn/potoo-ide",
        ),
        (
            tr(language, "Site do projeto"),
            "https://diegopn.github.io/potoo-site/",
        ),
        (
            tr(language, "Ajuda"),
            "https://diegopn.github.io/potoo-site/manual/",
        ),
        (tr(language, "Doar"), "https://github.com/sponsors/diegopn"),
        (
            tr(language, "Contato"),
            "https://diegopn.github.io/#contato",
        ),
    ];
    for (index, (label, uri)) in link_items.iter().enumerate() {
        let link = gtk::LinkButton::with_label(uri, label);
        link.add_css_class("potoo-about-link");
        link.set_hexpand(true);
        link.set_halign(gtk::Align::Fill);
        link.set_tooltip_text(Some(uri));
        let link_content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        link_content.set_hexpand(true);
        let link_label = gtk::Label::new(Some(label));
        link_label.set_xalign(0.0);
        link_label.set_hexpand(true);
        link_label.add_css_class("potoo-about-link-label");
        link_content.append(&link_label);
        let external_icon = icon("open-external");
        external_icon.set_pixel_size(14);
        link_content.append(&external_icon);
        link.set_child(Some(&link_content));
        about_links.append(&link);
        if index < link_items.len() - 1 {
            let separator = gtk::Separator::new(gtk::Orientation::Horizontal);
            separator.add_css_class("potoo-about-link-separator");
            about_links.append(&separator);
        }
    }
    about_content.append(&about_links);
    about_page.append(&about_content);

    let about_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .hexpand(true)
        .vexpand(true)
        .child(&about_page)
        .build();

    let stack = gtk::Stack::new();
    stack.add_css_class("potoo-settings-stack");
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.set_vhomogeneous(false);
    stack.add_named(&scroll, Some("preferences"));
    stack.add_named(&about_scroll, Some("about"));
    stack.set_visible_child_name("preferences");

    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.add_css_class("potoo-settings-root");
    content.append(&stack);
    window.set_child(Some(&content));

    let stack_for_about = stack.clone();
    let window_for_about = window.clone();
    let title_for_about = title_label.clone();
    let back_for_about = back_button.clone();
    let language_for_about = ui_language_state.clone();
    about_button.connect_clicked(move |_| {
        stack_for_about.set_visible_child_name("about");
        window_for_about.set_default_size(740, 560);
        back_for_about.set_visible(true);
        title_for_about.set_text(tr(language_for_about.get(), "Sobre o Potoo IDE"));
        window_for_about.set_title(Some(tr(language_for_about.get(), "Sobre o Potoo IDE")));
    });

    let stack_for_back = stack.clone();
    let window_for_back = window.clone();
    let title_for_back = title_label.clone();
    let back_for_back = back_button.clone();
    let language_for_back = ui_language_state.clone();
    back_button.connect_clicked(move |_| {
        stack_for_back.set_visible_child_name("preferences");
        window_for_back.set_default_size(740, 685);
        back_for_back.set_visible(false);
        title_for_back.set_text(tr(language_for_back.get(), "Configurações"));
        window_for_back.set_title(Some(tr(language_for_back.get(), "Configurações")));
    });

    let sender_for_theme = sender.clone();
    for (index, button) in theme_buttons.iter().enumerate() {
        let theme_buttons_for_click = theme_buttons.clone();
        let sender_for_click = sender_for_theme.clone();
        button.connect_clicked(move |_| {
            let index = index as u32;
            select_theme_button(&theme_buttons_for_click, index);
            sender_for_click.input(MainWindowMsg::ThemeSelected(
                ThemePreference::from_selected_index(index),
            ));
        });
    }

    let sender_for_font_size = sender.clone();
    font_size_picker.connect_selected_notify(move |picker| {
        let font_size = match picker.selected() {
            1 => 15,
            2 => 16,
            3 => 18,
            _ => 14,
        };
        sender_for_font_size.input(MainWindowMsg::FontSizeSelected(font_size));
    });

    let sender_for_line_numbers = sender.clone();
    line_numbers_switch.connect_active_notify(move |switch| {
        sender_for_line_numbers.input(MainWindowMsg::LineNumbersChanged(switch.is_active()));
    });

    let sender_for_word_wrap = sender.clone();
    word_wrap_switch.connect_active_notify(move |switch| {
        sender_for_word_wrap.input(MainWindowMsg::WordWrapChanged(switch.is_active()));
    });

    let sender_for_indent = sender.clone();
    indent_width_picker.connect_selected_notify(move |picker| {
        sender_for_indent.input(MainWindowMsg::IndentWidthSelected(
            if picker.selected() == 0 { 2 } else { 4 },
        ));
    });

    let sender_for_ui_language = sender.clone();
    ui_language_picker.connect_selected_notify(move |picker| {
        sender_for_ui_language.input(MainWindowMsg::InterfaceLanguageSelected(
            UiLanguage::from_selected_index(picker.selected()),
        ));
    });

    let stack_for_close = stack.clone();
    let title_for_close = title_label.clone();
    let back_for_close = back_button.clone();
    let window_for_close = window.clone();
    let language_for_close = ui_language_state.clone();
    window.connect_close_request(move |_| {
        stack_for_close.set_visible_child_name("preferences");
        window_for_close.set_default_size(740, 685);
        back_for_close.set_visible(false);
        title_for_close.set_text(tr(language_for_close.get(), "Configurações"));
        window_for_close.set_title(Some(tr(language_for_close.get(), "Configurações")));
        sender.input(MainWindowMsg::CloseSettings);
        gtk::glib::Propagation::Stop
    });

    SettingsUi {
        window,
        theme_buttons,
        font_size_picker,
        line_numbers_switch,
        word_wrap_switch,
        indent_width_picker,
        ui_language_picker,
    }
}

pub(super) fn settings_section(title: &str) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 0);
    section.add_css_class("potoo-settings-section");
    let heading = gtk::Label::new(Some(title));
    heading.set_xalign(0.0);
    heading.add_css_class("potoo-settings-section-title");
    section.append(&heading);
    section
}

pub(super) fn select_theme_button(buttons: &[gtk::Button; 3], selected_index: u32) {
    for (index, button) in buttons.iter().enumerate() {
        if index as u32 == selected_index {
            button.add_css_class("potoo-theme-choice-selected");
        } else {
            button.remove_css_class("potoo-theme-choice-selected");
        }
    }
}

pub(super) fn settings_row<W: IsA<gtk::Widget>>(
    title: &str,
    description: &str,
    control: &W,
) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 18);
    row.add_css_class("potoo-settings-row");
    control.set_valign(gtk::Align::Center);

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 2);
    labels.set_hexpand(true);
    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("potoo-settings-row-title");
    let description_label = gtk::Label::new(Some(description));
    description_label.set_xalign(0.0);
    description_label.set_wrap(true);
    description_label.add_css_class("potoo-settings-row-description");
    labels.append(&title_label);
    labels.append(&description_label);
    row.append(&labels);
    row.append(control);
    row
}

pub(super) fn font_size_index(font_size: u32) -> u32 {
    match font_size {
        15 => 1,
        16 => 2,
        18 => 3,
        _ => 0,
    }
}

pub(super) fn set_font_size(widget: &impl IsA<gtk::Widget>, old_size: u32, new_size: u32) {
    widget.remove_css_class(&format!("potoo-font-{old_size}"));
    widget.add_css_class(&format!("potoo-font-{new_size}"));
}

use crate::app::{MainWindow, MainWindowMsg, ThemePreference, UiLanguage, tr};
use gtk::prelude::*;
use relm4::prelude::*;
use std::path::Path;

fn app_icon() -> gtk::Image {
    let development_icon = "data/icons/hicolor/scalable/apps/io.github.diegopn.potooide.svg";
    let image = if Path::new(development_icon).exists() {
        gtk::Image::from_file(development_icon)
    } else {
        gtk::Image::from_icon_name("io.github.diegopn.potooide")
    };
    image.set_pixel_size(32);
    image
}

pub(crate) struct HeaderUi {
    pub(crate) widget: Option<gtk::Widget>,
    pub(crate) project_title: gtk::Label,
    pub(crate) settings_button: gtk::Button,
    pub(crate) popover: gtk::Popover,
    pub(crate) theme_options: Vec<(ThemePreference, gtk::ToggleButton)>,
    pub(crate) run_button: gtk::Button,
}

pub(crate) fn build_header(
    window: &gtk::ApplicationWindow,
    language: UiLanguage,
    theme: ThemePreference,
    sender: ComponentSender<MainWindow>,
) -> HeaderUi {
    let brand = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    brand.append(&app_icon());
    let brand_label = gtk::Label::new(Some("Potoo IDE"));
    brand_label.add_css_class("potoo-brand-name");
    brand.append(&brand_label);

    let project_title = gtk::Label::new(Some(tr(language, "Meu primeiro programa")));
    project_title.add_css_class("potoo-project-name");
    project_title.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let menu_button = gtk::MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_tooltip_text(Some(tr(language, "Abrir menu")));
    menu_button.add_css_class("potoo-menu-button");

    let popover = gtk::Popover::new();
    popover.add_css_class("potoo-header-popover");
    let menu_content = gtk::Box::new(gtk::Orientation::Vertical, 10);
    menu_content.add_css_class("potoo-header-menu-content");

    let theme_heading = gtk::Label::new(Some(tr(language, "Aparência")));
    theme_heading.set_xalign(0.0);
    theme_heading.add_css_class("potoo-theme-menu-heading");
    menu_content.append(&theme_heading);

    let theme_choices = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    theme_choices.set_homogeneous(true);
    theme_choices.add_css_class("potoo-theme-choices");

    let system_theme = gtk::ToggleButton::with_label(tr(language, "Sistema"));
    system_theme.add_css_class("potoo-theme-choice");
    let light_theme = gtk::ToggleButton::with_label(tr(language, "Claro"));
    light_theme.set_group(Some(&system_theme));
    light_theme.add_css_class("potoo-theme-choice");
    let dark_theme = gtk::ToggleButton::with_label(tr(language, "Escuro"));
    dark_theme.set_group(Some(&system_theme));
    dark_theme.add_css_class("potoo-theme-choice");

    match theme {
        ThemePreference::System => system_theme.set_active(true),
        ThemePreference::Light => light_theme.set_active(true),
        ThemePreference::Dark => dark_theme.set_active(true),
    }

    let theme_options = vec![
        (ThemePreference::System, system_theme.clone()),
        (ThemePreference::Light, light_theme.clone()),
        (ThemePreference::Dark, dark_theme.clone()),
    ];
    for (preference, button) in &theme_options {
        let sender = sender.clone();
        let preference = *preference;
        button.connect_toggled(move |button| {
            if button.is_active() {
                sender.input(MainWindowMsg::ThemeSelected(preference));
            }
        });
    }

    theme_choices.append(&system_theme);
    theme_choices.append(&light_theme);
    theme_choices.append(&dark_theme);
    menu_content.append(&theme_choices);
    menu_content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let settings_button = gtk::Button::new();
    settings_button.add_css_class("potoo-menu-settings-action");
    let settings_content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    settings_content.set_hexpand(true);
    settings_content.append(&gtk::Image::from_icon_name("emblem-system-symbolic"));
    let settings_label = gtk::Label::new(Some(tr(language, "Configurações")));
    settings_label.set_xalign(0.0);
    settings_label.set_hexpand(true);
    settings_content.append(&settings_label);
    settings_button.set_child(Some(&settings_content));
    settings_button.set_hexpand(true);
    menu_content.append(&settings_button);

    popover.set_child(Some(&menu_content));
    menu_button.set_popover(Some(&popover));

    let run_button = gtk::Button::new();
    run_button.add_css_class("potoo-run-button");
    let run_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    run_content.append(&gtk::Image::from_icon_name("media-playback-start-symbolic"));
    run_content.append(&gtk::Label::new(Some(tr(language, "Executar"))));
    run_button.set_child(Some(&run_content));
    run_button.set_tooltip_text(Some(tr(
        language,
        "A execução real usa o interpretador da linguagem selecionada",
    )));

    let header_actions = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    header_actions.append(&menu_button);
    header_actions.append(&run_button);

    #[cfg(target_os = "linux")]
    header_actions.set_margin_end(8);

    #[cfg(target_os = "linux")]
    let widget = {
        let titlebar = gtk::HeaderBar::new();
        titlebar.add_css_class("potoo-linux-titlebar");
        titlebar.set_show_title_buttons(true);
        titlebar.pack_start(&brand);
        titlebar.set_title_widget(Some(&project_title));
        titlebar.pack_end(&header_actions);
        window.set_titlebar(Some(&titlebar));
        None
    };

    #[cfg(not(target_os = "linux"))]
    let widget = {
        let header = gtk::CenterBox::new();
        header.add_css_class("potoo-header");
        header.set_margin_start(14);
        header.set_margin_end(14);
        header.set_start_widget(Some(&brand));
        header.set_center_widget(Some(&project_title));
        header.set_end_widget(Some(&header_actions));
        Some(header.upcast::<gtk::Widget>())
    };

    HeaderUi {
        widget,
        project_title,
        settings_button,
        popover,
        theme_options,
        run_button,
    }
}

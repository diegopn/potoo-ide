use crate::app::{UiLanguage, tr};
use gtk::prelude::*;
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
    pub(crate) run_button: gtk::Button,
}

pub(crate) fn build_header(window: &gtk::ApplicationWindow, language: UiLanguage) -> HeaderUi {
    let brand = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    brand.append(&app_icon());
    let brand_label = gtk::Label::new(Some("Potoo IDE"));
    brand_label.add_css_class("potoo-brand-name");
    brand.append(&brand_label);

    let project_title = gtk::Label::new(Some(tr(language, "Meu primeiro programa")));
    project_title.add_css_class("potoo-project-name");
    project_title.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let settings_button = icon_button(
        "preferences-system-symbolic",
        tr(language, "Configurações"),
        "potoo-settings-button",
    );

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

    let header_actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    header_actions.append(&settings_button);
    header_actions.append(&run_button);

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
        run_button,
    }
}

fn icon_button(icon_name: &str, tooltip: &str, css_class: &str) -> gtk::Button {
    let button = gtk::Button::new();
    button.set_child(Some(&gtk::Image::from_icon_name(icon_name)));
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class(css_class);
    button
}

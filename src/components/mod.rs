pub(crate) mod content_panel;
pub(crate) mod header;
pub(crate) mod sidebar;

use gtk::prelude::*;

pub(crate) fn icon(name: &str) -> gtk::Image {
    let image = gtk::Image::from_icon_name(&format!("potoo-{name}-symbolic"));
    image.set_pixel_size(16);
    image.add_css_class("potoo-icon");
    image
}

pub(crate) fn icon_label_button(
    icon_name: &str,
    label: &str,
    tooltip: &str,
    css_class: &str,
) -> gtk::Button {
    let button = gtk::Button::new();
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.set_hexpand(true);
    content.append(&icon(icon_name));
    let label = gtk::Label::new(Some(label));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    content.append(&label);
    button.set_child(Some(&content));
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class(css_class);
    button.set_halign(gtk::Align::Fill);
    button.set_hexpand(true);
    button
}

pub(crate) fn clear_box(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

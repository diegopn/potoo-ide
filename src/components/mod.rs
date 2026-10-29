pub(crate) mod content_panel;
pub(crate) mod header;
pub(crate) mod sidebar;

use gtk::prelude::*;

pub(crate) fn icon_label_button(
    icon_name: &str,
    label: &str,
    tooltip: &str,
    css_class: &str,
) -> gtk::Button {
    let button = gtk::Button::new();
    let content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    content.append(&gtk::Image::from_icon_name(icon_name));
    content.append(&gtk::Label::new(Some(label)));
    button.set_child(Some(&content));
    button.set_tooltip_text(Some(tooltip));
    button.add_css_class(css_class);
    button
}

pub(crate) fn clear_box(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

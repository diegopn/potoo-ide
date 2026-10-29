pub(crate) mod content_panel;
pub(crate) mod header;
pub(crate) mod sidebar;

use gtk::prelude::*;

pub(crate) fn clear_box(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

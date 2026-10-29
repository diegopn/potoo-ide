mod model;
mod view;

pub(crate) use model::{Language, SourceDocument};
pub(crate) use view::{
    SyntaxTags, build_learning_panel, build_output_panel, highlight_buffer, make_syntax_tags,
    rebuild_editor_tabs, set_syntax_palette, update_line_numbers,
};

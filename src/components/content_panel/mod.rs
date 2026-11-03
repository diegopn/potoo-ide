mod completion;
mod model;
mod view;

pub(crate) use model::{Language, ProjectTemplate, SourceDocument};
pub(crate) use view::{
    CompletionPopup, DebuggerLearningUi, OutputPanelUi, SyntaxTags, build_learning_panel,
    build_output_panel, build_tool_panel, highlight_buffer, make_syntax_tags, rebuild_editor_tabs,
    set_syntax_palette, show_learning_problems, update_line_numbers,
};

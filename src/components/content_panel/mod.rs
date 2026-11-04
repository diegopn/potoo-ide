mod completion;
mod model;
mod view;

pub(crate) use completion::Provider as CompletionProvider;
pub(crate) use model::{Language, ProjectTemplate, SourceDocument};
pub(crate) use view::{
    DebuggerLearningUi, OutputPanelUi, SyntaxTags, apply_diagnostics, build_learning_panel,
    build_output_panel, build_tool_panel, highlight_buffer, install_diagnostic_markers,
    make_syntax_tags, rebuild_editor_tabs, set_syntax_palette, show_diagnostics,
    show_learning_problems,
};

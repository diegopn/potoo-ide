mod documents;
mod execution;
mod panels;
mod project;
mod settings;

use super::super::*;
use super::super::{dialogs::*, learning::*, settings::*};
use super::MainUi;

pub(super) struct ViewContext<'a> {
    active_document: Option<&'a SourceDocument>,
    active_language: Option<Language>,
    learning_available: bool,
    debugger_available: bool,
    game_preview_available: bool,
}

impl<'a> ViewContext<'a> {
    fn new(window: &'a MainWindow) -> Self {
        let active_document = window.active_document();
        let active_language = active_document.map(|document| document.language);
        let learning_available =
            active_language.map_or(true, crate::features::learning::is_available);
        let debugger_available =
            active_language.map_or(true, crate::features::debugging::is_available);
        let game_preview_available = active_language == Some(Language::Potigol);

        Self {
            active_document,
            active_language,
            learning_available,
            debugger_available,
            game_preview_available,
        }
    }
}

impl MainWindow {
    pub(super) fn update_view_component(
        &self,
        widgets: &mut MainUi,
        sender: ComponentSender<MainWindow>,
    ) {
        self.sync_new_item_context_view(widgets);
        let view = ViewContext::new(self);
        self.sync_completion_provider_view(widgets, &view);
        self.sync_tool_panels_view(widgets, &view);
        self.sync_learning_panel_view(widgets);
        let active_document_changed = self.sync_active_document_view(widgets, &view);
        self.sync_settings_view(widgets);
        self.sync_workspace_view(widgets, &view, sender.clone());
        self.sync_document_tabs_view(widgets, sender.clone());
        self.sync_close_app_view(widgets);
        self.sync_output_status_view(widgets);
        let (editor_diagnostics, diagnostics_changed) =
            self.sync_diagnostics_list_view(widgets, sender.clone());
        self.sync_execution_controls_view(widgets, &view);
        self.sync_editor_marks_view(
            widgets,
            &view,
            active_document_changed,
            &editor_diagnostics,
            diagnostics_changed,
        );
    }
}

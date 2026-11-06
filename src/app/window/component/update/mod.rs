mod analysis;
mod documents;
mod execution;
mod preferences;
mod projects;
use super::super::*;
use super::super::{analysis::*, dialogs::*, persistence::*};

impl MainWindow {
    pub(super) fn update_component(
        &mut self,
        message: MainWindowMsg,
        sender: ComponentSender<MainWindow>,
    ) {
        let persist_after = should_persist_after(&message);
        match message {
            message @ (MainWindowMsg::InterfaceLanguageSelected(..)
            | MainWindowMsg::ThemeSelected(..)
            | MainWindowMsg::SystemThemeChanged(..)
            | MainWindowMsg::FontSizeSelected(..)
            | MainWindowMsg::LineNumbersChanged(..)
            | MainWindowMsg::WordWrapChanged(..)
            | MainWindowMsg::IndentWidthSelected(..)
            | MainWindowMsg::OpenSettings
            | MainWindowMsg::CloseSettings
            | MainWindowMsg::SelectRightPanel(..)) => {
                self.handle_preferences(message, sender.clone())
            }
            message @ (MainWindowMsg::ProjectSelected(..)
            | MainWindowMsg::RecentSelected(..)
            | MainWindowMsg::RecentRemoved(..)
            | MainWindowMsg::ProjectSwitchDecision(..)
            | MainWindowMsg::ProjectScanCompleted { .. }
            | MainWindowMsg::CreateProject(..)
            | MainWindowMsg::CreateFile(..)
            | MainWindowMsg::CreateFolder(..)
            | MainWindowMsg::RenameProjectItem(..)
            | MainWindowMsg::RenameProjectItemConfirmed(..)
            | MainWindowMsg::DeleteProjectItem(..)
            | MainWindowMsg::DeleteProjectItemConfirmed(..)
            | MainWindowMsg::FileOperationFailed(..)) => {
                self.handle_projects(message, sender.clone())
            }
            message @ (MainWindowMsg::DocumentReadCompleted { .. }
            | MainWindowMsg::OpenFile(..)
            | MainWindowMsg::OpenMaplerText(..)
            | MainWindowMsg::ActivateDocument(..)
            | MainWindowMsg::CloseDocument(..)
            | MainWindowMsg::CloseDocumentDecision(..)
            | MainWindowMsg::SaveAndCloseDocument(..)
            | MainWindowMsg::SaveCurrent
            | MainWindowMsg::SaveAsSelected(..)
            | MainWindowMsg::SaveAsCanceled(..)
            | MainWindowMsg::RequestClose
            | MainWindowMsg::CloseAppDecision(..)
            | MainWindowMsg::SourceChanged(..)
            | MainWindowMsg::PersistRecovery(..)) => self.handle_documents(message, sender.clone()),
            message @ (MainWindowMsg::Run
            | MainWindowMsg::StartDebug
            | MainWindowMsg::StepDebug
            | MainWindowMsg::StopDebug
            | MainWindowMsg::ExecutionEvent(..)
            | MainWindowMsg::ExecutionLost(..)
            | MainWindowMsg::FileSelectionResolved(..)
            | MainWindowMsg::SubmitProgramInput(..)
            | MainWindowMsg::ProgramInputChanged(..)
            | MainWindowMsg::OpenGamePreviewWindow) => {
                self.handle_execution(message, sender.clone())
            }
            message @ (MainWindowMsg::CursorMoved(..)
            | MainWindowMsg::LearningDebounceElapsed(..)
            | MainWindowMsg::LearningAnalysisReady { .. }
            | MainWindowMsg::DiagnosticsDebounceElapsed(..)
            | MainWindowMsg::DiagnosticsAnalysisReady { .. }
            | MainWindowMsg::NavigateToDiagnostic(..)) => {
                self.handle_analysis(message, sender.clone())
            }
        }

        let active_language = self.active_document().map(|document| document.language);
        if self.learning_document_id != self.active_document_id
            || self.learning_language != active_language
        {
            self.learning_revision = self.learning_revision.wrapping_add(1);
            self.schedule_learning_refresh(sender.clone(), Duration::from_millis(80));
        }
        self.ensure_diagnostics_for_active_document(sender.clone());

        if persist_after {
            self.persist_session(false, false);
        }
    }
}

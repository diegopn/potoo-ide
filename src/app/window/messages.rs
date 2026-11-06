use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RightPanelSection {
    Learning,
    Debugger,
    GamePreview,
}

#[derive(Debug)]
pub(crate) enum MainWindowMsg {
    InterfaceLanguageSelected(UiLanguage),
    ProjectSelected(PathBuf),
    RecentSelected(PathBuf),
    RecentRemoved(PathBuf),
    ProjectSwitchDecision(bool),
    ProjectScanCompleted {
        project_generation: u64,
        scan_generation: u64,
        root: PathBuf,
        load_project: bool,
        result: Result<Vec<ProjectFile>, String>,
    },
    DocumentReadCompleted {
        project_generation: u64,
        path: PathBuf,
        language_override: Option<Language>,
        standalone: bool,
        result: Result<String, String>,
    },
    CreateProject(PathBuf, String, ProjectTemplate),
    CreateFile(String, ProjectTemplate, PathBuf),
    CreateFolder(String, PathBuf),
    RenameProjectItem(PathBuf),
    RenameProjectItemConfirmed(PathBuf, String),
    DeleteProjectItem(PathBuf),
    DeleteProjectItemConfirmed(PathBuf),
    OpenFile(PathBuf),
    OpenMaplerText(PathBuf),
    ActivateDocument(u64),
    CloseDocument(u64),
    CloseDocumentDecision(u64, bool),
    SaveAndCloseDocument(u64),
    SaveCurrent,
    SaveAsSelected(u64, PathBuf),
    SaveAsCanceled(u64),
    FileOperationFailed(String),
    RequestClose,
    CloseAppDecision(bool),
    ThemeSelected(ThemePreference),
    SystemThemeChanged(bool),
    FontSizeSelected(u32),
    LineNumbersChanged(bool),
    WordWrapChanged(bool),
    IndentWidthSelected(u32),
    OpenSettings,
    CloseSettings,
    Run,
    StartDebug,
    StepDebug,
    StopDebug,
    OpenGamePreviewWindow,
    ExecutionEvent(u64, ExecutionEvent),
    ExecutionLost(u64),
    FileSelectionResolved(u64, Result<String, String>),
    SubmitProgramInput(String),
    ProgramInputChanged(String),
    SourceChanged(String),
    CursorMoved(usize),
    SelectRightPanel(RightPanelSection),
    LearningDebounceElapsed(u64),
    DiagnosticsDebounceElapsed(AnalysisIdentity),
    DiagnosticsAnalysisReady {
        identity: AnalysisIdentity,
        diagnostics: Vec<EditorDiagnostic>,
    },
    NavigateToDiagnostic(EditorDiagnostic),
    LearningAnalysisReady {
        generation: u64,
        document_id: u64,
        revision: u64,
        line: usize,
        language: Language,
        hint: LearningHint,
    },
    PersistRecovery(u64),
}

pub(crate) fn should_persist_after(message: &MainWindowMsg) -> bool {
    matches!(
        message,
        MainWindowMsg::ProjectSelected(_)
            | MainWindowMsg::RecentSelected(_)
            | MainWindowMsg::ProjectSwitchDecision(_)
            | MainWindowMsg::ProjectScanCompleted { .. }
            | MainWindowMsg::DocumentReadCompleted { .. }
            | MainWindowMsg::CreateProject(_, _, _)
            | MainWindowMsg::CreateFile(_, _, _)
            | MainWindowMsg::CreateFolder(_, _)
            | MainWindowMsg::RenameProjectItemConfirmed(_, _)
            | MainWindowMsg::DeleteProjectItemConfirmed(_)
            | MainWindowMsg::OpenFile(_)
            | MainWindowMsg::OpenMaplerText(_)
            | MainWindowMsg::ActivateDocument(_)
            | MainWindowMsg::CloseDocument(_)
            | MainWindowMsg::CloseDocumentDecision(_, _)
            | MainWindowMsg::SaveAndCloseDocument(_)
            | MainWindowMsg::SaveCurrent
            | MainWindowMsg::SaveAsSelected(_, _)
    )
}

#[cfg(test)]
mod tests {
    use super::{MainWindowMsg, RightPanelSection, should_persist_after};
    use std::path::PathBuf;

    #[test]
    fn persists_after_project_and_document_updates() {
        let messages = [
            MainWindowMsg::ProjectSelected(PathBuf::new()),
            MainWindowMsg::CloseDocumentDecision(7, false),
            MainWindowMsg::SaveCurrent,
        ];

        for message in messages {
            assert!(should_persist_after(&message));
        }
    }

    #[test]
    fn skips_persistence_for_transient_updates() {
        let messages = [
            MainWindowMsg::Run,
            MainWindowMsg::SourceChanged(String::from("escreva(1)")),
            MainWindowMsg::SelectRightPanel(RightPanelSection::Learning),
            MainWindowMsg::PersistRecovery(1),
        ];

        for message in messages {
            assert!(!should_persist_after(&message));
        }
    }
}

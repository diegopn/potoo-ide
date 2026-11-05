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

pub(super) struct LearningAnalysisRequest {
    pub(super) generation: u64,
    pub(super) document_id: u64,
    pub(super) revision: u64,
    pub(super) line: usize,
    pub(super) contents: String,
    pub(super) language: Language,
    pub(super) runtime_diagnostics: Vec<execution::ExecutionDiagnostic>,
    pub(super) editor_diagnostics: Vec<EditorDiagnostic>,
}

pub(super) struct DiagnosticsAnalysisRequest {
    pub(super) identity: AnalysisIdentity,
    pub(super) source_name: String,
    pub(super) source_path: Option<PathBuf>,
    pub(super) contents: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LearningAnalysisIdentity {
    pub(super) generation: u64,
    pub(super) document_id: u64,
    pub(super) revision: u64,
    pub(super) line: usize,
    pub(super) language: Language,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct CurrentLearningAnalysis {
    pub(super) generation: u64,
    pub(super) active_document_id: Option<u64>,
    pub(super) learning_document_id: Option<u64>,
    pub(super) revision: u64,
    pub(super) line: usize,
    pub(super) language: Option<Language>,
}

pub(super) fn merge_editor_diagnostics(hint: &mut LearningHint, diagnostics: &[EditorDiagnostic]) {
    if diagnostics.iter().any(|diagnostic| {
        matches!(
            diagnostic.category,
            diagnostics::DiagnosticCategory::Lexical | diagnostics::DiagnosticCategory::Syntax
        )
    }) {
        hint.problems.retain(|problem| {
            !(problem.kind == crate::features::learning::LearningProblemKind::Syntax
                && problem.text
                    == crate::features::learning::LearningProblemText::InterpreterDiagnostic)
        });
    }

    for diagnostic in diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.line == hint.line)
    {
        let severity = match diagnostic.severity {
            diagnostics::DiagnosticSeverity::Error => {
                crate::features::learning::LearningSeverity::Error
            }
            diagnostics::DiagnosticSeverity::Warning => {
                crate::features::learning::LearningSeverity::Warning
            }
            diagnostics::DiagnosticSeverity::Information
            | diagnostics::DiagnosticSeverity::Hint => {
                continue;
            }
        };
        let kind = match diagnostic.category {
            diagnostics::DiagnosticCategory::Lexical | diagnostics::DiagnosticCategory::Syntax => {
                crate::features::learning::LearningProblemKind::Syntax
            }
            diagnostics::DiagnosticCategory::Type => {
                crate::features::learning::LearningProblemKind::Type
            }
            diagnostics::DiagnosticCategory::Runtime => {
                crate::features::learning::LearningProblemKind::Runtime
            }
            diagnostics::DiagnosticCategory::StaticAnalysis => {
                crate::features::learning::LearningProblemKind::PossibleLogic
            }
        };
        if hint.problems.iter().any(|problem| {
            problem.kind == kind
                && problem.line == diagnostic.line
                && problem.column == diagnostic.column
                && problem.message == diagnostic.message
        }) {
            continue;
        }
        hint.problems
            .push(crate::features::learning::LearningProblem {
                kind,
                severity,
                line: diagnostic.line,
                column: diagnostic.column,
                text: crate::features::learning::LearningProblemText::InterpreterDiagnostic,
                message: diagnostic.message.clone(),
                suggestion: None,
            });
    }
}

impl CurrentLearningAnalysis {
    pub(super) fn accepts(self, result: LearningAnalysisIdentity) -> bool {
        self.generation == result.generation
            && self.active_document_id == Some(result.document_id)
            && self.learning_document_id == Some(result.document_id)
            && self.revision == result.revision
            && self.line == result.line
            && self.language == Some(result.language)
    }
}

pub(super) fn invalidate_learning_hint(
    generation: &mut u64,
    hint: &mut Option<LearningHint>,
) -> u64 {
    *generation = generation.wrapping_add(1).max(1);
    *hint = None;
    *generation
}

pub(super) fn project_scan_is_current(
    current_project_generation: u64,
    current_scan_generation: u64,
    current_root: Option<&Path>,
    project_generation: u64,
    scan_generation: u64,
    root: &Path,
    loading_project: bool,
) -> bool {
    current_project_generation == project_generation
        && current_scan_generation == scan_generation
        && (loading_project || current_root == Some(root))
}

#[derive(Default)]
pub(super) struct LearningAnalysisWorkerState {
    latest_request: Option<LearningAnalysisRequest>,
    shutdown: bool,
}

pub(super) struct LearningAnalysisWorker {
    state: Arc<(Mutex<LearningAnalysisWorkerState>, Condvar)>,
    generation: Arc<AtomicU64>,
}

impl LearningAnalysisWorker {
    pub(super) fn new(sender: ComponentSender<MainWindow>) -> Self {
        let state = Arc::new((
            Mutex::new(LearningAnalysisWorkerState::default()),
            Condvar::new(),
        ));
        let generation = Arc::new(AtomicU64::new(0));
        let state_for_worker = Arc::clone(&state);
        let generation_for_worker = Arc::clone(&generation);
        std::thread::spawn(move || {
            loop {
                let request = {
                    let (state_lock, wake) = &*state_for_worker;
                    let mut state = state_lock
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    while state.latest_request.is_none() && !state.shutdown {
                        state = wake
                            .wait(state)
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                    }
                    if state.shutdown {
                        return;
                    }
                    state.latest_request.take().expect("request was checked")
                };
                if generation_for_worker.load(Ordering::Acquire) != request.generation {
                    continue;
                }

                let request_generation = request.generation;
                let hint = learning_hint_with_runtime_cancellable(
                    &request.contents,
                    request.line,
                    request.language,
                    &request.runtime_diagnostics,
                    || generation_for_worker.load(Ordering::Acquire) != request_generation,
                )
                .map(|mut hint| {
                    merge_editor_diagnostics(&mut hint, &request.editor_diagnostics);
                    hint
                });
                if let Some(hint) = hint
                    && generation_for_worker.load(Ordering::Acquire) == request_generation
                {
                    sender.input(MainWindowMsg::LearningAnalysisReady {
                        generation: request.generation,
                        document_id: request.document_id,
                        revision: request.revision,
                        line: request.line,
                        language: request.language,
                        hint,
                    });
                }
            }
        });
        Self { state, generation }
    }

    pub(super) fn invalidate(&self, generation: u64) {
        self.generation.store(generation, Ordering::Release);
        let (state_lock, _) = &*self.state;
        let mut state = state_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.latest_request = None;
    }

    pub(super) fn submit(&self, request: LearningAnalysisRequest) {
        if self.generation.load(Ordering::Acquire) != request.generation {
            return;
        }
        let (state_lock, wake) = &*self.state;
        let mut state = state_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.shutdown || self.generation.load(Ordering::Acquire) != request.generation {
            return;
        }
        state.latest_request = Some(request);
        wake.notify_one();
    }
}

impl Drop for LearningAnalysisWorker {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        let (state_lock, wake) = &*self.state;
        let mut state = state_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.shutdown = true;
        state.latest_request = None;
        wake.notify_one();
    }
}

#[derive(Default)]
pub(super) struct DiagnosticsAnalysisWorkerState {
    latest_request: Option<DiagnosticsAnalysisRequest>,
    shutdown: bool,
}

pub(super) struct DiagnosticsAnalysisWorker {
    state: Arc<(Mutex<DiagnosticsAnalysisWorkerState>, Condvar)>,
    generation: Arc<AtomicU64>,
}

impl DiagnosticsAnalysisWorker {
    pub(super) fn new(sender: ComponentSender<MainWindow>) -> Self {
        let state = Arc::new((
            Mutex::new(DiagnosticsAnalysisWorkerState::default()),
            Condvar::new(),
        ));
        let generation = Arc::new(AtomicU64::new(0));
        let state_for_worker = Arc::clone(&state);
        let generation_for_worker = Arc::clone(&generation);
        std::thread::spawn(move || {
            loop {
                let request = {
                    let (state_lock, wake) = &*state_for_worker;
                    let mut state = state_lock
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    while state.latest_request.is_none() && !state.shutdown {
                        state = wake
                            .wait(state)
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                    }
                    if state.shutdown {
                        return;
                    }
                    state.latest_request.take().expect("request was checked")
                };
                if generation_for_worker.load(Ordering::Acquire) != request.identity.generation {
                    continue;
                }

                let diagnostics = diagnostics::analyze(
                    request.identity.document_id,
                    &request.source_name,
                    request.source_path.as_deref(),
                    request.identity.language,
                    &request.contents,
                );
                if generation_for_worker.load(Ordering::Acquire) == request.identity.generation {
                    sender.input(MainWindowMsg::DiagnosticsAnalysisReady {
                        identity: request.identity,
                        diagnostics,
                    });
                }
            }
        });
        Self { state, generation }
    }

    pub(super) fn invalidate(&self, generation: u64) {
        self.generation.store(generation, Ordering::Release);
        let (state_lock, _) = &*self.state;
        let mut state = state_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.latest_request = None;
    }

    pub(super) fn submit(&self, request: DiagnosticsAnalysisRequest) {
        if self.generation.load(Ordering::Acquire) != request.identity.generation {
            return;
        }
        let (state_lock, wake) = &*self.state;
        let mut state = state_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.shutdown || self.generation.load(Ordering::Acquire) != request.identity.generation
        {
            return;
        }
        state.latest_request = Some(request);
        wake.notify_one();
    }
}

impl Drop for DiagnosticsAnalysisWorker {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        let (state_lock, wake) = &*self.state;
        let mut state = state_lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.shutdown = true;
        state.latest_request = None;
        wake.notify_one();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct DiagnosticNavigation {
    pub(super) serial: u64,
    pub(super) document_id: u64,
    pub(super) offset: usize,
}

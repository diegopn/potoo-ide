use std::path::PathBuf;

use crate::languages::model::Language;

use crate::features::diagnostics::EditorDiagnostic;

#[derive(Clone, Debug)]
pub(crate) struct SourceDocument {
    pub(crate) id: u64,
    pub(crate) path: Option<PathBuf>,
    pub(crate) name: String,
    pub(crate) language: Language,
    pub(crate) contents: String,
    pub(crate) saved_contents: String,
    pub(crate) revision: u64,
    pub(crate) diagnostics: Option<Vec<EditorDiagnostic>>,
    pub(crate) runtime_diagnostics: Vec<EditorDiagnostic>,
}

impl SourceDocument {
    pub(crate) fn is_dirty(&self) -> bool {
        self.contents != self.saved_contents
    }
}

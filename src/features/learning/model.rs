use crate::languages::model::{Language, SourceCall};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LearningTopic {
    Function,
    Procedure,
    Decision,
    Loop,
    Input,
    Output,
    Variable,
    Assignment,
    Call,
    LibraryCall,
    Import,
    Return,
    Program,
    Unrecognized,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LearningProblemKind {
    Syntax,
    Type,
    Runtime,
    Analysis,
    Logic,
    PossibleLogic,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LearningProblemText {
    InterpreterDiagnostic,
    AnalysisIncomplete,
    ConstantCondition {
        always_true: bool,
    },
    ConstantIndex {
        index: i64,
        name: String,
        lower_bound: i64,
        upper_bound: i64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LearningSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearningProblem {
    pub(crate) kind: LearningProblemKind,
    pub(crate) severity: LearningSeverity,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) text: LearningProblemText,
    pub(crate) message: String,
    pub(crate) suggestion: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LearningDimension {
    pub(crate) lower_bound: Option<i64>,
    pub(crate) upper_bound: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearningSymbol {
    pub(crate) name: String,
    pub(crate) type_name: String,
    pub(crate) is_constant: bool,
    pub(crate) by_reference: bool,
    pub(crate) dimensions: Vec<LearningDimension>,
    pub(crate) initializer: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LearningHint {
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) language: Language,
    pub(crate) topic: LearningTopic,
    pub(crate) name: Option<String>,
    pub(crate) return_type: Option<String>,
    pub(crate) target: Option<String>,
    pub(crate) parameters: Vec<LearningSymbol>,
    pub(crate) variables: Vec<LearningSymbol>,
    pub(crate) expression: Option<String>,
    pub(crate) experiment_expression: Option<String>,
    pub(crate) call: Option<SourceCall>,
    pub(crate) problems: Vec<LearningProblem>,
}

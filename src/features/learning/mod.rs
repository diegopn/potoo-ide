mod analysis;
pub(crate) mod catalog;
mod facts;
mod model;
mod problems;

#[cfg(test)]
mod tests;

pub(crate) use crate::languages::model::{
    SourceCall, SourceConstruct, SourceLineAnalysis, SourceSymbol,
};
pub(crate) use analysis::learning_hint_with_runtime_cancellable;
#[cfg(test)]
pub(crate) use analysis::{learning_hint, learning_hint_with_runtime};
pub(crate) use facts::source_call;
pub(crate) use model::*;

use self::facts::{call_from_source, learning_facts, unrecognized_facts};
use self::problems::*;
use crate::features::execution::{DiagnosticKind as ExecutionDiagnosticKind, ExecutionDiagnostic};
use crate::languages::{
    model::{Language, SourceDimension},
    source::{read_identifier, read_integer, skip_spaces},
};
use std::collections::HashMap;

pub(crate) fn is_available(language: Language) -> bool {
    language.profile().learning.is_some()
}

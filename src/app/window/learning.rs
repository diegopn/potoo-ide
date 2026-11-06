use super::*;

mod copy;
pub(super) use self::copy::learning_copy;

pub(super) fn format_diagnostics(
    diagnostics: &[execution::ExecutionDiagnostic],
    source: Option<(Language, &str)>,
) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| {
            let (category, diagnostic_category) = match diagnostic.kind {
                execution::DiagnosticKind::Lexical => (
                    "Erro léxico",
                    crate::features::diagnostics::DiagnosticCategory::Lexical,
                ),
                execution::DiagnosticKind::Syntax => (
                    "Erro de sintaxe",
                    crate::features::diagnostics::DiagnosticCategory::Syntax,
                ),
                execution::DiagnosticKind::Type => (
                    "Erro de tipo",
                    crate::features::diagnostics::DiagnosticCategory::Type,
                ),
                execution::DiagnosticKind::Runtime => (
                    "Erro de execução",
                    crate::features::diagnostics::DiagnosticCategory::Runtime,
                ),
            };
            let message = source.map_or_else(
                || diagnostic.message.clone(),
                |(language, source)| {
                    crate::features::diagnostics::adapt_diagnostic_message(
                        language,
                        source,
                        diagnostic_category,
                        &diagnostic.message,
                        diagnostic.line,
                        diagnostic.column,
                    )
                },
            );
            format!(
                "{category} · Linha {}, coluna {}: {message}",
                diagnostic.line, diagnostic.column
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(super) fn learning_topic_label(topic: LearningTopic, language: UiLanguage) -> &'static str {
    tr(
        language,
        match topic {
            LearningTopic::Function => "função",
            LearningTopic::Procedure => "procedimento",
            LearningTopic::Decision => "decisão",
            LearningTopic::Loop => "laço",
            LearningTopic::Input => "entrada",
            LearningTopic::Output => "saída",
            LearningTopic::Variable => "variável",
            LearningTopic::Assignment => "atribuição",
            LearningTopic::Call => "chamada",
            LearningTopic::LibraryCall => "chamada de biblioteca",
            LearningTopic::Import => "importação",
            LearningTopic::Return => "retorno",
            LearningTopic::Program => "programa",
            LearningTopic::Unrecognized => "não identificada",
        },
    )
}

pub(super) fn update_cursor_position(
    buffer: &gtk::TextBuffer,
    label: &gtk::Label,
    language: UiLanguage,
) {
    let cursor = buffer.iter_at_offset(buffer.cursor_position());
    label.set_text(&format!(
        "{} {} · {} {}",
        tr(language, "Linha"),
        cursor.line() + 1,
        tr(language, "Coluna"),
        cursor.line_offset() + 1
    ));
}

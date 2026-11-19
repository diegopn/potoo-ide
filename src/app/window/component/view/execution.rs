use super::panels::learning_concept_is_visible;
use super::*;

impl MainWindow {
    pub(super) fn sync_output_status_view(&self, widgets: &mut MainUi) {
        let ui = widgets;
        if ui.displayed_output_text != self.output_text {
            ui.workspace_ui().output_buffer.set_text(&self.output_text);
            if self.output_text == self.text("A saída aparecerá aqui após a execução.") {
                ui.workspace_ui()
                    .output_view
                    .add_css_class("potoo-output-placeholder-text");
            } else {
                ui.workspace_ui()
                    .output_view
                    .remove_css_class("potoo-output-placeholder-text");
            }
            ui.displayed_output_text.clone_from(&self.output_text);
        }
        if ui.displayed_output_status != self.output_status {
            ui.workspace_ui()
                .output_status
                .set_text(&self.output_status);
            ui.displayed_output_status.clone_from(&self.output_status);
        }
    }
    pub(super) fn sync_execution_controls_view(
        &self,
        widgets: &mut MainUi,
        view: &ViewContext<'_>,
    ) {
        let ui = widgets;
        let debugger_available = view.debugger_available;
        if ui.displayed_input_requested != self.input_requested {
            ui.workspace_ui()
                .input_entry
                .set_visible(self.input_requested);
            if self.input_requested {
                if let Some(prompt) = &self.input_prompt {
                    let placeholder = match (&prompt.variable_name, &prompt.data_type) {
                        (Some(name), Some(data_type)) => format!("{} ({data_type})", name),
                        _ => self.text("Digite a entrada do programa").to_owned(),
                    };
                    ui.workspace_ui()
                        .input_entry
                        .set_placeholder_text(Some(&placeholder));
                }
                ui.workspace_ui().input_entry.grab_focus();
            } else {
                ui.workspace_ui().input_entry.set_text("");
            }
            ui.displayed_input_requested = self.input_requested;
        }
        ui.workspace_ui()
            .debugger_controls
            .set_visible(debugger_available);
        ui.workspace_ui()
            .debugger_controls
            .set_visible_child_name(if self.debugging_active {
                "controls"
            } else {
                "start"
            });
        ui.workspace_ui()
            .debugger_start
            .set_tooltip_text(Some(self.text(if self.active_execution_id.is_some() {
                "Interrompa a execução atual antes de iniciar a depuração."
            } else {
                "Iniciar depuração"
            })));
        ui.workspace_ui()
            .debugger_start
            .set_sensitive(self.active_execution_id.is_none());
        ui.workspace_ui().debugger_step.set_sensitive(
            self.debugging_active
                && self.active_execution_id.is_some()
                && self.debugger_snapshot.is_some(),
        );
        ui.workspace_ui()
            .debugger_stop
            .set_visible(self.debugging_active);
        ui.workspace_ui()
            .debugger_stop
            .set_sensitive(self.debugging_active && self.active_execution_id.is_some());
        let debugger_error = self.debugger_error
            && self.active_document_id.is_some()
            && self.execution_diagnostics_document_id == self.active_document_id;
        if ui.displayed_debugger_snapshot != self.debugger_snapshot
            || ui.displayed_debugger_error != debugger_error
        {
            if let Some(snapshot) = &self.debugger_snapshot {
                let reason = match snapshot.pause_reason {
                    Some(crate::features::execution::DebugPauseReason::Step) => {
                        self.text("passo a passo")
                    }
                    Some(crate::features::execution::DebugPauseReason::Pause) => self.text("pausa"),
                    Some(crate::features::execution::DebugPauseReason::Debug) => {
                        self.text("ponto de depuração")
                    }
                    Some(crate::features::execution::DebugPauseReason::StaticErrorNotReached) => {
                        self.text("erro encontrado; trecho anterior não executado")
                    }
                    None => self.text("depuração"),
                };
                ui.workspace_ui().debugger_position.set_text(&format!(
                    "{} {} · {reason}",
                    self.text("Linha"),
                    snapshot.line
                ));
                let variables = if snapshot.variables.is_empty() {
                    self.text("Nenhuma variável visível.").to_owned()
                } else {
                    snapshot
                        .variables
                        .iter()
                        .map(|(name, value)| format!("{name} = {value}"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                ui.workspace_ui().debugger_variables.set_text(&variables);
                if self.execution_document_id == self.active_document_id
                    && let Some(iter) = ui
                        .workspace_ui()
                        .code_buffer
                        .iter_at_line(snapshot.line.saturating_sub(1) as i32)
                {
                    ui.workspace_ui().code_buffer.place_cursor(&iter);
                }
            } else if debugger_error {
                ui.workspace_ui()
                    .debugger_position
                    .set_text(self.text("Depuração encerrada com erro"));
                ui.workspace_ui()
                    .debugger_variables
                    .set_text(&self.problem_text);
            } else {
                ui.workspace_ui()
                    .debugger_position
                    .set_text(self.text("Aguardando depuração"));
                ui.workspace_ui().debugger_variables.set_text(
                    self.text(
                        "Os valores das variáveis aparecerão durante a execução passo a passo.",
                    ),
                );
            }
            ui.displayed_debugger_snapshot
                .clone_from(&self.debugger_snapshot);
            ui.displayed_debugger_error = debugger_error;
        }
        let current_debugger_hint = self
            .debugger_learning_hint
            .as_ref()
            .filter(|hint| {
                self.debugger_document_id == self.active_document_id
                    && self
                        .debugger_snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.line == hint.line)
            })
            .cloned();
        if ui.displayed_debugger_learning_hint != current_debugger_hint {
            ui.workspace_ui()
                .debugger_learning
                .content_scroll
                .vadjustment()
                .set_value(0.0);
            if let Some(hint) = &current_debugger_hint {
                let topic_name = learning_topic_label(hint.topic, self.ui_language);
                ui.workspace_ui()
                    .debugger_learning
                    .line_title
                    .set_text(&format!(
                        "{} {} · {topic_name}",
                        self.text("Linha"),
                        hint.line
                    ));
                let (explanation, concept, experiment) = learning_copy(hint, self.ui_language);
                ui.workspace_ui()
                    .debugger_learning
                    .explanation
                    .set_text(&explanation);
                ui.workspace_ui()
                    .debugger_learning
                    .experiment
                    .set_text(&experiment);
                ui.workspace_ui()
                    .debugger_learning
                    .experiment_section
                    .set_visible(!experiment.is_empty());
                let show_learning_concept = learning_concept_is_visible(hint.topic, &concept);
                ui.workspace_ui()
                    .debugger_learning
                    .concept_section
                    .set_visible(show_learning_concept);
                ui.workspace_ui()
                    .debugger_learning
                    .concept_title
                    .set_text(&format!("{} {topic_name}", self.text("Conceito:")));
                ui.workspace_ui()
                    .debugger_learning
                    .concept_text
                    .set_text(&concept);
                content_panel::show_learning_problems(
                    &ui.workspace_ui().debugger_learning.problem_list,
                    &ui.workspace_ui().debugger_learning.problem_empty,
                    &hint.problems,
                    self.ui_language,
                    &ui.workspace_ui().code_buffer,
                    ui.workspace_ui().code_view.upcast_ref(),
                );
                ui.workspace_ui()
                    .debugger_learning
                    .problem_empty
                    .set_text(self.text("Nenhum problema conhecido nesta linha."));
                ui.workspace_ui().debugger_learning.problem_empty.set_text(self.text(
                    if hint.topic == LearningTopic::Unrecognized {
                        "A análise não reconheceu esta construção; não foi possível verificar problemas."
                    } else {
                        "Nenhum problema conhecido nesta linha."
                    },
                ));
            } else {
                ui.workspace_ui()
                    .debugger_learning
                    .concept_section
                    .set_visible(false);
                ui.workspace_ui()
                    .debugger_learning
                    .experiment_section
                    .set_visible(false);
                ui.workspace_ui().debugger_learning.experiment.set_text("");
                ui.workspace_ui()
                    .debugger_learning
                    .line_title
                    .set_text(self.text("Linha 1 · programa"));
                ui.workspace_ui().debugger_learning.explanation.set_text("");
                ui.workspace_ui()
                    .debugger_learning
                    .concept_title
                    .set_text(self.text("Conceito: programa"));
                ui.workspace_ui()
                    .debugger_learning
                    .concept_text
                    .set_text("");
                content_panel::show_learning_problems(
                    &ui.workspace_ui().debugger_learning.problem_list,
                    &ui.workspace_ui().debugger_learning.problem_empty,
                    &[],
                    self.ui_language,
                    &ui.workspace_ui().code_buffer,
                    ui.workspace_ui().code_view.upcast_ref(),
                );
            }
            ui.displayed_debugger_learning_hint
                .clone_from(&current_debugger_hint);
        }
        let debugger_analysis_state = if !self.debugging_active {
            "idle"
        } else if self.debugger_snapshot.is_none()
            || self.debugger_document_id != self.active_document_id
        {
            "waiting"
        } else if current_debugger_hint.is_some() {
            "content"
        } else {
            "loading"
        };
        if ui.displayed_debugger_analysis_state != debugger_analysis_state {
            let loading = matches!(debugger_analysis_state, "waiting" | "loading");
            ui.workspace_ui()
                .debugger_learning
                .spinner
                .set_spinning(loading);
            ui.workspace_ui()
                .debugger_learning
                .spinner
                .set_visible(loading);
            if loading {
                ui.workspace_ui()
                    .debugger_learning
                    .loading_label
                    .set_text(self.text(if debugger_analysis_state == "waiting" {
                        "Executando o programa…"
                    } else {
                        "Analisando linha…"
                    }));
            }
            let idle = debugger_analysis_state == "idle";
            ui.workspace_ui()
                .debugger_learning
                .analysis_stack
                .set_visible(!idle);
            if !idle {
                ui.workspace_ui()
                    .debugger_learning
                    .analysis_stack
                    .set_visible_child_name(if debugger_analysis_state == "content" {
                        "content"
                    } else {
                        "loading"
                    });
            }
            ui.displayed_debugger_analysis_state = debugger_analysis_state;
        }
        let execution_active = self.active_execution_id.is_some();
        if ui.displayed_run_active != execution_active
            || ui.displayed_run_language != self.ui_language
        {
            if execution_active && !ui.displayed_run_active {
                ui.workspace_ui().code_view.completion().hide();
            }
            let (icon_name, label) = if execution_active && !self.debugging_active {
                ("potoo-stop-symbolic", "Parar")
            } else {
                ("potoo-play-symbolic", "Executar")
            };
            if execution_active && !self.debugging_active {
                ui.run_button.add_css_class("potoo-stop-button");
            } else {
                ui.run_button.remove_css_class("potoo-stop-button");
            }
            ui.run_icon.set_icon_name(Some(icon_name));
            ui.run_label.set_text(self.text(label));
            ui.run_button.set_tooltip_text(Some(self.text(
                if execution_active && !self.debugging_active {
                    "Parar"
                } else {
                    "Executar o programa atual"
                },
            )));
            ui.displayed_run_active = execution_active;
            ui.displayed_run_language = self.ui_language;
        }
        ui.run_button.set_sensitive(!self.debugging_active);
    }
}

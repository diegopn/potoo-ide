use super::*;

impl MainWindow {
    pub(super) fn sync_tool_panels_view(&self, widgets: &mut MainUi, view: &ViewContext<'_>) {
        let ui = widgets;
        let learning_available = view.learning_available;
        let debugger_available = view.debugger_available;
        let game_preview_available = view.game_preview_available;
        ui.game_preview_button.set_visible(game_preview_available);
        ui.learning_tool_button.set_visible(learning_available);
        ui.debugger_tool_button.set_visible(debugger_available);
        let right_panel_section = available_panel_section(
            self.right_panel_section,
            learning_available,
            debugger_available,
            game_preview_available,
        );
        ui.tools_panel.set_visible(right_panel_section.is_some());

        if let Some(section) = right_panel_section
            && (ui.displayed_right_panel_section != Some(section)
                || ui.displayed_right_panel_collapsed != self.right_panel_collapsed)
        {
            let collapsed = self.right_panel_collapsed;
            ui.right_panel_stack.set_visible_child_name(match section {
                RightPanelSection::Learning => "learning",
                RightPanelSection::Debugger => "debugger",
                RightPanelSection::GamePreview => "game-preview",
            });
            ui.right_panel_revealer.set_reveal_child(!collapsed);

            let learning_selected = section == RightPanelSection::Learning;
            let debugger_selected = section == RightPanelSection::Debugger;
            let game_preview_selected = section == RightPanelSection::GamePreview;
            ui.learning_tool_button
                .set_css_classes(if learning_selected {
                    &["potoo-tool-rail-button", "potoo-tool-rail-button-selected"]
                } else {
                    &["potoo-tool-rail-button"]
                });
            ui.debugger_tool_button
                .set_css_classes(if debugger_selected {
                    &["potoo-tool-rail-button", "potoo-tool-rail-button-selected"]
                } else {
                    &["potoo-tool-rail-button"]
                });
            ui.game_preview_button
                .set_css_classes(if game_preview_selected {
                    &["potoo-tool-rail-button", "potoo-tool-rail-button-selected"]
                } else {
                    &["potoo-tool-rail-button"]
                });
            ui.learning_tool_button.set_tooltip_text(Some(self.text(
                if learning_selected && !collapsed {
                    "Recolher Aprenda enquanto programa"
                } else {
                    "Abrir Aprenda enquanto programa"
                },
            )));
            ui.debugger_tool_button.set_tooltip_text(Some(self.text(
                if section == RightPanelSection::Debugger && !collapsed {
                    "Recolher depurador"
                } else {
                    "Abrir depurador"
                },
            )));
            ui.game_preview_button.set_tooltip_text(Some(self.text(
                if game_preview_selected && !collapsed {
                    "Recolher prévia do jogo"
                } else {
                    "Abrir prévia do jogo"
                },
            )));
            ui.displayed_right_panel_section = Some(section);
            ui.displayed_right_panel_collapsed = collapsed;
        } else if right_panel_section.is_none() {
            ui.right_panel_revealer.set_reveal_child(false);
            ui.displayed_right_panel_section = None;
            ui.displayed_right_panel_collapsed = self.right_panel_collapsed;
        }

        if ui.displayed_game_preview_active != self.game_preview_active {
            ui.game_preview_stack
                .set_visible_child_name(if self.game_preview_active {
                    "canvas"
                } else {
                    "idle"
                });
            ui.displayed_game_preview_active = self.game_preview_active;
        }
        ui.game_preview_open_button
            .set_sensitive(self.game_preview_active);
    }
    pub(super) fn sync_learning_panel_view(&self, widgets: &mut MainUi) {
        let ui = widgets;
        let current_learning_hint = if self.learning_loading {
            ui.displayed_learning_hint.clone()
        } else {
            self.learning_hint.clone()
        };
        if ui.displayed_learning_hint != current_learning_hint {
            if let Some(hint) = &current_learning_hint {
                let topic_name = learning_topic_label(hint.topic, self.ui_language);
                ui.learning_line_title.set_text(&format!(
                    "{} {} · {topic_name}",
                    self.text("Linha"),
                    hint.line
                ));
                let (explanation, concept, experiment) = learning_copy(hint, self.ui_language);
                ui.learning_explanation.set_text(&explanation);
                ui.learning_experiment.set_text(&experiment);
                let show_learning_concept = learning_concept_is_visible(hint.topic, &concept);
                ui.learning_concept_section
                    .set_visible(show_learning_concept);
                ui.learning_experiment_section
                    .set_visible(show_learning_concept && !experiment.is_empty());
                ui.learning_concept_title
                    .set_text(&format!("{} {topic_name}", self.text("Conceito:")));
                ui.learning_concept_text.set_text(&concept);
                content_panel::show_learning_problems(
                    &ui.learning_problem_list,
                    &ui.learning_problem_empty,
                    &hint.problems,
                    self.ui_language,
                    &ui.code_buffer,
                    ui.code_view.upcast_ref(),
                );
                ui.learning_problem_empty
                    .set_text(self.text("Nenhum problema conhecido nesta linha."));
                ui.learning_problem_empty.set_text(self.text(
                    if hint.topic == LearningTopic::Unrecognized {
                        "A análise não reconheceu esta construção; não foi possível verificar problemas."
                    } else {
                        "Nenhum problema conhecido nesta linha."
                    },
                ));
            } else {
                ui.learning_concept_section.set_visible(true);
                ui.learning_experiment_section.set_visible(true);
                ui.learning_line_title
                    .set_text(self.text("Linha 1 · programa"));
                ui.learning_explanation.set_text(
                    self.text(
                        "Selecione uma linha para ver uma explicação contextual do programa.",
                    ),
                );
                ui.learning_experiment.set_text(
                    self.text(
                        "Mova o cursor para uma condição, um laço, uma entrada ou uma saída.",
                    ),
                );
                ui.learning_concept_title
                    .set_text(self.text("Conceito: programa"));
                ui.learning_concept_text.set_text(
                    self.text(
                        "O painel acompanha a linha selecionada e o conteúdo atual do editor.",
                    ),
                );
                content_panel::show_learning_problems(
                    &ui.learning_problem_list,
                    &ui.learning_problem_empty,
                    &[],
                    self.ui_language,
                    &ui.code_buffer,
                    ui.code_view.upcast_ref(),
                );
            }
            ui.displayed_learning_hint
                .clone_from(&current_learning_hint);
        }
        if ui.displayed_learning_loading != self.learning_loading {
            ui.learning_spinner.set_spinning(self.learning_loading);
            ui.learning_spinner.set_visible(self.learning_loading);
            ui.learning_analysis_stack
                .set_visible_child_name(if self.learning_loading {
                    "loading"
                } else {
                    "content"
                });
            ui.displayed_learning_loading = self.learning_loading;
        }
    }
}

fn available_panel_section(
    requested: RightPanelSection,
    learning_available: bool,
    debugger_available: bool,
    game_preview_available: bool,
) -> Option<RightPanelSection> {
    match requested {
        RightPanelSection::Learning if !learning_available => {
            if debugger_available {
                Some(RightPanelSection::Debugger)
            } else if game_preview_available {
                Some(RightPanelSection::GamePreview)
            } else {
                None
            }
        }
        RightPanelSection::Debugger if !debugger_available => {
            if learning_available {
                Some(RightPanelSection::Learning)
            } else if game_preview_available {
                Some(RightPanelSection::GamePreview)
            } else {
                None
            }
        }
        RightPanelSection::GamePreview if !game_preview_available => {
            if learning_available {
                Some(RightPanelSection::Learning)
            } else if debugger_available {
                Some(RightPanelSection::Debugger)
            } else {
                None
            }
        }
        section => Some(section),
    }
}

pub(super) fn learning_concept_is_visible(topic: LearningTopic, concept: &str) -> bool {
    topic != LearningTopic::Unrecognized && !concept.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use super::{RightPanelSection, available_panel_section, learning_concept_is_visible};
    use crate::features::learning::LearningTopic;

    #[test]
    fn unavailable_panel_falls_back_to_an_available_feature() {
        assert_eq!(
            available_panel_section(RightPanelSection::Learning, false, true, false),
            Some(RightPanelSection::Debugger)
        );
        assert_eq!(
            available_panel_section(RightPanelSection::Debugger, false, false, true),
            Some(RightPanelSection::GamePreview)
        );
        assert_eq!(
            available_panel_section(RightPanelSection::GamePreview, false, true, false),
            Some(RightPanelSection::Debugger)
        );
    }

    #[test]
    fn returns_no_panel_when_all_features_are_unavailable() {
        for requested in [
            RightPanelSection::Learning,
            RightPanelSection::Debugger,
            RightPanelSection::GamePreview,
        ] {
            assert_eq!(
                available_panel_section(requested, false, false, false),
                None
            );
        }
    }

    #[test]
    fn hides_empty_concepts_for_structural_and_unverified_lines() {
        assert!(!learning_concept_is_visible(LearningTopic::Program, ""));
        assert!(!learning_concept_is_visible(
            LearningTopic::LibraryCall,
            "  "
        ));
        assert!(!learning_concept_is_visible(
            LearningTopic::Unrecognized,
            "concept"
        ));
        assert!(learning_concept_is_visible(
            LearningTopic::Decision,
            "condição"
        ));
    }
}

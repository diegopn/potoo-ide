use super::super::*;
use super::super::{dialogs::*, learning::*, settings::*};
use super::MainUi;

impl MainWindow {
    pub(super) fn update_view_component(
        &self,
        widgets: &mut MainUi,
        sender: ComponentSender<MainWindow>,
    ) {
        let ui = widgets;
        ui.project_root_for_new_items
            .replace(self.project_root.clone());
        ui.project_files_for_new_items
            .replace(self.project_files.clone());

        let active_document = self.active_document();
        let active_language = active_document.map(|document| document.language);
        let learning_available =
            active_language.map_or(true, crate::features::learning::is_available);
        let debugger_available =
            active_language.map_or(true, crate::features::debugging::is_available);
        ui.completion_provider.set_language(
            active_language
                .filter(|language| crate::languages::completion::is_available(*language)),
        );
        ui.completion_provider.set_ui_language(self.ui_language);
        let game_preview_available = active_language == Some(Language::Potigol);
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

        let active_document_id = active_document.map(|document| document.id);
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
                let show_learning_concept = hint.topic != LearningTopic::Unrecognized;
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
        let active_document_changed = ui.displayed_active_document_id != active_document_id;
        if active_document_changed {
            ui.code_view.completion().hide();
            ui.code_buffer.set_text(
                active_document
                    .map(|document| document.contents.as_str())
                    .unwrap_or_default(),
            );
            ui.displayed_active_document_id = active_document_id;
        }
        if active_document_changed || ui.displayed_execution_line != self.execution_line {
            if self.execution_document_id == self.active_document_id
                && let Some(line) = self.execution_line
                && let Some(iter) = ui.code_buffer.iter_at_line(line.saturating_sub(1) as i32)
            {
                ui.code_buffer.place_cursor(&iter);
            }
            ui.displayed_execution_line = self.execution_line;
        }
        if ui.displayed_language != active_language {
            ui.status_language
                .set_text(active_language.map(Language::label).unwrap_or(""));
            ui.status_language.set_visible(active_language.is_some());
            ui.displayed_language = active_language;
        }
        ui.default_new_language.set(self.new_file_language);

        if ui.displayed_indent_width != self.indent_width {
            ui.code_view.set_indent_width(self.indent_width as i32);
            ui.code_view.set_tab_width(self.indent_width);
            ui.settings
                .indent_width_picker
                .set_selected(if self.indent_width == 2 { 0 } else { 1 });
            ui.status_details.set_text(&format!(
                "UTF-8  ·  {}: {}",
                tr(self.ui_language, "Espaços"),
                self.indent_width
            ));
            ui.displayed_indent_width = self.indent_width;
        }

        let displayed_preferred_ui_language = self
            .pending_ui_language
            .unwrap_or(self.preferred_ui_language);
        if ui.displayed_preferred_ui_language != displayed_preferred_ui_language {
            ui.settings
                .ui_language_picker
                .set_selected(displayed_preferred_ui_language.selected_index());
            ui.displayed_preferred_ui_language = displayed_preferred_ui_language;
        }

        if ui.displayed_dark != self.dark {
            ui.dark_theme.set(self.dark);
            if self.dark {
                ui.window.add_css_class("potoo-dark");
                ui.settings.window.add_css_class("potoo-dark");
            } else {
                ui.window.remove_css_class("potoo-dark");
                ui.settings.window.remove_css_class("potoo-dark");
            }
            content_panel::set_syntax_palette(&ui.syntax_tags, self.dark);
            content_panel::install_diagnostic_markers(&ui.code_view, self.dark);
            ui.displayed_dark = self.dark;
        }

        if ui.displayed_theme != self.theme {
            select_theme_button(&ui.settings.theme_buttons, self.theme.selected_index());
            ui.displayed_theme = self.theme;
        }
        if ui.displayed_font_size != self.font_size {
            set_font_size(&ui.code_view, ui.displayed_font_size, self.font_size);
            ui.settings
                .font_size_picker
                .set_selected(font_size_index(self.font_size));
            ui.displayed_font_size = self.font_size;
        }
        if ui.displayed_line_numbers_visible != self.line_numbers_visible {
            ui.code_view
                .set_show_line_numbers(self.line_numbers_visible);
            ui.settings
                .line_numbers_switch
                .set_active(self.line_numbers_visible);
            ui.displayed_line_numbers_visible = self.line_numbers_visible;
        }
        if ui.displayed_word_wrap != self.word_wrap {
            ui.code_view.set_wrap_mode(if self.word_wrap {
                gtk::WrapMode::WordChar
            } else {
                gtk::WrapMode::None
            });
            ui.settings.word_wrap_switch.set_active(self.word_wrap);
            ui.displayed_word_wrap = self.word_wrap;
        }
        if ui.displayed_settings_visible != self.settings_visible {
            if self.settings_visible {
                ui.settings.window.present();
            } else {
                ui.settings.window.set_visible(false);
            }
            ui.displayed_settings_visible = self.settings_visible;
        }

        if ui.displayed_start_page != self.start_page_visible {
            if ui.displayed_start_page && !self.start_page_visible {
                ui.window.maximize();
            }
            ui.main_stack
                .set_visible_child_name(if self.start_page_visible {
                    "start"
                } else {
                    "workspace"
                });
            ui.run_button.set_visible(!self.start_page_visible);
            ui.project_title.set_text(if self.start_page_visible {
                tr(self.ui_language, "Início")
            } else {
                &self.project_name
            });
            ui.displayed_start_page = self.start_page_visible;
        }

        if ui.displayed_project_name != self.project_name {
            if !self.start_page_visible {
                ui.project_title.set_text(&self.project_name);
            }
            ui.project_label.set_text(&self.project_name);
            ui.displayed_project_name.clone_from(&self.project_name);
        }

        if ui.displayed_recent_paths != self.recent_paths {
            let recent_preview = &self.recent_paths[..self.recent_paths.len().min(2)];
            ui.start_page
                .recent_section
                .set_visible(!self.recent_paths.is_empty());
            start_page::rebuild_recent_items(
                &ui.start_page.recent_list,
                recent_preview,
                self.ui_language,
                sender.clone(),
            );
            start_page::rebuild_recent_items(
                &ui.start_page.all_recent_list,
                &self.recent_paths,
                self.ui_language,
                sender.clone(),
            );
            ui.start_page
                .recent_more_button
                .set_visible(self.recent_paths.len() > 2);
            ui.displayed_recent_paths.clone_from(&self.recent_paths);
        }

        let active_path = active_document.and_then(|document| document.path.as_ref());
        let project_file_signature = self
            .project_files
            .iter()
            .map(|file| (file.path.clone(), active_path == Some(&file.path)))
            .collect::<Vec<_>>();
        let project_open = self.project_root.is_some();
        if ui.displayed_project_files != project_file_signature
            || ui.displayed_project_open != project_open
        {
            ui.expanded_folders.borrow_mut().retain(|expanded| {
                self.project_files
                    .iter()
                    .any(|item| item.is_directory && item.path == *expanded)
            });
            sidebar::rebuild_file_list(
                &ui.file_list,
                &self.project_files,
                active_path,
                project_open,
                self.project_root.as_deref(),
                &ui.empty_state,
                self.ui_language,
                &ui.expanded_folders,
                sender.clone(),
            );
            ui.displayed_project_files = project_file_signature;
            ui.displayed_project_open = project_open;
            ui.project_row.set_visible(project_open);
            ui.new_file_button.set_sensitive(project_open);
        }

        let tab_signature = self
            .documents
            .iter()
            .map(|document| {
                (
                    document.id,
                    document.name.clone(),
                    document.is_dirty(),
                    Some(document.id) == self.active_document_id,
                )
            })
            .collect::<Vec<_>>();
        if ui.displayed_tabs != tab_signature {
            content_panel::rebuild_editor_tabs(
                &ui.editor_tabs,
                &self.documents,
                self.active_document_id,
                self.ui_language,
                sender.clone(),
            );
            ui.displayed_tabs = tab_signature;
        }

        if ui.displayed_pending_project != self.pending_project {
            ui.displayed_pending_project
                .clone_from(&self.pending_project);
            if self.pending_project.is_some() {
                show_project_switch_confirmation(
                    &ui.window,
                    self.ui_language,
                    ui.dark_theme.get(),
                    sender.clone(),
                );
            }
        }

        if ui.displayed_pending_close_document_id != self.pending_close_document_id {
            ui.displayed_pending_close_document_id = self.pending_close_document_id;
            if let Some(id) = self.pending_close_document_id {
                show_close_document_confirmation(
                    &ui.window,
                    id,
                    self.ui_language,
                    ui.dark_theme.get(),
                    sender.clone(),
                );
            }
        }

        if ui.displayed_pending_close_app != self.pending_close_app {
            ui.displayed_pending_close_app = self.pending_close_app;
            if self.pending_close_app {
                show_close_app_confirmation(
                    &ui.window,
                    self.ui_language,
                    ui.dark_theme.get(),
                    sender.clone(),
                );
            }
        }

        if ui.displayed_pending_save_as_id != self.pending_save_as_id {
            ui.displayed_pending_save_as_id = self.pending_save_as_id;
            if let Some(id) = self.pending_save_as_id {
                if let Some(document) = self.documents.iter().find(|doc| doc.id == id) {
                    show_save_dialog(
                        &ui.window,
                        document,
                        self.project_root.as_deref(),
                        self.ui_language,
                        sender.clone(),
                    );
                }
            }
        }

        if self.close_app_ready && !ui.displayed_close_app_ready {
            ui.displayed_close_app_ready = true;
            self.graphics_host.close_preview_window();
            ui.close_authorized.set(true);
            ui.window.close();
        }

        if ui.displayed_output_text != self.output_text {
            ui.output_buffer.set_text(&self.output_text);
            if self.output_text == self.text("A saída aparecerá aqui após a execução.") {
                ui.output_view
                    .add_css_class("potoo-output-placeholder-text");
            } else {
                ui.output_view
                    .remove_css_class("potoo-output-placeholder-text");
            }
            ui.displayed_output_text.clone_from(&self.output_text);
        }
        if ui.displayed_output_status != self.output_status {
            ui.output_status.set_text(&self.output_status);
            ui.displayed_output_status.clone_from(&self.output_status);
        }
        let editor_diagnostics = self.all_diagnostics();
        let diagnostics_changed = ui.displayed_diagnostics != editor_diagnostics;
        if diagnostics_changed {
            content_panel::show_diagnostics(
                &ui.problems_list,
                &editor_diagnostics,
                self.ui_language,
                sender.clone(),
            );
            ui.displayed_diagnostics.clone_from(&editor_diagnostics);
        }
        if ui.displayed_execution_diagnostics_document_id != self.execution_diagnostics_document_id
        {
            if self.execution_diagnostics_document_id.is_some()
                && self.execution_diagnostics_document_id == self.active_document_id
            {
                ui.problems_tab.emit_clicked();
            }
            ui.displayed_execution_diagnostics_document_id = self.execution_diagnostics_document_id;
        }
        if ui.displayed_input_requested != self.input_requested {
            ui.input_entry.set_visible(self.input_requested);
            if self.input_requested {
                if let Some(prompt) = &self.input_prompt {
                    let placeholder = match (&prompt.variable_name, &prompt.data_type) {
                        (Some(name), Some(data_type)) => format!("{} ({data_type})", name),
                        _ => self.text("Digite a entrada do programa").to_owned(),
                    };
                    ui.input_entry.set_placeholder_text(Some(&placeholder));
                }
                ui.input_entry.grab_focus();
            } else {
                ui.input_entry.set_text("");
            }
            ui.displayed_input_requested = self.input_requested;
        }
        ui.debugger_controls.set_visible(debugger_available);
        ui.debugger_controls
            .set_visible_child_name(if self.debugging_active {
                "controls"
            } else {
                "start"
            });
        ui.debugger_start.set_tooltip_text(Some(self.text(
            if self.active_execution_id.is_some() {
                "Interrompa a execução atual antes de iniciar a depuração."
            } else {
                "Iniciar depuração"
            },
        )));
        ui.debugger_start
            .set_sensitive(self.active_execution_id.is_none());
        ui.debugger_step.set_sensitive(
            self.debugging_active
                && self.active_execution_id.is_some()
                && self.debugger_snapshot.is_some(),
        );
        ui.debugger_stop.set_visible(self.debugging_active);
        ui.debugger_stop
            .set_sensitive(self.debugging_active && self.active_execution_id.is_some());
        let debugger_error = self.debugger_error
            && self.active_document_id.is_some()
            && self.execution_diagnostics_document_id == self.active_document_id;
        if ui.displayed_debugger_snapshot != self.debugger_snapshot
            || ui.displayed_debugger_error != debugger_error
        {
            if let Some(snapshot) = &self.debugger_snapshot {
                let reason = match snapshot.pause_reason {
                    Some(execution::DebugPauseReason::Step) => self.text("passo a passo"),
                    Some(execution::DebugPauseReason::Pause) => self.text("pausa"),
                    Some(execution::DebugPauseReason::Debug) => self.text("ponto de depuração"),
                    None => self.text("depuração"),
                };
                ui.debugger_position.set_text(&format!(
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
                ui.debugger_variables.set_text(&variables);
                if self.execution_document_id == self.active_document_id
                    && let Some(iter) = ui
                        .code_buffer
                        .iter_at_line(snapshot.line.saturating_sub(1) as i32)
                {
                    ui.code_buffer.place_cursor(&iter);
                }
            } else if debugger_error {
                ui.debugger_position
                    .set_text(self.text("Depuração encerrada com erro"));
                ui.debugger_variables.set_text(&self.problem_text);
            } else {
                ui.debugger_position
                    .set_text(self.text("Aguardando depuração"));
                ui.debugger_variables.set_text(
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
            ui.debugger_learning
                .content_scroll
                .vadjustment()
                .set_value(0.0);
            if let Some(hint) = &current_debugger_hint {
                let topic_name = learning_topic_label(hint.topic, self.ui_language);
                ui.debugger_learning.line_title.set_text(&format!(
                    "{} {} · {topic_name}",
                    self.text("Linha"),
                    hint.line
                ));
                let (explanation, concept, _) = learning_copy(hint, self.ui_language);
                ui.debugger_learning.explanation.set_text(&explanation);
                let show_learning_concept = hint.topic != LearningTopic::Unrecognized;
                ui.debugger_learning
                    .concept_section
                    .set_visible(show_learning_concept);
                ui.debugger_learning
                    .concept_title
                    .set_text(&format!("{} {topic_name}", self.text("Conceito:")));
                ui.debugger_learning.concept_text.set_text(&concept);
                content_panel::show_learning_problems(
                    &ui.debugger_learning.problem_list,
                    &ui.debugger_learning.problem_empty,
                    &hint.problems,
                    self.ui_language,
                    &ui.code_buffer,
                    ui.code_view.upcast_ref(),
                );
                ui.debugger_learning
                    .problem_empty
                    .set_text(self.text("Nenhum problema conhecido nesta linha."));
                ui.debugger_learning.problem_empty.set_text(self.text(
                    if hint.topic == LearningTopic::Unrecognized {
                        "A análise não reconheceu esta construção; não foi possível verificar problemas."
                    } else {
                        "Nenhum problema conhecido nesta linha."
                    },
                ));
            } else {
                ui.debugger_learning.concept_section.set_visible(false);
                ui.debugger_learning
                    .line_title
                    .set_text(self.text("Linha 1 · programa"));
                ui.debugger_learning.explanation.set_text("");
                ui.debugger_learning
                    .concept_title
                    .set_text(self.text("Conceito: programa"));
                ui.debugger_learning.concept_text.set_text("");
                content_panel::show_learning_problems(
                    &ui.debugger_learning.problem_list,
                    &ui.debugger_learning.problem_empty,
                    &[],
                    self.ui_language,
                    &ui.code_buffer,
                    ui.code_view.upcast_ref(),
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
            ui.debugger_learning.spinner.set_spinning(loading);
            ui.debugger_learning.spinner.set_visible(loading);
            if loading {
                ui.debugger_learning.loading_label.set_text(self.text(
                    if debugger_analysis_state == "waiting" {
                        "Executando o programa…"
                    } else {
                        "Analisando linha…"
                    },
                ));
            }
            let idle = debugger_analysis_state == "idle";
            ui.debugger_learning.analysis_stack.set_visible(!idle);
            if !idle {
                ui.debugger_learning.analysis_stack.set_visible_child_name(
                    if debugger_analysis_state == "content" {
                        "content"
                    } else {
                        "loading"
                    },
                );
            }
            ui.displayed_debugger_analysis_state = debugger_analysis_state;
        }
        let execution_active = self.active_execution_id.is_some();
        if ui.displayed_run_active != execution_active
            || ui.displayed_run_language != self.ui_language
        {
            if execution_active && !ui.displayed_run_active {
                ui.code_view.completion().hide();
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

        if let Some(target) = self.navigation_target
            && target.serial != ui.displayed_navigation_serial
            && self.active_document_id == Some(target.document_id)
        {
            let mut iter = ui.code_buffer.iter_at_offset(
                target
                    .offset
                    .min(ui.code_buffer.char_count().max(0) as usize) as i32,
            );
            ui.code_buffer.place_cursor(&iter);
            ui.code_view.scroll_to_iter(&mut iter, 0.2, false, 0.0, 0.0);
            ui.code_view.grab_focus();
            ui.displayed_navigation_serial = target.serial;
        }

        content_panel::highlight_buffer(
            &ui.code_buffer,
            active_language.unwrap_or(self.new_file_language),
            &ui.syntax_tags,
        );
        if active_document_changed || diagnostics_changed {
            let active_diagnostics = editor_diagnostics
                .iter()
                .filter(|diagnostic| Some(diagnostic.document_id) == self.active_document_id)
                .cloned()
                .collect::<Vec<_>>();
            content_panel::apply_diagnostics(
                &ui.code_view,
                &ui.code_buffer,
                &active_diagnostics,
                &ui.syntax_tags,
                self.dark,
            );
        }
        update_cursor_position(&ui.code_buffer, &ui.status_position, self.ui_language);
        ui.status_position.set_visible(active_document.is_some());
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

#[cfg(test)]
mod tests {
    use super::{RightPanelSection, available_panel_section};

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
}

use super::*;

impl MainWindow {
    pub(super) fn sync_settings_view(&self, widgets: &mut MainUi) {
        let ui = widgets;
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
    }
}

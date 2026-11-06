use super::*;

impl MainWindow {
    pub(super) fn handle_preferences(
        &mut self,
        message: MainWindowMsg,
        sender: ComponentSender<Self>,
    ) {
        match message {
            MainWindowMsg::InterfaceLanguageSelected(language) => {
                if self.ui_language != language
                    && self.preferred_ui_language != language
                    && self.pending_ui_language.is_none()
                    && self.pending_save_as_id.is_none()
                    && self.pending_close_after_save_id.is_none()
                {
                    self.pending_ui_language = Some(language);
                    self.reopen_settings_after_language_restart = self.settings_visible;
                    self.settings_visible = false;
                    self.finish_language_change(sender.clone());
                }
            }
            MainWindowMsg::ThemeSelected(theme) => {
                self.theme = theme;
                self.dark = theme_is_dark(theme);
                self.save_preferences();
            }
            MainWindowMsg::SystemThemeChanged(dark) => {
                if self.theme == ThemePreference::System {
                    self.dark = dark;
                }
            }
            MainWindowMsg::FontSizeSelected(font_size) => {
                self.font_size = font_size;
                self.save_preferences();
            }
            MainWindowMsg::LineNumbersChanged(visible) => {
                self.line_numbers_visible = visible;
                self.save_preferences();
            }
            MainWindowMsg::WordWrapChanged(enabled) => {
                self.word_wrap = enabled;
                self.save_preferences();
            }
            MainWindowMsg::IndentWidthSelected(width) => {
                self.indent_width = if width == 2 { 2 } else { 4 };
                self.save_preferences();
            }
            MainWindowMsg::OpenSettings => self.settings_visible = true,
            MainWindowMsg::CloseSettings => self.settings_visible = false,
            MainWindowMsg::SelectRightPanel(section) => {
                if self.right_panel_collapsed {
                    self.right_panel_section = section;
                    self.right_panel_collapsed = false;
                } else if self.right_panel_section == section {
                    self.right_panel_collapsed = true;
                } else {
                    self.right_panel_section = section;
                }
            }
            _ => unreachable!("update message routed to the wrong domain"),
        }
    }
}

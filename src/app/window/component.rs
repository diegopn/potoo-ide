mod init;
mod update;
mod view;
mod workspace;

use self::workspace::WorkspaceUi;
use super::settings::SettingsUi;
use super::*;

pub struct MainUi {
    pub(super) window: gtk::ApplicationWindow,
    pub(super) content: gtk::Box,
    pub(super) main_stack: gtk::Stack,
    pub(super) dark_theme: Rc<Cell<bool>>,
    pub(super) default_new_language: Rc<Cell<Language>>,
    pub(super) project_root_for_new_items: Rc<RefCell<Option<PathBuf>>>,
    pub(super) project_files_for_new_items: Rc<RefCell<Vec<ProjectFile>>>,
    pub(super) settings: SettingsUi,
    pub(super) project_title: gtk::Label,
    pub(super) run_button: gtk::Button,
    pub(super) run_icon: gtk::Image,
    pub(super) run_label: gtk::Label,
    pub(super) start_page: start_page::StartPageUi,
    pub(super) project_row: gtk::Box,
    pub(super) project_label: gtk::Label,
    pub(super) file_list: gtk::Box,
    pub(super) new_file_button: gtk::MenuButton,
    pub(super) empty_state: gtk::Widget,
    pub(super) expanded_folders: sidebar::ExpandedFolders,
    pub(super) sidebar_widget: gtk::Widget,
    pub(super) workspace_host: gtk::Box,
    pub(super) workspace: Option<WorkspaceUi>,
    pub(super) status_language: gtk::Label,
    pub(super) status_details: gtk::Label,
    pub(super) status_position: gtk::Label,
    pub(super) displayed_language: Option<Language>,
    pub(super) displayed_active_document_id: Option<u64>,
    pub(super) displayed_project_name: String,
    pub(super) displayed_project_files: Vec<(PathBuf, bool)>,
    pub(super) displayed_project_open: bool,
    pub(super) displayed_recent_paths: Vec<PathBuf>,
    pub(super) displayed_start_page: bool,
    pub(super) displayed_run_active: bool,
    pub(super) displayed_run_language: UiLanguage,
    pub(super) displayed_tabs: Vec<(u64, String, bool, bool)>,
    pub(super) displayed_pending_project: Option<PathBuf>,
    pub(super) displayed_pending_close_document_id: Option<u64>,
    pub(super) displayed_pending_close_app: bool,
    pub(super) displayed_pending_save_as_id: Option<u64>,
    pub(super) displayed_close_app_ready: bool,
    pub(super) close_authorized: Rc<Cell<bool>>,
    pub(super) displayed_theme: ThemePreference,
    pub(super) displayed_dark: bool,
    pub(super) displayed_font_size: u32,
    pub(super) displayed_line_numbers_visible: bool,
    pub(super) displayed_word_wrap: bool,
    pub(super) displayed_indent_width: u32,
    pub(super) displayed_preferred_ui_language: UiLanguage,
    pub(super) displayed_settings_visible: bool,
    pub(super) displayed_output_text: String,
    pub(super) displayed_diagnostics: Vec<EditorDiagnostic>,
    pub(super) displayed_execution_diagnostics_document_id: Option<u64>,
    pub(super) displayed_navigation_serial: u64,
    pub(super) displayed_input_requested: bool,
    pub(super) displayed_output_status: String,
    pub(super) displayed_debugger_snapshot: Option<DebuggerSnapshot>,
    pub(super) displayed_debugger_error: bool,
    pub(super) displayed_execution_line: Option<usize>,
    pub(super) displayed_debugger_learning_hint: Option<LearningHint>,
    pub(super) displayed_debugger_analysis_state: &'static str,
    pub(super) displayed_learning_hint: Option<LearningHint>,
    pub(super) displayed_learning_loading: bool,
    pub(super) displayed_right_panel_section: Option<RightPanelSection>,
    pub(super) displayed_right_panel_collapsed: bool,
    pub(super) displayed_game_preview_active: bool,
}

impl MainUi {
    pub(super) fn workspace_ui(&self) -> &WorkspaceUi {
        self.workspace
            .as_ref()
            .expect("workspace widgets must exist before workspace view synchronization")
    }
}

impl SimpleComponent for MainWindow {
    type Init = ();
    type Input = MainWindowMsg;
    type Output = ();
    type Root = gtk::ApplicationWindow;
    type Widgets = MainUi;

    fn init_root() -> Self::Root {
        gtk::ApplicationWindow::default()
    }

    fn init(
        init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        init::init(init, root, sender)
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        self.update_component(message, sender);
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        self.update_view_component(widgets, sender);
    }
}

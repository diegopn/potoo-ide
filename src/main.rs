use gtk::prelude::*;
use relm4::prelude::*;
use std::{
    cell::Cell,
    fs,
    fs::OpenOptions,
    path::{Path, PathBuf},
    rc::Rc,
};

const PORTUGOL_SAMPLE: &str = "programa {\n  funcao inicio() {\n    inteiro idade\n\n    escreva(\"Qual é sua idade? \")\n    leia(idade)\n    se (idade >= 18) {\n      escreva(\"Pode votar.\")\n    } senao {\n      escreva(\"Ainda não pode votar.\")\n    }\n  }\n}";

const VISUALG_SAMPLE: &str = "algoritmo \"votacao\"\nvar\n  idade: inteiro\ninicio\n  escreva(\"Qual é sua idade? \")\n  leia(idade)\n  se idade >= 18 entao\n    escreval(\"Pode votar.\")\n  senao\n    escreval(\"Ainda não pode votar.\")\n  fimse\nfimalgoritmo";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Language {
    Portugol,
    Visualg,
}

impl Language {
    fn label(self) -> &'static str {
        match self {
            Self::Portugol => "Portugol",
            Self::Visualg => "VisuAlg",
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            Self::Portugol => "principal.por",
            Self::Visualg => "principal.alg",
        }
    }

    fn sample(self) -> &'static str {
        match self {
            Self::Portugol => PORTUGOL_SAMPLE,
            Self::Visualg => VISUALG_SAMPLE,
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Self::Portugol => "por",
            Self::Visualg => "alg",
        }
    }

    fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "por" => Some(Self::Portugol),
            "alg" => Some(Self::Visualg),
            _ => None,
        }
    }

    fn keywords(self) -> &'static [&'static str] {
        match self {
            Self::Portugol => &[
                "programa", "funcao", "inicio", "inteiro", "escreva", "leia", "se", "senao",
            ],
            Self::Visualg => &[
                "algoritmo",
                "var",
                "inteiro",
                "inicio",
                "escreva",
                "escreval",
                "leia",
                "se",
                "entao",
                "senao",
                "fimse",
                "fimalgoritmo",
            ],
        }
    }
}

#[derive(Clone, Debug)]
struct SourceDocument {
    id: u64,
    path: Option<PathBuf>,
    name: String,
    language: Language,
    contents: String,
    saved_contents: String,
}

impl SourceDocument {
    fn is_dirty(&self) -> bool {
        self.contents != self.saved_contents
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectFile {
    path: PathBuf,
    relative_path: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ThemePreference {
    System,
    Light,
    Dark,
}

impl ThemePreference {
    fn selected_index(self) -> u32 {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }

    fn from_selected_index(index: u32) -> Self {
        match index {
            1 => Self::Light,
            2 => Self::Dark,
            _ => Self::System,
        }
    }

    fn color_scheme(self) -> adw::ColorScheme {
        match self {
            Self::System => adw::ColorScheme::Default,
            Self::Light => adw::ColorScheme::ForceLight,
            Self::Dark => adw::ColorScheme::ForceDark,
        }
    }
}

#[derive(Clone)]
struct SyntaxTags {
    keyword: gtk::TextTag,
    string: gtk::TextTag,
    number: gtk::TextTag,
    current_line: gtk::TextTag,
}

#[derive(Debug)]
enum MainWindowMsg {
    LanguageSelected(Language),
    ProjectSelected(PathBuf),
    ProjectSwitchDecision(bool),
    CreateFile(String, Language),
    OpenFile(PathBuf),
    ActivateDocument(u64),
    CloseDocument(u64),
    CloseDocumentDecision(u64, bool),
    SaveCurrent,
    SaveAsSelected(u64, PathBuf),
    SaveAsCanceled(u64),
    FileOperationFailed(String),
    RequestClose,
    CloseAppDecision(bool),
    ThemeSelected(ThemePreference),
    SystemThemeChanged(bool),
    FontSizeSelected(u32),
    LineNumbersChanged(bool),
    WordWrapChanged(bool),
    OpenSettings,
    CloseSettings,
    Run,
    SourceChanged(String),
}

struct MainWindow {
    new_file_language: Language,
    documents: Vec<SourceDocument>,
    active_document_id: Option<u64>,
    next_document_id: u64,
    project_root: Option<PathBuf>,
    project_name: String,
    project_files: Vec<ProjectFile>,
    pending_project: Option<PathBuf>,
    pending_close_document_id: Option<u64>,
    pending_save_as_id: Option<u64>,
    pending_close_app: bool,
    close_app_ready: bool,
    theme: ThemePreference,
    dark: bool,
    font_size: u32,
    line_numbers_visible: bool,
    word_wrap: bool,
    settings_visible: bool,
    output_text: String,
    output_status: String,
}

impl MainWindow {
    fn active_document(&self) -> Option<&SourceDocument> {
        self.documents
            .iter()
            .find(|document| Some(document.id) == self.active_document_id)
    }

    fn create_document(
        &mut self,
        path: Option<PathBuf>,
        name: String,
        language: Language,
        contents: String,
        saved_contents: String,
    ) -> u64 {
        let id = self.next_document_id;
        self.next_document_id += 1;
        self.documents.push(SourceDocument {
            id,
            path,
            name,
            language,
            contents,
            saved_contents,
        });
        self.active_document_id = Some(id);
        id
    }

    fn open_document(&mut self, path: PathBuf) {
        if let Some(document) = self
            .documents
            .iter()
            .find(|doc| doc.path.as_ref() == Some(&path))
        {
            self.active_document_id = Some(document.id);
            self.new_file_language = document.language;
            return;
        }

        let Some(language) = Language::from_path(&path) else {
            self.output_status = "Abra um arquivo .por ou .alg.".to_string();
            return;
        };
        match fs::read_to_string(&path) {
            Ok(contents) => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.create_document(Some(path), name, language, contents.clone(), contents);
                self.new_file_language = language;
                self.output_status = "Arquivo aberto".to_string();
            }
            Err(error) => {
                self.output_status = format!("Não foi possível abrir o arquivo: {error}");
            }
        }
    }

    fn load_project(&mut self, root: PathBuf) {
        let files = match scan_project_files(&root) {
            Ok(files) => files,
            Err(error) => {
                self.output_status = format!("Não foi possível abrir a pasta: {error}");
                return;
            }
        };

        self.project_name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.display().to_string());
        self.project_root = Some(root);
        self.project_files = files;
        self.documents.clear();
        self.active_document_id = None;
        if let Some(path) = self.project_files.first().map(|file| file.path.clone()) {
            self.open_document(path);
        } else {
            self.output_status =
                "Pasta aberta. Crie um arquivo .por ou .alg para começar.".to_string();
        }
    }

    fn save_document(&mut self, id: u64, path: PathBuf) {
        let Some(index) = self.documents.iter().position(|doc| doc.id == id) else {
            return;
        };
        let (path, language) = match supported_save_path(path, self.documents[index].language) {
            Ok(path) => path,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };
        match fs::write(&path, self.documents[index].contents.as_bytes()) {
            Ok(()) => {
                let document = &mut self.documents[index];
                document.path = Some(path.clone());
                document.language = language;
                document.name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                document.saved_contents.clone_from(&document.contents);
                if self.project_root.is_none() {
                    self.project_root = path.parent().map(Path::to_path_buf);
                    if let Some(root) = &self.project_root {
                        self.project_name = root
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| root.display().to_string());
                    }
                }
                if let Some(root) = &self.project_root {
                    if let Ok(files) = scan_project_files(root) {
                        self.project_files = files;
                    }
                }
                self.output_status = "Arquivo salvo".to_string();
            }
            Err(error) => {
                self.output_status = format!("Não foi possível salvar o arquivo: {error}");
            }
        }
    }

    fn create_project_file(&mut self, name: String, language: Language) {
        let Some(root) = self.project_root.clone() else {
            self.output_status = "Abra uma pasta de projeto antes de criar arquivos.".to_string();
            return;
        };
        let (path, language) = match new_source_path(&root, &name, language) {
            Ok(source) => source,
            Err(error) => {
                self.output_status = error;
                return;
            }
        };

        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => {
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| path.display().to_string());
                self.create_document(
                    Some(path.clone()),
                    name,
                    language,
                    String::new(),
                    String::new(),
                );
                self.new_file_language = language;
                if let Ok(files) = scan_project_files(&root) {
                    self.project_files = files;
                }
                self.output_status = "Novo arquivo criado".to_string();
            }
            Err(error) => {
                self.output_status = if error.kind() == std::io::ErrorKind::AlreadyExists {
                    "Já existe um arquivo com esse nome.".to_string()
                } else {
                    format!("Não foi possível criar o arquivo: {error}")
                };
            }
        }
    }

    fn close_document(&mut self, id: u64) {
        let Some(index) = self.documents.iter().position(|document| document.id == id) else {
            return;
        };
        self.documents.remove(index);
        if self.active_document_id == Some(id) {
            self.active_document_id = self
                .documents
                .get(index)
                .or_else(|| {
                    index
                        .checked_sub(1)
                        .and_then(|previous| self.documents.get(previous))
                })
                .map(|document| document.id);
        }
        if let Some(document) = self.active_document() {
            self.new_file_language = document.language;
        }
    }
}

struct SettingsUi {
    window: gtk::Window,
    theme_picker: gtk::DropDown,
    font_size_picker: gtk::DropDown,
    line_numbers_switch: gtk::Switch,
    word_wrap_switch: gtk::Switch,
}

struct MainUi {
    window: gtk::ApplicationWindow,
    content: gtk::Box,
    language_picker: gtk::DropDown,
    settings: SettingsUi,
    project_title: gtk::Label,
    project_label: gtk::Label,
    file_list: gtk::Box,
    new_file_button: gtk::Button,
    editor_tabs: gtk::Box,
    status_language: gtk::Label,
    code_buffer: gtk::TextBuffer,
    code_view: gtk::TextView,
    syntax_tags: SyntaxTags,
    line_numbers: gtk::Label,
    line_numbers_separator: gtk::Separator,
    output_buffer: gtk::TextBuffer,
    output_status: gtk::Label,
    displayed_language: Option<Language>,
    displayed_new_file_language: Language,
    displayed_active_document_id: Option<u64>,
    displayed_project_name: String,
    displayed_project_files: Vec<(PathBuf, bool)>,
    displayed_project_open: bool,
    displayed_tabs: Vec<(u64, String, bool, bool)>,
    displayed_pending_project: Option<PathBuf>,
    displayed_pending_close_document_id: Option<u64>,
    displayed_pending_close_app: bool,
    displayed_pending_save_as_id: Option<u64>,
    displayed_close_app_ready: bool,
    close_authorized: Rc<Cell<bool>>,
    displayed_theme: ThemePreference,
    displayed_dark: bool,
    displayed_font_size: u32,
    displayed_line_numbers_visible: bool,
    displayed_word_wrap: bool,
    displayed_settings_visible: bool,
    displayed_output_text: String,
    displayed_output_status: String,
}

fn main() {
    let application = adw::Application::builder()
        .application_id("io.github.diegopn.potooide")
        .build();

    application.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("../data/potoo.css"));
        if let Some(display) = gtk::gdk::Display::default() {
            gtk::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
    });

    RelmApp::from_app(application).run::<MainWindow>(());
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
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.set_title(Some("Potoo IDE"));
        root.set_default_size(1440, 900);
        root.add_css_class("potoo-window");
        let style_manager = adw::StyleManager::default();
        style_manager.set_color_scheme(ThemePreference::System.color_scheme());
        let sender_for_system_theme = sender.clone();
        style_manager.connect_dark_notify(move |manager| {
            sender_for_system_theme.input(MainWindowMsg::SystemThemeChanged(manager.is_dark()));
        });

        let model = Self {
            new_file_language: Language::Portugol,
            documents: vec![SourceDocument {
                id: 1,
                path: None,
                name: Language::Portugol.file_name().to_string(),
                language: Language::Portugol,
                contents: Language::Portugol.sample().to_string(),
                saved_contents: Language::Portugol.sample().to_string(),
            }],
            active_document_id: Some(1),
            next_document_id: 2,
            project_root: None,
            project_name: "Meu primeiro programa".to_string(),
            project_files: Vec::new(),
            pending_project: None,
            pending_close_document_id: None,
            pending_save_as_id: None,
            pending_close_app: false,
            close_app_ready: false,
            theme: ThemePreference::System,
            dark: style_manager.is_dark(),
            font_size: 13,
            line_numbers_visible: true,
            word_wrap: false,
            settings_visible: false,
            output_text: "A saída aparecerá aqui após a execução.".to_string(),
            output_status: "Pronto para executar".to_string(),
        };
        let widgets = build_window_content(root.clone(), sender);
        root.set_child(Some(&widgets.content));

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            MainWindowMsg::LanguageSelected(language) => self.new_file_language = language,
            MainWindowMsg::ProjectSelected(path) => {
                if self.documents.iter().any(SourceDocument::is_dirty) {
                    self.pending_project = Some(path);
                } else {
                    self.load_project(path);
                }
            }
            MainWindowMsg::ProjectSwitchDecision(discard_changes) => {
                if discard_changes {
                    if let Some(path) = self.pending_project.take() {
                        self.load_project(path);
                    }
                } else {
                    self.pending_project = None;
                }
            }
            MainWindowMsg::CreateFile(name, language) => self.create_project_file(name, language),
            MainWindowMsg::OpenFile(path) => self.open_document(path),
            MainWindowMsg::ActivateDocument(id) => {
                if let Some(document) = self.documents.iter().find(|doc| doc.id == id) {
                    self.active_document_id = Some(id);
                    self.new_file_language = document.language;
                }
            }
            MainWindowMsg::CloseDocument(id) => {
                if self
                    .documents
                    .iter()
                    .find(|document| document.id == id)
                    .is_some_and(SourceDocument::is_dirty)
                {
                    self.pending_close_document_id = Some(id);
                } else {
                    self.close_document(id);
                }
            }
            MainWindowMsg::CloseDocumentDecision(id, discard_changes) => {
                if self.pending_close_document_id == Some(id) {
                    self.pending_close_document_id = None;
                    if discard_changes {
                        self.close_document(id);
                    }
                }
            }
            MainWindowMsg::SaveCurrent => {
                if let Some(document) = self.active_document() {
                    if let Some(path) = document.path.clone() {
                        self.save_document(document.id, path);
                    } else {
                        self.pending_save_as_id = Some(document.id);
                    }
                } else {
                    self.output_status = "Crie ou abra um arquivo antes de salvar.".to_string();
                }
            }
            MainWindowMsg::SaveAsSelected(id, path) => {
                self.pending_save_as_id = None;
                self.save_document(id, path);
            }
            MainWindowMsg::SaveAsCanceled(id) => {
                if self.pending_save_as_id == Some(id) {
                    self.pending_save_as_id = None;
                }
            }
            MainWindowMsg::FileOperationFailed(message) => self.output_status = message,
            MainWindowMsg::RequestClose => {
                if self.documents.iter().any(SourceDocument::is_dirty) {
                    self.pending_close_app = true;
                } else {
                    self.close_app_ready = true;
                }
            }
            MainWindowMsg::CloseAppDecision(discard_changes) => {
                self.pending_close_app = false;
                self.close_app_ready = discard_changes;
            }
            MainWindowMsg::ThemeSelected(theme) => {
                self.theme = theme;
                let style_manager = adw::StyleManager::default();
                style_manager.set_color_scheme(theme.color_scheme());
                self.dark = match theme {
                    ThemePreference::System => style_manager.is_dark(),
                    ThemePreference::Light => false,
                    ThemePreference::Dark => true,
                };
            }
            MainWindowMsg::SystemThemeChanged(dark) => {
                if self.theme == ThemePreference::System {
                    self.dark = dark;
                }
            }
            MainWindowMsg::FontSizeSelected(font_size) => self.font_size = font_size,
            MainWindowMsg::LineNumbersChanged(visible) => self.line_numbers_visible = visible,
            MainWindowMsg::WordWrapChanged(enabled) => self.word_wrap = enabled,
            MainWindowMsg::OpenSettings => self.settings_visible = true,
            MainWindowMsg::CloseSettings => self.settings_visible = false,
            MainWindowMsg::Run => {
                self.output_text = "A execução real depende da integração com o interpretador oficial selecionado.".to_string();
                self.output_status = "Interpretador ainda não conectado nesta prévia".to_string();
            }
            MainWindowMsg::SourceChanged(contents) => {
                if let Some(id) = self.active_document_id {
                    if let Some(document) = self.documents.iter_mut().find(|doc| doc.id == id) {
                        document.contents = contents;
                    }
                }
            }
        }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        let ui = widgets;

        let active_document = self.active_document();
        let active_document_id = active_document.map(|document| document.id);
        let active_language = active_document.map(|document| document.language);
        if ui.displayed_active_document_id != active_document_id {
            ui.code_buffer.set_text(
                active_document
                    .map(|document| document.contents.as_str())
                    .unwrap_or_default(),
            );
            ui.displayed_active_document_id = active_document_id;
        }
        if ui.displayed_language != active_language {
            ui.status_language
                .set_text(active_language.map(Language::label).unwrap_or(""));
            ui.status_language.set_visible(active_language.is_some());
            ui.displayed_language = active_language;
        }
        if ui.displayed_new_file_language != self.new_file_language {
            ui.language_picker
                .set_selected(match self.new_file_language {
                    Language::Portugol => 0,
                    Language::Visualg => 1,
                });
            ui.displayed_new_file_language = self.new_file_language;
        }

        if ui.displayed_dark != self.dark {
            if self.dark {
                ui.window.add_css_class("potoo-dark");
                ui.settings.window.add_css_class("potoo-dark");
            } else {
                ui.window.remove_css_class("potoo-dark");
                ui.settings.window.remove_css_class("potoo-dark");
            }
            set_syntax_palette(&ui.syntax_tags, self.dark);
            ui.displayed_dark = self.dark;
        }

        if ui.displayed_theme != self.theme {
            ui.settings
                .theme_picker
                .set_selected(self.theme.selected_index());
            ui.displayed_theme = self.theme;
        }
        if ui.displayed_font_size != self.font_size {
            set_font_size(&ui.code_view, ui.displayed_font_size, self.font_size);
            set_font_size(&ui.line_numbers, ui.displayed_font_size, self.font_size);
            ui.settings
                .font_size_picker
                .set_selected(font_size_index(self.font_size));
            ui.displayed_font_size = self.font_size;
        }
        if ui.displayed_line_numbers_visible != self.line_numbers_visible {
            ui.line_numbers.set_visible(self.line_numbers_visible);
            ui.line_numbers_separator
                .set_visible(self.line_numbers_visible);
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

        if ui.displayed_project_name != self.project_name {
            ui.project_title.set_text(&self.project_name);
            ui.project_label.set_text(&self.project_name);
            ui.displayed_project_name.clone_from(&self.project_name);
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
            rebuild_file_list(
                &ui.file_list,
                &self.project_files,
                active_path,
                project_open,
                sender.clone(),
            );
            ui.displayed_project_files = project_file_signature;
            ui.displayed_project_open = project_open;
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
            rebuild_editor_tabs(
                &ui.editor_tabs,
                &self.documents,
                self.active_document_id,
                sender.clone(),
            );
            ui.displayed_tabs = tab_signature;
        }

        if ui.displayed_pending_project != self.pending_project {
            ui.displayed_pending_project
                .clone_from(&self.pending_project);
            if self.pending_project.is_some() {
                show_project_switch_confirmation(&ui.window, sender.clone());
            }
        }

        if ui.displayed_pending_close_document_id != self.pending_close_document_id {
            ui.displayed_pending_close_document_id = self.pending_close_document_id;
            if let Some(id) = self.pending_close_document_id {
                show_close_document_confirmation(&ui.window, id, sender.clone());
            }
        }

        if ui.displayed_pending_close_app != self.pending_close_app {
            ui.displayed_pending_close_app = self.pending_close_app;
            if self.pending_close_app {
                show_close_app_confirmation(&ui.window, sender.clone());
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
                        sender.clone(),
                    );
                }
            }
        }

        if self.close_app_ready && !ui.displayed_close_app_ready {
            ui.displayed_close_app_ready = true;
            ui.close_authorized.set(true);
            ui.window.close();
        }

        if ui.displayed_output_text != self.output_text {
            ui.output_buffer.set_text(&self.output_text);
            ui.displayed_output_text.clone_from(&self.output_text);
        }
        if ui.displayed_output_status != self.output_status {
            ui.output_status.set_text(&self.output_status);
            ui.displayed_output_status.clone_from(&self.output_status);
        }

        update_line_numbers(&ui.code_buffer, &ui.line_numbers);
        highlight_buffer(
            &ui.code_buffer,
            active_language.unwrap_or(self.new_file_language),
            &ui.syntax_tags,
        );
    }
}

fn build_window_content(
    window: gtk::ApplicationWindow,
    sender: ComponentSender<MainWindow>,
) -> MainUi {
    let close_authorized = Rc::new(Cell::new(false));
    let close_authorized_for_window = close_authorized.clone();
    let sender_for_close = sender.clone();
    window.connect_close_request(move |_| {
        if close_authorized_for_window.replace(false) {
            gtk::glib::Propagation::Proceed
        } else {
            sender_for_close.input(MainWindowMsg::RequestClose);
            gtk::glib::Propagation::Stop
        }
    });

    let brand = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    brand.append(&app_icon());
    let brand_label = gtk::Label::new(Some("Potoo IDE"));
    brand_label.add_css_class("potoo-brand-name");
    brand.append(&brand_label);

    let project_title = gtk::Label::new(Some("Meu primeiro programa"));
    project_title.add_css_class("potoo-project-name");
    project_title.set_ellipsize(gtk::pango::EllipsizeMode::End);

    let open_project_button = gtk::Button::new();
    open_project_button.set_child(Some(&gtk::Image::from_icon_name("folder-open-symbolic")));
    open_project_button.set_tooltip_text(Some("Abrir pasta de projeto"));
    open_project_button.add_css_class("potoo-toolbar-button");

    let save_button = gtk::Button::new();
    save_button.set_child(Some(&gtk::Image::from_icon_name("document-save-symbolic")));
    save_button.set_tooltip_text(Some(if cfg!(target_os = "macos") {
        "Salvar arquivo (⌘S)"
    } else {
        "Salvar arquivo (Ctrl+S)"
    }));
    save_button.add_css_class("potoo-toolbar-button");

    let language_picker = gtk::DropDown::from_strings(&["Portugol", "VisuAlg"]);
    language_picker.add_css_class("potoo-language-picker");
    language_picker.set_tooltip_text(Some("Linguagem para novos arquivos"));

    let settings_button = gtk::Button::new();
    settings_button.set_child(Some(&gtk::Image::from_icon_name(
        "preferences-system-symbolic",
    )));
    settings_button.set_tooltip_text(Some("Configurações"));
    settings_button.add_css_class("potoo-settings-button");

    let run_button = gtk::Button::new();
    run_button.add_css_class("potoo-run-button");
    let run_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    run_content.append(&gtk::Image::from_icon_name("media-playback-start-symbolic"));
    run_content.append(&gtk::Label::new(Some("Executar")));
    run_button.set_child(Some(&run_content));
    run_button.set_tooltip_text(Some(
        "A execução real usa o interpretador da linguagem selecionada",
    ));

    let header_actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    header_actions.append(&open_project_button);
    header_actions.append(&language_picker);
    header_actions.append(&save_button);
    header_actions.append(&settings_button);
    header_actions.append(&run_button);

    #[cfg(target_os = "linux")]
    {
        let titlebar = gtk::HeaderBar::new();
        titlebar.add_css_class("potoo-linux-titlebar");
        titlebar.set_show_title_buttons(true);
        titlebar.pack_start(&brand);
        titlebar.set_title_widget(Some(&project_title));
        titlebar.pack_end(&header_actions);
        window.set_titlebar(Some(&titlebar));
    }

    #[cfg(not(target_os = "linux"))]
    let header = {
        let header = gtk::CenterBox::new();
        header.add_css_class("potoo-header");
        header.set_margin_start(14);
        header.set_margin_end(14);
        header.set_start_widget(Some(&brand));
        header.set_center_widget(Some(&project_title));
        header.set_end_widget(Some(&header_actions));
        header
    };

    let settings = build_settings_window(&window, sender.clone());

    let (sidebar, project_label, file_list, new_file_button) = build_file_sidebar();

    let code_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
    code_buffer.set_text(Language::Portugol.sample());
    let syntax_tags = make_syntax_tags(&code_buffer);
    let initial_dark = adw::StyleManager::default().is_dark();
    set_syntax_palette(&syntax_tags, initial_dark);
    highlight_buffer(&code_buffer, Language::Portugol, &syntax_tags);

    let line_numbers = gtk::Label::new(None);
    line_numbers.set_xalign(1.0);
    line_numbers.set_yalign(0.0);
    line_numbers.add_css_class("dim-label");
    line_numbers.add_css_class("monospace");
    line_numbers.add_css_class("potoo-line-numbers");
    line_numbers.add_css_class("potoo-font-13");
    line_numbers.set_margin_top(14);
    line_numbers.set_margin_bottom(14);
    line_numbers.set_margin_start(12);
    line_numbers.set_margin_end(12);
    update_line_numbers(&code_buffer, &line_numbers);

    let code_view = gtk::TextView::builder()
        .buffer(&code_buffer)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::None)
        .left_margin(12)
        .right_margin(16)
        .top_margin(14)
        .bottom_margin(14)
        .build();
    code_view.add_css_class("potoo-code-editor");
    code_view.add_css_class("potoo-font-13");

    let code_scroll = gtk::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .child(&code_view)
        .build();

    let editor_row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_row.add_css_class("potoo-editor-row");
    editor_row.append(&line_numbers);
    let line_numbers_separator = gtk::Separator::new(gtk::Orientation::Vertical);
    editor_row.append(&line_numbers_separator);
    editor_row.append(&code_scroll);

    let editor_tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    editor_tabs.set_hexpand(true);
    let tab_bar = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tab_bar.add_css_class("potoo-editor-tabbar");
    tab_bar.append(&editor_tabs);
    tab_bar.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let editor = gtk::Box::new(gtk::Orientation::Vertical, 0);
    editor.add_css_class("potoo-editor");
    editor.append(&tab_bar);
    editor.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    editor.append(&editor_row);

    let (output_notebook, output_buffer) = build_output_panel();
    let output_status = gtk::Label::new(Some("Pronto para executar"));
    output_status.set_xalign(0.0);
    output_status.add_css_class("potoo-muted");
    let output_status_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    output_status_row.add_css_class("potoo-output-status-row");
    output_status_row.set_margin_start(12);
    output_status_row.set_margin_end(12);
    output_status_row.set_margin_bottom(8);
    output_status_row.append(&gtk::Image::from_icon_name("emblem-synchronizing-symbolic"));
    output_status_row.append(&output_status);

    let output_area = gtk::Box::new(gtk::Orientation::Vertical, 0);
    output_area.add_css_class("potoo-output-area");
    output_area.append(&output_notebook);
    output_area.append(&output_status_row);

    let editor_output = gtk::Paned::new(gtk::Orientation::Vertical);
    editor_output.set_wide_handle(true);
    editor_output.set_start_child(Some(&editor));
    editor_output.set_end_child(Some(&output_area));
    editor_output.set_resize_start_child(true);
    editor_output.set_resize_end_child(false);
    editor_output.set_shrink_start_child(false);
    editor_output.set_shrink_end_child(false);
    editor_output.set_position(570);

    let learning_panel = build_learning_panel();
    let editor_and_learning = gtk::Paned::new(gtk::Orientation::Horizontal);
    editor_and_learning.set_wide_handle(true);
    editor_and_learning.set_start_child(Some(&editor_output));
    editor_and_learning.set_end_child(Some(&learning_panel));
    editor_and_learning.set_resize_start_child(true);
    editor_and_learning.set_resize_end_child(false);
    editor_and_learning.set_shrink_start_child(false);
    editor_and_learning.set_shrink_end_child(false);
    editor_and_learning.set_position(820);

    let workspace = gtk::Paned::new(gtk::Orientation::Horizontal);
    workspace.set_wide_handle(true);
    workspace.set_start_child(Some(&sidebar));
    workspace.set_end_child(Some(&editor_and_learning));
    workspace.set_resize_start_child(false);
    workspace.set_resize_end_child(true);
    workspace.set_shrink_start_child(false);
    workspace.set_shrink_end_child(false);
    workspace.set_position(250);
    workspace.set_vexpand(true);

    let status_language = gtk::Label::new(Some("Portugol"));
    let status_details = gtk::Label::new(Some("UTF-8  ·  Espaços: 4"));
    status_language.add_css_class("potoo-status-primary");
    status_details.add_css_class("potoo-muted");
    let status_bar = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    status_bar.add_css_class("potoo-statusbar");
    status_bar.set_margin_start(12);
    status_bar.set_margin_end(12);
    status_bar.set_margin_top(6);
    status_bar.set_margin_bottom(6);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_language);
    status_bar.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    status_bar.append(&status_details);
    let content = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content.append(&workspace);
    content.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
    content.append(&status_bar);

    let content_root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    content_root.add_css_class("potoo-root");
    #[cfg(not(target_os = "linux"))]
    content_root.append(&header);
    content_root.append(&content);
    if initial_dark {
        window.add_css_class("potoo-dark");
    }

    let sender_for_language = sender.clone();
    language_picker.connect_selected_notify(move |picker| {
        let selected = if picker.selected() == 1 {
            Language::Visualg
        } else {
            Language::Portugol
        };
        sender_for_language.input(MainWindowMsg::LanguageSelected(selected));
    });

    let sender_for_open_project = sender.clone();
    let window_for_open_project = window.clone();
    open_project_button.connect_clicked(move |_| {
        show_open_project_dialog(&window_for_open_project, sender_for_open_project.clone());
    });

    let sender_for_new_file = sender.clone();
    let window_for_new_file = window.clone();
    let language_picker_for_new_file = language_picker.clone();
    new_file_button.connect_clicked(move |_| {
        show_new_file_dialog(
            &window_for_new_file,
            &language_picker_for_new_file,
            sender_for_new_file.clone(),
        );
    });

    let sender_for_save = sender.clone();
    save_button.connect_clicked(move |_| {
        sender_for_save.input(MainWindowMsg::SaveCurrent);
    });

    let sender_for_settings = sender.clone();
    settings_button.connect_clicked(move |_| {
        sender_for_settings.input(MainWindowMsg::OpenSettings);
    });

    let sender_for_run = sender.clone();
    let output_notebook_for_run = output_notebook.clone();
    run_button.connect_clicked(move |_| {
        output_notebook_for_run.set_current_page(Some(0));
        sender_for_run.input(MainWindowMsg::Run);
    });

    let sender_for_source = sender.clone();
    code_buffer.connect_changed(move |buffer| {
        let contents = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        sender_for_source.input(MainWindowMsg::SourceChanged(contents.to_string()));
    });

    let key_controller = gtk::EventControllerKey::new();
    let sender_for_shortcuts = sender;
    key_controller.connect_key_pressed(move |_, key, _, modifiers| {
        let primary_modifier = modifiers.contains(gtk::gdk::ModifierType::CONTROL_MASK)
            || (cfg!(target_os = "macos") && modifiers.contains(gtk::gdk::ModifierType::META_MASK));
        if key == gtk::gdk::Key::s && primary_modifier {
            sender_for_shortcuts.input(MainWindowMsg::SaveCurrent);
            gtk::glib::Propagation::Stop
        } else {
            gtk::glib::Propagation::Proceed
        }
    });
    window.add_controller(key_controller);

    MainUi {
        window,
        content: content_root,
        language_picker,
        settings,
        project_title,
        project_label,
        file_list,
        new_file_button,
        editor_tabs,
        status_language,
        code_buffer,
        code_view,
        syntax_tags,
        line_numbers,
        line_numbers_separator,
        output_buffer,
        output_status,
        displayed_language: Some(Language::Portugol),
        displayed_new_file_language: Language::Portugol,
        displayed_active_document_id: None,
        displayed_project_name: String::new(),
        displayed_project_files: Vec::new(),
        displayed_project_open: false,
        displayed_tabs: Vec::new(),
        displayed_pending_project: None,
        displayed_pending_close_document_id: None,
        displayed_pending_close_app: false,
        displayed_pending_save_as_id: None,
        displayed_close_app_ready: false,
        close_authorized,
        displayed_theme: ThemePreference::System,
        displayed_dark: initial_dark,
        displayed_font_size: 13,
        displayed_line_numbers_visible: true,
        displayed_word_wrap: false,
        displayed_settings_visible: false,
        displayed_output_text: "A saída aparecerá aqui após a execução.".to_string(),
        displayed_output_status: "Pronto para executar".to_string(),
    }
}

fn build_settings_window(
    parent: &gtk::ApplicationWindow,
    sender: ComponentSender<MainWindow>,
) -> SettingsUi {
    let window = gtk::Window::new();
    window.set_title(Some("Configurações"));
    window.set_default_size(500, 470);
    window.set_resizable(false);
    window.set_modal(true);
    window.set_transient_for(Some(parent));
    window.add_css_class("potoo-window");
    window.add_css_class("potoo-settings-window");
    if adw::StyleManager::default().is_dark() {
        window.add_css_class("potoo-dark");
    }

    let page = gtk::Box::new(gtk::Orientation::Vertical, 20);
    page.add_css_class("potoo-settings-page");
    page.set_margin_start(24);
    page.set_margin_end(24);
    page.set_margin_top(24);
    page.set_margin_bottom(24);

    let title = gtk::Label::new(Some("Configurações"));
    title.set_xalign(0.0);
    title.add_css_class("potoo-settings-title");
    let description = gtk::Label::new(Some("Personalize a aparência e o editor."));
    description.set_xalign(0.0);
    description.add_css_class("potoo-settings-description");
    page.append(&title);
    page.append(&description);

    let appearance = settings_section("Aparência");
    let theme_picker = gtk::DropDown::from_strings(&["Seguir o sistema", "Claro", "Escuro"]);
    theme_picker.add_css_class("potoo-settings-picker");
    theme_picker.set_selected(ThemePreference::System.selected_index());
    appearance.append(&settings_row(
        "Tema",
        "Use a aparência do sistema ou escolha um tema.",
        &theme_picker,
    ));
    page.append(&appearance);

    let editor = settings_section("Editor");
    let font_size_picker = gtk::DropDown::from_strings(&["13 px", "15 px", "17 px"]);
    font_size_picker.add_css_class("potoo-settings-picker");
    editor.append(&settings_row(
        "Tamanho da fonte",
        "Ajuste a leitura do código.",
        &font_size_picker,
    ));

    let line_numbers_switch = gtk::Switch::new();
    line_numbers_switch.set_active(true);
    editor.append(&settings_row(
        "Numeração de linhas",
        "Mostra a linha atual no editor.",
        &line_numbers_switch,
    ));

    let word_wrap_switch = gtk::Switch::new();
    editor.append(&settings_row(
        "Quebra automática de linha",
        "Ajusta linhas longas à largura do editor.",
        &word_wrap_switch,
    ));
    page.append(&editor);
    window.set_child(Some(&page));

    let sender_for_theme = sender.clone();
    theme_picker.connect_selected_notify(move |picker| {
        sender_for_theme.input(MainWindowMsg::ThemeSelected(
            ThemePreference::from_selected_index(picker.selected()),
        ));
    });

    let sender_for_font_size = sender.clone();
    font_size_picker.connect_selected_notify(move |picker| {
        let font_size = match picker.selected() {
            1 => 15,
            2 => 17,
            _ => 13,
        };
        sender_for_font_size.input(MainWindowMsg::FontSizeSelected(font_size));
    });

    let sender_for_line_numbers = sender.clone();
    line_numbers_switch.connect_active_notify(move |switch| {
        sender_for_line_numbers.input(MainWindowMsg::LineNumbersChanged(switch.is_active()));
    });

    let sender_for_word_wrap = sender.clone();
    word_wrap_switch.connect_active_notify(move |switch| {
        sender_for_word_wrap.input(MainWindowMsg::WordWrapChanged(switch.is_active()));
    });

    window.connect_close_request(move |_| {
        sender.input(MainWindowMsg::CloseSettings);
        gtk::glib::Propagation::Stop
    });

    SettingsUi {
        window,
        theme_picker,
        font_size_picker,
        line_numbers_switch,
        word_wrap_switch,
    }
}

fn settings_section(title: &str) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 4);
    section.add_css_class("potoo-settings-section");
    let heading = gtk::Label::new(Some(title));
    heading.set_xalign(0.0);
    heading.add_css_class("potoo-settings-section-title");
    section.append(&heading);
    section
}

fn settings_row<W: IsA<gtk::Widget>>(title: &str, description: &str, control: &W) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    row.add_css_class("potoo-settings-row");

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 4);
    labels.set_hexpand(true);
    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("potoo-settings-row-title");
    let description_label = gtk::Label::new(Some(description));
    description_label.set_xalign(0.0);
    description_label.set_wrap(true);
    description_label.add_css_class("potoo-settings-row-description");
    labels.append(&title_label);
    labels.append(&description_label);
    row.append(&labels);
    row.append(control);
    row
}

fn font_size_index(font_size: u32) -> u32 {
    match font_size {
        15 => 1,
        17 => 2,
        _ => 0,
    }
}

fn set_font_size(widget: &impl IsA<gtk::Widget>, old_size: u32, new_size: u32) {
    widget.remove_css_class(&format!("potoo-font-{old_size}"));
    widget.add_css_class(&format!("potoo-font-{new_size}"));
}

fn app_icon() -> gtk::Image {
    let development_icon = "data/icons/hicolor/scalable/apps/io.github.diegopn.potooide.svg";
    let image = if Path::new(development_icon).exists() {
        gtk::Image::from_file(development_icon)
    } else {
        gtk::Image::from_icon_name("io.github.diegopn.potooide")
    };
    image.set_pixel_size(32);
    image
}

fn build_file_sidebar() -> (gtk::Widget, gtk::Label, gtk::Box, gtk::Button) {
    let sidebar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sidebar.add_css_class("potoo-file-sidebar");
    sidebar.set_size_request(220, -1);

    let heading_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    heading_row.set_margin_start(16);
    heading_row.set_margin_end(10);
    heading_row.set_margin_top(12);
    heading_row.set_margin_bottom(8);
    let heading = gtk::Label::new(Some("Arquivos"));
    heading.set_xalign(0.0);
    heading.add_css_class("heading");
    heading.set_hexpand(true);
    let new_file_button = gtk::Button::new();
    new_file_button.set_child(Some(&gtk::Image::from_icon_name("list-add-symbolic")));
    new_file_button.set_tooltip_text(Some("Criar arquivo"));
    new_file_button.add_css_class("potoo-sidebar-action");
    new_file_button.set_sensitive(false);
    heading_row.append(&heading);
    heading_row.append(&new_file_button);
    sidebar.append(&heading_row);

    let project = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    project.set_margin_start(12);
    project.set_margin_end(12);
    project.set_margin_top(4);
    project.set_margin_bottom(8);
    project.append(&gtk::Image::from_icon_name("folder-symbolic"));
    let project_label = gtk::Label::new(Some("Meu primeiro programa"));
    project_label.set_xalign(0.0);
    project_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
    project_label.set_hexpand(true);
    project.append(&project_label);
    sidebar.append(&project);

    let file_list = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let empty_label = gtk::Label::new(Some(
        "Abra uma pasta de projeto para navegar pelos arquivos.",
    ));
    empty_label.set_wrap(true);
    empty_label.add_css_class("potoo-muted");
    empty_label.set_margin_start(14);
    empty_label.set_margin_end(14);
    empty_label.set_margin_top(12);
    file_list.append(&empty_label);
    let file_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&file_list)
        .build();
    sidebar.append(&file_scroll);

    (sidebar.upcast(), project_label, file_list, new_file_button)
}

fn rebuild_file_list(
    file_list: &gtk::Box,
    files: &[ProjectFile],
    active_path: Option<&PathBuf>,
    project_open: bool,
    sender: ComponentSender<MainWindow>,
) {
    clear_box(file_list);
    if files.is_empty() {
        let message = if project_open {
            "Esta pasta ainda não tem arquivos .por ou .alg."
        } else {
            "Abra uma pasta de projeto para navegar pelos arquivos."
        };
        let empty_label = gtk::Label::new(Some(message));
        empty_label.set_wrap(true);
        empty_label.set_xalign(0.0);
        empty_label.add_css_class("potoo-muted");
        empty_label.set_margin_start(14);
        empty_label.set_margin_end(14);
        empty_label.set_margin_top(12);
        file_list.append(&empty_label);
        return;
    }

    for file in files {
        let button = gtk::Button::new();
        button.add_css_class("potoo-file-row");
        if active_path == Some(&file.path) {
            button.add_css_class("potoo-file-row-selected");
        }
        button.set_tooltip_text(Some(&file.path.display().to_string()));
        button.set_hexpand(true);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_hexpand(true);
        row.set_margin_start(10);
        row.set_margin_end(10);
        row.set_margin_top(7);
        row.set_margin_bottom(7);
        row.append(&gtk::Image::from_icon_name("text-x-generic-symbolic"));
        let label = gtk::Label::new(Some(&file.relative_path));
        label.set_xalign(0.0);
        label.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        label.set_max_width_chars(28);
        label.set_hexpand(true);
        row.append(&label);
        button.set_child(Some(&row));
        let path = file.path.clone();
        let sender_for_file = sender.clone();
        button.connect_clicked(move |_| {
            sender_for_file.input(MainWindowMsg::OpenFile(path.clone()));
        });
        file_list.append(&button);
    }
}

fn rebuild_editor_tabs(
    editor_tabs: &gtk::Box,
    documents: &[SourceDocument],
    active_document_id: Option<u64>,
    sender: ComponentSender<MainWindow>,
) {
    clear_box(editor_tabs);
    for document in documents {
        let tab = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        tab.add_css_class("potoo-editor-tab-container");
        let button = gtk::Button::new();
        button.add_css_class("potoo-editor-tab");
        if Some(document.id) == active_document_id {
            button.add_css_class("potoo-editor-tab-selected");
        }
        if document.is_dirty() {
            button.add_css_class("potoo-editor-tab-dirty");
        }
        button.set_tooltip_text(
            document
                .path
                .as_ref()
                .map(|path| path.display().to_string())
                .as_deref(),
        );
        let content = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        content.set_margin_start(12);
        content.set_margin_end(4);
        content.set_margin_top(8);
        content.set_margin_bottom(8);
        content.append(&gtk::Image::from_icon_name("text-x-generic-symbolic"));
        let name = gtk::Label::new(Some(&document.name));
        name.set_max_width_chars(22);
        name.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
        content.append(&name);
        if document.is_dirty() {
            let modified = gtk::Label::new(Some("•"));
            modified.add_css_class("potoo-tab-modified");
            content.append(&modified);
        }
        button.set_child(Some(&content));
        button.set_tooltip_text(Some(&document.name));
        let id = document.id;
        let sender_for_tab = sender.clone();
        button.connect_clicked(move |_| {
            sender_for_tab.input(MainWindowMsg::ActivateDocument(id));
        });

        let close_button = gtk::Button::new();
        close_button.set_child(Some(&gtk::Image::from_icon_name("window-close-symbolic")));
        close_button.set_tooltip_text(Some("Fechar arquivo"));
        close_button.add_css_class("potoo-close-tab-button");
        let sender_for_close = sender.clone();
        close_button.connect_clicked(move |_| {
            sender_for_close.input(MainWindowMsg::CloseDocument(id));
        });
        tab.append(&button);
        tab.append(&close_button);
        editor_tabs.append(&tab);
    }
}

fn clear_box(container: &gtk::Box) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }
}

fn show_open_project_dialog(parent: &gtk::ApplicationWindow, sender: ComponentSender<MainWindow>) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title("Abrir pasta de projeto");
    dialog.set_modal(true);
    dialog.select_folder(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(folder) => match folder.path() {
                Some(path) => sender.input(MainWindowMsg::ProjectSelected(path)),
                None => sender.input(MainWindowMsg::FileOperationFailed(
                    "Selecione uma pasta local para abrir.".to_string(),
                )),
            },
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {}
            Err(error) => sender.input(MainWindowMsg::FileOperationFailed(format!(
                "Não foi possível abrir a pasta: {error}"
            ))),
        },
    );
}

fn show_new_file_dialog(
    parent: &gtk::ApplicationWindow,
    language_picker: &gtk::DropDown,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::Window::new();
    dialog.set_title(Some("Criar arquivo"));
    dialog.set_default_size(420, 300);
    dialog.set_resizable(false);
    dialog.set_transient_for(Some(parent));
    dialog.set_modal(true);
    dialog.add_css_class("potoo-window");
    dialog.add_css_class("potoo-dialog-window");
    if adw::StyleManager::default().is_dark() {
        dialog.add_css_class("potoo-dark");
    }

    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.add_css_class("potoo-dialog-surface");
    content.set_margin_start(24);
    content.set_margin_end(24);
    content.set_margin_top(24);
    content.set_margin_bottom(20);
    let title = gtk::Label::new(Some("Criar arquivo"));
    title.set_xalign(0.0);
    title.add_css_class("potoo-settings-title");
    content.append(&title);
    let description = gtk::Label::new(Some("Escolha um nome e a linguagem do novo arquivo."));
    description.set_xalign(0.0);
    description.add_css_class("potoo-settings-description");
    content.append(&description);
    content.append(&gtk::Label::new(Some("Nome do arquivo")));
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some("Nome do arquivo, por exemplo: exercicio-1"));
    name.set_text("novo-programa");
    content.append(&name);
    content.append(&gtk::Label::new(Some("Linguagem")));
    let language = gtk::DropDown::from_strings(&["Portugol (.por)", "VisuAlg (.alg)"]);
    language.add_css_class("potoo-settings-picker");
    language.set_selected(language_picker.selected());
    content.append(&language);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::End);
    let cancel_button = gtk::Button::with_label("Cancelar");
    let create_button = gtk::Button::with_label("Criar arquivo");
    create_button.add_css_class("potoo-run-button");
    actions.append(&cancel_button);
    actions.append(&create_button);
    content.append(&actions);
    dialog.set_child(Some(&content));

    let dialog_for_cancel = dialog.clone();
    cancel_button.connect_clicked(move |_| dialog_for_cancel.close());
    let dialog_for_create = dialog.clone();
    let name_for_create = name.clone();
    create_button.connect_clicked(move |_| {
        let selected_language = if language.selected() == 1 {
            Language::Visualg
        } else {
            Language::Portugol
        };
        sender.input(MainWindowMsg::CreateFile(
            name_for_create.text().to_string(),
            selected_language,
        ));
        dialog_for_create.close();
    });
    let create_button_for_entry = create_button.clone();
    name.connect_activate(move |_| create_button_for_entry.emit_clicked());
    dialog.present();
}

fn show_save_dialog(
    parent: &gtk::ApplicationWindow,
    document: &SourceDocument,
    project_root: Option<&Path>,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::FileDialog::new();
    dialog.set_title("Salvar arquivo");
    dialog.set_modal(true);
    dialog.set_initial_name(Some(&document.name));
    if let Some(folder) = document
        .path
        .as_deref()
        .and_then(Path::parent)
        .or(project_root)
    {
        dialog.set_initial_folder(Some(&gtk::gio::File::for_path(folder)));
    }
    let id = document.id;
    dialog.save(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |result| match result {
            Ok(file) => match file.path() {
                Some(path) => sender.input(MainWindowMsg::SaveAsSelected(id, path)),
                None => {
                    sender.input(MainWindowMsg::SaveAsCanceled(id));
                    sender.input(MainWindowMsg::FileOperationFailed(
                        "Escolha um caminho local para salvar o arquivo.".to_string(),
                    ));
                }
            },
            Err(error) if error.matches(gtk::gio::IOErrorEnum::Cancelled) => {
                sender.input(MainWindowMsg::SaveAsCanceled(id));
            }
            Err(error) => {
                sender.input(MainWindowMsg::SaveAsCanceled(id));
                sender.input(MainWindowMsg::FileOperationFailed(format!(
                    "Não foi possível abrir a janela de salvamento: {error}"
                )));
            }
        },
    );
}

fn show_project_switch_confirmation(
    parent: &gtk::ApplicationWindow,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::AlertDialog::builder()
        .message("Alterações não salvas")
        .detail("Salve os arquivos abertos antes de trocar de pasta ou descarte as alterações para continuar.")
        .buttons(["Cancelar", "Descartar e abrir"])
        .cancel_button(0)
        .default_button(0)
        .modal(true)
        .build();
    dialog.choose(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |response| {
            sender.input(MainWindowMsg::ProjectSwitchDecision(matches!(
                response,
                Ok(1)
            )));
        },
    );
}

fn show_close_document_confirmation(
    parent: &gtk::ApplicationWindow,
    id: u64,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::AlertDialog::builder()
        .message("Este arquivo tem alterações não salvas")
        .detail("Salve o arquivo antes de fechar ou descarte as alterações para continuar.")
        .buttons(["Continuar editando", "Descartar alterações"])
        .cancel_button(0)
        .default_button(0)
        .modal(true)
        .build();
    dialog.choose(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |response| {
            sender.input(MainWindowMsg::CloseDocumentDecision(
                id,
                matches!(response, Ok(1)),
            ));
        },
    );
}

fn show_close_app_confirmation(
    parent: &gtk::ApplicationWindow,
    sender: ComponentSender<MainWindow>,
) {
    let dialog = gtk::AlertDialog::builder()
        .message("Há alterações não salvas")
        .detail(
            "Salve os arquivos antes de sair ou descarte as alterações para fechar o Potoo IDE.",
        )
        .buttons(["Continuar editando", "Descartar e sair"])
        .cancel_button(0)
        .default_button(0)
        .modal(true)
        .build();
    dialog.choose(
        Some(parent),
        None::<&gtk::gio::Cancellable>,
        move |response| {
            sender.input(MainWindowMsg::CloseAppDecision(matches!(response, Ok(1))));
        },
    );
}

fn new_source_path(
    root: &Path,
    name: &str,
    default_language: Language,
) -> Result<(PathBuf, Language), String> {
    let name = name.trim();
    if name.is_empty()
        || name.starts_with('.')
        || name.ends_with('.')
        || name.contains('/')
        || name.contains('\\')
        || is_reserved_windows_file_name(name)
        || name
            .chars()
            .any(|character| character.is_control() || "<>:\"|?*".contains(character))
    {
        return Err("Use um nome simples, sem pastas ou caracteres especiais.".to_string());
    }

    let requested_path = Path::new(name);
    let language = match requested_path.extension() {
        Some(_) => Language::from_path(requested_path)
            .ok_or_else(|| "Use a extensão .por ou .alg.".to_string())?,
        None => default_language,
    };
    let file_name = if requested_path.extension().is_some() {
        name.to_string()
    } else {
        format!("{name}.{}", language.extension())
    };
    Ok((root.join(file_name), language))
}

fn is_reserved_windows_file_name(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or(name)
        .trim_end_matches(' ')
        .to_ascii_uppercase();
    matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(prefix).is_some_and(|number| {
            matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
    })
}

fn supported_save_path(
    path: PathBuf,
    default_language: Language,
) -> Result<(PathBuf, Language), String> {
    match path.extension() {
        Some(_) => Language::from_path(&path)
            .map(|language| (path, language))
            .ok_or_else(|| "Salve o código com a extensão .por ou .alg.".to_string()),
        None => Ok((
            path.with_extension(default_language.extension()),
            default_language,
        )),
    }
}

fn scan_project_files(root: &Path) -> std::io::Result<Vec<ProjectFile>> {
    let mut paths = Vec::new();
    scan_source_directory(root, &mut paths)?;
    let mut files = paths
        .into_iter()
        .map(|path| ProjectFile {
            relative_path: path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned(),
            path,
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|file| file.relative_path.to_lowercase());
    Ok(files)
}

fn scan_source_directory(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut entries = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let name = entry.file_name();
        let visible_name = name.to_string_lossy();
        if visible_name.starts_with('.')
            || matches!(visible_name.as_ref(), "target" | "node_modules")
        {
            continue;
        }
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            scan_source_directory(&path, files)?;
        } else if file_type.is_file() && Language::from_path(&path).is_some() {
            files.push(path);
        }
    }
    Ok(())
}

fn build_learning_panel() -> gtk::Widget {
    let panel = gtk::Box::new(gtk::Orientation::Vertical, 16);
    panel.add_css_class("potoo-learning-panel");
    panel.set_size_request(320, -1);
    panel.set_margin_start(18);
    panel.set_margin_end(18);
    panel.set_margin_top(18);
    panel.set_margin_bottom(18);

    let title = gtk::Label::new(Some("Aprenda enquanto programa"));
    title.set_xalign(0.0);
    title.add_css_class("potoo-learning-title");
    title.set_wrap(true);
    panel.append(&title);

    let line_title = gtk::Label::new(Some("Linha 7 · decisão"));
    line_title.set_xalign(0.0);
    line_title.add_css_class("heading");
    panel.append(&line_title);

    let explanation = gtk::Label::new(Some(
        "Uma condição verifica se uma expressão é verdadeira antes de executar um bloco.",
    ));
    explanation.set_xalign(0.0);
    explanation.set_wrap(true);
    explanation.set_selectable(true);
    panel.append(&explanation);
    panel.append(&gtk::Separator::new(gtk::Orientation::Horizontal));

    let experiment_title = gtk::Label::new(Some("Experimente"));
    experiment_title.set_xalign(0.0);
    experiment_title.add_css_class("heading");
    panel.append(&experiment_title);

    let experiment = gtk::Label::new(Some(
        "Troque o valor 18 por 16 e execute de novo. O resultado muda?",
    ));
    experiment.set_xalign(0.0);
    experiment.set_wrap(true);
    experiment.set_selectable(true);
    experiment.add_css_class("potoo-card");
    experiment.add_css_class("potoo-experiment-card");
    experiment.set_margin_start(4);
    experiment.set_margin_end(4);
    experiment.set_margin_top(4);
    experiment.set_margin_bottom(4);
    panel.append(&experiment);

    let concept = gtk::Box::new(gtk::Orientation::Vertical, 8);
    concept.add_css_class("potoo-card");
    concept.add_css_class("potoo-concept-card");
    concept.set_margin_start(4);
    concept.set_margin_end(4);
    concept.set_margin_top(8);
    concept.set_margin_bottom(8);
    let concept_title = gtk::Label::new(Some("Conceito: condição"));
    concept_title.set_xalign(0.0);
    concept_title.add_css_class("heading");
    let concept_text = gtk::Label::new(Some(
        "Uma comparação como idade >= 18 resulta em verdadeiro ou falso.",
    ));
    concept_text.set_xalign(0.0);
    concept_text.set_wrap(true);
    concept_text.set_selectable(true);
    concept.append(&concept_title);
    concept.append(&concept_text);
    panel.append(&concept);

    panel.upcast()
}

fn build_output_panel() -> (gtk::Notebook, gtk::TextBuffer) {
    let notebook = gtk::Notebook::new();
    notebook.add_css_class("potoo-output-notebook");
    notebook.set_tab_pos(gtk::PositionType::Top);
    notebook.set_vexpand(true);

    let output = gtk::TextView::builder()
        .editable(false)
        .cursor_visible(false)
        .monospace(true)
        .wrap_mode(gtk::WrapMode::WordChar)
        .left_margin(14)
        .right_margin(14)
        .top_margin(12)
        .bottom_margin(12)
        .build();
    output.add_css_class("potoo-output-view");
    let output_buffer = output.buffer();
    output_buffer.set_text("A saída aparecerá aqui após a execução.");
    let output_scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .child(&output)
        .build();
    notebook.append_page(&output_scroll, Some(&gtk::Label::new(Some("Saída"))));

    let problems = empty_tool_page(
        "Nenhum problema detectado",
        "Os erros de sintaxe e execução aparecerão aqui.",
    );
    notebook.append_page(&problems, Some(&gtk::Label::new(Some("Problemas"))));

    let debugger = empty_tool_page(
        "Depurador",
        "Acompanhe a execução linha por linha e os valores das variáveis.",
    );
    notebook.append_page(&debugger, Some(&gtk::Label::new(Some("Depurador"))));
    (notebook, output_buffer)
}

fn empty_tool_page(title: &str, description: &str) -> gtk::Widget {
    let page = gtk::Box::new(gtk::Orientation::Vertical, 8);
    page.set_margin_start(16);
    page.set_margin_end(16);
    page.set_margin_top(16);
    page.set_margin_bottom(16);
    let title_label = gtk::Label::new(Some(title));
    title_label.set_xalign(0.0);
    title_label.add_css_class("heading");
    let description_label = gtk::Label::new(Some(description));
    description_label.set_xalign(0.0);
    description_label.set_wrap(true);
    description_label.add_css_class("dim-label");
    page.append(&title_label);
    page.append(&description_label);
    page.upcast()
}

fn make_syntax_tags(buffer: &gtk::TextBuffer) -> SyntaxTags {
    let table = buffer.tag_table();
    let keyword = gtk::TextTag::builder().name("keyword").build();
    let string = gtk::TextTag::builder().name("string").build();
    let number = gtk::TextTag::builder().name("number").build();
    let current_line = gtk::TextTag::builder().name("current-line").build();
    table.add(&keyword);
    table.add(&string);
    table.add(&number);
    table.add(&current_line);
    SyntaxTags {
        keyword,
        string,
        number,
        current_line,
    }
}

fn set_syntax_palette(tags: &SyntaxTags, dark: bool) {
    tags.keyword
        .set_property("foreground", if dark { "#65d48d" } else { "#15945b" });
    tags.string
        .set_property("foreground", if dark { "#a6e9b8" } else { "#225b42" });
    tags.number
        .set_property("foreground", if dark { "#5fd1b5" } else { "#168b78" });
    tags.current_line
        .set_property("background", if dark { "#1e241f" } else { "#e5f6e9" });
}

fn highlight_buffer(buffer: &gtk::TextBuffer, language: Language, tags: &SyntaxTags) {
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.remove_all_tags(&start, &end);
    let source = buffer.text(&start, &end, false);
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '"' {
            let first = index;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                let closes_string = chars[index] == '"';
                index += 1;
                if closes_string {
                    break;
                }
            }
            apply_tag_range(buffer, &tags.string, first, index);
            continue;
        }

        if chars[index].is_alphabetic() || chars[index] == '_' {
            let first = index;
            index += 1;
            while index < chars.len() && (chars[index].is_alphanumeric() || chars[index] == '_') {
                index += 1;
            }
            let word: String = chars[first..index].iter().collect();
            if language.keywords().contains(&word.as_str()) {
                apply_tag_range(buffer, &tags.keyword, first, index);
            }
            continue;
        }

        if chars[index].is_ascii_digit() {
            let first = index;
            index += 1;
            while index < chars.len() && chars[index].is_ascii_digit() {
                index += 1;
            }
            apply_tag_range(buffer, &tags.number, first, index);
            continue;
        }

        index += 1;
    }

    if let Some(mut line) = buffer.iter_at_line(6) {
        let first = line.offset() as usize;
        line.forward_to_line_end();
        apply_tag_range(buffer, &tags.current_line, first, line.offset() as usize);
    }
}

fn apply_tag_range(buffer: &gtk::TextBuffer, tag: &gtk::TextTag, first: usize, last: usize) {
    if first >= last {
        return;
    }
    let start = buffer.iter_at_offset(first as i32);
    let end = buffer.iter_at_offset(last as i32);
    buffer.apply_tag(tag, &start, &end);
}

fn update_line_numbers(buffer: &gtk::TextBuffer, gutter: &gtk::Label) {
    let count = buffer.line_count().max(1);
    let numbers = (1..=count)
        .map(|line| line.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    gutter.set_text(&numbers);
}

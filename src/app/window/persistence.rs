use super::*;

static NEXT_ATOMIC_SAVE_ID: AtomicU64 = AtomicU64::new(1);

pub(super) fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let file_name = path.file_name().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "destination has no file name")
    })?;
    let mut created = None;
    for _ in 0..16 {
        let id = NEXT_ATOMIC_SAVE_ID.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            id
        ));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => {
                created = Some((temporary, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    let (temporary, mut file) = created.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "could not allocate temporary save file",
        )
    })?;
    let result = (|| {
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

#[derive(Clone, Copy)]
pub(super) struct Preferences {
    pub(super) theme: ThemePreference,
    pub(super) font_size: u32,
    pub(super) line_numbers_visible: bool,
    pub(super) word_wrap: bool,
    pub(super) indent_width: u32,
    pub(super) default_language: Language,
    pub(super) ui_language: UiLanguage,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            font_size: 14,
            line_numbers_visible: true,
            word_wrap: false,
            indent_width: 4,
            default_language: Language::GPortugol,
            ui_language: UiLanguage::from_system(),
        }
    }
}

#[derive(Clone)]
pub(super) struct SessionDocument {
    pub(super) path: Option<PathBuf>,
    pub(super) name: String,
    pub(super) language: Language,
    pub(super) contents: String,
    pub(super) saved_contents: String,
}

pub(super) struct SessionSnapshot {
    pub(super) clean_exit: bool,
    pub(super) project_root: Option<PathBuf>,
    pub(super) active_index: usize,
    pub(super) documents: Vec<SessionDocument>,
}

impl MainWindow {
    pub(super) fn text(&self, portuguese: &'static str) -> &'static str {
        tr(self.ui_language, portuguese)
    }

    pub(super) fn preferences(&self, ui_language: UiLanguage) -> Preferences {
        Preferences {
            theme: self.theme,
            font_size: self.font_size,
            line_numbers_visible: self.line_numbers_visible,
            word_wrap: self.word_wrap,
            indent_width: self.indent_width,
            default_language: self.new_file_language,
            ui_language,
        }
    }

    pub(super) fn save_preferences(&mut self) {
        let preferences = self.preferences(self.preferred_ui_language);
        if let Err(error) = persist_preferences(preferences) {
            self.output_status = format!(
                "{}: {error}",
                self.text("Não foi possível salvar as configurações")
            );
        }
    }

    pub(super) fn persist_session(&mut self, clean_exit: bool, discard_unsaved: bool) {
        if let Err(error) = persist_session(self, clean_exit, discard_unsaved) {
            self.output_status =
                format!("{}: {error}", self.text("Não foi possível salvar a sessão"));
        }
    }

    pub(super) fn remember_recent(&mut self, path: PathBuf) {
        let Some(path) = normalize_recent_path(path) else {
            return;
        };
        self.recent_paths.retain(|recent| recent != &path);
        self.recent_paths.insert(0, path);
        self.recent_paths.truncate(MAX_RECENT_ITEMS);
        if let Err(error) = persist_recent_paths(&self.recent_paths) {
            self.output_status = format!(
                "{}: {error}",
                self.text("Não foi possível atualizar programas recentes")
            );
        }
    }

    pub(super) fn forget_recent(&mut self, path: &Path) -> Result<(), String> {
        self.recent_paths.retain(|recent| recent != path);
        persist_recent_paths(&self.recent_paths)
    }

    pub(super) fn finish_language_change(&mut self, sender: ComponentSender<Self>) {
        let Some(language) = self.pending_ui_language else {
            return;
        };

        if let Some(document_id) = self
            .documents
            .iter()
            .find(|document| document.is_dirty() && document.path.is_none())
            .map(|document| document.id)
        {
            self.pending_language_save_as_id = Some(document_id);
            self.pending_save_as_id = Some(document_id);
            self.output_status = self
                .text("Salve este arquivo antes de alterar o idioma.")
                .to_string();
            return;
        }

        let dirty_files = self
            .documents
            .iter()
            .filter(|document| document.is_dirty())
            .filter_map(|document| document.path.clone().map(|path| (document.id, path)))
            .collect::<Vec<_>>();
        for (id, path) in dirty_files {
            if !self.save_document(id, path, sender.clone()) {
                self.abort_language_change(false);
                return;
            }
        }

        if let Err(error) = persist_session(self, false, false) {
            self.output_status =
                format!("{}: {error}", self.text("Não foi possível salvar a sessão"));
            self.abort_language_change(false);
            return;
        }
        if let Err(error) = persist_preferences(self.preferences(language)) {
            self.output_status = format!(
                "{}: {error}",
                self.text("Não foi possível salvar as configurações")
            );
            self.abort_language_change(false);
            return;
        }

        self.preferred_ui_language = language;
        match crate::restart_application() {
            Ok(()) => {
                #[cfg(windows)]
                {
                    self.pending_ui_language = None;
                    self.reopen_settings_after_language_restart = false;
                    self.close_app_ready = true;
                }
            }
            Err(error) => {
                self.preferred_ui_language = self.ui_language;
                if let Err(rollback_error) = persist_preferences(self.preferences(self.ui_language))
                {
                    self.output_status = format!(
                        "{}: {error}; {}: {rollback_error}",
                        self.text("Não foi possível reiniciar o aplicativo"),
                        self.text("Não foi possível restaurar as configurações")
                    );
                } else {
                    self.output_status = format!(
                        "{}: {error}",
                        self.text("Não foi possível reiniciar o aplicativo")
                    );
                }
                self.abort_language_change(false);
            }
        }
    }

    pub(super) fn abort_language_change(&mut self, canceled: bool) {
        self.pending_ui_language = None;
        self.pending_language_save_as_id = None;
        self.pending_save_as_id = None;
        self.preferred_ui_language = self.ui_language;
        if canceled {
            self.output_status = self
                .text("A alteração de idioma foi cancelada.")
                .to_string();
        }
        self.settings_visible = std::mem::take(&mut self.reopen_settings_after_language_restart);
    }
}

pub(super) fn preference_file() -> PathBuf {
    gtk::glib::user_config_dir()
        .join("potoo-ide")
        .join("settings.ini")
}

pub(super) fn session_file() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("potoo-ide")
        .join("session.ini")
}

pub(super) fn recent_file() -> PathBuf {
    gtk::glib::user_data_dir()
        .join("potoo-ide")
        .join("recent.ini")
}

pub(super) const MAX_RECENT_ITEMS: usize = 5;

pub(super) fn normalize_recent_path(path: PathBuf) -> Option<PathBuf> {
    let path = path.canonicalize().unwrap_or(path);
    if path.is_dir() || (path.is_file() && Language::from_path(&path).is_some()) {
        Some(path)
    } else {
        None
    }
}

pub(super) fn load_recent_paths() -> Vec<PathBuf> {
    let Some(key_file) = load_key_file(&recent_file()) else {
        return Vec::new();
    };
    let count = key_file
        .integer("recent", "count")
        .unwrap_or(0)
        .clamp(0, MAX_RECENT_ITEMS as i32) as usize;
    let mut paths = Vec::with_capacity(count);
    for index in 0..count {
        let key = format!("path-{index}");
        let Ok(path) = key_file.string("recent", &key) else {
            continue;
        };
        let Some(path) = normalize_recent_path(PathBuf::from(path.as_str())) else {
            continue;
        };
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths
}

pub(super) fn persist_recent_paths(paths: &[PathBuf]) -> Result<(), String> {
    let key_file = gtk::glib::KeyFile::new();
    let paths = paths.iter().take(MAX_RECENT_ITEMS).collect::<Vec<_>>();
    key_file.set_integer("recent", "count", paths.len() as i32);
    for (index, path) in paths.into_iter().enumerate() {
        key_file.set_string("recent", &format!("path-{index}"), &path.to_string_lossy());
    }
    save_key_file(&key_file, &recent_file())
}

pub(super) fn load_key_file(path: &Path) -> Option<gtk::glib::KeyFile> {
    let key_file = gtk::glib::KeyFile::new();
    if key_file
        .load_from_file(path, gtk::glib::KeyFileFlags::NONE)
        .is_ok()
    {
        return Some(key_file);
    }

    let temporary_path = path.with_extension("ini.tmp");
    let fallback = gtk::glib::KeyFile::new();
    fallback
        .load_from_file(&temporary_path, gtk::glib::KeyFileFlags::NONE)
        .ok()
        .map(|()| fallback)
}

pub(super) fn save_key_file(key_file: &gtk::glib::KeyFile, path: &Path) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "caminho de configuração inválido".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary_path = path.with_extension("ini.tmp");
    key_file
        .save_to_file(&temporary_path)
        .map_err(|error| error.to_string())?;

    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).map_err(|error| error.to_string())?;
    }

    fs::rename(&temporary_path, path).map_err(|error| error.to_string())
}

pub(super) fn load_preferences() -> Preferences {
    let Some(key_file) = load_key_file(&preference_file()) else {
        return Preferences::default();
    };
    let defaults = Preferences::default();
    let settings_version = key_file.integer("editor", "settings-version").unwrap_or(1);
    Preferences {
        theme: key_file
            .string("editor", "theme")
            .map(|value| ThemePreference::from_key(&value))
            .unwrap_or(defaults.theme),
        font_size: key_file
            .integer("editor", "font-size")
            .ok()
            .map(|size| {
                if settings_version < 2 {
                    match size {
                        13 => 14,
                        15 => 16,
                        17 => 18,
                        14 | 16 | 18 => size as u32,
                        _ => defaults.font_size,
                    }
                } else {
                    match size {
                        14 | 15 | 16 | 18 => size as u32,
                        _ => defaults.font_size,
                    }
                }
            })
            .unwrap_or(defaults.font_size),
        line_numbers_visible: key_file
            .boolean("editor", "line-numbers")
            .unwrap_or(defaults.line_numbers_visible),
        word_wrap: key_file
            .boolean("editor", "word-wrap")
            .unwrap_or(defaults.word_wrap),
        indent_width: key_file
            .integer("editor", "indent-width")
            .ok()
            .filter(|width| matches!(width, 2 | 4))
            .map(|width| width as u32)
            .unwrap_or(defaults.indent_width),
        default_language: key_file
            .string("editor", "default-language")
            .map(|value| Language::from_key(&value))
            .unwrap_or(defaults.default_language),
        ui_language: UiLanguage::from_preference(
            key_file.string("interface", "language").ok().as_deref(),
            gtk::glib::language_names(),
        ),
    }
}

pub(super) fn persist_preferences(preferences: Preferences) -> Result<(), String> {
    let key_file = gtk::glib::KeyFile::new();
    key_file.set_string("editor", "theme", preferences.theme.key());
    key_file.set_integer("editor", "settings-version", 2);
    key_file.set_integer("editor", "font-size", preferences.font_size as i32);
    key_file.set_boolean("editor", "line-numbers", preferences.line_numbers_visible);
    key_file.set_boolean("editor", "word-wrap", preferences.word_wrap);
    key_file.set_integer("editor", "indent-width", preferences.indent_width as i32);
    key_file.set_string(
        "editor",
        "default-language",
        preferences.default_language.key(),
    );
    key_file.set_string("interface", "language", preferences.ui_language.key());
    save_key_file(&key_file, &preference_file())
}

pub(super) fn load_session() -> Option<SessionSnapshot> {
    let key_file = load_key_file(&session_file())?;
    let document_count = key_file
        .integer("session", "document-count")
        .ok()?
        .clamp(0, 500) as usize;
    let mut documents = Vec::with_capacity(document_count);
    for index in 0..document_count {
        let group = format!("document-{index}");
        let path = key_file
            .string(&group, "path")
            .ok()
            .and_then(|path| (!path.is_empty()).then(|| PathBuf::from(path.as_str())));
        let contents = key_file
            .string(&group, "contents")
            .map(|value| value.to_string())
            .ok()?;
        let saved_contents = key_file
            .string(&group, "saved-contents")
            .map(|value| value.to_string())
            .ok()?;
        let name = key_file
            .string(&group, "name")
            .map(|value| value.to_string())
            .unwrap_or_else(|_| "principal.por".to_string());
        let language = key_file
            .string(&group, "language")
            .map(|value| Language::from_key(&value))
            .unwrap_or_else(|_| {
                Language::from_path(path.as_deref().unwrap_or(Path::new(".por")))
                    .unwrap_or(Language::Portugol)
            });
        documents.push(SessionDocument {
            path,
            name,
            language,
            contents,
            saved_contents,
        });
    }

    let project_root = key_file
        .string("session", "project-root")
        .ok()
        .and_then(|path| (!path.is_empty()).then(|| PathBuf::from(path.as_str())));
    Some(SessionSnapshot {
        clean_exit: key_file.boolean("session", "clean-exit").unwrap_or(true),
        project_root,
        active_index: key_file
            .integer("session", "active-index")
            .unwrap_or(0)
            .max(0) as usize,
        documents,
    })
}

pub(super) fn persist_session(
    model: &MainWindow,
    clean_exit: bool,
    discard_unsaved: bool,
) -> Result<(), String> {
    let key_file = gtk::glib::KeyFile::new();
    key_file.set_boolean("session", "clean-exit", clean_exit);
    key_file.set_integer("session", "document-count", model.documents.len() as i32);
    key_file.set_integer(
        "session",
        "active-index",
        model
            .documents
            .iter()
            .position(|document| Some(document.id) == model.active_document_id)
            .unwrap_or(0) as i32,
    );
    let project_root = model
        .project_root
        .as_deref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_default();
    key_file.set_string("session", "project-root", &project_root);

    for (index, document) in model.documents.iter().enumerate() {
        let group = format!("document-{index}");
        let path = document
            .path
            .as_deref()
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_default();
        key_file.set_string(&group, "path", &path);
        key_file.set_string(&group, "name", &document.name);
        key_file.set_string(&group, "language", document.language.key());
        let contents = if discard_unsaved {
            &document.saved_contents
        } else {
            &document.contents
        };
        key_file.set_string(&group, "contents", contents);
        key_file.set_string(&group, "saved-contents", &document.saved_contents);
    }
    save_key_file(&key_file, &session_file())
}

pub(super) fn restore_session(
    snapshot: SessionSnapshot,
) -> (Vec<SourceDocument>, Option<u64>, bool) {
    let mut documents = Vec::new();
    let mut active_document_id = None;
    let mut recovered_changes = false;
    for (index, saved) in snapshot.documents.into_iter().enumerate() {
        let (contents, saved_contents) = if snapshot.clean_exit {
            match saved.path.as_deref() {
                Some(path) => match fs::read_to_string(path) {
                    Ok(contents) => (contents.clone(), contents),
                    Err(_) => continue,
                },
                None => (saved.saved_contents.clone(), saved.saved_contents),
            }
        } else {
            (saved.contents, saved.saved_contents)
        };
        let id = documents.len() as u64 + 1;
        recovered_changes |= contents != saved_contents;
        if index == snapshot.active_index {
            active_document_id = Some(id);
        }
        documents.push(SourceDocument {
            id,
            path: saved.path,
            name: saved.name,
            language: saved.language,
            contents,
            saved_contents,
            revision: 0,
            diagnostics: None,
            runtime_diagnostics: Vec::new(),
        });
    }
    if active_document_id.is_none() {
        active_document_id = documents.first().map(|document| document.id);
    }
    (
        documents,
        active_document_id,
        !snapshot.clean_exit && recovered_changes,
    )
}

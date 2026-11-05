use super::*;

pub(super) fn new_source_path(
    root: &Path,
    name: &str,
    default_language: Language,
    ui_language: UiLanguage,
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
        return Err(tr(
            ui_language,
            "Use um nome simples, sem pastas ou caracteres especiais.",
        )
        .to_string());
    }

    let requested_path = Path::new(name);
    let language = match requested_path.extension() {
        Some(_) => Language::from_path(requested_path).ok_or_else(|| {
            tr(
                ui_language,
                "Use a extensão .por, .alg, .poti, .gpt, .clg ou .mapler.",
            )
            .to_string()
        })?,
        None => default_language,
    };
    let file_name = if requested_path.extension().is_some() {
        name.to_string()
    } else {
        format!("{name}.{}", language.extension())
    };
    Ok((root.join(file_name), language))
}

pub(super) fn resolve_project_directory(
    root: &Path,
    relative_directory: &Path,
    ui_language: UiLanguage,
) -> Result<PathBuf, String> {
    if relative_directory
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(tr(ui_language, "A pasta selecionada não está disponível.").to_string());
    }

    let project_root = fs::canonicalize(root)
        .map_err(|_| tr(ui_language, "A pasta selecionada não está disponível.").to_string())?;
    let directory = fs::canonicalize(project_root.join(relative_directory))
        .map_err(|_| tr(ui_language, "A pasta selecionada não está disponível.").to_string())?;
    if directory.is_dir() && directory.starts_with(&project_root) {
        Ok(directory)
    } else {
        Err(tr(ui_language, "A pasta selecionada não está disponível.").to_string())
    }
}

pub(super) fn is_reserved_windows_file_name(name: &str) -> bool {
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

pub(super) fn is_valid_directory_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && !name.ends_with('.')
        && !name.contains('/')
        && !name.contains('\\')
        && !is_reserved_windows_file_name(name)
        && !name
            .chars()
            .any(|character| character.is_control() || "<>:\"|?*".contains(character))
}

pub(super) fn normalized_project_item_name(
    path: &Path,
    name: &str,
    is_directory: bool,
    ui_language: UiLanguage,
) -> Result<String, String> {
    let name = name.trim();
    if !is_valid_directory_name(name) {
        return Err(tr(
            ui_language,
            "Use um nome simples, sem pastas ou caracteres especiais.",
        )
        .to_string());
    }

    if !is_directory && let Some(language) = Language::from_path(path) {
        let requested_path = Path::new(name);
        return match requested_path.extension() {
            None => Ok(format!("{name}.{}", language.extension())),
            Some(_) if Language::from_path(requested_path).is_some() => Ok(name.to_string()),
            Some(_) => Err(tr(
                ui_language,
                "Use a extensão .por, .alg, .poti, .gpt, .clg ou .mapler.",
            )
            .to_string()),
        };
    }

    Ok(name.to_string())
}

pub(super) fn supported_save_path(
    path: PathBuf,
    default_language: Language,
    ui_language: UiLanguage,
) -> Result<(PathBuf, Language), String> {
    if default_language == Language::Mapler
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("txt"))
    {
        return Ok((path.with_extension("mapler"), Language::Mapler));
    }
    match path.extension() {
        Some(_) => Language::from_path(&path)
            .map(|language| (path, language))
            .ok_or_else(|| {
                tr(
                    ui_language,
                    "Salve o código com a extensão .por, .alg, .poti, .gpt, .clg ou .mapler.",
                )
                .to_string()
            }),
        None => Ok((
            path.with_extension(default_language.extension()),
            default_language,
        )),
    }
}

pub(super) fn scan_project_files(root: &Path) -> std::io::Result<Vec<ProjectFile>> {
    let mut entries = Vec::new();
    scan_source_directory(root, root, &mut entries)?;
    Ok(entries)
}

pub(super) fn scan_source_directory(
    root: &Path,
    directory: &Path,
    project_entries: &mut Vec<ProjectFile>,
) -> std::io::Result<()> {
    let mut children = fs::read_dir(directory)?.collect::<Result<Vec<_>, _>>()?;
    children.sort_by(|left, right| {
        let left_is_directory = left.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        let right_is_directory = right.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        right_is_directory.cmp(&left_is_directory).then_with(|| {
            left.file_name()
                .to_string_lossy()
                .to_lowercase()
                .cmp(&right.file_name().to_string_lossy().to_lowercase())
        })
    });
    for entry in children {
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
            project_entries.push(ProjectFile {
                relative_path: path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned(),
                path: path.clone(),
                is_directory: true,
            });
            scan_source_directory(root, &path, project_entries)?;
        } else if file_type.is_file() {
            project_entries.push(ProjectFile {
                relative_path: path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned(),
                path,
                is_directory: false,
            });
        }
    }
    Ok(())
}

use super::*;

pub(super) fn safe_project_file(root: Option<&Path>, relative: &str) -> Result<PathBuf, String> {
    safe_project_path(root, relative, false)?
        .ok_or_else(|| "arquivo do projeto indisponível".to_owned())
}

pub(super) fn safe_project_image_file(
    root: Option<&Path>,
    relative: &str,
) -> Result<Option<PathBuf>, String> {
    safe_project_path(root, relative, true)
}

pub(super) fn safe_project_path(
    root: Option<&Path>,
    relative: &str,
    allow_missing_file: bool,
) -> Result<Option<PathBuf>, String> {
    let root = root.ok_or_else(|| "abra um projeto para carregar recursos gráficos".to_owned())?;
    let root = std::fs::canonicalize(root)
        .map_err(|error| format!("pasta do projeto indisponível: {error}"))?;
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("o caminho deve permanecer dentro do projeto".to_owned());
    }
    let candidate = root.join(relative_path);
    match std::fs::canonicalize(&candidate) {
        Ok(path) if path.starts_with(&root) && path.is_file() => Ok(Some(path)),
        Ok(_) => Err("o caminho deve apontar para um arquivo do projeto".to_owned()),
        Err(error) if allow_missing_file && error.kind() == std::io::ErrorKind::NotFound => {
            match std::fs::symlink_metadata(&candidate) {
                Ok(metadata) => {
                    if metadata.file_type().is_symlink() {
                        return Err("o caminho deve apontar para um arquivo do projeto".to_owned());
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("arquivo do projeto indisponível: {error}")),
            }
            if candidate.file_name().is_none() {
                return Err("o caminho deve apontar para um arquivo do projeto".to_owned());
            }
            let mut parent = candidate.parent().unwrap_or(&root);
            loop {
                match std::fs::symlink_metadata(parent) {
                    Ok(metadata) if metadata.is_dir() || metadata.file_type().is_symlink() => {
                        let parent = std::fs::canonicalize(parent)
                            .map_err(|error| format!("arquivo do projeto indisponível: {error}"))?;
                        return if parent.starts_with(&root) && parent.is_dir() {
                            Ok(None)
                        } else {
                            Err("o caminho deve apontar para um arquivo do projeto".to_owned())
                        };
                    }
                    Ok(_) => {
                        return Err("o caminho deve apontar para um arquivo do projeto".to_owned());
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        parent = parent
                            .parent()
                            .ok_or_else(|| "arquivo do projeto indisponível".to_owned())?;
                    }
                    Err(error) => {
                        return Err(format!("arquivo do projeto indisponível: {error}"));
                    }
                }
            }
        }
        Err(error) => Err(format!("arquivo do projeto indisponível: {error}")),
    }
}

pub(super) fn safe_project_output(root: Option<&Path>, relative: &str) -> Result<PathBuf, String> {
    let root = root.ok_or_else(|| "abra um projeto para salvar imagens".to_owned())?;
    let root = std::fs::canonicalize(root)
        .map_err(|error| format!("pasta do projeto indisponível: {error}"))?;
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("o caminho deve permanecer dentro do projeto".to_owned());
    }
    let file_name = relative_path
        .file_name()
        .ok_or_else(|| "o caminho do arquivo é inválido".to_owned())?;
    let relative_parent = relative_path.parent().unwrap_or_else(|| Path::new(""));
    let parent = std::fs::canonicalize(root.join(relative_parent))
        .map_err(|error| format!("pasta de destino indisponível: {error}"))?;
    if !parent.starts_with(&root) {
        return Err("a pasta de destino sai da pasta do projeto".to_owned());
    }
    let target = parent.join(file_name);
    match std::fs::symlink_metadata(&target) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return Err("o arquivo de destino não pode ser um link simbólico".to_owned());
        }
        Ok(_) => {
            let canonical = std::fs::canonicalize(&target)
                .map_err(|error| format!("arquivo de destino inválido: {error}"))?;
            if !canonical.starts_with(&root) || !canonical.is_file() {
                return Err("o arquivo de destino sai da pasta do projeto".to_owned());
            }
            Ok(canonical)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(target),
        Err(error) => Err(format!("arquivo de destino inválido: {error}")),
    }
}

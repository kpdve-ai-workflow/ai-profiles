use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

use serde_json::Value;

use crate::{codec::sha, error::FilesystemError};

pub(crate) fn file_path(project: &Path, relative: &str) -> Result<PathBuf, FilesystemError> {
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(FilesystemError::InvalidManagedPath(relative.to_owned()));
    }

    let mut path = project.to_owned();
    for component in relative_path.components() {
        if let Component::Normal(part) = component {
            path.push(part);
            if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
                return Err(FilesystemError::Symlink(path));
            }
        }
    }
    Ok(project.join(relative_path))
}

/// Validate ownership and every destination before the caller starts writing.
pub(crate) fn check_files(
    project: &Path,
    old_state: Option<&Value>,
    next_files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), FilesystemError> {
    let owned = old_state
        .and_then(|state| state.get("files"))
        .and_then(Value::as_object);

    if let Some(owned) = owned {
        for (relative, expected_hash) in owned {
            let path = file_path(project, relative)?;
            let expected_hash = expected_hash.as_str().unwrap_or_default();
            let actual = fs::metadata(&path)
                .ok()
                .filter(std::fs::Metadata::is_file)
                .and_then(|_| fs::read(&path).ok())
                .map(|contents| sha(&contents));
            if actual.as_deref() != Some(expected_hash) {
                return Err(FilesystemError::ManagedFileChanged {
                    relative: relative.clone(),
                });
            }
        }
    }

    for relative in next_files.keys() {
        let path = file_path(project, relative)?;
        let exists = match fs::symlink_metadata(&path) {
            Ok(_) => true,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(source) => {
                return Err(FilesystemError::Io { path, source });
            }
        };
        if exists && !owned.is_some_and(|files| files.contains_key(relative)) {
            return Err(FilesystemError::UnmanagedFileExists(relative.clone()));
        }
    }
    Ok(())
}

pub(crate) fn write_bytes(path: &Path, contents: &[u8]) -> Result<(), FilesystemError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| FilesystemError::Io {
            path: parent.to_owned(),
            source,
        })?;
    }
    fs::write(path, contents).map_err(|source| FilesystemError::Io {
        path: path.to_owned(),
        source,
    })
}

pub(crate) fn cleanup_empty_directories(
    project: &Path,
    paths: &[String],
) -> Result<(), FilesystemError> {
    for relative in paths {
        let managed = file_path(project, relative)?;
        let mut directory = managed.parent().map(Path::to_owned);
        while let Some(path) = directory {
            if path == project || path == project.join(".opencode") {
                break;
            }
            match fs::remove_dir(&path) {
                Ok(()) => directory = path.parent().map(Path::to_owned),
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
                    ) =>
                {
                    break
                }
                Err(source) => {
                    return Err(FilesystemError::Io { path, source });
                }
            }
        }
    }
    let opencode = project.join(".opencode");
    match fs::remove_dir(&opencode) {
        Ok(()) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound | std::io::ErrorKind::DirectoryNotEmpty
            ) =>
        {
            Ok(())
        }
        Err(source) => Err(FilesystemError::Io {
            path: opencode,
            source,
        }),
    }
}

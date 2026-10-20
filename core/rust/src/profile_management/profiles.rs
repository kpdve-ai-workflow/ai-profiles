use std::{fs, io::Write, path::Path, str::FromStr};

use serde_json::{json, Value};
use tempfile::Builder;

use crate::{
    codec::json_bytes,
    error::ProfileManagementError,
    filesystem::file_path,
    model::{ProfileName, RepositoryContext, Version, VersionBump},
    repository::{profile_metadata, profile_names},
};

pub(crate) fn create_profile(
    context: &RepositoryContext,
    profile: &str,
    description: Option<&str>,
) -> Result<(), ProfileManagementError> {
    let profile_name =
        ProfileName::from_str(profile).map_err(|_| ProfileManagementError::InvalidProfileName)?;
    if description.is_some_and(|value| value.trim().is_empty()) {
        return Err(ProfileManagementError::EmptyDescription);
    }

    let relative = format!("profiles/{}", profile_name.as_str());
    let destination = file_path(&context.sources, &relative)?;
    if destination.exists() {
        return Err(ProfileManagementError::ProfileAlreadyExists(
            profile.to_owned(),
        ));
    }
    let parent = destination
        .parent()
        .ok_or_else(|| ProfileManagementError::InvalidProfileName)?;
    let temporary = Builder::new()
        .prefix(".create-")
        .tempdir_in(parent)
        .map_err(|source| ProfileManagementError::Io {
            path: parent.to_owned(),
            source,
        })?;
    let staged = temporary.path().join(profile_name.as_str());
    let skills = staged.join("skills");
    fs::create_dir_all(&skills).map_err(|source| ProfileManagementError::Io {
        path: skills.clone(),
        source,
    })?;

    let description = description
        .map(str::trim)
        .map(str::to_owned)
        .unwrap_or_else(|| format!("Profil {}", profile_name.as_str()));
    let metadata = json!({
        "name": profile_name.as_str(),
        "version": "1.0.0",
        "description": description,
    });
    write_staged(&staged.join("profile.json"), &json_bytes(&metadata)?)?;
    let mcp = json!({"servers": []});
    write_staged(&staged.join("mcp.json"), &json_bytes(&mcp)?)?;
    write_staged(&skills.join(".gitkeep"), &[])?;

    fs::create_dir(&destination).map_err(|source| ProfileManagementError::Io {
        path: destination.clone(),
        source,
    })?;
    let entries = fs::read_dir(&staged).map_err(|_| ProfileManagementError::CreateFailed {
        destination: destination.clone(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|_| ProfileManagementError::CreateFailed {
            destination: destination.clone(),
        })?;
        let target = destination.join(entry.file_name());
        fs::rename(entry.path(), &target).map_err(|_| ProfileManagementError::CreateFailed {
            destination: destination.clone(),
        })?;
    }

    println!("OK: utworzono profil {} (1.0.0)", profile_name.as_str());
    Ok(())
}

fn write_staged(path: &Path, contents: &[u8]) -> Result<(), ProfileManagementError> {
    let mut file = fs::File::create(path).map_err(|source| ProfileManagementError::Io {
        path: path.to_owned(),
        source,
    })?;
    file.write_all(contents)
        .map_err(|source| ProfileManagementError::Io {
            path: path.to_owned(),
            source,
        })
}

pub(crate) fn bump_profile(
    context: &RepositoryContext,
    profile: &str,
    part: VersionBump,
) -> Result<String, ProfileManagementError> {
    if !profile_names(context)?.iter().any(|name| name == profile) {
        return Err(ProfileManagementError::UnknownProfile(profile.to_owned()));
    }
    let relative = format!("profiles/{profile}/profile.json");
    let path = file_path(&context.sources, &relative)?;
    let mut metadata = profile_metadata(context, profile)?;
    let previous = metadata
        .get("version")
        .and_then(Value::as_str)
        .ok_or_else(|| crate::error::RepositoryError::InvalidMetadata(profile.to_owned()))?
        .to_owned();
    let next = Version::from_str(&previous)?.bump(part).as_string();
    let object = metadata
        .as_object_mut()
        .ok_or_else(|| crate::error::RepositoryError::InvalidMetadata(profile.to_owned()))?;
    object.insert("version".to_owned(), Value::String(next.clone()));

    let parent = path
        .parent()
        .ok_or_else(|| ProfileManagementError::Persist("brak katalogu metadanych".to_owned()))?;
    let mut temporary = Builder::new()
        .prefix(".profile-")
        .suffix(".tmp")
        .tempfile_in(parent)
        .map_err(|source| ProfileManagementError::Io {
            path: parent.to_owned(),
            source,
        })?;
    temporary
        .write_all(&json_bytes(&metadata)?)
        .map_err(|source| ProfileManagementError::Io {
            path: temporary.path().to_owned(),
            source,
        })?;
    let permissions = fs::metadata(&path)
        .map_err(|source| ProfileManagementError::Io {
            path: path.clone(),
            source,
        })?
        .permissions();
    set_temporary_permissions(&mut temporary, permissions)?;
    temporary
        .persist(&path)
        .map_err(|error| ProfileManagementError::Persist(error.error.to_string()))?;

    println!("OK: {}: {} -> {}", profile, previous, next);
    Ok(next)
}

#[cfg(unix)]
fn set_temporary_permissions(
    temporary: &mut tempfile::NamedTempFile,
    permissions: fs::Permissions,
) -> Result<(), ProfileManagementError> {
    use std::os::unix::fs::PermissionsExt;

    let mode = permissions.mode() & 0o777;
    let permissions = fs::Permissions::from_mode(mode);
    temporary
        .as_file()
        .set_permissions(permissions)
        .map_err(|source| ProfileManagementError::Io {
            path: temporary.path().to_owned(),
            source,
        })
}

#[cfg(not(unix))]
fn set_temporary_permissions(
    temporary: &mut tempfile::NamedTempFile,
    permissions: fs::Permissions,
) -> Result<(), ProfileManagementError> {
    temporary
        .as_file()
        .set_permissions(permissions)
        .map_err(|source| ProfileManagementError::Io {
            path: temporary.path().to_owned(),
            source,
        })
}

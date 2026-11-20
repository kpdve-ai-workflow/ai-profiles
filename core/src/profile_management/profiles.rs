use std::{fs, io::Cursor, io::Write, path::Path, str::FromStr};

use serde_json::{json, Value};
use tempfile::Builder;

use crate::{
    codec::json_bytes,
    error::ProfileManagementError,
    filesystem::file_path,
    model::{ProfileAction, ProfileName, RepositoryContext, Version, VersionBump},
    repository::{profile_metadata, profile_names},
};

pub(crate) fn create_profile(
    context: &RepositoryContext,
    profile: &str,
    description: Option<&str>,
) -> Result<(), ProfileManagementError> {
    super::diagnostics::repository_operation(
        ProfileAction::Create,
        &context.sources,
        |operation_id| create_profile_inner(context, profile, description, operation_id),
    )
}

fn create_profile_inner(
    context: &RepositoryContext,
    profile: &str,
    description: Option<&str>,
    operation_id: &str,
) -> Result<(), ProfileManagementError> {
    tracing::info!(
        event = "profile.profile.create.started",
        operation_id,
        profile,
        description = description.unwrap_or(""),
        description_provided = description.is_some(),
    );
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
        .ok_or(ProfileManagementError::InvalidProfileName)?;
    let temporary = Builder::new()
        .prefix(".create-")
        .tempdir_in(parent)
        .map_err(|source| ProfileManagementError::Io {
            path: parent.to_owned(),
            source,
        })?;
    let staged = temporary.path().join(profile_name.as_str());
    fs::create_dir_all(&staged).map_err(|source| ProfileManagementError::Io {
        path: staged.clone(),
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
    let metadata_bytes = json_bytes(&metadata)?;
    write_staged(&staged.join("profile.json"), &metadata_bytes)?;
    record_payload(
        operation_id,
        "profile_metadata",
        "profile.json",
        &metadata_bytes,
    );
    let mcp = json!({"servers": []});
    let mcp_bytes = json_bytes(&mcp)?;
    write_staged(&staged.join("mcp.json"), &mcp_bytes)?;
    record_payload(operation_id, "mcp_selection", "mcp.json", &mcp_bytes);
    let skills_bytes = json_bytes(&json!({"skills": []}))?;
    write_staged(&staged.join("skills.json"), &skills_bytes)?;
    record_payload(
        operation_id,
        "skill_selection",
        "skills.json",
        &skills_bytes,
    );

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
        tracing::info!(
            event = "profile.profile.file_published",
            operation_id,
            profile = profile_name.as_str(),
            path = %format!("profiles/{}/{}", profile_name.as_str(), entry.file_name().to_string_lossy()),
            result = "success",
        );
    }

    tracing::info!(
        event = "profile.profile.created",
        operation_id,
        profile = profile_name.as_str(),
        version = "1.0.0",
        description,
        result = "success",
    );
    println!("OK: utworzono profil {} (1.0.0)", profile_name.as_str());
    Ok(())
}

pub(crate) fn rename_profile(
    context: &RepositoryContext,
    profile: &str,
    new_name: &str,
) -> Result<(), ProfileManagementError> {
    super::diagnostics::repository_operation(
        ProfileAction::Rename,
        &context.sources,
        |operation_id| rename_profile_inner(context, profile, new_name, operation_id),
    )
}

fn rename_profile_inner(
    context: &RepositoryContext,
    profile: &str,
    new_name: &str,
    operation_id: &str,
) -> Result<(), ProfileManagementError> {
    tracing::info!(
        event = "profile.profile.rename.started",
        operation_id,
        previous_name = profile,
        profile = new_name,
    );
    let old =
        ProfileName::from_str(profile).map_err(|_| ProfileManagementError::InvalidProfileName)?;
    let new =
        ProfileName::from_str(new_name).map_err(|_| ProfileManagementError::InvalidProfileName)?;
    if old == new {
        return Err(ProfileManagementError::SameProfileName);
    }
    let original = file_path(&context.sources, &format!("profiles/{}", old.as_str()))?;
    let destination = file_path(&context.sources, &format!("profiles/{}", new.as_str()))?;
    if !original.is_dir() {
        return Err(ProfileManagementError::UnknownProfile(profile.to_owned()));
    }
    if destination.exists() {
        return Err(ProfileManagementError::ProfileAlreadyExists(
            new_name.to_owned(),
        ));
    }
    let metadata_path = file_path(
        &context.sources,
        &format!("profiles/{profile}/profile.json"),
    )?;
    let mut metadata = profile_metadata(context, profile)?;
    let object = metadata
        .as_object_mut()
        .ok_or_else(|| crate::error::RepositoryError::InvalidMetadata(profile.to_owned()))?;
    object.insert("name".to_owned(), Value::String(new.as_str().to_owned()));
    let bytes = json_bytes(&metadata)?;
    record_payload(operation_id, "profile_metadata", "profile.json", &bytes);
    let permissions = fs::metadata(&metadata_path)
        .map_err(|source| ProfileManagementError::Io {
            path: metadata_path.clone(),
            source,
        })?
        .permissions();
    // Stage outside the moved directory so the temporary path remains valid.
    let parent = original
        .parent()
        .ok_or(ProfileManagementError::InvalidProfileName)?;
    let mut temporary = Builder::new()
        .prefix(".rename-")
        .tempfile_in(parent)
        .map_err(|source| ProfileManagementError::Io {
            path: parent.to_owned(),
            source,
        })?;
    temporary
        .write_all(&bytes)
        .map_err(|source| ProfileManagementError::Io {
            path: temporary.path().to_owned(),
            source,
        })?;
    set_temporary_permissions(&mut temporary, permissions)?;
    fs::rename(&original, &destination).map_err(|source| ProfileManagementError::Io {
        path: original.clone(),
        source,
    })?;
    if let Err(error) = temporary.persist(destination.join("profile.json")) {
        if let Err(rollback) = fs::rename(&destination, &original) {
            return Err(ProfileManagementError::RenameRollback {
                original,
                destination,
                source: error.error,
                rollback,
            });
        }
        return Err(ProfileManagementError::Persist(error.error.to_string()));
    }
    tracing::info!(
        event = "profile.profile.renamed",
        operation_id,
        previous_name = old.as_str(),
        profile = new.as_str(),
        path = %format!("profiles/{}/profile.json", new.as_str()),
        result = "success",
    );
    println!("OK: zmieniono nazwę profilu {profile} -> {new_name} w repozytorium ai-profiles. Zainstalowane projekty nie zostały zmienione.");
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
    super::diagnostics::repository_operation(
        ProfileAction::Bump,
        &context.sources,
        |operation_id| bump_profile_inner(context, profile, part, operation_id),
    )
}

fn bump_profile_inner(
    context: &RepositoryContext,
    profile: &str,
    part: VersionBump,
    operation_id: &str,
) -> Result<String, ProfileManagementError> {
    tracing::info!(
        event = "profile.profile.bump.started",
        operation_id,
        profile,
        part = match part {
            VersionBump::Major => "major",
            VersionBump::Minor => "minor",
            VersionBump::Patch => "patch",
        },
    );
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
    let bytes = json_bytes(&metadata)?;
    record_payload(operation_id, "profile_metadata", "profile.json", &bytes);

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
        .write_all(&bytes)
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

    tracing::info!(
        event = "profile.profile.version_bumped",
        operation_id,
        profile,
        previous_version = previous,
        version = next,
        path = %relative,
        result = "success",
    );
    println!("OK: {}: {} -> {}", profile, previous, next);
    Ok(next)
}

fn record_payload(operation_id: &str, role: &str, path: &str, bytes: &[u8]) {
    let Some(session) = crate::logging::current_session() else {
        return;
    };
    let summary = session.payload(Cursor::new(bytes));
    tracing::info!(
        event = "profile.repository.payload",
        operation_id,
        role,
        path,
        payload_id = summary.payload_id,
        bytes = summary.length,
        complete = summary.complete,
    );
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

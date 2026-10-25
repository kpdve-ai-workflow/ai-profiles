use std::{ffi::OsString, fs, path::Path, str::FromStr};

use serde_json::Value;

use crate::{
    error::RepositoryError,
    model::{ProfileName, RepositoryContext, Version},
    process::{
        CancellationToken, CommandSpec, ProcessRunner, StdinMode, StreamMode, SystemProcessRunner,
    },
};

pub(crate) fn is_source_root(path: &Path) -> bool {
    ["core/src", "profiles", "skills", "mcp/servers"]
        .into_iter()
        .all(|relative| path.join(relative).is_dir())
}

pub(crate) fn validate_source_root(path: &Path) -> Result<(), RepositoryError> {
    if is_source_root(path) {
        Ok(())
    } else {
        Err(RepositoryError::InvalidSourceRoot(path.to_owned()))
    }
}

pub(crate) fn profile_names(context: &RepositoryContext) -> Result<Vec<String>, RepositoryError> {
    let directory = context.sources.join("profiles");
    let entries = fs::read_dir(&directory).map_err(|source| RepositoryError::Io {
        path: directory,
        source,
    })?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| RepositoryError::Io {
            path: context.sources.join("profiles"),
            source,
        })?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                names.push(name.to_owned());
            }
        }
    }
    names.sort();
    Ok(names)
}

pub(crate) fn profile_metadata(
    context: &RepositoryContext,
    profile: &str,
) -> Result<Value, RepositoryError> {
    let _name = ProfileName::from_str(profile)?;
    let path = context
        .sources
        .join("profiles")
        .join(profile)
        .join("profile.json");
    let metadata = crate::codec::read_json(&path)?;
    let Some(object) = metadata.as_object() else {
        return Err(RepositoryError::InvalidMetadata(profile.to_owned()));
    };
    let valid_name = object.get("name").and_then(Value::as_str) == Some(profile);
    let valid_description = object
        .get("description")
        .and_then(Value::as_str)
        .is_some_and(|description| !description.trim().is_empty());
    let version = object.get("version").and_then(Value::as_str);
    if !valid_name || !valid_description {
        return Err(RepositoryError::InvalidMetadata(profile.to_owned()));
    }
    Version::from_str(
        version.ok_or_else(|| RepositoryError::InvalidMetadata(profile.to_owned()))?,
    )?;
    Ok(metadata)
}

pub(crate) fn source_commit(context: &RepositoryContext) -> Option<String> {
    source_commit_with(context, &SystemProcessRunner)
}

pub(crate) fn source_commit_with(
    context: &RepositoryContext,
    runner: &impl ProcessRunner,
) -> Option<String> {
    let output = runner
        .run(CommandSpec {
            program: OsString::from("git"),
            args: [
                OsString::from("-C"),
                context.sources.as_os_str().to_owned(),
                OsString::from("rev-parse"),
                OsString::from("HEAD"),
            ]
            .into(),
            cwd: None,
            env: Default::default(),
            clear_env: false,
            stdin: StdinMode::Null,
            stdout: StreamMode::Capture,
            stderr: StreamMode::Null,
            timeout: None,
            cancellation: CancellationToken::new(),
        })
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_owned())
}

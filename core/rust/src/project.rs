use std::{
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde_json::{json, Value};

use crate::{
    error::ProjectError,
    filesystem::file_path,
    model::{RepositoryContext, Version},
    repository::profile_names,
};

const STATE: &str = ".opencode/ai-profile-state.json";
const IGNORE_LINE: &str = ".opencode/workflow-approval.json";
const SCHEMA: &str = "https://opencode.ai/config.json";

pub(crate) fn selected_project(
    context: &RepositoryContext,
    project_option: Option<&Path>,
) -> Result<PathBuf, ProjectError> {
    let requested = match project_option {
        Some(path) => expand_home(path),
        None => env::current_dir().map_err(|source| ProjectError::Io {
            path: PathBuf::from("<current directory>"),
            source,
        })?,
    };
    let project = fs::canonicalize(&requested).map_err(|source| {
        if source.kind() == std::io::ErrorKind::NotFound {
            ProjectError::ProjectNotFound(requested.clone())
        } else {
            ProjectError::Io {
                path: requested.clone(),
                source,
            }
        }
    })?;
    if !project.is_dir() {
        return Err(ProjectError::ProjectNotFound(project));
    }
    if project == context.sources || project.starts_with(&context.sources) {
        return Err(ProjectError::ProjectInsideSources);
    }
    Ok(project)
}

fn expand_home(path: &Path) -> PathBuf {
    let Some(value) = path.to_str() else {
        return path.to_owned();
    };
    if value == "~" || value.starts_with("~/") {
        if let Some(home) = env::var_os("HOME") {
            let mut expanded = PathBuf::from(home);
            if value.len() > 2 {
                expanded.push(&value[2..]);
            }
            return expanded;
        }
    }
    path.to_owned()
}

pub(crate) fn state_for(
    context: &RepositoryContext,
    project: &Path,
) -> Result<Option<Value>, ProjectError> {
    let path = file_path(project, STATE)?;
    if !path.exists() {
        return Ok(None);
    }
    let value = crate::codec::read_json(&path)?;
    let Some(object) = value.as_object() else {
        return Err(ProjectError::UnsupportedState(path));
    };
    let format_ok = object.get("format").is_some_and(is_format_one);
    let files_ok = object.get("files").is_some_and(Value::is_object);
    let mcp_ok = object.get("mcp").is_some_and(Value::is_object);
    if !format_ok || !files_ok || !mcp_ok {
        return Err(ProjectError::UnsupportedState(path));
    }
    let available_profiles = profile_names(context)?;
    let profile_ok = object
        .get("profile")
        .and_then(Value::as_str)
        .is_some_and(|name| available_profiles.iter().any(|profile| profile == name));
    if !profile_ok {
        return Err(ProjectError::UnsupportedState(path));
    }
    if let Some(version) = object.get("version") {
        if !version.is_null() {
            let Some(version) = version.as_str() else {
                return Err(ProjectError::UnsupportedState(path));
            };
            Version::from_str(version).map_err(|_| ProjectError::UnsupportedState(path.clone()))?;
        }
    }
    Ok(Some(value))
}

fn is_format_one(value: &Value) -> bool {
    if value.as_bool() == Some(true) || value.as_i64() == Some(1) || value.as_u64() == Some(1) {
        return true;
    }
    value.as_f64() == Some(1.0)
}

pub(crate) fn config_for(project: &Path) -> Result<(Value, bool), ProjectError> {
    if project.join("opencode.jsonc").exists() {
        return Err(ProjectError::JsoncPresent);
    }
    let path = file_path(project, "opencode.json")?;
    let existed = path.exists();
    let config = if existed {
        crate::codec::read_json(&path)?
    } else {
        json!({"$schema": SCHEMA})
    };
    let Some(object) = config.as_object() else {
        return Err(ProjectError::ConfigMustBeObject);
    };
    if object.get("mcp").is_some_and(|mcp| !mcp.is_object()) {
        return Err(ProjectError::McpMustBeObject);
    }
    if object
        .get("instructions")
        .is_some_and(|instructions| !instructions.is_array())
    {
        return Err(ProjectError::InstructionsMustBeArray);
    }
    Ok((config, existed))
}

pub(crate) fn remove_gitignore_line(project: &Path, old_state: &Value) -> Result<(), ProjectError> {
    if !old_state
        .get("gitignore_added")
        .is_some_and(|flag| flag.as_bool() == Some(true))
    {
        return Ok(());
    }
    let path = file_path(project, ".gitignore")?;
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(source) => return Err(ProjectError::Io { path, source }),
    };
    let Some((start, end)) = matching_ignore_line(&contents) else {
        return Ok(());
    };
    let mut updated = String::with_capacity(contents.len() - (end - start));
    updated.push_str(&contents[..start]);
    updated.push_str(&contents[end..]);
    fs::write(&path, updated).map_err(|source| ProjectError::Io { path, source })
}

fn matching_ignore_line(contents: &str) -> Option<(usize, usize)> {
    let bytes = contents.as_bytes();
    let mut start = 0;
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\r' || bytes[cursor] == b'\n' {
            let content_end = cursor;
            let end = if bytes[cursor] == b'\r' && bytes.get(cursor + 1) == Some(&b'\n') {
                cursor + 2
            } else {
                cursor + 1
            };
            if &contents[start..content_end] == IGNORE_LINE {
                return Some((start, end));
            }
            start = end;
            cursor = end;
        } else {
            cursor += 1;
        }
    }
    if &contents[start..] == IGNORE_LINE {
        return Some((start, contents.len()));
    }
    None
}

use std::{
    env, fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use serde_json::{json, Value};

use crate::{
    error::ProjectError,
    filesystem::file_path,
    model::{Client, RepositoryContext, Version},
    repository::profile_names,
};

const STATE: &str = crate::layout::LEGACY_STATE;
const IGNORE_LINE: &str = ".opencode/workflow-approval.json";
const PLAYWRIGHT_AUTH_IGNORE: &[u8] = b"/base-auth.json";
const SCHEMA: &str = "https://opencode.ai/config.json";

/// Format 2 is authoritative, even when its client map is empty. Legacy import
/// is deliberately read-only until a selected operation saves the manifest.
pub(crate) fn manifest_for(
    context: &RepositoryContext,
    project: &Path,
) -> Result<Value, ProjectError> {
    let path = file_path(project, crate::layout::STATE)?;
    if path.exists() {
        let manifest = crate::codec::read_json(&path)?;
        let clients = manifest
            .get("clients")
            .and_then(Value::as_object)
            .filter(|_| manifest.get("format").and_then(Value::as_u64) == Some(2))
            .ok_or_else(|| ProjectError::UnsupportedState(path.clone()))?;
        for (name, record) in clients {
            name.parse::<Client>()
                .map_err(|_| ProjectError::UnsupportedState(path.clone()))?;
            validate_record(record, &path)?;
            if !matches!(
                record.get("layout").and_then(Value::as_str),
                Some("legacy" | "isolated")
            ) {
                return Err(ProjectError::UnsupportedState(path.clone()));
            }
        }
        return Ok(manifest);
    }
    let mut manifest = json!({"format":2,"clients":{}});
    let Some(legacy) = state_for(context, project)? else {
        return Ok(manifest);
    };
    let legacy_files = legacy
        .get("files")
        .and_then(Value::as_object)
        .ok_or_else(|| ProjectError::UnsupportedState(project.join(STATE)))?;
    let mut retained = legacy_files.clone();
    for client in Client::ALL {
        let native_key = client.as_str();
        let catalog = match client {
            Client::OpenCode => None,
            Client::Claude => Some(".claude-plugin/marketplace.json"),
            Client::Codex => Some(".agents/plugins/marketplace.json"),
        };
        let recorded = client == Client::OpenCode
            || legacy
                .pointer(&format!("/native/{native_key}"))
                .is_some_and(Value::is_object)
            || catalog.is_some_and(|path| legacy_files.contains_key(path));
        if !recorded {
            continue;
        }
        let mut record = legacy.clone();
        record
            .as_object_mut()
            .expect("validated legacy object")
            .remove("format");
        record["layout"] = json!("legacy");
        if record.get("mcp_base").is_none() {
            record["mcp_base"] = record["mcp"].clone();
        }
        record["files"] = json!({});
        if let Some(catalog) = catalog {
            if let Some(hash) = legacy_files.get(catalog) {
                record["files"][catalog] = hash.clone();
                retained.remove(catalog);
            }
        }
        let mut native = serde_json::Map::new();
        let keys: &[&str] = match client {
            Client::OpenCode => &["opencode_skill_path_added"],
            Client::Claude => &["claude", "mcp_claude"],
            Client::Codex => &["codex", "mcp_codex"],
        };
        for key in keys {
            if let Some(value) = legacy.get("native").and_then(|v| v.get(key)) {
                native.insert((*key).into(), value.clone());
            }
        }
        record["native"] = Value::Object(native);
        manifest["clients"][client.as_str()] = record;
    }
    manifest["retained_legacy"] = json!({"state":STATE,"files":retained});
    Ok(manifest)
}

fn validate_record(record: &Value, path: &Path) -> Result<(), ProjectError> {
    let valid = record.is_object()
        && record.get("profile").and_then(Value::as_str).is_some()
        && record
            .get("files")
            .and_then(Value::as_object)
            .is_some_and(|files| files.values().all(Value::is_string))
        && record.get("mcp").is_some_and(Value::is_object);
    let valid = valid
        && record.get("mcp_base").is_some_and(Value::is_object)
        && record.get("native").is_some_and(Value::is_object);
    if !valid {
        return Err(ProjectError::UnsupportedState(path.to_owned()));
    }
    if let Some(version) = record.get("version").filter(|v| !v.is_null()) {
        let version = version
            .as_str()
            .ok_or_else(|| ProjectError::UnsupportedState(path.to_owned()))?;
        Version::from_str(version).map_err(|_| ProjectError::UnsupportedState(path.to_owned()))?;
    }
    Ok(())
}

pub(crate) fn save_manifest(project: &Path, manifest: &Value) -> Result<(), ProjectError> {
    let path = file_path(project, crate::layout::STATE)?;
    crate::filesystem::write_atomic(&path, &crate::codec::json_bytes(manifest)?)?;
    Ok(())
}

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
    if let Some(skills) = object.get("skills") {
        if let Some(paths) = skills.get("paths") {
            if !paths
                .as_array()
                .is_some_and(|entries| entries.iter().all(Value::is_string))
            {
                return Err(ProjectError::ConfigMustBeObject);
            }
        } else if (!skills.is_array() && !skills.is_object())
            || skills
                .as_array()
                .is_some_and(|entries| !entries.iter().all(Value::is_string))
        {
            return Err(ProjectError::ConfigMustBeObject);
        }
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

/// Return an append-only .gitignore update when Playwright's default credentials
/// path is not already ignored. The caller preflights this during preparation
/// and commits it without recording ownership, so uninstall never removes it.
pub(crate) fn playwright_gitignore_update(project: &Path) -> Result<Option<Vec<u8>>, ProjectError> {
    let path = file_path(project, ".gitignore")?;
    let contents = match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => fs::read(&path)
            .map_err(|source| ProjectError::Io {
                path: path.clone(),
                source,
            })?,
        Ok(_) => return Err(ProjectError::GitignoreNotFile),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(source) => {
            return Err(ProjectError::Io { path, source });
        }
    };
    if contents
        .split(|byte| *byte == b'\r' || *byte == b'\n')
        .any(|line| line == PLAYWRIGHT_AUTH_IGNORE)
    {
        return Ok(None);
    }

    let newline: &[u8] = if contents.windows(2).any(|bytes| bytes == b"\r\n") {
        b"\r\n"
    } else if contents.contains(&b'\n') {
        b"\n"
    } else if contents.contains(&b'\r') {
        b"\r"
    } else {
        b"\n"
    };
    let needs_separator =
        !contents.is_empty() && !contents.ends_with(b"\n") && !contents.ends_with(b"\r");
    let mut updated = contents;
    if needs_separator {
        updated.extend_from_slice(newline);
    }
    updated.extend_from_slice(PLAYWRIGHT_AUTH_IGNORE);
    updated.extend_from_slice(newline);
    Ok(Some(updated))
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

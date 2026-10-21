use std::{cmp::Ordering, fs, path::Path, str::FromStr};

use serde_json::{Map, Value};

use crate::{
    codec::{json_bytes, sha},
    error::StatusError,
    filesystem::file_path,
    mcp::config::{configured_mcp, definitions_to_map, desired_mcp},
    model::{RepositoryContext, Version},
    profile_management::payload::desired_files,
    project::{config_for, state_for},
    repository::profile_metadata,
};

const LEGACY_GATE: &str = ".opencode/plugins/approval-gate.js";

pub(crate) fn status(context: &RepositoryContext, project: &Path) -> Result<(), StatusError> {
    let Some(old) = state_for(context, project)? else {
        println!("{}: brak profilu", project.display());
        return Ok(());
    };
    let old_files = old
        .get("files")
        .and_then(Value::as_object)
        .ok_or(StatusError::InvalidState)?;
    let mut mismatches = Vec::new();
    for (relative, expected) in old_files {
        let path = file_path(project, relative)?;
        let matches = if path.is_file() {
            let contents = fs::read(&path).map_err(|source| StatusError::Io {
                path: path.clone(),
                source,
            })?;
            expected.as_str() == Some(sha(&contents).as_str())
        } else {
            false
        };
        if !matches {
            mismatches.push(relative.clone());
        }
    }

    let profile = old
        .get("profile")
        .and_then(Value::as_str)
        .ok_or(StatusError::InvalidState)?;
    let available = profile_metadata(context, profile)?
        .get("version")
        .and_then(Value::as_str)
        .ok_or(StatusError::InvalidState)?
        .to_owned();
    let installed = old.get("version").and_then(Value::as_str);

    let files = desired_files(context, profile)?;
    let mut expected_hashes = Map::new();
    for file in files {
        expected_hashes.insert(file.relative_path, Value::String(sha(&file.contents)));
    }
    let definitions = desired_mcp(context, profile)?;
    let base = definitions_to_map(&definitions);
    let installed_base = old
        .get("mcp_base")
        .or_else(|| old.get("mcp"))
        .ok_or(StatusError::InvalidState)?;
    let content_changed = !object_maps_equal(old_files, &expected_hashes)
        || !object_matches_map(installed_base, &base);

    let overrides = configured_mcp(project, &base)?.overrides;
    let (config, _) = config_for(project)?;
    let config_object = config.as_object().ok_or(StatusError::InvalidState)?;
    let empty_mcp = Map::new();
    let current_mcp = match config_object.get("mcp") {
        None => &empty_mcp,
        Some(value) => value.as_object().ok_or(StatusError::InvalidState)?,
    };
    let owned_mcp = old
        .get("mcp")
        .and_then(Value::as_object)
        .ok_or(StatusError::InvalidState)?;
    let modified_mcp = owned_mcp
        .iter()
        .filter(|(name, expected)| current_mcp.get(*name) != Some(*expected))
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();

    let label = match installed {
        None => "VERSION UNKNOWN (legacy installation)",
        Some(installed) => match Version::from_str(installed)?.cmp(&Version::from_str(&available)?)
        {
            Ordering::Less => "UPDATE AVAILABLE",
            Ordering::Greater => "INSTALLED NEWER (downgrade blocked)",
            Ordering::Equal if content_changed => "CONTENT CHANGED WITHOUT VERSION BUMP",
            Ordering::Equal => "UP TO DATE",
        },
    };

    println!("Profile: {profile}");
    println!("Installed: {}", installed.unwrap_or("UNKNOWN"));
    println!("Available: {available}");
    println!("Status: {label}");
    println!(
        "Source commit: {}",
        old.get("source_commit")
            .and_then(Value::as_str)
            .filter(|commit| !commit.is_empty())
            .unwrap_or("UNKNOWN")
    );
    println!(
        "Content: {}",
        if content_changed {
            "CHANGED"
        } else {
            "UNCHANGED"
        }
    );
    let mut local_modifications = mismatches;
    local_modifications.extend(modified_mcp.iter().map(|name| format!("MCP:{name}")));
    println!(
        "Local modifications: {}",
        if local_modifications.is_empty() {
            "none".to_owned()
        } else {
            local_modifications.join(", ")
        }
    );
    let mut override_names = overrides.keys().cloned().collect::<Vec<_>>();
    override_names.sort();
    println!(
        "MCP local overrides: {}",
        if override_names.is_empty() {
            "none".to_owned()
        } else {
            override_names.join(", ")
        }
    );
    let current_override_hash = sha(&json_bytes(&overrides)?);
    if old.get("mcp_local_hash").and_then(Value::as_str) != Some(&current_override_hash) {
        println!("MCP local configuration: changed; run update to synchronize generated settings");
    }
    let has_legacy_gate = old_files.contains_key(LEGACY_GATE);
    println!(
        "Approval gate: {}",
        if has_legacy_gate {
            "legacy installation; removed on update"
        } else {
            "not installed"
        }
    );
    Ok(())
}

fn object_maps_equal(left: &Map<String, Value>, right: &Map<String, Value>) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .all(|(key, value)| right.get(key).is_some_and(|other| other == value))
}

fn object_matches_map(value: &Value, expected: &Map<String, Value>) -> bool {
    value
        .as_object()
        .is_some_and(|actual| object_maps_equal(actual, expected))
}

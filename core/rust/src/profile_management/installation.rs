use std::{fs, path::Path, str::FromStr};

use serde_json::{Map, Value};

use crate::{
    codec::{json_bytes, sha},
    error::InstallationError,
    filesystem::{check_files, cleanup_empty_directories, file_path, write_bytes},
    mcp::config::{
        changed_override_paths, check_mcp, configured_mcp, definitions_to_map, desired_mcp,
    },
    model::{RepositoryContext, Version},
    profile_management::payload::{desired_files, PayloadFile},
    project::{config_for, remove_gitignore_line, state_for},
    repository::{profile_metadata, source_commit},
};

const RULE: &str = ".opencode/workflow-rules.md";
const GATE: &str = ".opencode/workflow-approval.json";
const STATE: &str = ".opencode/ai-profile-state.json";

pub(crate) fn apply_profile(
    context: &RepositoryContext,
    project: &Path,
    profile: &str,
) -> Result<(), InstallationError> {
    let old = state_for(context, project)?;
    let metadata = profile_metadata(context, profile)?;
    let version = metadata
        .get("version")
        .and_then(Value::as_str)
        .ok_or(InstallationError::InvalidState)?;
    if let Some(old_state) = &old {
        let same_profile = old_state.get("profile").and_then(Value::as_str) == Some(profile);
        if same_profile {
            if let Some(previous) = old_state.get("version").and_then(Value::as_str) {
                if Version::from_str(previous)? > Version::from_str(version)? {
                    return Err(InstallationError::Downgrade);
                }
            }
        }
    }

    let files = desired_files(context, profile)?;
    let desired_definitions = desired_mcp(context, profile)?;
    let base_mcp = definitions_to_map(&desired_definitions);
    let (mcp, overrides) = configured_mcp(project, &base_mcp)?;
    let (mut config, existed) = config_for(project)?;

    check_files(
        project,
        old.as_ref(),
        files.iter().map(|file| file.relative_path.as_str()),
    )?;
    let config_object = config.as_object().ok_or(InstallationError::InvalidState)?;
    check_mcp(config_object, old.as_ref(), &mcp)?;

    let prior_base = prior_base(old.as_ref())?;
    print_override_warnings(&prior_base, &base_mcp, &overrides);

    let mut destinations = files
        .iter()
        .map(|file| file.relative_path.as_str())
        .collect::<Vec<_>>();
    destinations.extend([STATE, GATE, ".gitignore", "opencode.json"]);
    for relative in destinations {
        let destination = file_path(project, relative)?;
        if destination.exists() && !destination.is_file() {
            return Err(InstallationError::DestinationNotFile(relative.to_owned()));
        }
        let mut parent = destination.parent();
        while let Some(parent_path) = parent {
            if parent_path == project {
                break;
            }
            if parent_path.exists() && !parent_path.is_dir() {
                return Err(InstallationError::ParentNotDirectory(
                    parent_path.to_owned(),
                ));
            }
            parent = parent_path.parent();
        }
    }
    let gitignore = file_path(project, ".gitignore")?;
    if gitignore.exists() {
        fs::read_to_string(&gitignore).map_err(|_| InstallationError::GitignoreRead)?;
    }

    let commit = source_commit(context);
    update_project_config(&mut config, old.as_ref(), &mcp)?;

    if let Some(old_files) = old.as_ref().and_then(|state| state.get("files")) {
        let Some(old_files) = old_files.as_object() else {
            return Err(InstallationError::InvalidState);
        };
        for relative in old_files.keys() {
            if !files.iter().any(|file| file.relative_path == *relative) {
                let path = file_path(project, relative)?;
                fs::remove_file(&path).map_err(|source| InstallationError::Io { path, source })?;
            }
        }
    }
    for file in &files {
        let path = file_path(project, &file.relative_path)?;
        write_bytes(&path, &file.contents)?;
    }
    let config_path = file_path(project, "opencode.json")?;
    write_bytes(&config_path, &json_bytes(&config)?)?;

    if let Some(old_state) = &old {
        remove_gitignore_line(project, old_state)?;
    }

    if old.is_some() {
        let gate = file_path(project, GATE)?;
        match fs::remove_file(&gate) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(source) => {
                return Err(InstallationError::Io { path: gate, source });
            }
        }
        if let Some(old_files) = old.as_ref().and_then(|state| state.get("files")) {
            let old_paths = old_files
                .as_object()
                .ok_or(InstallationError::InvalidState)?
                .keys()
                .filter(|relative| !files.iter().any(|file| file.relative_path == **relative))
                .cloned()
                .collect::<Vec<_>>();
            cleanup_empty_directories(project, &old_paths)?;
        }
    }
    let updated_state = build_updated_state(
        profile,
        version,
        commit,
        &files,
        &mcp,
        &base_mcp,
        &overrides,
        old.as_ref(),
        existed,
    )?;
    let state_path = file_path(project, STATE)?;
    write_bytes(&state_path, &json_bytes(&updated_state)?)?;

    println!(
        "OK: profil {profile} w {}. Zrestartuj OpenCode.",
        project.display()
    );
    if files
        .iter()
        .any(|file| file.relative_path == ".opencode/ai-profile/README.md")
    {
        println!(
            "Instrukcja konfiguracji: {}",
            project.join(".opencode/ai-profile/README.md").display()
        );
    }
    Ok(())
}

fn prior_base(old: Option<&Value>) -> Result<Map<String, Value>, InstallationError> {
    let Some(old) = old else {
        return Ok(Map::new());
    };
    let base = old.get("mcp_base").or_else(|| old.get("mcp"));
    match base {
        None => Ok(Map::new()),
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or(InstallationError::InvalidState),
    }
}

fn print_override_warnings(
    prior_base: &Map<String, Value>,
    new_base: &Map<String, Value>,
    overrides: &Map<String, Value>,
) {
    for (name, override_entry) in overrides {
        let (Some(previous), Some(current), Some(override_object)) = (
            prior_base.get(name).and_then(Value::as_object),
            new_base.get(name).and_then(Value::as_object),
            override_entry.as_object(),
        ) else {
            continue;
        };
        for path in changed_override_paths(previous, current, override_object) {
            println!(
                "WARNING: upstream changed overridden field {name}.{path}; local override wins."
            );
        }
    }
}

fn update_project_config(
    config: &mut Value,
    old: Option<&Value>,
    managed_mcp: &Map<String, Value>,
) -> Result<(), InstallationError> {
    let object = config
        .as_object_mut()
        .ok_or(InstallationError::InvalidState)?;
    let mut current = match object.get("mcp") {
        None => Map::new(),
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or(InstallationError::InvalidState)?,
    };
    if let Some(old_mcp) = old.and_then(|state| state.get("mcp")) {
        let Some(old_mcp) = old_mcp.as_object() else {
            return Err(InstallationError::InvalidState);
        };
        for name in old_mcp.keys() {
            current.shift_remove(name);
        }
    }
    for (name, entry) in managed_mcp {
        current.insert(name.clone(), entry.clone());
    }
    if current.is_empty() {
        object.remove("mcp");
    } else {
        object.insert("mcp".to_owned(), Value::Object(current));
    }

    let mut instructions = match object.get("instructions") {
        None => Vec::new(),
        Some(value) => value
            .as_array()
            .cloned()
            .ok_or(InstallationError::InvalidState)?,
    };
    let instructions_added = old
        .and_then(|state| state.get("instructions_added"))
        .is_some_and(is_truthy);
    if instructions_added {
        instructions.retain(|item| item != RULE);
    }
    if instructions.is_empty() {
        object.remove("instructions");
    } else {
        object.insert("instructions".to_owned(), Value::Array(instructions));
    }
    Ok(())
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(value) => *value,
        Value::Number(value) => value.as_f64().is_some_and(|number| number != 0.0),
        Value::String(value) => !value.is_empty(),
        Value::Array(value) => !value.is_empty(),
        Value::Object(value) => !value.is_empty(),
    }
}

fn build_updated_state(
    profile: &str,
    version: &str,
    commit: Option<String>,
    files: &[PayloadFile],
    mcp: &Map<String, Value>,
    base_mcp: &Map<String, Value>,
    overrides: &Map<String, Value>,
    old: Option<&Value>,
    existed: bool,
) -> Result<Value, InstallationError> {
    let mut file_hashes = Map::new();
    for file in files {
        file_hashes.insert(
            file.relative_path.clone(),
            Value::String(sha(&file.contents)),
        );
    }
    let local_hash = sha(&json_bytes(overrides)?);
    let had_config = old
        .and_then(|state| state.get("had_config"))
        .cloned()
        .unwrap_or(Value::Bool(existed));
    let mut state = Map::new();
    state.insert("format".to_owned(), Value::Number(1.into()));
    state.insert("profile".to_owned(), Value::String(profile.to_owned()));
    state.insert("version".to_owned(), Value::String(version.to_owned()));
    state.insert(
        "source_commit".to_owned(),
        commit.map_or(Value::Null, Value::String),
    );
    state.insert("files".to_owned(), Value::Object(file_hashes));
    state.insert("mcp".to_owned(), Value::Object(mcp.clone()));
    state.insert("mcp_base".to_owned(), Value::Object(base_mcp.clone()));
    state.insert("mcp_local_hash".to_owned(), Value::String(local_hash));
    state.insert("had_config".to_owned(), had_config);
    state.insert("gitignore_added".to_owned(), Value::Bool(false));
    state.insert("instructions_added".to_owned(), Value::Bool(false));
    Ok(Value::Object(state))
}

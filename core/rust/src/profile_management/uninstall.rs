use std::{fs, path::Path};

use serde_json::{Map, Value};

use crate::{
    codec::json_bytes,
    error::UninstallError,
    filesystem::{check_files, cleanup_empty_directories, file_path, write_bytes},
    mcp::config::check_mcp,
    model::RepositoryContext,
    project::{config_for, remove_gitignore_line, state_for},
};

const RULE: &str = ".opencode/workflow-rules.md";
const GATE: &str = ".opencode/workflow-approval.json";
const STATE: &str = ".opencode/ai-profile-state.json";
const SCHEMA: &str = "https://opencode.ai/config.json";

pub(crate) fn uninstall(context: &RepositoryContext, project: &Path) -> Result<(), UninstallError> {
    let old = state_for(context, project)?.ok_or(UninstallError::NoInstalledProfile)?;
    let (mut config, _) = config_for(project)?;
    check_files(project, Some(&old), std::iter::empty())?;
    let config_object = config.as_object().ok_or(UninstallError::InvalidState)?;
    check_mcp(config_object, Some(&old), &Map::new())?;
    crate::profile_management::native::preflight_cleanup(project)
        .map_err(UninstallError::Native)?;
    let empty_native_mcp = Map::new();
    let (claude_mcp_bytes, _) = crate::profile_management::native::configure_claude_mcp(
        project,
        Some(&old),
        &empty_native_mcp,
    )
    .map_err(UninstallError::Native)?;
    let codex_config_path = file_path(project, ".codex/config.toml")?;
    if old.pointer("/native/mcp_codex/entries").is_some() {
        let codex_config_before = if codex_config_path.exists() {
            fs::read(&codex_config_path).map_err(|source| UninstallError::Io {
                path: codex_config_path.clone(),
                source,
            })?
        } else {
            Vec::new()
        };
        let _ = crate::profile_management::native::configure_codex_mcp(
            &codex_config_before,
            Some(&old),
            &empty_native_mcp,
            None,
        )
        .map_err(UninstallError::Native)?;
    }
    if old
        .pointer("/native/claude/mcp_timeout/added")
        .and_then(Value::as_bool)
        == Some(true)
    {
        let settings = file_path(project, ".claude/settings.json")?;
        let value = crate::codec::read_json(&settings)?;
        let current = value.pointer("/env/MCP_TIMEOUT");
        if current != old.pointer("/native/claude/mcp_timeout/expected") {
            return Err(UninstallError::Native(
                "managed Claude env.MCP_TIMEOUT was manually changed".into(),
            ));
        }
    }
    let native_cleanup_warning = crate::profile_management::native::cleanup_claude(project, &old);
    let codex_cleanup_warning = crate::profile_management::native::cleanup_codex(project, &old);
    crate::profile_management::native::remove_claude(project, &old)
        .map_err(UninstallError::Native)?;
    crate::profile_management::native::remove_codex(project, &old)
        .map_err(UninstallError::Native)?;
    if old.pointer("/native/mcp_codex/entries").is_some() {
        let codex_config_after = if codex_config_path.exists() {
            fs::read(&codex_config_path).map_err(|source| UninstallError::Io {
                path: codex_config_path.clone(),
                source,
            })?
        } else {
            Vec::new()
        };
        let (codex_clean, _) = crate::profile_management::native::configure_codex_mcp(
            &codex_config_after,
            Some(&old),
            &empty_native_mcp,
            None,
        )
        .map_err(UninstallError::Native)?;
        if old
            .pointer("/native/mcp_codex/config_had_file")
            .and_then(Value::as_bool)
            == Some(false)
            && String::from_utf8_lossy(&codex_clean).trim().is_empty()
        {
            if codex_config_path.exists() {
                fs::remove_file(&codex_config_path).map_err(|source| UninstallError::Io {
                    path: codex_config_path.clone(),
                    source,
                })?;
            }
        } else {
            crate::profile_management::native::write_config(
                project,
                ".codex/config.toml",
                &codex_clean,
            )
            .map_err(UninstallError::Native)?;
        }
    }
    let claude_mcp_path = file_path(project, ".mcp.json")?;
    let claude_json: Value =
        serde_json::from_slice(&claude_mcp_bytes).map_err(|_| UninstallError::InvalidState)?;
    if old
        .pointer("/native/mcp_claude/had_file")
        .and_then(Value::as_bool)
        == Some(false)
        && claude_json.as_object().is_some_and(Map::is_empty)
    {
        if claude_mcp_path.exists() {
            fs::remove_file(&claude_mcp_path).map_err(|source| UninstallError::Io {
                path: claude_mcp_path,
                source,
            })?;
        }
    } else if old.pointer("/native/mcp_claude/entries").is_some() {
        crate::profile_management::native::write_config(project, ".mcp.json", &claude_mcp_bytes)
            .map_err(UninstallError::Native)?;
    }

    let old_files = old
        .get("files")
        .and_then(Value::as_object)
        .ok_or(UninstallError::InvalidState)?;
    for relative in old_files.keys() {
        let path = file_path(project, relative)?;
        fs::remove_file(&path).map_err(|source| UninstallError::Io { path, source })?;
    }

    remove_managed_mcp(&mut config, &old)?;
    remove_managed_instruction(&mut config, &old)?;
    remove_managed_skill_source(&mut config, project, &old)?;

    let config_path = file_path(project, "opencode.json")?;
    if !is_truthy(old.get("had_config").unwrap_or(&Value::Null)) && is_schema_only(&config) {
        fs::remove_file(&config_path).map_err(|source| UninstallError::Io {
            path: config_path,
            source,
        })?;
    } else {
        write_bytes(&config_path, &json_bytes(&config)?)?;
    }

    let gate = file_path(project, GATE)?;
    match fs::remove_file(&gate) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(UninstallError::Io { path: gate, source }),
    }
    let state_path = file_path(project, STATE)?;
    fs::remove_file(&state_path).map_err(|source| UninstallError::Io {
        path: state_path,
        source,
    })?;
    remove_gitignore_line(project, &old)?;

    let mut cleanup_paths = old_files.keys().cloned().collect::<Vec<_>>();
    cleanup_paths.extend([STATE.to_owned(), GATE.to_owned()]);
    cleanup_empty_directories(project, &cleanup_paths)?;

    println!(
        "OK: profil usunięty z {}. Pozostałe pliki projektu zachowane.",
        project.display()
    );
    if let Some(warning) = native_cleanup_warning {
        println!("WARNING: {warning}.");
    }
    if let Some(warning) = codex_cleanup_warning {
        println!("WARNING: {warning}.");
    }
    Ok(())
}

fn remove_managed_skill_source(
    config: &mut Value,
    project: &Path,
    old: &Value,
) -> Result<(), UninstallError> {
    if old
        .pointer("/native/opencode_skill_path_added")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Ok(());
    }
    let object = config.as_object_mut().ok_or(UninstallError::InvalidState)?;
    let canonical_project = project
        .canonicalize()
        .map_err(|source| UninstallError::Io {
            path: project.to_owned(),
            source,
        })?;
    let source = canonical_project
        .join(".opencode/ai-profile/plugin/skills")
        .to_string_lossy()
        .into_owned();
    let remove_skills = {
        let Some(skills) = object.get_mut("skills") else {
            return Ok(());
        };
        if let Some(paths) = skills.as_array_mut() {
            paths.retain(|entry| entry.as_str() != Some(&source));
            paths.is_empty()
        } else {
            let skills = skills.as_object_mut().ok_or(UninstallError::InvalidState)?;
            let paths = skills
                .get_mut("paths")
                .and_then(Value::as_array_mut)
                .ok_or(UninstallError::InvalidState)?;
            paths.retain(|entry| entry.as_str() != Some(&source));
            if paths.is_empty() {
                skills.remove("paths");
            }
            skills.is_empty()
        }
    };
    if remove_skills {
        object.remove("skills");
    }
    Ok(())
}

fn remove_managed_mcp(config: &mut Value, old: &Value) -> Result<(), UninstallError> {
    let object = config.as_object_mut().ok_or(UninstallError::InvalidState)?;
    let mut current = match object.get("mcp") {
        None => Map::new(),
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or(UninstallError::InvalidState)?,
    };
    let owned = old
        .get("mcp")
        .and_then(Value::as_object)
        .ok_or(UninstallError::InvalidState)?;
    for name in owned.keys() {
        current.shift_remove(name);
    }
    if current.is_empty() {
        object.remove("mcp");
    } else {
        object.insert("mcp".to_owned(), Value::Object(current));
    }
    Ok(())
}

fn remove_managed_instruction(config: &mut Value, old: &Value) -> Result<(), UninstallError> {
    let object = config.as_object_mut().ok_or(UninstallError::InvalidState)?;
    let mut instructions = match object.get("instructions") {
        None => Vec::new(),
        Some(value) => value
            .as_array()
            .cloned()
            .ok_or(UninstallError::InvalidState)?,
    };
    if old.get("instructions_added").is_some_and(is_truthy) {
        instructions.retain(|item| item != RULE);
    }
    if instructions.is_empty() {
        object.remove("instructions");
    } else {
        object.insert("instructions".to_owned(), Value::Array(instructions));
    }
    Ok(())
}

fn is_schema_only(config: &Value) -> bool {
    let Some(object) = config.as_object() else {
        return false;
    };
    object.len() == 1 && object.get("$schema").and_then(Value::as_str) == Some(SCHEMA)
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

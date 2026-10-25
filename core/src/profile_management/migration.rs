use std::path::Path;

use serde_json::{Map, Value};

use crate::{
    codec::{json_bytes, read_json, sha},
    error::MigrationError,
    filesystem::{file_path, write_bytes},
    mcp::config::{definitions_to_map, desired_mcp},
    model::RepositoryContext,
    project::config_for,
};

const STATE: &str = ".opencode/ai-profile-state.json";
const GATE: &str = ".opencode/workflow-approval.json";
const RULE: &str = ".opencode/workflow-rules.md";
const LEGACY_PROFILES: [(&str, &str); 2] = [
    ("backend-go", "go-backend"),
    ("frontend-react", "react-frontend"),
];
const LEGACY_SKILLS: [&str; 2] = ["implementation-breakdown", "refine-plan"];

pub(crate) fn migrate_legacy(
    context: &RepositoryContext,
    project: &Path,
    accept_existing: bool,
) -> Result<(), MigrationError> {
    if !accept_existing {
        return Err(MigrationError::AcceptanceRequired);
    }
    if file_path(project, crate::layout::STATE)?.exists() {
        return Err(MigrationError::NotLegacyFormat);
    }

    let state_path = file_path(project, STATE)?;
    if !state_path.exists() {
        return Err(MigrationError::MissingLegacyState);
    }
    let previous = read_json(&state_path)?;
    let Some(old) = previous.as_object() else {
        return Err(MigrationError::NotLegacyFormat);
    };
    if old.get("format").is_some_and(is_format_one) {
        return Err(MigrationError::NotLegacyFormat);
    }

    let profile = old
        .get("profile")
        .and_then(Value::as_str)
        .ok_or(MigrationError::UnknownLegacyFormat)?;
    let skill = old
        .get("skill")
        .and_then(Value::as_str)
        .ok_or(MigrationError::UnknownLegacyFormat)?;
    if !LEGACY_PROFILES
        .iter()
        .any(|(known_profile, known_skill)| *known_profile == profile && *known_skill == skill)
    {
        return Err(MigrationError::UnknownLegacyFormat);
    }

    let desired = definitions_to_map(&desired_mcp(context, profile)?);
    let mcp_name = old
        .get("mcp")
        .and_then(Value::as_str)
        .filter(|name| desired.contains_key(*name))
        .ok_or(MigrationError::UnknownLegacyFormat)?;

    let (config, _) = config_for(project)?;
    let config = config
        .as_object()
        .ok_or(MigrationError::InvalidLegacyState)?;
    let configured_mcp = config
        .get("mcp")
        .and_then(Value::as_object)
        .ok_or(MigrationError::MissingMcpConfiguration)?;
    let managed_entry = configured_mcp
        .get(mcp_name)
        .ok_or(MigrationError::MissingMcpConfiguration)?;

    let mut paths = vec![
        RULE.to_owned(),
        ".opencode/plugins/approval-gate.js".to_owned(),
    ];
    paths.extend(
        LEGACY_SKILLS
            .into_iter()
            .map(|name| format!(".opencode/skills/{name}/SKILL.md")),
    );
    paths.push(format!(".opencode/skills/{skill}/SKILL.md"));

    let mut files = Map::new();
    for relative in &paths {
        let path = file_path(project, relative)?;
        if !path.is_file() {
            return Err(MigrationError::MissingLegacyFile(relative.clone()));
        }
        let contents = std::fs::read(&path).map_err(|source| MigrationError::Io {
            path: path.clone(),
            source,
        })?;
        files.insert(relative.clone(), Value::String(sha(&contents)));
    }

    let mut managed_mcp = Map::new();
    managed_mcp.insert(mcp_name.to_owned(), managed_entry.clone());
    let mut new_state = Map::new();
    new_state.insert("format".to_owned(), Value::Number(1.into()));
    new_state.insert("profile".to_owned(), Value::String(profile.to_owned()));
    new_state.insert("files".to_owned(), Value::Object(files));
    new_state.insert("mcp".to_owned(), Value::Object(managed_mcp));
    new_state.insert("had_config".to_owned(), Value::Bool(true));
    new_state.insert("gitignore_added".to_owned(), Value::Bool(false));
    new_state.insert("instructions_added".to_owned(), Value::Bool(false));

    write_bytes(&state_path, &json_bytes(&Value::Object(new_state))?)?;
    let gate = file_path(project, GATE)?;
    match std::fs::remove_file(&gate) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(source) => return Err(MigrationError::Io { path: gate, source }),
    }
    println!("OK: zaadoptowano stary profil {profile}; teraz uruchom make update CLIENT=opencode PROJECT=\"{}\"", project.display());
    Ok(())
}

fn is_format_one(value: &Value) -> bool {
    value.as_bool() == Some(true)
        || value.as_i64() == Some(1)
        || value.as_u64() == Some(1)
        || value.as_f64() == Some(1.0)
}

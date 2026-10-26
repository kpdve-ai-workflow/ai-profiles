use std::{io::Cursor, path::Path};

use serde_json::{Map, Value};

use crate::{
    codec::{json_bytes, read_json_with_bytes, sha},
    error::MigrationError,
    filesystem::{file_path, write_bytes},
    mcp::config::{definitions_to_map, desired_mcp},
    model::{ProfileAction, RepositoryContext},
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
    super::diagnostics::repository_operation(
        ProfileAction::MigrateLegacy,
        project,
        |operation_id| migrate_legacy_inner(context, project, accept_existing, operation_id),
    )
}

fn migrate_legacy_inner(
    context: &RepositoryContext,
    project: &Path,
    accept_existing: bool,
    operation_id: &str,
) -> Result<(), MigrationError> {
    tracing::info!(
        event = "profile.migration.started",
        operation_id,
        migration = "legacy_opencode",
        accept_existing,
    );
    if !accept_existing {
        tracing::warn!(
            event = "profile.migration.check",
            operation_id,
            check = "acceptance",
            result = "rejected",
            reason = "accept_existing_required"
        );
        return Err(MigrationError::AcceptanceRequired);
    }
    let current_state = file_path(project, crate::layout::STATE)?;
    if current_state.exists() {
        tracing::warn!(event = "profile.migration.check", operation_id, check = "current_state", result = "rejected", path = %current_state.display(), reason = "not_legacy_format");
        return Err(MigrationError::NotLegacyFormat);
    }
    tracing::info!(event = "profile.migration.check", operation_id, check = "current_state", result = "passed", path = %current_state.display());

    let state_path = file_path(project, STATE)?;
    if !state_path.exists() {
        tracing::warn!(event = "profile.migration.check", operation_id, check = "legacy_state", result = "missing", path = %state_path.display());
        return Err(MigrationError::MissingLegacyState);
    }
    tracing::info!(event = "profile.migration.check", operation_id, check = "legacy_state", result = "found", path = %state_path.display());
    let (previous, previous_bytes) = read_json_with_bytes(&state_path)?;
    record_payload(operation_id, "legacy_state", STATE, &previous_bytes);
    let Some(old) = previous.as_object() else {
        tracing::warn!(
            event = "profile.migration.check",
            operation_id,
            check = "legacy_format",
            result = "rejected",
            reason = "state_not_object"
        );
        return Err(MigrationError::NotLegacyFormat);
    };
    if old.get("format").is_some_and(is_format_one) {
        tracing::warn!(
            event = "profile.migration.check",
            operation_id,
            check = "legacy_format",
            result = "rejected",
            reason = "already_format_one"
        );
        return Err(MigrationError::NotLegacyFormat);
    }
    tracing::info!(
        event = "profile.migration.check",
        operation_id,
        check = "legacy_format",
        result = "passed"
    );

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
        tracing::warn!(
            event = "profile.migration.check",
            operation_id,
            check = "profile_skill_pair",
            result = "rejected",
            profile,
            skill
        );
        return Err(MigrationError::UnknownLegacyFormat);
    }
    tracing::info!(
        event = "profile.migration.check",
        operation_id,
        check = "profile_skill_pair",
        result = "passed",
        profile,
        skill
    );

    let desired = definitions_to_map(&desired_mcp(context, profile)?);
    let mcp_name = old
        .get("mcp")
        .and_then(Value::as_str)
        .filter(|name| desired.contains_key(*name))
        .ok_or(MigrationError::UnknownLegacyFormat)?;
    tracing::info!(
        event = "profile.migration.mcp_selected",
        operation_id,
        profile,
        mcp = mcp_name
    );

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
    record_payload(
        operation_id,
        "managed_mcp_configuration",
        "opencode.json#mcp",
        &json_bytes(managed_entry)?,
    );

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
            tracing::warn!(
                event = "profile.migration.file_checked",
                operation_id,
                path = relative,
                result = "missing"
            );
            return Err(MigrationError::MissingLegacyFile(relative.clone()));
        }
        let contents = std::fs::read(&path).map_err(|source| MigrationError::Io {
            path: path.clone(),
            source,
        })?;
        tracing::info!(
            event = "profile.migration.file_checked",
            operation_id,
            path = relative,
            result = "found",
            bytes = contents.len()
        );
        record_payload(operation_id, "legacy_managed_file", relative, &contents);
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

    let state_bytes = json_bytes(&Value::Object(new_state))?;
    record_payload(operation_id, "migrated_state", STATE, &state_bytes);
    write_bytes(&state_path, &state_bytes)?;
    tracing::info!(
        event = "profile.migration.state_written",
        operation_id,
        path = STATE,
        bytes = state_bytes.len(),
        result = "success"
    );
    let gate = file_path(project, GATE)?;
    match std::fs::remove_file(&gate) {
        Ok(()) => tracing::info!(
            event = "profile.migration.gate_removed",
            operation_id,
            path = GATE,
            result = "success"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => tracing::info!(
            event = "profile.migration.gate_removed",
            operation_id,
            path = GATE,
            result = "already_absent"
        ),
        Err(source) => {
            tracing::error!(event = "profile.migration.gate_removed", operation_id, path = GATE, result = "error", error = %source);
            return Err(MigrationError::Io { path: gate, source });
        }
    }
    println!("OK: zaadoptowano stary profil {profile}; teraz uruchom make update CLIENT=opencode PROJECT=\"{}\"", project.display());
    Ok(())
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

fn is_format_one(value: &Value) -> bool {
    value.as_bool() == Some(true)
        || value.as_i64() == Some(1)
        || value.as_u64() == Some(1)
        || value.as_f64() == Some(1.0)
}

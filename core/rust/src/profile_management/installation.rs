use std::{fs, path::Path, str::FromStr};

use serde_json::{Map, Value};

use crate::{
    codec::{json_bytes, sha},
    error::InstallationError,
    filesystem::{
        check_files, cleanup_empty_directories, file_path, write_bytes, write_executable,
    },
    mcp::config::{
        changed_override_paths, check_mcp, configured_mcp, definitions_to_map, desired_mcp,
        McpConfiguration,
    },
    model::{RepositoryContext, Version},
    profile_management::payload::{desired_files, PayloadFile, PayloadKind},
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

    let metadata_description = metadata
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let files = crate::profile_management::native::project_files(
        project,
        metadata_description,
        version,
        old.as_ref(),
        desired_files(context, profile)?,
    )
    .map_err(InstallationError::Native)?;
    let codex_catalog = files
        .iter()
        .find(|file| file.relative_path == ".agents/plugins/marketplace.json")
        .ok_or(InstallationError::InvalidState)?;
    let (codex_config, mut codex_state) = crate::profile_management::native::configure_codex(
        project,
        old.as_ref(),
        &codex_catalog.contents,
    )
    .map_err(InstallationError::Native)?;
    let desired_definitions = desired_mcp(context, profile)?;
    let base_mcp = definitions_to_map(&desired_definitions);
    let McpConfiguration {
        servers: mcp,
        overrides,
    } = configured_mcp(project, &base_mcp)?;
    let (claude_settings, mut claude_state) =
        crate::profile_management::native::configure_claude(project, old.as_ref(), &mcp)
            .map_err(InstallationError::Native)?;
    let (mut config, existed) = config_for(project)?;
    let codex_mcp = crate::mcp::config::native_client_mcp(&mcp, "codex")?;
    let claude_mcp = crate::mcp::config::native_client_mcp(&mcp, "claude")?;
    let (claude_mcp_config, claude_mcp_state) =
        crate::profile_management::native::configure_claude_mcp(project, old.as_ref(), &claude_mcp)
            .map_err(InstallationError::Native)?;
    let (codex_mcp_config, codex_mcp_state) =
        crate::profile_management::native::configure_codex_mcp(
            &codex_config,
            old.as_ref(),
            &codex_mcp,
            codex_state.get("config_had_file").and_then(Value::as_bool),
        )
        .map_err(InstallationError::Native)?;

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
    destinations.extend([
        STATE,
        GATE,
        ".gitignore",
        "opencode.json",
        ".claude/settings.json",
        ".codex/config.toml",
        ".mcp.json",
    ]);
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
    let skill_source_added = update_project_config(&mut config, old.as_ref(), &mcp, project)?;

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
        match file.kind {
            PayloadKind::Data => write_bytes(&path, &file.contents)?,
            PayloadKind::Executable => write_executable(&path, &file.contents)?,
        }
    }
    fs::create_dir_all(project.join(".opencode/ai-profile/plugin/skills")).map_err(|source| {
        InstallationError::Io {
            path: project.join(".opencode/ai-profile/plugin/skills"),
            source,
        }
    })?;
    let config_path = file_path(project, "opencode.json")?;
    write_bytes(&config_path, &json_bytes(&config)?)?;
    crate::profile_management::native::write_config(
        project,
        ".claude/settings.json",
        &claude_settings,
    )
    .map_err(InstallationError::Native)?;
    crate::profile_management::native::write_config(
        project,
        ".codex/config.toml",
        &codex_mcp_config,
    )
    .map_err(InstallationError::Native)?;
    crate::profile_management::native::write_config(project, ".mcp.json", &claude_mcp_config)
        .map_err(InstallationError::Native)?;
    let claude_installed = old
        .as_ref()
        .and_then(|state| state.pointer("/native/claude/activation"))
        .and_then(Value::as_str)
        == Some("synchronized");
    let (claude_activation, codex_activation) = crate::profile_management::native::activate(
        project,
        &claude_state,
        &codex_state,
        claude_installed,
    );
    if let Some(object) = claude_state.as_object_mut() {
        object.insert(
            "activation".into(),
            claude_activation
                .get("activation")
                .cloned()
                .unwrap_or(Value::String("pending".into())),
        );
        object.remove("activation_reason");
        object.remove("activation_stage");
        if let Some(reason) = claude_activation.get("reason") {
            object.insert("activation_reason".into(), reason.clone());
        }
        if let Some(stage) = claude_activation.get("stage") {
            object.insert("activation_stage".into(), stage.clone());
        }
        if claude_activation.get("activation").and_then(Value::as_str) == Some("synchronized") {
            object.insert("installed_before".into(), Value::Bool(true));
        }
    }
    if let Some(object) = codex_state.as_object_mut() {
        object.insert(
            "activation".into(),
            codex_activation
                .get("activation")
                .cloned()
                .unwrap_or(Value::String("pending".into())),
        );
        object.remove("activation_reason");
        object.remove("activation_stage");
        if let Some(reason) = codex_activation.get("reason") {
            object.insert("activation_reason".into(), reason.clone());
        }
        if let Some(stage) = codex_activation.get("stage") {
            object.insert("activation_stage".into(), stage.clone());
        }
        if let Some(value) = codex_activation.get("plugin_added") {
            object.insert("plugin_added".into(), value.clone());
        }
        if let Some(value) = codex_activation.get("marketplace_added") {
            object.insert("marketplace_added".into(), value.clone());
        }
    }

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
    let updated_state = build_updated_state(InstallationState {
        profile,
        version,
        commit,
        files: &files,
        mcp: &mcp,
        base_mcp: &base_mcp,
        overrides: &overrides,
        old: old.as_ref(),
        existed,
    })?;
    let mut updated_state = updated_state;
    if let Some(native) = updated_state
        .get_mut("native")
        .and_then(Value::as_object_mut)
    {
        native.insert(
            "opencode_skill_path_added".into(),
            Value::Bool(skill_source_added),
        );
        native.insert("claude".into(), claude_state.clone());
        native.insert("codex".into(), codex_state.clone());
        native.insert("mcp_claude".into(), claude_mcp_state);
        native.insert("mcp_codex".into(), codex_mcp_state);
    }
    let state_path = file_path(project, STATE)?;
    write_bytes(&state_path, &json_bytes(&updated_state)?)?;

    println!(
        "OK: profil {profile} w {}. Zrestartuj OpenCode.",
        project.display()
    );
    println!(
        "Claude plugin activation: {}",
        claude_state
            .get("activation")
            .and_then(Value::as_str)
            .unwrap_or("pending")
    );
    println!(
        "Codex last recorded native installation result: {}",
        codex_state
            .get("activation")
            .and_then(Value::as_str)
            .unwrap_or("pending")
    );
    if let Some(reason) = claude_state
        .get("activation_reason")
        .and_then(Value::as_str)
    {
        println!("Claude activation detail: {reason}");
    }
    if let Some(reason) = codex_state.get("activation_reason").and_then(Value::as_str) {
        println!("Codex activation detail: {reason}");
    }
    if let Some(stage) = codex_state.get("activation_stage").and_then(Value::as_str) {
        println!("Codex activation stage: {stage}");
    }
    if claude_state.get("activation").and_then(Value::as_str) == Some("pending") {
        let marketplace = claude_state
            .get("marketplace_name")
            .and_then(Value::as_str)
            .unwrap_or("<marketplace>");
        println!("Claude activation pending: trust the project if prompted, then run `claude plugin install ai-profile@{marketplace} --scope project` from the project.");
    }
    if codex_state.get("activation").and_then(Value::as_str) == Some("pending") {
        let marketplace = codex_state
            .get("marketplace_name")
            .and_then(Value::as_str)
            .unwrap_or("<marketplace>");
        let plugin = codex_state
            .get("plugin_name")
            .and_then(Value::as_str)
            .unwrap_or("<plugin>");
        let source = project
            .canonicalize()
            .unwrap_or_else(|_| project.to_owned());
        println!("Codex activation pending: from the project, run `codex plugin marketplace add {} --json`, then `codex plugin add {plugin}@{marketplace} --json`; restart Codex to assess skill loading.", source.display());
    }
    if codex_state.get("activation").and_then(Value::as_str) == Some("failed") {
        println!("Project files were prepared, but Codex activation failed. Run `make update PROJECT={}` after correcting the reported failure.", project.display());
    }
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
    project: &Path,
) -> Result<bool, InstallationError> {
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
    let canonical_project = project
        .canonicalize()
        .map_err(|source| InstallationError::Io {
            path: project.to_owned(),
            source,
        })?;
    let skill_path = canonical_project
        .join(".opencode/ai-profile/plugin/skills")
        .to_string_lossy()
        .into_owned();
    let skills = object
        .entry("skills")
        .or_insert_with(|| serde_json::json!({"paths":[]}));
    let added = old
        .and_then(|state| state.pointer("/native/opencode_skill_path_added"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut source_added = false;
    if skills.is_array() {
        let paths = skills
            .as_array_mut()
            .ok_or(InstallationError::InvalidState)?;
        if added {
            paths.retain(|entry| entry.as_str() != Some(&skill_path));
        }
        if !paths
            .iter()
            .any(|entry| entry.as_str() == Some(&skill_path))
        {
            paths.push(Value::String(skill_path));
            source_added = true;
        }
    } else {
        let skills = skills
            .as_object_mut()
            .ok_or(InstallationError::InvalidState)?;
        let paths = skills
            .entry("paths")
            .or_insert_with(|| Value::Array(Vec::new()));
        let paths = paths
            .as_array_mut()
            .ok_or(InstallationError::InvalidState)?;
        if added {
            paths.retain(|entry| entry.as_str() != Some(&skill_path));
        }
        if !paths
            .iter()
            .any(|entry| entry.as_str() == Some(&skill_path))
        {
            paths.push(Value::String(skill_path));
            source_added = true;
        }
    }
    Ok(if added { true } else { source_added })
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

struct InstallationState<'a> {
    profile: &'a str,
    version: &'a str,
    commit: Option<String>,
    files: &'a [PayloadFile],
    mcp: &'a Map<String, Value>,
    base_mcp: &'a Map<String, Value>,
    overrides: &'a Map<String, Value>,
    old: Option<&'a Value>,
    existed: bool,
}

fn build_updated_state(input: InstallationState<'_>) -> Result<Value, InstallationError> {
    let InstallationState {
        profile,
        version,
        commit,
        files,
        mcp,
        base_mcp,
        overrides,
        old,
        existed,
    } = input;
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
    let opencode_path = old
        .and_then(|state| state.pointer("/native/opencode_skill_path_added"))
        .cloned()
        .unwrap_or(Value::Bool(true));
    state.insert(
        "native".to_owned(),
        serde_json::json!({"opencode_skill_path_added": opencode_path, "activation":"pending"}),
    );
    Ok(Value::Object(state))
}

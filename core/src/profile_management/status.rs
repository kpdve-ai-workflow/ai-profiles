use crate::{
    codec::{json_bytes, sha},
    error::StatusError,
    filesystem::file_path,
    layout::Layout,
    mcp::config,
    model::{Client, RepositoryContext, Version},
    repository::profile_metadata,
};
use serde_json::{Map, Value};
use std::{cmp::Ordering, fs, io::Cursor, path::Path, str::FromStr};

pub(crate) fn status(
    context: &RepositoryContext,
    project: &Path,
    client: Client,
    old: Option<&Value>,
    operation_id: &str,
) -> Result<(), StatusError> {
    let mut observation = StatusObservation::default();
    let result = status_inner(
        context,
        project,
        client,
        old,
        operation_id,
        &mut observation,
    );
    let error = result
        .as_ref()
        .err()
        .map(|error| super::diagnostics::error_chain(error));
    tracing::info!(
        event = "profile.status.client.finished",
        operation_id,
        client = client.as_str(),
        result = if error.is_some() { "error" } else if observation.installed { "success" } else { "not_installed" },
        installed = observation.installed,
        profile = observation.profile.as_deref().unwrap_or("none"),
        layout = observation.layout.as_deref().unwrap_or("unavailable"),
        installed_version = observation.installed_version.as_deref().unwrap_or("unknown"),
        available_version = observation.available_version.as_deref().unwrap_or("unknown"),
        managed_changes = ?observation.managed_changes,
        source_content_changed = observation.source_content_changed.unwrap_or(false),
        source_comparison_performed = observation.source_content_changed.is_some(),
        configuration_result = observation.configuration_result.as_deref().unwrap_or("not_checked"),
        local_overrides = ?observation.local_overrides,
        activation = observation.activation.as_deref().unwrap_or("not_applicable"),
        activation_saved = observation.activation_saved.as_deref().unwrap_or("not_applicable"),
        last_successful_sync = observation.last_successful_sync.as_deref().unwrap_or("unknown"),
        error = error.as_deref().unwrap_or(""),
    );
    result
}

#[derive(Default)]
struct StatusObservation {
    installed: bool,
    profile: Option<String>,
    layout: Option<String>,
    installed_version: Option<String>,
    available_version: Option<String>,
    managed_changes: Vec<String>,
    source_content_changed: Option<bool>,
    configuration_result: Option<String>,
    local_overrides: Vec<String>,
    activation: Option<String>,
    activation_saved: Option<String>,
    last_successful_sync: Option<String>,
}

fn status_inner(
    context: &RepositoryContext,
    project: &Path,
    client: Client,
    old: Option<&Value>,
    operation_id: &str,
    observation: &mut StatusObservation,
) -> Result<(), StatusError> {
    let key = client.as_str();
    let Some(old) = old else {
        observation.installed = false;
        println!("{key}: brak profilu");
        return Ok(());
    };
    observation.installed = true;
    let layout = Layout::from_state(client, Some(old));
    let profile = old
        .get("profile")
        .and_then(Value::as_str)
        .ok_or(StatusError::InvalidState)?;
    observation.profile = Some(profile.to_owned());
    observation.layout = old.get("layout").and_then(Value::as_str).map(str::to_owned);
    let metadata = profile_metadata(context, profile)?;
    let available = metadata
        .get("version")
        .and_then(Value::as_str)
        .ok_or(StatusError::InvalidState)?;
    let installed = old.get("version").and_then(Value::as_str);
    observation.installed_version = installed.map(str::to_owned);
    observation.available_version = Some(available.to_owned());
    let files = old
        .get("files")
        .and_then(Value::as_object)
        .ok_or(StatusError::InvalidState)?;
    let mut modifications = Vec::new();
    for (relative, hash) in files {
        let path = file_path(project, relative)?;
        let actual_hash = if path.is_file() {
            let contents = fs::read(&path).map_err(|source| StatusError::Io {
                path: path.clone(),
                source,
            })?;
            record_payload(
                operation_id,
                client,
                "installed_managed_file",
                relative,
                &contents,
            );
            Some(sha(&contents))
        } else {
            None
        };
        if hash.as_str() != actual_hash.as_deref() {
            modifications.push(relative.clone());
        }
    }
    observation.managed_changes = modifications.clone();
    let mut base = config::definitions_to_map(&config::desired_mcp(context, profile)?);
    super::scoped::render_mcp(&mut base, layout);
    let content_changed = if layout == Layout::Legacy {
        true
    } else {
        let expected = super::scoped::files_for(context, project, profile, client, Some(old))
            .map_err(|error| {
                tracing::error!(
                    event = "profile.status.check.failed",
                    operation_id,
                    client = client.as_str(),
                    stage = "source_content_comparison",
                    error = %super::diagnostics::error_chain(error.as_ref()),
                );
                StatusError::Native(error.to_string())
            })?;
        for file in &expected {
            record_payload(
                operation_id,
                client,
                "available_managed_file",
                &file.relative_path,
                &file.contents,
            );
        }
        let expected: Map<String, Value> = expected
            .into_iter()
            .map(|f| (f.relative_path, Value::String(sha(&f.contents))))
            .collect();
        files != &expected || old.get("mcp_base") != Some(&Value::Object(base))
    };
    observation.source_content_changed = Some(content_changed);
    let label = match installed {
        None => "VERSION UNKNOWN (legacy)",
        Some(installed) => {
            match Version::from_str(installed)?.cmp(&Version::from_str(available)?) {
                Ordering::Less => "UPDATE AVAILABLE",
                Ordering::Greater => "INSTALLED NEWER (downgrade blocked)",
                Ordering::Equal if content_changed => "CONTENT CHANGED WITHOUT VERSION BUMP",
                Ordering::Equal => "UP TO DATE",
            }
        }
    };
    println!(
        "{key}: profil {profile}; wersja {}; dostępna {available}; {label}",
        installed.unwrap_or("UNKNOWN")
    );
    println!(
        "{key}: layout {}; lokalne zmiany: {}",
        old["layout"],
        if modifications.is_empty() {
            "brak".into()
        } else {
            modifications.join(", ")
        }
    );
    // Pure reconciliation only. No CLI discovery, registration or activation.
    super::scoped::inspect_configuration(project, client, old).map_err(|error| {
        tracing::error!(
            event = "profile.status.check.failed",
            operation_id,
            client = client.as_str(),
            stage = "configuration_inspection",
            error = %super::diagnostics::error_chain(error.as_ref()),
        );
        StatusError::Native(error.to_string())
    })?;
    observation.configuration_result = Some("success".to_owned());
    let overrides = config::resolve_at(
        project,
        old.get("mcp_base")
            .and_then(Value::as_object)
            .ok_or(StatusError::InvalidState)?,
        &layout.local(),
    )?
    .overrides;
    observation.local_overrides = overrides.keys().cloned().collect();
    println!(
        "{key}: nadpisania MCP: {}; połączenie runtime: niesprawdzone",
        overrides.keys().cloned().collect::<Vec<_>>().join(", ")
    );
    if old.get("mcp_local_hash").and_then(Value::as_str)
        != Some(sha(&json_bytes(&overrides)?).as_str())
    {
        println!(
            "{key}: nadpisania zmienione; make update CLIENT={key} PROJECT=\"{}\"",
            project.display()
        );
    }
    if client != Client::OpenCode {
        let meta = &old["native"][key];
        observation.activation_saved = meta
            .get("activation")
            .and_then(Value::as_str)
            .map(str::to_owned);
        observation.last_successful_sync = meta
            .get("last_successful_sync")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let disabled = match client {
            Client::Claude => {
                super::native::configure_claude(project, Some(old), &Map::new())
                    .map_err(StatusError::Native)?
                    .1
                    .get("enabled_current")
                    .and_then(Value::as_bool)
                    == Some(false)
            }
            Client::Codex => {
                let path = file_path(project, ".agents/plugins/marketplace.json")?;
                let catalog = fs::read(&path).map_err(|source| StatusError::Io { path, source })?;
                super::native::configure_codex(project, Some(old), &catalog)
                    .map_err(StatusError::Native)?
                    .1
                    .get("activation")
                    .and_then(Value::as_str)
                    == Some("disabled")
            }
            Client::OpenCode => unreachable!("handled above"),
        };
        observation.activation = Some(if disabled {
            "disabled".to_owned()
        } else {
            meta.get("activation")
                .and_then(Value::as_str)
                .unwrap_or("pending")
                .to_owned()
        });
        println!("{key}: aktywacja {}; ostatnia udana synchronizacja {}; ładowanie umiejętności w sesji: niesprawdzone", if disabled { "disabled" } else { meta.get("activation").and_then(Value::as_str).unwrap_or("pending") }, meta["last_successful_sync"]);
    }
    println!(
        "{key}: commit źródłowy {}",
        old.get("source_commit")
            .and_then(Value::as_str)
            .unwrap_or("UNKNOWN")
    );
    Ok(())
}

fn record_payload(operation_id: &str, client: Client, role: &str, path: &str, bytes: &[u8]) {
    let Some(session) = crate::logging::current_session() else {
        return;
    };
    let summary = session.payload(Cursor::new(bytes));
    tracing::info!(
        event = "profile.status.payload",
        operation_id,
        client = client.as_str(),
        role,
        path,
        payload_id = summary.payload_id,
        bytes = summary.length,
        complete = summary.complete,
    );
}

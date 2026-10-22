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
use std::{cmp::Ordering, fs, path::Path, str::FromStr};

pub(crate) fn status(
    context: &RepositoryContext,
    project: &Path,
    client: Client,
    old: Option<&Value>,
) -> Result<(), StatusError> {
    let key = client.as_str();
    let Some(old) = old else {
        println!("{key}: brak profilu");
        return Ok(());
    };
    let layout = Layout::from_state(client, Some(old));
    let profile = old
        .get("profile")
        .and_then(Value::as_str)
        .ok_or(StatusError::InvalidState)?;
    let metadata = profile_metadata(context, profile)?;
    let available = metadata
        .get("version")
        .and_then(Value::as_str)
        .ok_or(StatusError::InvalidState)?;
    let installed = old.get("version").and_then(Value::as_str);
    let files = old
        .get("files")
        .and_then(Value::as_object)
        .ok_or(StatusError::InvalidState)?;
    let mut modifications = Vec::new();
    for (relative, hash) in files {
        let path = file_path(project, relative)?;
        if !path.is_file()
            || hash.as_str()
                != Some(
                    sha(&fs::read(&path).map_err(|source| StatusError::Io { path, source })?)
                        .as_str(),
                )
        {
            modifications.push(relative.clone());
        }
    }
    let mut base = config::definitions_to_map(&config::desired_mcp(context, profile)?);
    super::scoped::render_mcp(&mut base, layout);
    let content_changed = if layout == Layout::Legacy {
        true
    } else {
        let expected = super::scoped::files_for(context, project, profile, client, Some(old))
            .map_err(|e| StatusError::Native(e.to_string()))?;
        let expected: Map<String, Value> = expected
            .into_iter()
            .map(|f| (f.relative_path, Value::String(sha(&f.contents))))
            .collect();
        files != &expected || old.get("mcp_base") != Some(&Value::Object(base))
    };
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
    super::scoped::inspect_configuration(project, client, old)
        .map_err(|e| StatusError::Native(e.to_string()))?;
    let overrides = config::resolve_at(
        project,
        old.get("mcp_base")
            .and_then(Value::as_object)
            .ok_or(StatusError::InvalidState)?,
        &layout.local(),
    )?
    .overrides;
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

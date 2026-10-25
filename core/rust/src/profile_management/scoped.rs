//! Client-scoped lifecycle. Preparation is read-only; commits are sequential,
//! deliberately not a cross-client transaction.
use std::{collections::BTreeMap, fs, path::Path, str::FromStr};

use serde_json::{json, Map, Value};

use crate::{
    codec::{json_bytes, sha},
    error::{InstallationError, UninstallError},
    filesystem::{
        check_files, cleanup_empty_directories, file_path, write_bytes, write_executable,
    },
    layout::{Layout, LEGACY_LOCAL, STATE},
    mcp::config::{self, McpConfiguration},
    model::{Client, ClientSelection, ProfileAction, RepositoryContext, Version},
    project::config_for,
    repository::{profile_metadata, source_commit},
};

use super::{
    installation, native,
    payload::{desired_files_for, PayloadFile, PayloadKind},
    uninstall,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

struct Manifest {
    clients: Map<String, Value>,
    retained_legacy: Option<Value>,
}

impl Manifest {
    fn load(context: &RepositoryContext, project: &Path) -> Result<Self> {
        let value = crate::project::manifest_for(context, project)?;
        Ok(Self {
            clients: value["clients"]
                .as_object()
                .ok_or("Brak mapy klientów")?
                .clone(),
            retained_legacy: value.get("retained_legacy").cloned(),
        })
    }

    fn save(&self, project: &Path) -> Result<()> {
        let mut value = json!({"format":2,"clients":self.clients});
        if let Some(retained) = &self.retained_legacy {
            value["retained_legacy"] = retained.clone();
        }
        crate::project::save_manifest(project, &value)?;
        Ok(())
    }
}

fn layout(client: Client, record: &Value) -> Layout {
    Layout::from_state(client, Some(record))
}

/// Render managed command references, not arbitrary values or secrets.
pub(crate) fn render_mcp(servers: &mut Map<String, Value>, layout: Layout) {
    for entry in servers.values_mut() {
        if let Some(command) = entry.get_mut("command").and_then(Value::as_array_mut) {
            for argument in command {
                if let Some(text) = argument.as_str() {
                    if text.starts_with(".opencode/mcp/")
                        || text.starts_with("./.opencode/ai-profile-mcp")
                    {
                        *argument = json!(layout.render(text));
                    }
                }
            }
        }
        if let Some(compose) = entry.pointer_mut("/environment/COMPOSE_FILE") {
            if let Some(text) = compose.as_str() {
                let rendered = text
                    .split(':')
                    .map(|part| {
                        if [
                            "compose.yaml",
                            "compose.host.yaml",
                            "compose.ca.yaml",
                            "compose.auth.yaml",
                            "compose.basic-auth.yaml",
                        ]
                        .iter()
                        .any(|name| part == format!(".opencode/mcp/playwright/{name}"))
                        {
                            layout.managed_path(part)
                        } else {
                            part.to_owned()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(":");
                *compose = json!(rendered);
            }
        }
    }
}

pub(crate) fn files_for(
    context: &RepositoryContext,
    project: &Path,
    profile: &str,
    client: Client,
    old: Option<&Value>,
) -> Result<Vec<PayloadFile>> {
    let metadata = profile_metadata(context, profile)?;
    let layout = Layout::Isolated(client);
    let mut files = desired_files_for(context, profile, client)?;
    for file in &mut files {
        file.relative_path = layout.managed_path(&file.relative_path);
        // Only repository-managed documentation/scripts contain layout references.
        // Native executables and arbitrary binary skill assets stay byte-identical.
        if file.kind == PayloadKind::Data
            && matches!(
                Path::new(&file.relative_path)
                    .extension()
                    .and_then(|s| s.to_str()),
                Some("md" | "sh" | "php" | "yaml")
            )
        {
            if let Ok(text) = std::str::from_utf8(&file.contents) {
                file.contents = layout.render(text).into_bytes();
            }
        }
    }
    Ok(native::project_files_for(
        project,
        metadata
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or(""),
        metadata
            .get("version")
            .and_then(Value::as_str)
            .ok_or("Brak wersji")?,
        old,
        files,
        Some(client),
    )
    .map_err(InstallationError::Native)?)
}

struct Prepared {
    client: Client,
    files: Vec<PayloadFile>,
    writes: BTreeMap<String, Vec<u8>>,
    old: Option<Value>,
    record: Value,
    novamira_archive: Option<Vec<u8>>,
}

fn check_destination(project: &Path, relative: &str) -> Result<()> {
    let path = file_path(project, relative)?;
    if path.exists() && !path.is_file() {
        return Err(InstallationError::DestinationNotFile(relative.into()).into());
    }
    for parent in path.ancestors().skip(1).take_while(|p| *p != project) {
        if parent.exists() && !parent.is_dir() {
            return Err(InstallationError::ParentNotDirectory(parent.into()).into());
        }
    }
    Ok(())
}

fn prepare(
    context: &RepositoryContext,
    project: &Path,
    client: Client,
    profile: &str,
    old: Option<&Value>,
) -> Result<Prepared> {
    let metadata = profile_metadata(context, profile)?;
    let version = metadata
        .get("version")
        .and_then(Value::as_str)
        .ok_or("Brak wersji")?;
    if let Some(old) = old {
        if old.get("profile").and_then(Value::as_str) == Some(profile) {
            if let Some(previous) = old.get("version").and_then(Value::as_str) {
                if Version::from_str(previous)? > Version::from_str(version)? {
                    return Err(InstallationError::Downgrade.into());
                }
            }
        }
    }
    let target = Layout::Isolated(client);
    let skill_directory = file_path(project, &target.skills())?;
    if skill_directory.exists() && !skill_directory.is_dir() {
        return Err("Cel skills nie jest katalogiem".into());
    }
    let skills_directory_added = old
        .filter(|s| layout(client, s) == target)
        .and_then(|s| s.get("skills_directory_added"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| !skill_directory.exists());
    let files = files_for(context, project, profile, client, old)?;
    check_files(
        project,
        old,
        files.iter().map(|file| file.relative_path.as_str()),
    )?;
    let mut base = config::definitions_to_map(&config::desired_mcp(context, profile)?);
    if base.contains_key("novamira") {
        super::credentials::check(project)?;
    }
    render_mcp(&mut base, target);
    let mut writes = BTreeMap::new();
    if base.contains_key("playwright") {
        if let Some(updated) = crate::project::playwright_gitignore_update(project)? {
            writes.insert(".gitignore".to_owned(), updated);
        }
    }
    let legacy = old.is_some_and(|s| layout(client, s) == Layout::Legacy);
    let local = target.local();
    let effective_local = if legacy {
        let source = file_path(project, LEGACY_LOCAL)?;
        let destination = file_path(project, &local)?;
        if source.exists() {
            let bytes = fs::read(&source)?;
            if destination.exists() && fs::read(&destination)? != bytes {
                return Err("Konflikt docelowych local.json; zachowano oba pliki".into());
            }
            if !destination.exists() {
                writes.insert(local.clone(), bytes);
            }
            LEGACY_LOCAL.to_owned()
        } else {
            local.clone()
        }
    } else {
        local.clone()
    };
    let mut resolved = config::resolve_at(project, &base, &effective_local)?;
    if legacy && effective_local == LEGACY_LOCAL {
        render_mcp(&mut resolved.overrides, target);
        resolved = config::resolve_overrides(project, &base, resolved.overrides)?;
        writes.insert(
            local.clone(),
            json_bytes(&json!({"mcp":resolved.overrides}))?,
        );
    }
    let McpConfiguration {
        servers: mcp,
        overrides,
    } = if legacy {
        config::configured_from(project, resolved, &target.adapter())?
    } else {
        config::configured_mcp_at(project, &base, &local, &target.adapter())?
    };
    let mut native_state = Map::new();
    let mut had_config = old
        .and_then(|s| s.get("had_config"))
        .cloned()
        .unwrap_or(json!(false));
    match client {
        Client::OpenCode => {
            let (mut configuration, existed) = config_for(project)?;
            config::check_mcp(
                configuration
                    .as_object()
                    .ok_or("Nieprawidłowa konfiguracja")?,
                old,
                &mcp,
            )?;
            if legacy {
                if let Some(old) = old {
                    uninstall::remove_managed_skill_source(
                        &mut configuration,
                        project,
                        old,
                        Layout::Legacy,
                    )?;
                }
            }
            let added = installation::update_project_config(
                &mut configuration,
                old,
                &mcp,
                project,
                target,
            )?;
            native_state.insert("opencode_skill_path_added".into(), json!(added));
            if old.is_none() {
                had_config = json!(existed);
            }
            writes.insert("opencode.json".into(), json_bytes(&configuration)?);
        }
        Client::Claude => {
            let translated = config::native_client_mcp(&mcp, "claude")?;
            let (settings, mut meta) = native::configure_claude(project, old, &mcp)?;
            preserve_original_file_flag(old, &mut meta, "claude", "settings_had_file");
            let (configuration, mcp_meta) =
                native::configure_claude_mcp(project, old, &translated)?;
            writes.insert(".claude/settings.json".into(), settings);
            writes.insert(".mcp.json".into(), configuration);
            native_state.insert("claude".into(), meta);
            native_state.insert("mcp_claude".into(), mcp_meta);
        }
        Client::Codex => {
            let catalog = files
                .iter()
                .find(|file| file.relative_path == ".agents/plugins/marketplace.json")
                .ok_or("Brak katalogu Codex")?;
            let (configuration, mut meta) =
                native::configure_codex(project, old, &catalog.contents)?;
            preserve_original_file_flag(old, &mut meta, "codex", "config_had_file");
            let translated = config::native_client_mcp(&mcp, "codex")?;
            let (configuration, mcp_meta) = native::configure_codex_mcp(
                &configuration,
                old,
                &translated,
                meta.get("config_had_file").and_then(Value::as_bool),
            )?;
            writes.insert(".codex/config.toml".into(), configuration);
            native_state.insert("codex".into(), meta);
            native_state.insert("mcp_codex".into(), mcp_meta);
        }
    }
    for relative in writes
        .keys()
        .map(String::as_str)
        .chain(files.iter().map(|f| f.relative_path.as_str()))
        .chain([STATE])
    {
        check_destination(project, relative)?;
    }
    let hashes: Map<String, Value> = files
        .iter()
        .map(|f| (f.relative_path.clone(), json!(sha(&f.contents))))
        .collect();
    if let Some(prior) = old
        .and_then(|s| s.get("mcp_base"))
        .and_then(Value::as_object)
    {
        installation::print_override_warnings(prior, &base, &overrides);
    }
    let record = json!({"profile":profile,"version":version,"source_commit":source_commit(context),
        "layout":"isolated","files":hashes,"mcp":mcp,"mcp_base":base,"skills_directory_added":skills_directory_added,
        "mcp_local_hash":sha(&json_bytes(&overrides)?),"had_config":had_config,
        "gitignore_added":false,"instructions_added":false,"native":native_state});
    let novamira_archive = if base.contains_key("novamira") { Some(super::credentials::archive_for(project)?) } else { None };
    Ok(Prepared {
        client,
        files,
        writes,
        old: old.cloned(),
        record,
        novamira_archive,
    })
}

fn preserve_original_file_flag(old: Option<&Value>, meta: &mut Value, client: &str, key: &str) {
    if let Some(original) = old.and_then(|s| s.pointer(&format!("/native/{client}/{key}"))) {
        meta[key] = original.clone();
    }
}

impl Prepared {
    fn commit(
        mut self,
        project: &Path,
        manifest: &mut Manifest,
        runner: &impl crate::process::ProcessRunner,
    ) -> Result<()> {
        if let Some(archive) = &self.novamira_archive {
            super::credentials::prepare(project, archive)?;
        }
        if let Some(old) = &self.old {
            if let Some(files) = old.get("files").and_then(Value::as_object) {
                let removed: Vec<String> = files
                    .keys()
                    .filter(|p| !self.files.iter().any(|f| f.relative_path == **p))
                    .cloned()
                    .collect();
                for relative in &removed {
                    fs::remove_file(file_path(project, relative)?)?;
                }
                cleanup_empty_directories(project, &removed)?;
            }
        }
        for file in &self.files {
            let path = file_path(project, &file.relative_path)?;
            match file.kind {
                PayloadKind::Data => write_bytes(&path, &file.contents)?,
                PayloadKind::Executable => write_executable(&path, &file.contents)?,
            }
        }
        // Native manifests allow an empty skill selection.
        fs::create_dir_all(
            project.join(format!("{}/skills", Layout::Isolated(self.client).plugin())),
        )?;
        for (relative, bytes) in &self.writes {
            native::write_config(project, relative, bytes).map_err(InstallationError::Native)?;
        }
        if self.client == Client::OpenCode {
            if let Some(old) = &self.old {
                crate::project::remove_gitignore_line(project, old)?;
            }
        }
        let key = self.client.as_str();
        let activation = match self.client {
            Client::OpenCode => json!({"activation":"prepared"}),
            Client::Claude => native::activate_claude_with_runner(
                project,
                &self.record["native"][key],
                self.record
                    .pointer("/native/claude/installed_before")
                    .and_then(Value::as_bool)
                    == Some(true),
                runner,
            ),
            Client::Codex => {
                native::activate_selected_codex(project, &self.record["native"][key], runner)
            }
        };
        if self.client != Client::OpenCode {
            let meta = self.record["native"][key]
                .as_object_mut()
                .ok_or("Brak metadanych aktywacji")?;
            meta.insert("activation".into(), activation["activation"].clone());
            for (source, destination) in [
                ("reason", "activation_reason"),
                ("stage", "activation_stage"),
                ("plugin_added", "plugin_added"),
                ("marketplace_added", "marketplace_added"),
            ] {
                if let Some(value) = activation.get(source) {
                    meta.insert(destination.into(), value.clone());
                }
            }
            if activation["activation"] == "synchronized" {
                meta.insert(
                    "last_successful_sync".into(),
                    json!(std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs()),
                );
                if self.client == Client::Claude {
                    meta.insert("installed_before".into(), json!(true));
                }
            } else if let Some(previous) = self
                .old
                .as_ref()
                .and_then(|s| s.pointer(&format!("/native/{key}/last_successful_sync")))
            {
                meta.insert("last_successful_sync".into(), previous.clone());
            }
        }
        manifest.clients.insert(key.into(), self.record);
        manifest.save(project)?;
        let guide = Layout::Isolated(self.client).readme();
        if project.join(&guide).is_file() {
            println!("Instrukcja konfiguracji: {guide}");
        }
        if activation["activation"] == "pending" || activation["activation"] == "failed" {
            println!(
                "Ponów: make update CLIENT={key} PROJECT=\"{}\"",
                project.display()
            );
        }
        if activation["activation"] == "failed" {
            return Err(
                format!("{key}: aktywacja nie powiodła się; przygotowany stan zachowano").into(),
            );
        }
        Ok(())
    }
}

pub(crate) fn run(
    context: &RepositoryContext,
    project: &Path,
    action: ProfileAction,
    selection: ClientSelection,
    profile: Option<&str>,
) -> Result<()> {
    run_with_runner(
        context,
        project,
        action,
        selection,
        profile,
        &native::default_runner(),
    )
}

pub(crate) fn run_with_runner(
    context: &RepositoryContext,
    project: &Path,
    action: ProfileAction,
    selection: ClientSelection,
    profile: Option<&str>,
    runner: &impl crate::process::ProcessRunner,
) -> Result<()> {
    let mut manifest = Manifest::load(context, project)?;
    let selected: Vec<Client> = match selection {
        ClientSelection::One(c) => vec![c],
        ClientSelection::All => Client::ALL.to_vec(),
    };
    if action == ProfileAction::Status {
        let mut errors = Vec::new();
        for client in selected {
            if let Err(error) = super::status::status(
                context,
                project,
                client,
                manifest.clients.get(client.as_str()),
            ) {
                eprintln!("{}: {error}", client.as_str());
                errors.push(client.as_str());
            }
        }
        return if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("Błędy statusu: {}", errors.join(", ")).into())
        };
    }
    let mut prepared = Vec::new();
    let mut removals = Vec::new();
    for client in selected {
        let old = manifest.clients.get(client.as_str());
        match action {
            ProfileAction::Install => {
                let profile = profile.ok_or("install wymaga profilu")?;
                if let Some(old) = old {
                    if selection == ClientSelection::All
                        && old.get("profile").and_then(Value::as_str) == Some(profile)
                    {
                        continue;
                    }
                    return Err(format!(
                        "{}: profil już zainstalowany; użyj update albo switch",
                        client.as_str()
                    )
                    .into());
                }
                prepared.push(prepare(context, project, client, profile, None)?);
            }
            ProfileAction::Update | ProfileAction::Switch | ProfileAction::Uninstall => {
                let Some(old) = old else {
                    if selection == ClientSelection::All {
                        continue;
                    }
                    return Err(format!("{}: brak profilu; użyj install", client.as_str()).into());
                };
                if action == ProfileAction::Uninstall {
                    preflight_remove(project, client, old)?;
                    removals.push((client, old.clone()));
                } else {
                    let profile = if action == ProfileAction::Switch {
                        profile
                    } else {
                        old.get("profile").and_then(Value::as_str)
                    }
                    .ok_or("Brak profilu")?;
                    prepared.push(prepare(context, project, client, profile, Some(old))?);
                }
            }
            ProfileAction::Help
            | ProfileAction::Create
            | ProfileAction::Rename
            | ProfileAction::Status
            | ProfileAction::Validate
            | ProfileAction::MigrateLegacy
            | ProfileAction::Bump => return Err("Nieobsługiwana operacja klienta".into()),
        }
    }
    if prepared.is_empty() && removals.is_empty() {
        if action == ProfileAction::Install {
            println!("OK: wszyscy klienci mają wskazany profil; bez zmian");
            return Ok(());
        }
        return Err(UninstallError::NoInstalledProfile.into());
    }
    let mut errors = Vec::new();
    for operation in prepared {
        let client = operation.client;
        if let Err(error) = operation.commit(project, &mut manifest, runner) {
            eprintln!("{}: {error}", client.as_str());
            errors.push(client.as_str());
        }
    }
    for (client, old) in removals {
        match remove(project, client, &old) {
            Ok(()) => {
                manifest.clients.remove(client.as_str());
                manifest.save(project)?;
                println!("OK: {} usunięty", client.as_str());
            }
            Err(error) => {
                eprintln!("{}: {error}", client.as_str());
                errors.push(client.as_str());
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Operacja nie powiodła się dla: {}; pozostałe wyniki zachowano",
            errors.join(", ")
        )
        .into())
    }
}

fn preflight_remove(project: &Path, client: Client, old: &Value) -> Result<()> {
    check_files(project, Some(old), std::iter::empty())?;
    if layout(client, old) != Layout::Legacy {
        file_path(project, &layout(client, old).skills())?;
    }
    inspect_configuration(project, client, old)
}

pub(crate) fn inspect_configuration(project: &Path, client: Client, old: &Value) -> Result<()> {
    let empty = Map::new();
    match client {
        Client::OpenCode => {
            let (config, _) = config_for(project)?;
            config::check_mcp(
                config.as_object().ok_or("Nieprawidłowa konfiguracja")?,
                Some(old),
                &empty,
            )?;
        }
        Client::Claude => {
            native::configure_claude_mcp(project, Some(old), &empty)?;
            native::configure_claude(project, Some(old), &empty).map_err(UninstallError::Native)?;
        }
        Client::Codex => {
            let path = file_path(project, ".codex/config.toml")?;
            let bytes = if path.exists() {
                fs::read(path)?
            } else {
                Vec::new()
            };
            native::configure_codex_mcp(&bytes, Some(old), &empty, None)?;
            let catalog = fs::read(file_path(project, ".agents/plugins/marketplace.json")?)?;
            native::configure_codex(project, Some(old), &catalog)?;
        }
    }
    Ok(())
}

fn remove(project: &Path, client: Client, old: &Value) -> Result<()> {
    let empty = Map::new();
    match client {
        Client::OpenCode => {
            let (mut config, _) = config_for(project)?;
            uninstall::remove_managed_mcp(&mut config, old)?;
            uninstall::remove_managed_instruction(&mut config, old)?;
            uninstall::remove_managed_skill_source(&mut config, project, old, layout(client, old))?;
            crate::project::remove_gitignore_line(project, old)?;
            let path = file_path(project, "opencode.json")?;
            if old.get("had_config").and_then(Value::as_bool) == Some(false)
                && config
                    .as_object()
                    .is_some_and(|o| o.is_empty() || (o.len() == 1 && o.contains_key("$schema")))
            {
                if path.exists() {
                    fs::remove_file(path)?;
                }
            } else {
                write_bytes(&path, &json_bytes(&config)?)?;
            }
        }
        Client::Claude => {
            let (bytes, _) = native::configure_claude_mcp(project, Some(old), &empty)?;
            if let Some(warning) = native::cleanup_claude(project, old) {
                println!("WARNING: {warning}");
            }
            native::remove_claude(project, old)?;
            let value: Value = serde_json::from_slice(&bytes)?;
            let path = file_path(project, ".mcp.json")?;
            if old
                .pointer("/native/mcp_claude/had_file")
                .and_then(Value::as_bool)
                == Some(false)
                && value.as_object().is_some_and(Map::is_empty)
            {
                if path.exists() {
                    fs::remove_file(path)?;
                }
            } else {
                write_bytes(&path, &bytes)?;
            }
        }
        Client::Codex => {
            if let Some(warning) = native::cleanup_codex(project, old) {
                println!("WARNING: {warning}");
            }
            native::remove_codex(project, old)?;
            let path = file_path(project, ".codex/config.toml")?;
            let bytes = if path.exists() {
                fs::read(&path)?
            } else {
                Vec::new()
            };
            let (bytes, _) = native::configure_codex_mcp(&bytes, Some(old), &empty, None)?;
            if old
                .pointer("/native/mcp_codex/config_had_file")
                .and_then(Value::as_bool)
                == Some(false)
                && String::from_utf8_lossy(&bytes).trim().is_empty()
            {
                if path.exists() {
                    fs::remove_file(path)?;
                }
            } else {
                write_bytes(&path, &bytes)?;
            }
        }
    }
    let files = old
        .get("files")
        .and_then(Value::as_object)
        .ok_or("Brak listy plików")?;
    for relative in files.keys() {
        fs::remove_file(file_path(project, relative)?)?;
    }
    cleanup_empty_directories(project, &files.keys().cloned().collect::<Vec<_>>())?;
    if old.get("skills_directory_added").and_then(Value::as_bool) == Some(true) {
        cleanup_empty_directories(
            project,
            &[format!(
                "{}/.directory-cleanup",
                layout(client, old).skills()
            )],
        )?;
    }
    Ok(())
}

use std::{fs, path::Path, time::Duration};

use serde_json::{json, Map, Value};
use toml_edit::{value, DocumentMut, Item, Table};

use crate::{
    codec::{json_bytes, sha},
    filesystem::{file_path, write_bytes},
    process::{CancellationToken, CommandSpec, ProcessRunner, StdinMode, StreamMode},
    profile_management::payload::{PayloadFile, PayloadKind},
};

const CLAUDE_CATALOG: &str = ".claude-plugin/marketplace.json";
const CODEX_CATALOG: &str = ".agents/plugins/marketplace.json";
const CLAUDE_SETTINGS: &str = ".claude/settings.json";
const CODEX_CONFIG: &str = ".codex/config.toml";
const PLUGIN_ROOT: &str = ".opencode/ai-profile/plugin";

#[cfg(test)]
use self::UnavailableRunner as SystemProcessRunner;
#[cfg(not(test))]
use crate::process::SystemProcessRunner;

pub(crate) fn default_runner() -> impl ProcessRunner {
    SystemProcessRunner
}

#[cfg(test)]
struct UnavailableRunner;

#[cfg(test)]
impl ProcessRunner for UnavailableRunner {
    fn run(
        &self,
        _spec: CommandSpec,
    ) -> Result<crate::process::CommandOutput, crate::error::ProcessError> {
        Err(crate::error::ProcessError::Spawn {
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        })
    }
}

#[cfg(test)]
pub(crate) fn project_files(
    project: &Path,
    description: &str,
    version: &str,
    old: Option<&Value>,
    files: Vec<PayloadFile>,
) -> Result<Vec<PayloadFile>, String> {
    project_files_for(project, description, version, old, files, None)
}

pub(crate) fn project_files_for(
    project: &Path,
    description: &str,
    version: &str,
    old: Option<&Value>,
    mut files: Vec<PayloadFile>,
    client: Option<crate::model::Client>,
) -> Result<Vec<PayloadFile>, String> {
    use crate::{layout::Layout, model::Client};
    let plugin_root =
        client.map_or_else(|| PLUGIN_ROOT.to_owned(), |c| Layout::Isolated(c).plugin());
    let canonical = fs::canonicalize(project).map_err(|error| error.to_string())?;
    let marketplace = format!(
        "ai-profiles-{}",
        &sha(canonical.to_string_lossy().as_bytes())[..12]
    );
    let claude = json!({
        "name": marketplace,
        "owner": {"name": "ai-profiles"},
        "description": description,
        "plugins": [{"name":"ai-profile", "source":format!("./{plugin_root}"), "description":description, "version":version}]
    });
    let codex = json!({
        "name": marketplace,
        "plugins": [{
            "name":"ai-profile",
            "source":{"source":"local", "path":format!("./{plugin_root}")},
            "policy":{"installation":"AVAILABLE", "authentication":"ON_INSTALL"},
            "category":"Developer Tools"
        }]
    });
    for (relative, value) in [(CLAUDE_CATALOG, claude), (CODEX_CATALOG, codex)] {
        if client.is_some_and(|c| match c {
            Client::OpenCode => true,
            Client::Claude => relative != CLAUDE_CATALOG,
            Client::Codex => relative != CODEX_CATALOG,
        }) {
            continue;
        }
        let path = file_path(project, relative).map_err(|error| error.to_string())?;
        if path.exists() {
            let current = fs::read(&path).map_err(|error| error.to_string())?;
            let expected = json_bytes(&value).map_err(|error| error.to_string())?;
            let previously_owned = old
                .and_then(|state| state.get("files"))
                .and_then(|files| files.get(relative))
                .is_some();
            if current != expected && !previously_owned {
                return Err(format!("unowned or conflicting catalog: {relative}"));
            }
        }
        files.push(PayloadFile {
            relative_path: relative.into(),
            contents: json_bytes(&value).map_err(|error| error.to_string())?,
            kind: PayloadKind::Data,
        });
    }
    Ok(files)
}

pub(crate) fn configure_claude(
    project: &Path,
    old: Option<&Value>,
    mcp: &Map<String, Value>,
) -> Result<(Vec<u8>, Value), String> {
    let path = file_path(project, CLAUDE_SETTINGS).map_err(|error| error.to_string())?;
    let mut settings = if path.exists() {
        crate::codec::read_json(&path).map_err(|error| error.to_string())?
    } else {
        json!({})
    };
    let settings_had_file = old
        .and_then(|s| s.pointer("/native/claude/settings_had_file"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| path.exists());
    let object = settings
        .as_object_mut()
        .ok_or_else(|| "Claude settings must be an object".to_owned())?;
    let canonical = fs::canonicalize(project).map_err(|error| error.to_string())?;
    let marketplace = format!(
        "ai-profiles-{}",
        &sha(canonical.to_string_lossy().as_bytes())[..12]
    );
    let marketplace_map = object
        .entry("extraKnownMarketplaces")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| "extraKnownMarketplaces must be an object".to_owned())?;
    let source = json!({"source":"directory", "path":canonical.to_string_lossy()});
    let was_present = marketplace_map.contains_key(&marketplace);
    let previous_marketplace = old
        .and_then(|state| state.pointer("/native/claude/marketplace_previous"))
        .cloned()
        .unwrap_or_else(|| {
            marketplace_map
                .get(&marketplace)
                .cloned()
                .unwrap_or(Value::Null)
        });
    if let Some(current) = marketplace_map.get(&marketplace) {
        if !current.is_object() {
            return Err("Claude marketplace source must be an object".into());
        }
        if current != &source {
            return Err(format!(
                "Claude marketplace identity {marketplace} already points to a different source"
            ));
        }
    }
    marketplace_map.insert(marketplace.clone(), source.clone());
    let enabled_key = format!("ai-profile@{marketplace}");
    let plugins = object
        .entry("enabledPlugins")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| "enabledPlugins must be an object".to_owned())?;
    let previous_enabled = old
        .and_then(|state| state.pointer("/native/claude/enabled_previous"))
        .cloned()
        .unwrap_or_else(|| plugins.get(&enabled_key).cloned().unwrap_or(Value::Null));
    if plugins
        .get(&enabled_key)
        .is_some_and(|entry| !entry.is_boolean())
    {
        return Err(format!("enabledPlugins.{enabled_key} must be a boolean"));
    }
    let explicitly_disabled = plugins.get(&enabled_key).and_then(Value::as_bool) == Some(false);
    let enabled_added = old
        .and_then(|state| state.pointer("/native/claude/enabled_added"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| !plugins.contains_key(&enabled_key));
    if enabled_added && !explicitly_disabled {
        plugins.insert(enabled_key.clone(), Value::Bool(true));
    }
    let required_timeout = mcp
        .values()
        .filter(|entry| {
            entry.get("disabled").and_then(Value::as_bool) != Some(true)
                && entry.get("enabled").and_then(Value::as_bool) != Some(false)
        })
        .filter_map(|entry| entry.pointer("/timeout/startup").and_then(Value::as_u64))
        .max();
    let timeout_meta = reconcile_claude_mcp_timeout(object, old, required_timeout)?;
    let metadata = json!({
        "marketplace_name":marketplace,
        "settings_had_file":settings_had_file,
        "plugin_key":enabled_key,
        "marketplace_previous":previous_marketplace,
        "marketplace_added":old.and_then(|s| s.pointer("/native/claude/marketplace_added")).and_then(Value::as_bool).unwrap_or(!was_present),
        "marketplace_expected":source,
        "enabled_previous":previous_enabled,
        "enabled_expected":true,
        "enabled_current":!explicitly_disabled,
        "enabled_added":enabled_added,
        "installed_before":old.and_then(|state| state.pointer("/native/claude/installed_before")).and_then(Value::as_bool).unwrap_or_else(|| old.and_then(|state| state.pointer("/native/claude/activation")).and_then(Value::as_str) == Some("synchronized")),
        "mcp_timeout":timeout_meta,
        "activation":"pending"
    });
    Ok((
        json_bytes(&settings).map_err(|error| error.to_string())?,
        metadata,
    ))
}

fn reconcile_claude_mcp_timeout(
    object: &mut Map<String, Value>,
    old: Option<&Value>,
    required: Option<u64>,
) -> Result<Value, String> {
    let old_meta = old.and_then(|v| v.pointer("/native/claude/mcp_timeout"));
    let old_added = old_meta
        .and_then(|v| v.get("added"))
        .and_then(Value::as_bool)
        == Some(true);
    let old_expected = old_meta.and_then(|v| v.get("expected"));
    let env_had = object.get("env").is_some();
    let env = object
        .entry("env")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("Claude settings env must be an object")?;
    let current = env.get("MCP_TIMEOUT").cloned();
    if old_added && current.as_ref() != old_expected {
        return Err("managed Claude env.MCP_TIMEOUT was manually changed".into());
    }
    let previous = old_meta
        .and_then(|v| v.get("previous"))
        .cloned()
        .unwrap_or_else(|| current.clone().unwrap_or(Value::Null));
    let mut added = old_added;
    if let Some(required) = required {
        if old_added || current.is_none() {
            let expected = Value::String(required.to_string());
            env.insert("MCP_TIMEOUT".into(), expected.clone());
            return Ok(
                json!({"added":true,"expected":expected,"previous":previous,"env_had":old_meta.and_then(|v|v.get("env_had")).and_then(Value::as_bool).unwrap_or(env_had)}),
            );
        }
        let configured = current.as_ref().and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .or_else(|| v.as_u64())
        });
        if configured.is_none_or(|value| value < required) {
            eprintln!("WARNING: Claude Code env.MCP_TIMEOUT is below the selected MCP startup timeout ({required} ms); the user's value was preserved.");
        }
    } else if old_added {
        if previous.is_null() {
            env.remove("MCP_TIMEOUT");
        } else {
            env.insert("MCP_TIMEOUT".into(), previous.clone());
        }
        added = false;
    }
    if env.is_empty() && !env_had {
        object.remove("env");
    }
    Ok(
        json!({"added":added,"expected":old_expected.cloned().unwrap_or(Value::Null),"previous":previous,"env_had":old_meta.and_then(|v|v.get("env_had")).and_then(Value::as_bool).unwrap_or(env_had)}),
    )
}

/// Reconcile Claude Code's project MCP file by entry ownership. Other root
/// fields and unrelated server entries are preserved byte-for-byte semantically.
pub(crate) fn configure_claude_mcp(
    project: &Path,
    old: Option<&Value>,
    desired: &Map<String, Value>,
) -> Result<(Vec<u8>, Value), String> {
    let relative = ".mcp.json";
    let path = file_path(project, relative).map_err(|e| e.to_string())?;
    let had_file = path.exists();
    let mut root = if had_file {
        crate::codec::read_json(&path).map_err(|e| e.to_string())?
    } else {
        json!({})
    };
    let root_obj = root
        .as_object_mut()
        .ok_or(".mcp.json must contain an object")?;
    let container_had_key = old
        .and_then(|v| v.pointer("/native/mcp_claude/container_had_key"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| root_obj.contains_key("mcpServers"));
    let prior = old
        .and_then(|v| v.pointer("/native/mcp_claude/entries"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    if desired.is_empty() && prior.is_empty() {
        return Ok((
            json_bytes(&root).map_err(|e| e.to_string())?,
            json!({"entries":{},"had_file":old.and_then(|v|v.pointer("/native/mcp_claude/had_file")).and_then(Value::as_bool).unwrap_or(had_file),"container_had_key":container_had_key}),
        ));
    }
    let entries = root_obj
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or(".mcp.json mcpServers must be an object")?;
    for (name, expected) in &prior {
        if entries.get(name) != Some(expected) {
            return Err(format!(
                "managed Claude MCP entry {name} was manually changed"
            ));
        }
    }
    for name in desired.keys() {
        if entries.contains_key(name) && !prior.contains_key(name) {
            return Err(format!("Claude MCP name collision: {name}"));
        }
    }
    for name in prior.keys() {
        entries.remove(name);
    }
    for (name, entry) in desired {
        entries.insert(name.clone(), entry.clone());
    }
    if entries.is_empty() && !container_had_key {
        root_obj.remove("mcpServers");
    }
    Ok((
        json_bytes(&root).map_err(|e| e.to_string())?,
        json!({"entries":desired,"had_file":old.and_then(|v|v.pointer("/native/mcp_claude/had_file")).and_then(Value::as_bool).unwrap_or(had_file),"container_had_key":container_had_key}),
    ))
}

/// Reconcile Codex MCP entries with toml_edit, preserving comments and all
/// unrelated TOML tables. This intentionally shares the file with plugin setup.
pub(crate) fn configure_codex_mcp(
    config: &[u8],
    old: Option<&Value>,
    desired: &Map<String, Value>,
    original_config_existed: Option<bool>,
) -> Result<(Vec<u8>, Value), String> {
    let text = std::str::from_utf8(config).map_err(|e| e.to_string())?;
    let mut document = text
        .parse::<DocumentMut>()
        .map_err(|_| "Nieprawidłowy TOML Codex (treść konfiguracji ukryta)".to_owned())?;
    let prior = old
        .and_then(|v| v.pointer("/native/mcp_codex/entries"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let table_had_key = old
        .and_then(|v| v.pointer("/native/mcp_codex/table_had_key"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| document.get("mcp_servers").is_some());
    let config_had_file = old
        .and_then(|v| v.pointer("/native/mcp_codex/config_had_file"))
        .and_then(Value::as_bool)
        .or(original_config_existed)
        .or_else(|| {
            old.and_then(|v| v.pointer("/native/codex/config_had_file"))
                .and_then(Value::as_bool)
        })
        .unwrap_or(!config.is_empty());
    let existing = document
        .get("mcp_servers")
        .and_then(Item::as_table)
        .cloned()
        .unwrap_or_default();
    for (name, expected) in &prior {
        let current = existing.get(name).and_then(toml_item_to_json);
        if current.as_ref() != Some(expected) {
            return Err(format!(
                "managed Codex MCP entry {name} was manually changed"
            ));
        }
    }
    for name in desired.keys() {
        if existing.contains_key(name) && !prior.contains_key(name) {
            return Err(format!("Codex MCP name collision: {name}"));
        }
    }
    if desired.is_empty() && prior.is_empty() {
        return Ok((
            document.to_string().into_bytes(),
            json!({"entries":{},"table_had_key":table_had_key,"config_had_file":config_had_file}),
        ));
    }
    let table = document
        .as_table_mut()
        .entry("mcp_servers")
        .or_insert(Item::Table(Table::new()))
        .as_table_mut()
        .ok_or("Codex mcp_servers must be a table")?;
    for name in prior.keys() {
        table.remove(name);
    }
    for (name, entry) in desired {
        let item = json_to_toml(entry).ok_or_else(|| format!("cannot encode Codex MCP {name}"))?;
        table.insert(name, item);
    }
    if table.is_empty() && !table_had_key {
        document.as_table_mut().remove("mcp_servers");
    }
    Ok((
        document.to_string().into_bytes(),
        json!({"entries":desired,"table_had_key":table_had_key,"config_had_file":config_had_file}),
    ))
}

fn toml_item_to_json(item: &Item) -> Option<Value> {
    match item {
        Item::Value(value) => toml_value_to_json(value),
        Item::Table(table) => Some(Value::Object(
            table
                .iter()
                .filter_map(|(k, v)| Some((k.to_owned(), toml_item_to_json(v)?)))
                .collect(),
        )),
        _ => None,
    }
}

fn toml_value_to_json(value: &toml_edit::Value) -> Option<Value> {
    Some(match value {
        toml_edit::Value::String(v) => Value::String(v.value().clone()),
        toml_edit::Value::Integer(v) => Value::from(*v.value()),
        toml_edit::Value::Float(v) => Value::Number(serde_json::Number::from_f64(*v.value())?),
        toml_edit::Value::Boolean(v) => Value::Bool(*v.value()),
        toml_edit::Value::Datetime(v) => Value::String(v.value().to_string()),
        toml_edit::Value::Array(a) => Value::Array(
            a.iter()
                .map(toml_value_to_json)
                .collect::<Option<Vec<_>>>()?,
        ),
        toml_edit::Value::InlineTable(t) => Value::Object(
            t.iter()
                .map(|(k, v)| Some((k.to_owned(), toml_value_to_json(v)?)))
                .collect::<Option<serde_json::Map<_, _>>>()?,
        ),
    })
}

fn json_to_toml(value: &Value) -> Option<Item> {
    use toml_edit::{Array, InlineTable, Value as TomlValue};
    Some(match value {
        Value::Null => return None,
        Value::Bool(v) => Item::Value(TomlValue::from(*v)),
        Value::Number(v) => {
            if let Some(i) = v.as_i64() {
                Item::Value(TomlValue::from(i))
            } else {
                Item::Value(TomlValue::from(v.as_f64()?))
            }
        }
        Value::String(v) => Item::Value(TomlValue::from(v.as_str())),
        Value::Array(values) => {
            let mut a = Array::new();
            for v in values {
                let Item::Value(v) = json_to_toml(v)? else {
                    return None;
                };
                a.push(v);
            }
            Item::Value(TomlValue::Array(a))
        }
        Value::Object(values) => {
            let mut t = InlineTable::new();
            for (k, v) in values {
                let Item::Value(v) = json_to_toml(v)? else {
                    return None;
                };
                t.insert(k, v);
            }
            Item::Value(TomlValue::InlineTable(t))
        }
    })
}

pub(crate) fn configure_codex(
    project: &Path,
    old: Option<&Value>,
    catalog_bytes: &[u8],
) -> Result<(Vec<u8>, Value), String> {
    let path = file_path(project, CODEX_CONFIG).map_err(|error| error.to_string())?;
    let text = if path.exists() {
        fs::read_to_string(&path).map_err(|error| error.to_string())?
    } else {
        String::new()
    };
    let config_had_file = old
        .and_then(|s| s.pointer("/native/codex/config_had_file"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| path.exists());
    let mut document = text
        .parse::<DocumentMut>()
        .map_err(|_| "Nieprawidłowy TOML Codex (treść konfiguracji ukryta)".to_owned())?;
    let plugins_had_table = old
        .and_then(|s| s.pointer("/native/codex/plugins_had_table"))
        .and_then(Value::as_bool)
        .unwrap_or_else(|| document.as_table().contains_key("plugins"));
    let (marketplace, plugin_name, source_path) = codex_identity(project, catalog_bytes)?;
    let key = format!("{plugin_name}@{marketplace}");
    let plugins = table_mut(document.as_table_mut(), "plugins")?;
    let prior_item = plugins.get(&key).cloned();
    if let Some(item) = &prior_item {
        if item
            .get("enabled")
            .is_some_and(|enabled| enabled.as_bool().is_none())
        {
            return Err(format!("Codex plugins.{key}.enabled must be a boolean"));
        }
        if item.get("enabled").and_then(Item::as_bool) == Some(false) {
            return Ok((
                document.to_string().into_bytes(),
                json!({"plugin_key":key,"marketplace_name":marketplace,"plugin_name":plugin_name,"source_path":source_path,"config_had_file":config_had_file,"plugins_had_table":plugins_had_table,"enabled_current":false,"enabled_added":old.and_then(|state| state.pointer("/native/codex/enabled_added")).and_then(Value::as_bool).unwrap_or(false),"enabled_previous":old.and_then(|state| state.pointer("/native/codex/enabled_previous")).cloned().unwrap_or(json!(false)),"marketplace_added":old.and_then(|state| state.pointer("/native/codex/marketplace_added")).and_then(Value::as_bool).unwrap_or(false),"plugin_added":old.and_then(|state| state.pointer("/native/codex/plugin_added")).and_then(Value::as_bool).unwrap_or(false),"activation":"disabled"}),
            ));
        }
        if old.is_none() && item.get("enabled").and_then(Item::as_bool) != Some(true) {
            return Err(format!(
                "Codex plugin {key} already has a conflicting configuration"
            ));
        }
    }
    let plugin = table_mut(plugins, &key)?;
    plugin["enabled"] = value(true);
    let enabled_added = old
        .and_then(|state| state.pointer("/native/codex/enabled_added"))
        .and_then(Value::as_bool)
        .unwrap_or(prior_item.is_none());
    let previous = old
        .and_then(|state| state.pointer("/native/codex/enabled_previous"))
        .cloned()
        .unwrap_or_else(|| {
            prior_item
                .as_ref()
                .and_then(|item| item.get("enabled").and_then(Item::as_bool))
                .map(Value::Bool)
                .unwrap_or(Value::Null)
        });
    Ok((
        document.to_string().into_bytes(),
        json!({"plugin_key":key,"marketplace_name":marketplace,"plugin_name":plugin_name,"source_path":source_path,"config_had_file":config_had_file,"plugins_had_table":plugins_had_table,"enabled_added":enabled_added,"enabled_previous":previous,"marketplace_added":old.and_then(|state| state.pointer("/native/codex/marketplace_added")).and_then(Value::as_bool).unwrap_or(false),"plugin_added":old.and_then(|state| state.pointer("/native/codex/plugin_added")).and_then(Value::as_bool).unwrap_or(false),"activation":"pending"}),
    ))
}

fn codex_identity(
    project: &Path,
    catalog_bytes: &[u8],
) -> Result<(String, String, String), String> {
    let value: Value = serde_json::from_slice(catalog_bytes)
        .map_err(|error| format!("invalid generated Codex marketplace catalog: {error}"))?;
    let marketplace = value
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "Codex marketplace catalog has no valid name".to_owned())?;
    let plugins = value
        .get("plugins")
        .and_then(Value::as_array)
        .ok_or_else(|| "Codex marketplace catalog has no plugin list".to_owned())?;
    let [plugin] = plugins.as_slice() else {
        return Err("Codex marketplace catalog must contain exactly one plugin".into());
    };
    let plugin_name = plugin
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "Codex marketplace plugin has no valid name".to_owned())?;
    let source_path = plugin
        .pointer("/source/path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .ok_or_else(|| "Codex ai-profile catalog entry has no local source path".to_owned())?;
    let expected_source = format!("./{PLUGIN_ROOT}");
    let isolated_source = format!(
        "./{}",
        crate::layout::Layout::Isolated(crate::model::Client::Codex).plugin()
    );
    if source_path != expected_source && source_path != isolated_source {
        return Err(format!(
            "Codex ai-profile source must be {expected_source} or {isolated_source}"
        ));
    }
    let canonical = fs::canonicalize(project).map_err(|error| error.to_string())?;
    Ok((
        marketplace.to_owned(),
        plugin_name.to_owned(),
        canonical.to_string_lossy().into_owned(),
    ))
}

fn table_mut<'a>(table: &'a mut Table, key: &str) -> Result<&'a mut Table, String> {
    if !table.contains_key(key) {
        table.insert(key, Item::Table(Table::new()));
    }
    table
        .get_mut(key)
        .and_then(Item::as_table_mut)
        .ok_or_else(|| format!("TOML table {key} has an incompatible shape"))
}

pub(crate) fn write_config(project: &Path, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let path = file_path(project, relative).map_err(|error| error.to_string())?;
    write_bytes(&path, bytes).map_err(|error| error.to_string())
}

#[cfg(test)]
pub(crate) fn mcp_configuration_status(
    project: &Path,
    state: &Value,
) -> Result<(String, String), String> {
    let claude_owned = state
        .pointer("/native/mcp_claude/entries")
        .and_then(Value::as_object);
    let codex_owned = state
        .pointer("/native/mcp_codex/entries")
        .and_then(Value::as_object);
    let Some(claude_owned) = claude_owned else {
        return Ok((
            "not tracked; run update".into(),
            "not tracked; run update".into(),
        ));
    };
    let claude_path = file_path(project, ".mcp.json").map_err(|e| e.to_string())?;
    let claude_value = if claude_path.exists() {
        crate::codec::read_json(&claude_path).map_err(|e| e.to_string())?
    } else {
        json!({})
    };
    let current = claude_value
        .pointer("/mcpServers")
        .and_then(Value::as_object);
    let claude_changes = claude_owned
        .iter()
        .filter(|(name, expected)| current.and_then(|m| m.get(*name)) != Some(*expected))
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let codex_path = file_path(project, CODEX_CONFIG).map_err(|e| e.to_string())?;
    let codex_text = if codex_path.exists() {
        fs::read_to_string(&codex_path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };
    let doc = codex_text
        .parse::<DocumentMut>()
        .map_err(|e| format!("invalid Codex TOML: {e}"))?;
    let table = doc.get("mcp_servers").and_then(Item::as_table);
    let mut codex_changes = Vec::new();
    if let Some(codex_owned) = codex_owned {
        for (name, expected) in codex_owned {
            if table
                .and_then(|t| t.get(name))
                .and_then(toml_item_to_json)
                .as_ref()
                != Some(expected)
            {
                codex_changes.push(name.clone());
            }
        }
    } else if claude_owned.is_empty() { /* legacy state can be migrated by update */
    }
    let format = |changes: Vec<String>, owned: &Map<String, Value>| {
        if !changes.is_empty() {
            format!("locally modified ({})", changes.join(", "))
        } else if owned.is_empty() {
            "prepared (no enabled servers)".to_owned()
        } else {
            "prepared; runtime connection not assessed".to_owned()
        }
    };
    let claude = format(claude_changes, claude_owned);
    let codex = codex_owned
        .map(|owned| format(codex_changes, owned))
        .unwrap_or_else(|| "not tracked; run update".into());
    Ok((claude, codex))
}

pub(crate) fn remove_claude(project: &Path, old: &Value) -> Result<(), String> {
    let path = file_path(project, CLAUDE_SETTINGS).map_err(|error| error.to_string())?;
    if !path.exists() {
        return Ok(());
    }
    let mut settings = crate::codec::read_json(&path).map_err(|error| error.to_string())?;
    let object = settings
        .as_object_mut()
        .ok_or_else(|| "Claude settings must be an object".to_owned())?;
    if old
        .pointer("/native/claude/mcp_timeout/added")
        .and_then(Value::as_bool)
        == Some(true)
    {
        let expected = old
            .pointer("/native/claude/mcp_timeout/expected")
            .cloned()
            .unwrap_or(Value::Null);
        let previous = old
            .pointer("/native/claude/mcp_timeout/previous")
            .cloned()
            .unwrap_or(Value::Null);
        if let Some(env) = object.get_mut("env").and_then(Value::as_object_mut) {
            if env.get("MCP_TIMEOUT") == Some(&expected) {
                if previous.is_null() {
                    env.remove("MCP_TIMEOUT");
                } else {
                    env.insert("MCP_TIMEOUT".into(), previous);
                }
            }
            if env.is_empty()
                && old
                    .pointer("/native/claude/mcp_timeout/env_had")
                    .and_then(Value::as_bool)
                    != Some(true)
            {
                object.remove("env");
            }
        }
    }
    if old
        .pointer("/native/claude/marketplace_added")
        .and_then(Value::as_bool)
        == Some(true)
    {
        if let (Some(name), Some(previous), Some(expected)) = (
            old.pointer("/native/claude/marketplace_name")
                .and_then(Value::as_str),
            old.pointer("/native/claude/marketplace_previous"),
            old.pointer("/native/claude/marketplace_expected"),
        ) {
            if let Some(map) = object
                .get_mut("extraKnownMarketplaces")
                .and_then(Value::as_object_mut)
            {
                if map.get(name) == Some(expected) {
                    if previous.is_null() {
                        map.remove(name);
                    } else {
                        map.insert(name.to_owned(), previous.clone());
                    }
                }
                if map.is_empty() {
                    object.remove("extraKnownMarketplaces");
                }
            }
        }
    }
    if old
        .pointer("/native/claude/enabled_added")
        .and_then(Value::as_bool)
        == Some(true)
    {
        if let (Some(key), Some(previous)) = (
            old.pointer("/native/claude/plugin_key")
                .and_then(Value::as_str),
            old.pointer("/native/claude/enabled_previous"),
        ) {
            if let Some(map) = object
                .get_mut("enabledPlugins")
                .and_then(Value::as_object_mut)
            {
                if map.get(key).and_then(Value::as_bool) == Some(true) {
                    if previous.is_null() {
                        map.remove(key);
                    } else {
                        map.insert(key.to_owned(), previous.clone());
                    }
                }
                if map.is_empty() {
                    object.remove("enabledPlugins");
                }
            }
        }
    }
    if old
        .pointer("/native/claude/settings_had_file")
        .and_then(Value::as_bool)
        == Some(false)
        && settings.as_object().is_some_and(Map::is_empty)
    {
        fs::remove_file(&path).map_err(|error| error.to_string())
    } else {
        write_bytes(
            &path,
            &json_bytes(&settings).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }
}

pub(crate) fn remove_codex(project: &Path, old: &Value) -> Result<(), String> {
    if old
        .pointer("/native/codex/enabled_added")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Ok(());
    }
    let path = file_path(project, CODEX_CONFIG).map_err(|error| error.to_string())?;
    if !path.exists() {
        return Ok(());
    }
    let text = fs::read_to_string(&path).map_err(|error| error.to_string())?;
    let mut document = text
        .parse::<DocumentMut>()
        .map_err(|error| format!("invalid Codex TOML: {error}"))?;
    if let Some(key) = old
        .pointer("/native/codex/plugin_key")
        .and_then(Value::as_str)
    {
        if let Some(plugins) = document
            .as_table_mut()
            .get_mut("plugins")
            .and_then(Item::as_table_mut)
        {
            if plugins
                .get(key)
                .and_then(|plugin| plugin.get("enabled"))
                .and_then(Item::as_bool)
                == Some(true)
            {
                plugins.remove(key);
            }
            if plugins.is_empty()
                && old
                    .pointer("/native/codex/plugins_had_table")
                    .and_then(Value::as_bool)
                    == Some(false)
            {
                document.as_table_mut().remove("plugins");
            }
        }
    }
    if old
        .pointer("/native/codex/config_had_file")
        .and_then(Value::as_bool)
        == Some(false)
        && document.to_string().trim().is_empty()
    {
        fs::remove_file(&path).map_err(|error| error.to_string())
    } else {
        write_bytes(&path, document.to_string().as_bytes()).map_err(|error| error.to_string())
    }
}

pub(crate) fn cleanup_codex(project: &Path, old: &Value) -> Option<String> {
    let marketplace = old
        .pointer("/native/codex/marketplace_name")
        .and_then(Value::as_str)?;
    let plugin = old
        .pointer("/native/codex/plugin_name")
        .and_then(Value::as_str)?;
    let selector = format!("{plugin}@{marketplace}");
    let runner = SystemProcessRunner;
    if old
        .pointer("/native/codex/plugin_added")
        .and_then(Value::as_bool)
        == Some(true)
    {
        let removed = run_cli_value(
            &runner,
            "codex",
            &["plugin", "remove", &selector, "--json"],
            project,
        );
        if removed.get("activation").and_then(Value::as_str) != Some("synchronized") {
            return Some(format!(
                "Codex plugin cleanup failed: {}",
                removed
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
            ));
        }
    }
    if old
        .pointer("/native/codex/marketplace_added")
        .and_then(Value::as_bool)
        == Some(true)
    {
        let listed = run_cli(
            &runner,
            "codex",
            &["plugin", "marketplace", "list", "--json"],
            project,
        );
        if !listed.succeeded() {
            return Some(format!(
                "Codex marketplace cleanup skipped: {}",
                command_error(&listed)
            ));
        }
        let parsed: Value = match serde_json::from_str(&listed.stdout) {
            Ok(value) => value,
            Err(error) => {
                return Some(format!(
                    "Codex marketplace cleanup skipped: invalid listing JSON: {error}"
                ))
            }
        };
        let source_path = old
            .pointer("/native/codex/source_path")
            .and_then(Value::as_str)
            .unwrap_or("");
        let owned = parsed
            .get("marketplaces")
            .and_then(Value::as_array)
            .is_some_and(|entries| {
                entries.iter().any(|entry| {
                    entry.get("name").and_then(Value::as_str) == Some(marketplace)
                        && entry.get("root").and_then(Value::as_str) == Some(source_path)
                        && entry
                            .pointer("/marketplaceSource/sourceType")
                            .and_then(Value::as_str)
                            == Some("local")
                        && entry
                            .pointer("/marketplaceSource/source")
                            .and_then(Value::as_str)
                            == Some(source_path)
                })
            });
        if owned {
            let removed = run_cli_value(
                &runner,
                "codex",
                &["plugin", "marketplace", "remove", marketplace, "--json"],
                project,
            );
            if removed.get("activation").and_then(Value::as_str) != Some("synchronized") {
                return Some(format!(
                    "Codex marketplace cleanup failed: {}",
                    removed
                        .get("reason")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown error")
                ));
            }
        }
    }
    None
}

#[cfg(test)]
pub(crate) fn activate(
    project: &Path,
    claude: &Value,
    codex: &Value,
    claude_installed: bool,
) -> (Value, Value) {
    (
        activate_claude_with_runner(project, claude, claude_installed, &SystemProcessRunner),
        activate_selected_codex(project, codex, &SystemProcessRunner),
    )
}

pub(crate) fn activate_claude_with_runner(
    project: &Path,
    claude: &Value,
    claude_installed: bool,
    runner: &impl ProcessRunner,
) -> Value {
    let marketplace = claude
        .get("marketplace_name")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let plugin_key = claude
        .get("plugin_key")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty());
    let claude_disabled = claude.get("enabled_current").and_then(Value::as_bool) == Some(false);
    if claude_disabled {
        json!({"activation":"disabled"})
    } else if let (Some(_marketplace), Some(plugin_key)) = (marketplace, plugin_key) {
        if claude_installed {
            run_cli_value(
                runner,
                "claude",
                &[
                    "plugin", "update", plugin_key, "--scope", "project", "--yes",
                ],
                project,
            )
        } else {
            run_cli_value(
                runner,
                "claude",
                &[
                    "plugin", "install", plugin_key, "--scope", "project", "--yes",
                ],
                project,
            )
        }
    } else {
        json!({"activation":"failed","reason":"Claude plugin identity is missing"})
    }
}

pub(crate) fn activate_selected_codex(
    project: &Path,
    codex: &Value,
    runner: &impl ProcessRunner,
) -> Value {
    let codex_disabled = codex.get("enabled_current").and_then(Value::as_bool) == Some(false)
        || (codex.get("enabled_previous").and_then(Value::as_bool) == Some(false)
            && codex.get("enabled_added").and_then(Value::as_bool) == Some(false));
    if codex_disabled {
        json!({"activation":"disabled"})
    } else {
        activate_codex_with_runner(project, codex, runner)
    }
}

#[derive(Debug)]
struct NativeCommandOutcome {
    status: Option<i32>,
    stdout: String,
    stderr: String,
    error: Option<String>,
}

impl NativeCommandOutcome {
    fn succeeded(&self) -> bool {
        self.error.is_none() && self.status == Some(0)
    }
}

fn run_cli(
    runner: &impl ProcessRunner,
    program: &str,
    args: &[&str],
    project: &Path,
) -> NativeCommandOutcome {
    let result = runner.run(CommandSpec {
        program: program.into(),
        args: args.iter().map(Into::into).collect(),
        cwd: Some(project.to_owned()),
        env: Default::default(),
        clear_env: false,
        stdin: StdinMode::Null,
        stdout: StreamMode::Capture,
        stderr: StreamMode::Capture,
        timeout: Some(Duration::from_secs(30)),
        cancellation: CancellationToken::new(),
    });
    match result {
        Err(error) => NativeCommandOutcome {
            status: None,
            stdout: String::new(),
            stderr: String::new(),
            error: Some(
                if matches!(&error, crate::error::ProcessError::Spawn { source } if source.kind() == std::io::ErrorKind::NotFound)
                {
                    format!("{program} CLI is not installed")
                } else {
                    error.to_string()
                },
            ),
        },
        Ok(output) => NativeCommandOutcome {
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            error: None,
        },
    }
}

fn command_error(outcome: &NativeCommandOutcome) -> String {
    outcome.error.clone().unwrap_or_else(|| {
        let detail = if outcome.stderr.is_empty() {
            &outcome.stdout
        } else {
            &outcome.stderr
        };
        if detail.is_empty() {
            format!("command exited with status {:?}", outcome.status)
        } else {
            detail.clone()
        }
    })
}

fn command_activation_state(outcome: &NativeCommandOutcome) -> &'static str {
    if outcome
        .error
        .as_deref()
        .is_some_and(|error| error.contains("CLI is not installed"))
    {
        "pending"
    } else {
        "failed"
    }
}

fn run_cli_value(
    runner: &impl ProcessRunner,
    program: &str,
    args: &[&str],
    project: &Path,
) -> Value {
    let outcome = run_cli(runner, program, args, project);
    if outcome.succeeded() {
        json!({"activation":"synchronized"})
    } else {
        let state = if outcome
            .error
            .as_deref()
            .is_some_and(|error| error.contains("CLI is not installed"))
        {
            "pending"
        } else {
            "failed"
        };
        json!({"activation":state,"reason":command_error(&outcome)})
    }
}

pub(crate) fn activate_codex_with_runner(
    project: &Path,
    codex: &Value,
    runner: &impl ProcessRunner,
) -> Value {
    if codex.get("enabled_current").and_then(Value::as_bool) == Some(false)
        || (codex.get("enabled_previous").and_then(Value::as_bool) == Some(false)
            && codex.get("enabled_added").and_then(Value::as_bool) == Some(false))
    {
        return json!({"activation":"disabled"});
    }
    let Some(marketplace) = codex
        .get("marketplace_name")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
    else {
        return codex_failure("registration", "Codex marketplace identity is missing");
    };
    let Some(plugin) = codex
        .get("plugin_name")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
    else {
        return codex_failure("registration", "Codex plugin identity is missing");
    };
    let Some(source_path) = codex
        .get("source_path")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
    else {
        return codex_failure("registration", "Codex marketplace source path is missing");
    };
    let source_path = Path::new(source_path);
    let expected_key = format!("{plugin}@{marketplace}");
    let Some(configured_key) = codex
        .get("plugin_key")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
    else {
        return codex_failure("registration", "Codex configuration plugin key is missing");
    };
    if configured_key != expected_key {
        return codex_failure(
            "registration",
            format!(
                "Codex plugin key {configured_key} does not match catalog identity {expected_key}"
            ),
        );
    }
    let canonical_project = match fs::canonicalize(project) {
        Ok(path) => path,
        Err(error) => {
            return codex_failure(
                "registration",
                format!("cannot resolve Codex project path: {error}"),
            )
        }
    };
    if source_path != canonical_project.as_path() {
        return codex_failure(
            "registration",
            format!(
                "Codex marketplace source {} does not match project {}",
                source_path.display(),
                canonical_project.display()
            ),
        );
    }
    let previously_added_marketplace =
        codex.get("marketplace_added").and_then(Value::as_bool) == Some(true);
    let previously_added_plugin = codex.get("plugin_added").and_then(Value::as_bool) == Some(true);
    let list = run_cli(
        runner,
        "codex",
        &["plugin", "marketplace", "list", "--json"],
        project,
    );
    if !list.succeeded() {
        let state = if list
            .error
            .as_deref()
            .is_some_and(|error| error.contains("CLI is not installed"))
        {
            "pending"
        } else {
            "failed"
        };
        return json!({"activation":state,"stage":"registration","reason":command_error(&list)});
    }
    let registrations: Value = match serde_json::from_str(&list.stdout) {
        Ok(value) => value,
        Err(error) => {
            return codex_failure(
                "registration",
                format!("invalid marketplace list JSON: {error}"),
            )
        }
    };
    let entries = match registrations.get("marketplaces").and_then(Value::as_array) {
        Some(entries) => entries,
        None => {
            return codex_failure(
                "registration",
                "marketplace list JSON has no marketplaces array",
            )
        }
    };
    let named = entries
        .iter()
        .filter(|entry| entry.get("name").and_then(Value::as_str) == Some(marketplace))
        .collect::<Vec<_>>();
    let marketplace_added = named.is_empty() || previously_added_marketplace;
    if let Some(existing) = named.first() {
        let root_matches = existing
            .get("root")
            .and_then(Value::as_str)
            .is_some_and(|root| Path::new(root) == source_path);
        let source_matches = existing
            .pointer("/marketplaceSource/sourceType")
            .and_then(Value::as_str)
            == Some("local")
            && existing
                .pointer("/marketplaceSource/source")
                .and_then(Value::as_str)
                .is_some_and(|source| Path::new(source) == source_path);
        if named.len() != 1 || !root_matches || !source_matches {
            return codex_failure("registration", format!("marketplace {marketplace} is already registered with a different or ambiguous source"));
        }
    } else {
        let source = source_path.to_string_lossy().into_owned();
        let added = run_cli(
            runner,
            "codex",
            &["plugin", "marketplace", "add", &source, "--json"],
            project,
        );
        if !added.succeeded() {
            return json!({"activation":command_activation_state(&added),"stage":"registration","reason":command_error(&added)});
        }
        let confirmation = run_cli(
            runner,
            "codex",
            &["plugin", "marketplace", "list", "--json"],
            project,
        );
        if !confirmation.succeeded() {
            return json!({"activation":command_activation_state(&confirmation),"stage":"registration","marketplace_added":true,"reason":command_error(&confirmation)});
        }
        let confirmation_json: Value = match serde_json::from_str(&confirmation.stdout) {
            Ok(value) => value,
            Err(error) => {
                return json!({"activation":"failed","stage":"registration","marketplace_added":true,"reason":format!("invalid marketplace list JSON after registration: {error}")})
            }
        };
        let confirmed = confirmation_json
            .get("marketplaces")
            .and_then(Value::as_array)
            .ok_or_else(|| "marketplace list JSON has no marketplaces array".to_owned());
        let confirmed = match confirmed {
            Ok(entries) => {
                entries
                    .iter()
                    .filter(|entry| {
                        entry.get("name").and_then(Value::as_str) == Some(marketplace)
                            && entry
                                .get("root")
                                .and_then(Value::as_str)
                                .is_some_and(|root| Path::new(root) == source_path)
                            && entry
                                .pointer("/marketplaceSource/sourceType")
                                .and_then(Value::as_str)
                                == Some("local")
                            && entry
                                .pointer("/marketplaceSource/source")
                                .and_then(Value::as_str)
                                .is_some_and(|source| Path::new(source) == source_path)
                    })
                    .count()
                    == 1
            }
            Err(error) => {
                return json!({"activation":"failed","stage":"registration","marketplace_added":true,"reason":error})
            }
        };
        if !confirmed {
            return json!({"activation":"failed","stage":"registration","marketplace_added":true,"reason":format!("Codex did not confirm local marketplace {marketplace} at {}", source_path.display())});
        }
    }
    let selector = configured_key;
    let before = run_cli(
        runner,
        "codex",
        &["plugin", "list", "--marketplace", marketplace, "--json"],
        project,
    );
    if !before.succeeded() {
        return json!({"activation":command_activation_state(&before),"stage":"inventory confirmation","marketplace_added":marketplace_added,"reason":command_error(&before)});
    }
    let before_json: Value = match serde_json::from_str(&before.stdout) {
        Ok(value) => value,
        Err(error) => {
            return json!({"activation":"failed","stage":"inventory confirmation","marketplace_added":marketplace_added,"reason":format!("invalid plugin inventory JSON: {error}")})
        }
    };
    let was_installed = match inventory_has_plugin(&before_json, plugin, marketplace) {
        Ok(installed) => installed,
        Err(error) => {
            return json!({"activation":"failed","stage":"inventory confirmation","marketplace_added":marketplace_added,"reason":error})
        }
    };
    if !was_installed {
        let installed = run_cli(
            runner,
            "codex",
            &["plugin", "add", selector, "--json"],
            project,
        );
        if !installed.succeeded() {
            return json!({"activation":command_activation_state(&installed),"stage":"installation","marketplace_added":marketplace_added,"plugin_added":previously_added_plugin,"reason":command_error(&installed)});
        }
    }
    let inventory = run_cli(
        runner,
        "codex",
        &["plugin", "list", "--marketplace", marketplace, "--json"],
        project,
    );
    if !inventory.succeeded() {
        return json!({"activation":command_activation_state(&inventory),"stage":"inventory confirmation","marketplace_added":marketplace_added,"plugin_added":previously_added_plugin || !was_installed,"reason":command_error(&inventory)});
    }
    let inventory_json: Value = match serde_json::from_str(&inventory.stdout) {
        Ok(value) => value,
        Err(error) => {
            return json!({"activation":"failed","stage":"inventory confirmation","marketplace_added":marketplace_added,"plugin_added":previously_added_plugin || !was_installed,"reason":format!("invalid plugin inventory JSON: {error}")})
        }
    };
    let is_installed = match inventory_has_plugin(&inventory_json, plugin, marketplace) {
        Ok(installed) => installed,
        Err(error) => {
            return json!({"activation":"failed","stage":"inventory confirmation","marketplace_added":marketplace_added,"plugin_added":previously_added_plugin || !was_installed,"reason":error})
        }
    };
    if !is_installed {
        return json!({"activation":"failed","stage":"inventory confirmation","marketplace_added":marketplace_added,"plugin_added":previously_added_plugin || !was_installed,"reason":format!("Codex inventory did not confirm installed plugin {selector}")});
    }
    json!({"activation":"installed","marketplace_added":marketplace_added,"plugin_added":previously_added_plugin || !was_installed})
}

fn inventory_has_plugin(
    inventory: &Value,
    plugin: &str,
    marketplace: &str,
) -> Result<bool, String> {
    let entries = inventory
        .get("installed")
        .and_then(Value::as_array)
        .ok_or_else(|| "plugin inventory JSON has no installed array".to_owned())?;
    for entry in entries {
        if entry.get("name").and_then(Value::as_str).is_none()
            || entry
                .get("marketplaceName")
                .and_then(Value::as_str)
                .is_none()
            || entry.get("installed").and_then(Value::as_bool).is_none()
        {
            return Err("plugin inventory JSON contains an entry with an invalid shape".into());
        }
        if entry.get("name").and_then(Value::as_str) == Some(plugin)
            && entry.get("marketplaceName").and_then(Value::as_str) == Some(marketplace)
            && entry.get("installed").and_then(Value::as_bool) == Some(true)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn codex_failure(stage: &str, reason: impl Into<String>) -> Value {
    json!({"activation":"failed","stage":stage,"reason":reason.into()})
}

#[cfg(test)]
pub(crate) fn preflight_cleanup(project: &Path) -> Result<(), String> {
    let claude = file_path(project, CLAUDE_SETTINGS).map_err(|error| error.to_string())?;
    if claude.exists() {
        let value = crate::codec::read_json(&claude).map_err(|error| error.to_string())?;
        if !value.is_object() {
            return Err("Claude settings must be an object".into());
        }
        for key in ["extraKnownMarketplaces", "enabledPlugins"] {
            if value.get(key).is_some_and(|entry| !entry.is_object()) {
                return Err(format!("Claude settings {key} must be an object"));
            }
        }
    }
    let codex = file_path(project, CODEX_CONFIG).map_err(|error| error.to_string())?;
    if codex.exists() {
        let document = fs::read_to_string(&codex)
            .map_err(|error| error.to_string())?
            .parse::<DocumentMut>()
            .map_err(|error| format!("invalid Codex TOML: {error}"))?;
        if document
            .get("plugins")
            .is_some_and(|entry| entry.as_table().is_none())
        {
            return Err("Codex plugins must be a TOML table".into());
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn current_status(
    project: &Path,
    old: &Value,
) -> Result<(bool, bool, bool, bool, bool), String> {
    let opencode = file_path(project, "opencode.json").map_err(|error| error.to_string())?;
    let claude_path = file_path(project, CLAUDE_SETTINGS).map_err(|error| error.to_string())?;
    let codex_path = file_path(project, CODEX_CONFIG).map_err(|error| error.to_string())?;
    let open_configured = if opencode.is_file() {
        let opencode_value =
            crate::codec::read_json(&opencode).map_err(|error| error.to_string())?;
        let expected_path = fs::canonicalize(project)
            .map_err(|error| error.to_string())?
            .join(format!("{PLUGIN_ROOT}/skills"))
            .to_string_lossy()
            .into_owned();
        let open_skills = opencode_value.get("skills");
        if let Some(array) = open_skills.and_then(Value::as_array) {
            array
                .iter()
                .any(|entry| entry.as_str() == Some(&expected_path))
        } else {
            open_skills
                .and_then(|entry| entry.get("paths"))
                .and_then(Value::as_array)
                .is_some_and(|array| {
                    array
                        .iter()
                        .any(|entry| entry.as_str() == Some(&expected_path))
                })
        }
    } else {
        false
    };
    let claude = if claude_path.is_file() {
        Some(crate::codec::read_json(&claude_path).map_err(|error| error.to_string())?)
    } else {
        None
    };
    let market = old
        .pointer("/native/claude/marketplace_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let plugin_key = old
        .pointer("/native/claude/plugin_key")
        .and_then(Value::as_str)
        .unwrap_or("");
    let canonical_project = fs::canonicalize(project).map_err(|error| error.to_string())?;
    let source_matches = claude.as_ref().is_some_and(|settings| {
        settings
            .pointer(&format!("/extraKnownMarketplaces/{market}"))
            .is_some_and(|source| {
                source
                    == &json!({
                        "source":"directory",
                        "path":canonical_project.to_string_lossy()
                    })
            })
    });
    let claude_enabled = claude
        .as_ref()
        .and_then(|settings| settings.pointer(&format!("/enabledPlugins/{plugin_key}")))
        .and_then(Value::as_bool);
    let codex_key = old
        .pointer("/native/codex/plugin_key")
        .and_then(Value::as_str)
        .unwrap_or("");
    let codex_enabled = if codex_path.is_file() {
        let codex_text = fs::read_to_string(&codex_path).map_err(|error| error.to_string())?;
        let codex_doc = codex_text
            .parse::<DocumentMut>()
            .map_err(|error| format!("invalid Codex TOML: {error}"))?;
        codex_doc
            .get("plugins")
            .and_then(|plugins| plugins.get(codex_key))
            .and_then(|plugin| plugin.get("enabled"))
            .and_then(Item::as_bool)
    } else {
        None
    };
    Ok((
        open_configured,
        source_matches && claude_enabled.is_some(),
        claude_enabled == Some(false),
        codex_enabled.is_some(),
        codex_enabled == Some(false),
    ))
}

pub(crate) fn cleanup_claude(project: &Path, old: &Value) -> Option<String> {
    if old
        .pointer("/native/claude/installed_before")
        .and_then(Value::as_bool)
        != Some(true)
        && old
            .pointer("/native/claude/activation")
            .and_then(Value::as_str)
            != Some("synchronized")
    {
        return None;
    }
    let key = old
        .pointer("/native/claude/plugin_key")
        .and_then(Value::as_str)?;
    let settings_path = file_path(project, CLAUDE_SETTINGS).ok()?;
    if settings_path.is_file()
        && crate::codec::read_json(&settings_path)
            .ok()?
            .pointer(&format!("/enabledPlugins/{key}"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return Some("Claude uninstall skipped to preserve the project's explicit disabled setting; its harness-owned install record may remain".into());
    }
    let outcome = run_cli_value(
        &SystemProcessRunner,
        "claude",
        &["plugin", "uninstall", key, "--scope", "project"],
        project,
    );
    match outcome.get("activation").and_then(Value::as_str) {
        Some("synchronized") => None,
        Some("pending") => Some("Claude CLI is unavailable; project settings were cleaned but its native install record may remain".into()),
        _ => Some(format!("Claude project plugin cleanup failed: {}", outcome.get("reason").and_then(Value::as_str).unwrap_or("unknown error"))),
    }
}

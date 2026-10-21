use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::{Component, Path, PathBuf},
    str::FromStr,
};

use serde_json::{Map, Number, Value};

use crate::{
    codec::read_json,
    error::McpConfigError,
    filesystem::file_path,
    mcp::{
        McpAccessMode, McpName, McpOAuth, McpProtocol, McpServerDefinition, McpTimeout,
        McpTransport, PositiveInteger, TimeoutPhase,
    },
    model::{ProfileName, RepositoryContext},
};

const TIMEOUT_FIELDS: [&str; 3] = ["startup", "catalog", "execution"];
const LOCAL_CONFIG: &str = ".opencode/ai-profile.local.json";
const NATIVE_LAUNCHER: &str = ".opencode/ai-profile-mcp";
const OVERRIDE_FIELDS: [&str; 14] = [
    "type",
    "command",
    "environment",
    "env_file",
    "enabled",
    "disabled",
    "cwd",
    "url",
    "headers",
    "oauth",
    "timeout",
    "codemode",
    "protocol",
    "access_mode",
];

impl McpName {
    pub(crate) fn parse(value: &str) -> Result<Self, McpConfigError> {
        ProfileName::from_str(value).map_err(|_| McpConfigError::InvalidName)?;
        Ok(Self(value.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(crate) fn mcp_definition(
    context: &RepositoryContext,
    name: &str,
) -> Result<McpServerDefinition, McpConfigError> {
    let name = McpName::parse(name)?;
    let relative = format!("mcp/servers/{}.json", name.as_str());
    let path = file_path(&context.sources, &relative)?;
    if !path.is_file() {
        return Err(McpConfigError::MissingDefinition {
            name: name.as_str().to_owned(),
        });
    }
    let entry = read_json(&path)?;
    parse_server(&name, entry, None)
}

pub(crate) fn desired_mcp(
    context: &RepositoryContext,
    profile: &str,
) -> Result<Vec<McpServerDefinition>, McpConfigError> {
    let manifest_path = file_path(&context.sources, &format!("profiles/{profile}/mcp.json"))?;
    let manifest = read_json(&manifest_path)?;
    let Some(object) = manifest.as_object() else {
        return Err(McpConfigError::InvalidManifest {
            profile: profile.to_owned(),
        });
    };
    if object.len() != 1 || !object.get("servers").is_some_and(Value::is_array) {
        return Err(McpConfigError::InvalidManifest {
            profile: profile.to_owned(),
        });
    }
    let Some(server_names) = object.get("servers").and_then(Value::as_array) else {
        return Err(McpConfigError::InvalidManifest {
            profile: profile.to_owned(),
        });
    };
    let mut seen = HashSet::with_capacity(server_names.len());
    let mut definitions = Vec::with_capacity(server_names.len());
    for value in server_names {
        let Some(name) = value.as_str() else {
            return Err(McpConfigError::InvalidName);
        };
        let parsed_name = McpName::parse(name)?;
        if !seen.insert(parsed_name.as_str().to_owned()) {
            return Err(McpConfigError::DuplicateName {
                profile: profile.to_owned(),
                name: name.to_owned(),
            });
        }
        definitions.push(mcp_definition(context, parsed_name.as_str())?);
    }
    Ok(definitions)
}

pub(crate) fn validate_servers(
    servers: &Map<String, Value>,
    project: Option<&Path>,
) -> Result<Vec<McpServerDefinition>, McpConfigError> {
    servers
        .iter()
        .map(|(name, entry)| {
            let name = McpName::parse(name)?;
            parse_server(&name, entry.clone(), project)
        })
        .collect()
}

fn parse_server(
    name: &McpName,
    entry: Value,
    project: Option<&Path>,
) -> Result<McpServerDefinition, McpConfigError> {
    let Some(object) = entry.as_object() else {
        return Err(McpConfigError::InvalidType(name.as_str().to_owned()));
    };
    let transport = match object.get("type").and_then(Value::as_str) {
        Some("local") => McpTransport::Local,
        Some("remote") => McpTransport::Remote,
        _ => return Err(McpConfigError::InvalidType(name.as_str().to_owned())),
    };

    let command = match transport {
        McpTransport::Local => {
            let Some(values) = object.get("command").and_then(Value::as_array) else {
                return Err(McpConfigError::InvalidCommand(name.as_str().to_owned()));
            };
            let command = values.iter().map(Value::as_str).collect::<Option<Vec<_>>>();
            let Some(command) = command
                .filter(|parts| !parts.is_empty() && parts.iter().all(|part| !part.is_empty()))
            else {
                return Err(McpConfigError::InvalidCommand(name.as_str().to_owned()));
            };
            Some(command.into_iter().map(str::to_owned).collect::<Vec<_>>())
        }
        McpTransport::Remote => None,
    };
    let url = if transport == McpTransport::Remote {
        let Some(url) = object
            .get("url")
            .and_then(Value::as_str)
            .filter(|url| !url.is_empty())
        else {
            return Err(McpConfigError::MissingRemoteUrl(name.as_str().to_owned()));
        };
        Some(url.to_owned())
    } else {
        None
    };

    let access_mode = match object.get("access_mode") {
        None => None,
        Some(_) if name.as_str() != "postgres" || transport != McpTransport::Local => {
            return Err(McpConfigError::AccessModeUnsupported);
        }
        Some(value) => Some(parse_access_mode(value).ok_or(McpConfigError::InvalidAccessMode)?),
    };

    if name.as_str() == "postgres" && transport == McpTransport::Local {
        let Some(command) = command.as_deref() else {
            return Err(McpConfigError::InvalidCommand(name.as_str().to_owned()));
        };
        split_access_mode(command)?;
    }

    let environment = string_map(object, "environment", name.as_str())?;
    for key in environment.keys() {
        if !valid_environment_name(key) {
            return Err(McpConfigError::InvalidEnvironmentName(
                name.as_str().to_owned(),
            ));
        }
    }
    let headers = string_map(object, "headers", name.as_str())?;
    let enabled = optional_bool(object, "enabled", name.as_str())?;
    let disabled = optional_bool(object, "disabled", name.as_str())?;
    let codemode = optional_bool(object, "codemode", name.as_str())?;
    let cwd = match object.get("cwd") {
        None => None,
        Some(value) => Some(
            value
                .as_str()
                .filter(|cwd| !cwd.is_empty())
                .ok_or_else(|| McpConfigError::InvalidCwd(name.as_str().to_owned()))?
                .to_owned(),
        ),
    };
    let timeout = object
        .get("timeout")
        .map(|value| parse_timeout(value, name.as_str()))
        .transpose()?;
    let oauth = object
        .get("oauth")
        .map(|value| parse_oauth(value, name.as_str()))
        .transpose()?;
    let protocol = object
        .get("protocol")
        .map(|value| parse_protocol(value, name.as_str()))
        .transpose()?;
    let env_file = match object.get("env_file") {
        None => None,
        Some(value) => {
            if transport != McpTransport::Local {
                return Err(McpConfigError::RemoteEnvFile(name.as_str().to_owned()));
            }
            let relative = value.as_str().ok_or(McpConfigError::InvalidProjectPath)?;
            project_path(project.unwrap_or(Path::new(".")), relative)?;
            Some(relative.to_owned())
        }
    };

    Ok(McpServerDefinition {
        name: name.clone(),
        transport,
        command,
        url,
        environment,
        headers,
        enabled,
        disabled,
        codemode,
        cwd,
        timeout,
        oauth,
        protocol,
        access_mode,
        env_file,
        raw: entry,
    })
}

pub(crate) fn project_path(project: &Path, relative: &str) -> Result<PathBuf, McpConfigError> {
    let relative_path = Path::new(relative);
    if relative.is_empty()
        || relative_path.is_absolute()
        || relative_path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(McpConfigError::InvalidProjectPath);
    }
    let mut path = project.to_owned();
    for component in relative_path.components() {
        if let Component::Normal(part) = component {
            path.push(part);
            if fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
                return Err(McpConfigError::ProjectSymlink);
            }
        }
    }
    Ok(project.join(relative_path))
}

pub(crate) fn definitions_to_map(definitions: &[McpServerDefinition]) -> Map<String, Value> {
    definitions
        .iter()
        .map(|definition| (definition.name.as_str().to_owned(), definition.raw.clone()))
        .collect()
}

pub(crate) fn load_local(
    project: &Path,
    servers: &Map<String, Value>,
) -> Result<Map<String, Value>, McpConfigError> {
    let path = project_path(project, LOCAL_CONFIG)?;
    if !path.exists() {
        return Ok(Map::new());
    }
    let value = read_json(&path)?;
    let Some(object) = value.as_object() else {
        return Err(McpConfigError::InvalidLocalManifest);
    };
    if object.keys().any(|key| key != "mcp") {
        return Err(McpConfigError::InvalidLocalManifest);
    }
    let overrides = match object.get("mcp") {
        None => Map::new(),
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or(McpConfigError::InvalidLocalManifest)?,
    };
    for (name, entry) in &overrides {
        let Some(base) = servers.get(name) else {
            return Err(McpConfigError::UnmanagedOverride(name.clone()));
        };
        let Some(fields) = entry.as_object() else {
            return Err(McpConfigError::InvalidOverrideFields(name.clone()));
        };
        if fields
            .keys()
            .any(|field| !OVERRIDE_FIELDS.contains(&field.as_str()))
        {
            return Err(McpConfigError::InvalidOverrideFields(name.clone()));
        }
        if let Some(transport) = fields.get("type") {
            if Some(transport) != base.get("type") {
                return Err(McpConfigError::OverrideTransportChange(name.clone()));
            }
        }
    }
    Ok(overrides)
}

/// Recursively merge objects; a non-object override (including an array) replaces the base.
pub(crate) fn merge(base: &Value, override_value: &Value) -> Value {
    let (Some(base_object), Some(override_object)) = (base.as_object(), override_value.as_object())
    else {
        return override_value.clone();
    };
    let mut result = base_object.clone();
    for (key, value) in override_object {
        let merged = match result.get(key) {
            Some(previous) if previous.is_object() && value.is_object() => merge(previous, value),
            _ => value.clone(),
        };
        result.insert(key.clone(), merged);
    }
    Value::Object(result)
}

pub(crate) fn split_access_mode(
    command: &[String],
) -> Result<(Vec<String>, Option<McpAccessMode>), McpConfigError> {
    let mut remaining = Vec::with_capacity(command.len());
    let mut modes = Vec::new();
    let mut index = 0;
    while index < command.len() {
        let argument = &command[index];
        if argument == "--access-mode" {
            index += 1;
            let Some(value) = command.get(index) else {
                return Err(McpConfigError::MissingCommandAccessMode);
            };
            modes.push(value.as_str());
        } else if let Some(value) = argument.strip_prefix("--access-mode=") {
            modes.push(value);
        } else {
            remaining.push(argument.clone());
        }
        index += 1;
    }
    if modes.len() > 1 {
        return Err(McpConfigError::DuplicateCommandAccessMode);
    }
    let mode = modes
        .first()
        .map(|mode| match *mode {
            "restricted" => Ok(McpAccessMode::Restricted),
            "unrestricted" => Ok(McpAccessMode::Unrestricted),
            _ => Err(McpConfigError::InvalidCommandAccessMode),
        })
        .transpose()?;
    Ok((remaining, mode))
}

pub(crate) fn apply_access_mode(entry: &mut Value) -> Result<(), McpConfigError> {
    let object = entry
        .as_object_mut()
        .ok_or_else(|| McpConfigError::InvalidMcpObject("postgres".to_owned()))?;
    let command = object
        .get("command")
        .and_then(Value::as_array)
        .ok_or_else(|| McpConfigError::InvalidCommand("postgres".to_owned()))?
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| McpConfigError::InvalidCommand("postgres".to_owned()))?
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    let (mut command, _) = split_access_mode(&command)?;
    let access_mode = match object.get("access_mode") {
        None => McpAccessMode::Unrestricted,
        Some(value) => parse_access_mode(value).ok_or(McpConfigError::InvalidAccessMode)?,
    };
    command.push(match access_mode {
        McpAccessMode::Restricted => "--access-mode=restricted".to_owned(),
        McpAccessMode::Unrestricted => "--access-mode=unrestricted".to_owned(),
    });
    object.insert(
        "command".to_owned(),
        Value::Array(command.into_iter().map(Value::String).collect()),
    );
    Ok(())
}

pub(crate) struct McpConfiguration {
    pub(crate) servers: Map<String, Value>,
    pub(crate) overrides: Map<String, Value>,
}

pub(crate) fn resolve(
    project: &Path,
    base: &Map<String, Value>,
) -> Result<McpConfiguration, McpConfigError> {
    let overrides = load_local(project, base)?;
    let mut effective = Map::new();
    for (name, entry) in base {
        let merged = match overrides.get(name) {
            Some(override_entry) => merge(entry, override_entry),
            None => entry.clone(),
        };
        effective.insert(name.clone(), merged);
    }
    validate_servers(&effective, Some(project))?;
    if effective
        .get("postgres")
        .and_then(Value::as_object)
        .and_then(|entry| entry.get("type"))
        .and_then(Value::as_str)
        == Some("local")
    {
        let postgres = effective
            .get_mut("postgres")
            .ok_or_else(|| McpConfigError::InvalidMcpObject("postgres".to_owned()))?;
        apply_access_mode(postgres)?;
    }
    Ok(McpConfiguration {
        servers: effective,
        overrides,
    })
}

pub(crate) fn render(effective: &Map<String, Value>) -> Result<Map<String, Value>, McpConfigError> {
    let mut rendered = effective.clone();
    for (name, entry) in &mut rendered {
        let Some(object) = entry.as_object_mut() else {
            return Err(McpConfigError::InvalidMcpObject(name.clone()));
        };
        if object.get("type").and_then(Value::as_str) == Some("local") {
            for field in ["env_file", "environment", "cwd", "access_mode"] {
                object.remove(field);
            }
            object.insert(
                "command".to_owned(),
                Value::Array(vec![
                    Value::String(format!("./{NATIVE_LAUNCHER}")),
                    Value::String(name.clone()),
                ]),
            );
        }
    }
    Ok(rendered)
}

pub(crate) fn configured_mcp(
    project: &Path,
    base: &Map<String, Value>,
) -> Result<McpConfiguration, McpConfigError> {
    let McpConfiguration {
        servers: effective,
        overrides,
    } = resolve(project, base)?;
    let mut rendered = render(&effective)?;
    for (name, entry) in &mut rendered {
        if effective
            .get(name)
            .and_then(Value::as_object)
            .and_then(|value| value.get("type"))
            .and_then(Value::as_str)
            == Some("local")
        {
            let object = entry
                .as_object_mut()
                .ok_or_else(|| McpConfigError::InvalidMcpObject(name.clone()))?;
            object.insert(
                "command".to_owned(),
                Value::Array(vec![
                    Value::String(project.join(NATIVE_LAUNCHER).to_string_lossy().into_owned()),
                    Value::String(name.clone()),
                ]),
            );
        }
    }
    Ok(McpConfiguration {
        servers: rendered,
        overrides,
    })
}

pub(crate) fn changed_override_paths(
    old: &Map<String, Value>,
    new: &Map<String, Value>,
    override_value: &Map<String, Value>,
) -> Vec<String> {
    let mut paths = Vec::new();
    collect_changed_paths(old, new, override_value, "", &mut paths);
    paths
}

fn collect_changed_paths(
    old: &Map<String, Value>,
    new: &Map<String, Value>,
    override_value: &Map<String, Value>,
    prefix: &str,
    paths: &mut Vec<String>,
) {
    for (key, override_entry) in override_value {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        let before = old.get(key).filter(|value| !value.is_null());
        let after = new.get(key).filter(|value| !value.is_null());
        if let (Some(override_object), Some(before_object), Some(after_object)) = (
            override_entry.as_object(),
            before.and_then(Value::as_object),
            after.and_then(Value::as_object),
        ) {
            collect_changed_paths(before_object, after_object, override_object, &path, paths);
        } else if before != after {
            paths.push(path);
        }
    }
}

pub(crate) fn check_mcp(
    config: &Map<String, Value>,
    old_state: Option<&Value>,
    new: &Map<String, Value>,
) -> Result<(), McpConfigError> {
    let current = match config.get("mcp") {
        None => Map::new(),
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or_else(|| McpConfigError::InvalidMcpObject("project configuration".to_owned()))?,
    };
    let prior = match old_state.and_then(|state| state.get("mcp")) {
        None => Map::new(),
        Some(value) => value
            .as_object()
            .cloned()
            .ok_or_else(|| McpConfigError::InvalidMcpObject("installation state".to_owned()))?,
    };
    for (name, expected) in &prior {
        if current.get(name) != Some(expected) {
            return Err(McpConfigError::ManagedMcpChanged(name.clone()));
        }
    }
    for name in new.keys() {
        if current.contains_key(name) && !prior.contains_key(name) {
            return Err(McpConfigError::UnmanagedMcpExists(name.clone()));
        }
    }
    Ok(())
}

fn string_map(
    object: &Map<String, Value>,
    field: &'static str,
    name: &str,
) -> Result<BTreeMap<String, String>, McpConfigError> {
    let Some(value) = object.get(field) else {
        return Ok(BTreeMap::new());
    };
    let Some(map) = value.as_object() else {
        return Err(McpConfigError::InvalidStringMap {
            field,
            name: name.to_owned(),
        });
    };
    map.iter()
        .map(|(key, value)| {
            value
                .as_str()
                .map(|value| (key.clone(), value.to_owned()))
                .ok_or_else(|| McpConfigError::InvalidStringMap {
                    field,
                    name: name.to_owned(),
                })
        })
        .collect()
}

fn valid_environment_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn optional_bool(
    object: &Map<String, Value>,
    field: &'static str,
    name: &str,
) -> Result<Option<bool>, McpConfigError> {
    object
        .get(field)
        .map(|value| {
            value
                .as_bool()
                .ok_or_else(|| McpConfigError::InvalidBoolean {
                    field,
                    name: name.to_owned(),
                })
        })
        .transpose()
}

fn parse_timeout(value: &Value, name: &str) -> Result<McpTimeout, McpConfigError> {
    if let Some(number) = value.as_number() {
        return parse_positive_integer(number)
            .map(McpTimeout::Default)
            .ok_or_else(|| McpConfigError::InvalidTimeout(name.to_owned()));
    }
    let Some(values) = value.as_object() else {
        return Err(McpConfigError::InvalidTimeout(name.to_owned()));
    };
    if values
        .keys()
        .any(|key| !TIMEOUT_FIELDS.contains(&key.as_str()))
    {
        return Err(McpConfigError::InvalidTimeout(name.to_owned()));
    }
    let mut phases = BTreeMap::new();
    for (key, value) in values {
        let phase = match key.as_str() {
            "startup" => TimeoutPhase::Startup,
            "catalog" => TimeoutPhase::Catalog,
            "execution" => TimeoutPhase::Execution,
            _ => return Err(McpConfigError::InvalidTimeout(name.to_owned())),
        };
        let Some(number) = value.as_number().and_then(parse_positive_integer) else {
            return Err(McpConfigError::InvalidTimeout(name.to_owned()));
        };
        phases.insert(phase, number);
    }
    Ok(McpTimeout::Phases(phases))
}

fn parse_positive_integer(number: &Number) -> Option<PositiveInteger> {
    let value = number.to_string();
    if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) && value != "0" {
        Some(PositiveInteger(value))
    } else {
        None
    }
}

fn parse_oauth(value: &Value, name: &str) -> Result<McpOAuth, McpConfigError> {
    if let Some(enabled) = value.as_bool() {
        return Ok(McpOAuth::Enabled(enabled));
    }
    if let Some(options) = value.as_object() {
        return Ok(McpOAuth::Options(options.clone()));
    }
    Err(McpConfigError::InvalidOAuth(name.to_owned()))
}

fn parse_protocol(value: &Value, name: &str) -> Result<McpProtocol, McpConfigError> {
    match value.as_str() {
        Some("legacy") => Ok(McpProtocol::Legacy),
        Some("auto") => Ok(McpProtocol::Auto),
        Some("2026-07-28") => Ok(McpProtocol::Version20260728),
        _ => Err(McpConfigError::InvalidProtocol(name.to_owned())),
    }
}

fn parse_access_mode(value: &Value) -> Option<McpAccessMode> {
    match value.as_str() {
        Some("restricted") => Some(McpAccessMode::Restricted),
        Some("unrestricted") => Some(McpAccessMode::Unrestricted),
        _ => None,
    }
}

use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    process::Command,
};

use serde_json::Value;
use std::os::unix::process::CommandExt;

use crate::{
    error::McpRuntimeError,
    mcp::{config, environment::environment_for},
};

const STATE: &str = crate::layout::LEGACY_STATE;

pub fn main() -> i32 {
    let mut args = env::args_os();
    let _program = args.next();
    let Some(server) = args.next() else {
        eprintln!("Usage: ai-profile-mcp SERVER");
        return 2;
    };
    if args.next().is_some() {
        eprintln!("Usage: ai-profile-mcp SERVER");
        return 2;
    }
    let server = match server.into_string() {
        Ok(server) => server,
        Err(_) => {
            eprintln!("MCP startup failed: Unknown local MCP server");
            return 1;
        }
    };

    match launch(&server) {
        Ok(()) => 0,
        Err(McpRuntimeError::NovamiraRejected) => 1,
        Err(error @ McpRuntimeError::Exec) | Err(error @ McpRuntimeError::ProjectLocation) => {
            eprintln!("{error}");
            1
        }
        Err(error) => {
            eprintln!("MCP startup failed: {error}");
            1
        }
    }
}

fn launch(server_name: &str) -> Result<(), McpRuntimeError> {
    let executable = env::current_exe().map_err(|_| McpRuntimeError::ProjectLocation)?;
    let (project, client) = identity_from_adapter(&executable)?;
    let layout = client.map_or(
        crate::layout::Layout::Legacy,
        crate::layout::Layout::Isolated,
    );
    let state_path = config::project_path(
        &project,
        if client.is_some() {
            crate::layout::STATE
        } else {
            STATE
        },
    )?;
    let state = crate::codec::read_json(&state_path).map_err(|_| McpRuntimeError::StateRead)?;
    let state = if let Some(client) = client {
        if state.get("format").and_then(Value::as_u64) != Some(2) {
            return Err(McpRuntimeError::InvalidState);
        }
        state
            .get("clients")
            .and_then(|clients| clients.get(client.as_str()))
            .cloned()
            .ok_or(McpRuntimeError::InvalidState)?
    } else {
        state
    };
    let Some(state_object) = state.as_object() else {
        return Err(McpRuntimeError::InvalidState);
    };
    if client.is_none() && !state_object.get("format").is_some_and(is_format_one) {
        return Err(McpRuntimeError::InvalidState);
    }
    let Some(base) = state_object
        .get("mcp_base")
        .and_then(Value::as_object)
        .filter(|base| !base.is_empty())
    else {
        return Err(McpRuntimeError::InvalidState);
    };

    let servers = config::resolve_at(&project, base, &layout.local())?.servers;
    let Some(entry) = servers.get(server_name).and_then(Value::as_object) else {
        return Err(McpRuntimeError::UnknownLocalServer);
    };
    if entry.get("type").and_then(Value::as_str) != Some("local") {
        return Err(McpRuntimeError::UnknownLocalServer);
    }
    if entry.get("disabled").and_then(Value::as_bool) == Some(true)
        || entry.get("enabled").and_then(Value::as_bool) == Some(false)
    {
        return Err(McpRuntimeError::Disabled);
    }

    let inherited = inherited_environment();
    let env_file = entry
        .get("env_file")
        .and_then(Value::as_str)
        .unwrap_or("środowisku procesu (bez env_file)");
    let environment = match environment_for(&project, entry, &inherited) {
        Ok(environment) => environment,
        Err(_) if server_name == "novamira" => {
            super::novamira::configuration_failure(env_file);
            return Err(McpRuntimeError::NovamiraRejected);
        }
        Err(error) => return Err(error.into()),
    };
    if server_name == "novamira"
        && ["WP_API_URL", "WP_API_USERNAME", "WP_API_PASSWORD"]
            .iter()
            .any(|key| {
                environment
                    .get(*key)
                    .is_none_or(|value| value.trim().is_empty())
            })
    {
        super::novamira::configuration_failure(env_file);
        return Err(McpRuntimeError::NovamiraRejected);
    }
    let command = entry
        .get("command")
        .and_then(Value::as_array)
        .ok_or(McpRuntimeError::UnknownLocalServer)?;
    let Some(program) = command
        .first()
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    else {
        return Err(McpRuntimeError::UnknownLocalServer);
    };
    let args = command
        .iter()
        .skip(1)
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        .ok_or(McpRuntimeError::UnknownLocalServer)?;
    let cwd = entry
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let cwd = if cwd.is_absolute() {
        cwd
    } else {
        project.join(cwd)
    };

    // Managed launchers remain rooted in the project even with a custom cwd.
    let program = if program.starts_with(".opencode/mcp/") || program.starts_with(".ai-profiles/") {
        config::project_path(&project, program)?
    } else {
        PathBuf::from(program)
    };
    let mut process = Command::new(program);
    process.args(args).current_dir(cwd);
    apply_environment(&mut process, &project, environment);
    if server_name == "novamira" {
        return if super::novamira::run(&mut process, env_file) {
            Ok(())
        } else {
            Err(McpRuntimeError::NovamiraRejected)
        };
    }
    let _exec_error = process.exec();
    Err(McpRuntimeError::Exec)
}

pub(crate) fn apply_environment(
    process: &mut Command,
    project: &Path,
    environment: HashMap<String, String>,
) {
    // Reserved adapter metadata wins over inherited, dotenv and mapped values.
    // Keep the native path (including non-UTF-8 bytes) scoped to the child.
    process
        .envs(environment)
        .env("AI_PROFILE_PROJECT_DIR", project);
}

#[cfg(test)]
pub(crate) fn project_from_adapter(executable: &Path) -> Result<PathBuf, McpRuntimeError> {
    identity_from_adapter(executable).map(|(project, _)| project)
}

fn identity_from_adapter(
    executable: &Path,
) -> Result<(PathBuf, Option<crate::model::Client>), McpRuntimeError> {
    let executable = executable
        .canonicalize()
        .map_err(|_| McpRuntimeError::ProjectLocation)?;
    let parent = executable
        .parent()
        .ok_or(McpRuntimeError::ProjectLocation)?;
    if let Some(client) = parent
        .file_name()
        .and_then(|s| s.to_str())
        .and_then(crate::model::Client::parse)
    {
        let root = parent
            .parent()
            .filter(|p| p.file_name().is_some_and(|s| s == ".ai-profiles"))
            .ok_or(McpRuntimeError::ProjectLocation)?;
        let project = root.parent().ok_or(McpRuntimeError::ProjectLocation)?;
        return Ok((project.to_owned(), Some(client)));
    }
    let opencode = parent
        .file_name()
        .filter(|name| *name == ".opencode")
        .map(|_| parent)
        .ok_or(McpRuntimeError::ProjectLocation)?;
    let project = opencode.parent().ok_or(McpRuntimeError::ProjectLocation)?;
    if !project.is_dir() {
        return Err(McpRuntimeError::ProjectLocation);
    }
    Ok((project.to_owned(), None))
}

fn inherited_environment() -> HashMap<String, String> {
    env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}

fn is_format_one(value: &Value) -> bool {
    value.as_bool() == Some(true)
        || value.as_i64() == Some(1)
        || value.as_u64() == Some(1)
        || value.as_f64() == Some(1.0)
}

use std::{collections::HashMap, env, path::PathBuf, process::Command};

use serde_json::Value;
use std::os::unix::process::CommandExt;

use crate::{
    error::McpRuntimeError,
    mcp::{config, environment::environment_for},
};

const STATE: &str = ".opencode/ai-profile-state.json";

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
    let project = project_from_executable()?;
    let state_path = config::project_path(&project, STATE)?;
    let state = crate::codec::read_json(&state_path).map_err(|_| McpRuntimeError::StateRead)?;
    let Some(state_object) = state.as_object() else {
        return Err(McpRuntimeError::InvalidState);
    };
    if !state_object.get("format").is_some_and(is_format_one) {
        return Err(McpRuntimeError::InvalidState);
    }
    let Some(base) = state_object
        .get("mcp_base")
        .and_then(Value::as_object)
        .filter(|base| !base.is_empty())
    else {
        return Err(McpRuntimeError::InvalidState);
    };

    let servers = config::resolve(&project, base)?.servers;
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
    let environment = environment_for(&project, entry, &inherited)?;
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
    let program = if program.starts_with(".opencode/mcp/") {
        config::project_path(&project, program)?
    } else {
        PathBuf::from(program)
    };
    let mut process = Command::new(program);
    process.args(args).current_dir(cwd).envs(environment);
    let _exec_error = process.exec();
    Err(McpRuntimeError::Exec)
}

fn project_from_executable() -> Result<PathBuf, McpRuntimeError> {
    let executable = env::current_exe().map_err(|_| McpRuntimeError::ProjectLocation)?;
    let executable = executable
        .canonicalize()
        .map_err(|_| McpRuntimeError::ProjectLocation)?;
    let opencode = executable
        .parent()
        .filter(|parent| parent.file_name().is_some_and(|name| name == ".opencode"))
        .ok_or(McpRuntimeError::ProjectLocation)?;
    let project = opencode.parent().ok_or(McpRuntimeError::ProjectLocation)?;
    if !project.is_dir() {
        return Err(McpRuntimeError::ProjectLocation);
    }
    Ok(project.to_owned())
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

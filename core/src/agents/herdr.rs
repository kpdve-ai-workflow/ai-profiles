use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::process::{
    CancellationToken, CommandSpec, ProcessRunner, StdinMode, StreamMode, SystemProcessRunner,
};

use super::{io_error, session::Session, AgentError, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Context {
    pub environment: HashMap<String, String>,
    pub caller_pane: String,
}

pub(super) trait Control {
    fn call(&self, args: &[String], timeout: Duration) -> Result<Value>;
    fn mcp_names(&self) -> Result<Vec<String>>;
}

pub(super) struct Herdr {
    context: Context,
    repository: PathBuf,
}

impl Herdr {
    pub fn current(repository: &Path) -> Result<Self> {
        if env::var("HERDR_ENV").as_deref() != Ok("1") {
            return Err(AgentError::Input(
                "Uruchom make agent wewnątrz Herdr; launcher nie tworzy zagnieżdżonej sesji",
            ));
        }
        let caller_pane =
            env::var("HERDR_PANE_ID").map_err(|_| AgentError::Input("Brak HERDR_PANE_ID"))?;
        let environment = env::vars()
            .filter(|(name, _)| name.starts_with("HERDR_"))
            .collect();
        let herdr = Self {
            context: Context {
                environment,
                caller_pane,
            },
            repository: repository.to_owned(),
        };
        herdr.call(&strings(&["agent", "list"]), Duration::from_secs(10))?;
        Ok(herdr)
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn for_session(session: &Session) -> Self {
        Self {
            context: session.data.context.clone(),
            repository: session.data.repository.clone(),
        }
    }
}

impl Control for Herdr {
    fn mcp_names(&self) -> Result<Vec<String>> {
        configured_mcp(&self.repository)
    }

    fn call(&self, args: &[String], timeout: Duration) -> Result<Value> {
        let output = run(
            "herdr",
            args,
            &self.repository,
            &self.context.environment,
            timeout,
        )?;
        let value: Value = serde_json::from_slice(&output.stdout)?;
        value
            .get("result")
            .cloned()
            .ok_or(AgentError::Input("Brak result w odpowiedzi Herdr"))
    }
}

pub(super) fn run(
    program: &str,
    args: &[String],
    repository: &Path,
    environment: &HashMap<String, String>,
    timeout: Duration,
) -> Result<crate::process::CommandOutput> {
    let output = SystemProcessRunner
        .run(CommandSpec {
            program: program.into(),
            args: args.iter().map(Into::into).collect(),
            cwd: Some(repository.to_owned()),
            env: environment
                .iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
            clear_env: false,
            stdin: StdinMode::Null,
            stdout: StreamMode::CaptureLimited(1024 * 1024),
            stderr: StreamMode::CaptureLimited(8192),
            timeout: Some(timeout),
            cancellation: CancellationToken::new(),
        })
        .map_err(|error| AgentError::Herdr(format!("{program}: {error}")))?;
    if !output.status.success() {
        // Never forward terminal contents, questions or raw stderr into Brain's context.
        let code = serde_json::from_slice::<Value>(&output.stderr)
            .ok()
            .and_then(|v| {
                v.pointer("/error/code")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
            });
        return Err(AgentError::Herdr(format!(
            "{program} {}: exit {:?}, {}",
            args.first().map(String::as_str).unwrap_or(""),
            output.status.code(),
            code.as_deref().unwrap_or("sprawdź panel/log programu")
        )));
    }
    if output.stdout.len() > 1024 * 1024 {
        return Err(AgentError::Input("Zbyt duża odpowiedź programu"));
    }
    Ok(output)
}

pub(super) fn configured_mcp(repository: &Path) -> Result<Vec<String>> {
    let output = run(
        "codex",
        &strings(&["mcp", "list", "--json"]),
        repository,
        &HashMap::new(),
        Duration::from_secs(30),
    )?;
    let values: Vec<Value> = serde_json::from_slice(&output.stdout)?;
    let names = values
        .iter()
        .map(|v| {
            v.get("name")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or(AgentError::Input("Nieprawidłowy inwentarz MCP Codexa"))
        })
        .collect::<Result<Vec<_>>>()?;
    if names.iter().any(|name| name == "ai_profiles_session") {
        return Err(AgentError::Input("Nazwa MCP ai_profiles_session jest zajęta; istniejąca konfiguracja nie została zmieniona"));
    }
    Ok(names)
}

pub(super) fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| (*v).to_owned()).collect()
}

pub(super) fn field(value: &Value, pointer: &str) -> Result<String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(AgentError::Input(
            "Nieprawidłowe metadane panelu/agenta Herdr",
        ))
}

pub(super) fn executable() -> Result<PathBuf> {
    env::current_exe().map_err(io_error("Ścieżka launchera"))
}

//! Explicit, checkout-only Codex processes coordinated through the current Herdr session.
mod config;
mod herdr;
mod mcp;
mod session;

use std::{env, path::PathBuf};

use serde_json::json;
use thiserror::Error;

use config::{Config, Role};
use herdr::Herdr;
use session::Session;

#[derive(Debug, Error)]
enum AgentError {
    #[error("{0}")]
    Input(&'static str),
    #[error("{operation}: {source}")]
    Io {
        operation: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("Nieprawidłowe dane sesji/protokołu: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Herdr: {0}")]
    Herdr(String),
    #[error("Sesja jest zajęta. Nie ponawiaj wysłania; użyj research_status.")]
    Busy,
    #[error("Poprzednie zadanie {0} nie ma raportu. Użyj research_status; reset tylko po świadomym anulowaniu.")]
    Pending(String),
    #[error("Zadanie {task_id}: {reason}. Nie wysłano go ponownie; użyj research_status.")]
    Delivery { task_id: String, reason: String },
}

type Result<T> = std::result::Result<T, AgentError>;

fn io_error(operation: &'static str) -> impl FnOnce(std::io::Error) -> AgentError {
    move |source| AgentError::Io { operation, source }
}

pub fn main() -> u8 {
    let result = run(env::args().skip(1).collect());
    match result {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn run(args: Vec<String>) -> Result<()> {
    match args.as_slice() {
        [command] if command == "start" => {
            let config = Config::from_env()?;
            let repository = crate::model::RepositoryContext::discover()
                .map_err(|_| AgentError::Input("Uruchom launcher z checkoutu ai-profiles"))?
                .sources;
            let herdr = Herdr::current(&repository)?;
            let mut session = Session::create(repository, config, herdr.context().clone())?;
            eprintln!("SESSION={}", session.directory.display());
            session.start(&herdr)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&session.status(&herdr)?)?
            );
            Ok(())
        }
        [command] if command == "status" || command == "stop" => {
            let directory = env::var_os("AI_AGENT_SESSION")
                .filter(|v| !v.is_empty())
                .ok_or(AgentError::Input("Podaj AI_AGENT_SESSION / SESSION"))?;
            let mut session = Session::load(PathBuf::from(directory))?;
            let herdr = Herdr::for_session(&session);
            if command == "stop" {
                session.stop(&herdr)?;
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&session.status(&herdr)?)?
            );
            Ok(())
        }
        [command, directory, role] if command == "serve" => {
            let session = Session::load(PathBuf::from(directory))?;
            mcp::serve(session, Role::parse(role)?)
        }
        [] => {
            println!(
                "{}",
                json!({"usage":"make agent CLIENT=codex; make agent help"})
            );
            Ok(())
        }
        _ => Err(AgentError::Input("Użyj make agent help")),
    }
}

#[cfg(test)]
#[path = "../../tests/agents.rs"]
mod tests;

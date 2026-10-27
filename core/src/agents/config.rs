use std::{env, path::Path};

use serde::{Deserialize, Serialize};

use super::{AgentError, Result};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(super) enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl Effort {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "low" => Ok(Self::Low),
            "medium" => Ok(Self::Medium),
            "high" => Ok(Self::High),
            "xhigh" => Ok(Self::Xhigh),
            "max" => Ok(Self::Max),
            _ => Err(AgentError::Input(
                "Thinking: low|medium|high|xhigh|max (bez natywnych subagentów)",
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Xhigh => "xhigh",
            Self::Max => "max",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Model(String);

impl Model {
    fn parse(value: String) -> Result<Self> {
        if value.is_empty()
            || value.len() > 160
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._/".contains(&b))
        {
            return Err(AgentError::Input("Nieprawidłowy identyfikator modelu"));
        }
        Ok(Self(value))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ModelConfig {
    pub model: Model,
    pub thinking: Effort,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct Config {
    pub brain: ModelConfig,
    pub researcher: ModelConfig,
    pub timeout_seconds: u64,
    pub max_tasks: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        if env::var("AI_AGENT_CLIENT").as_deref() != Ok("codex") {
            return Err(AgentError::Input("Ten tryb wymaga CLIENT=codex"));
        }
        Self::parse(|name, default| env::var(name).unwrap_or_else(|_| default.to_owned()))
    }

    pub fn parse(get: impl Fn(&str, &str) -> String) -> Result<Self> {
        let timeout_seconds = get("AI_AGENT_RESEARCH_TIMEOUT", "180")
            .parse::<u64>()
            .ok()
            .filter(|v| (5..=1800).contains(v))
            .ok_or(AgentError::Input("RESEARCH_TIMEOUT: 5..1800 sekund"))?;
        let max_tasks = get("AI_AGENT_RESEARCHER_MAX_TASKS", "8")
            .parse::<u32>()
            .ok()
            .filter(|v| (1..=100).contains(v))
            .ok_or(AgentError::Input("RESEARCHER_MAX_TASKS: 1..100"))?;
        Ok(Self {
            brain: ModelConfig {
                model: Model::parse(get("AI_AGENT_BRAIN_MODEL", "gpt-6.1-sol"))?,
                thinking: Effort::parse(&get("AI_AGENT_BRAIN_THINKING", "xhigh"))?,
            },
            researcher: ModelConfig {
                model: Model::parse(get("AI_AGENT_RESEARCHER_MODEL", "gpt-6-luna"))?,
                thinking: Effort::parse(&get("AI_AGENT_RESEARCHER_THINKING", "medium"))?,
            },
            timeout_seconds,
            max_tasks,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    Brain,
    Researcher,
}

impl Role {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "brain" => Ok(Self::Brain),
            "researcher" => Ok(Self::Researcher),
            _ => Err(AgentError::Input("Rola: brain|researcher")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Brain => "brain",
            Self::Researcher => "researcher",
        }
    }
}

pub(super) fn codex_args(
    config: &Config,
    role: Role,
    repository: &Path,
    session: &Path,
    executable: &Path,
    disabled_mcp: &[String],
) -> Vec<String> {
    let model = match role {
        Role::Brain => &config.brain,
        Role::Researcher => &config.researcher,
    };
    let mut args = vec![
        "--no-daemon".into(),
        "--cd".into(),
        repository.to_string_lossy().into_owned(),
        "--model".into(),
        model.model.0.clone(),
        "--sandbox".into(),
        match role {
            Role::Brain => "workspace-write",
            Role::Researcher => "read-only",
        }
        .into(),
        "--ask-for-approval".into(),
        match role {
            Role::Brain => "on-request",
            Role::Researcher => "never",
        }
        .into(),
    ];
    let mut set = |value: String| {
        args.push("--config".into());
        args.push(value);
    };
    set(format!(
        "model_reasoning_effort={}",
        toml_string(model.thinking.as_str())
    ));
    set("features.multi_agent=false".into());
    set("features.multi_agent_v2=false".into());
    // Source discovery belongs exclusively to Researcher, including verification.
    // Explicitly enable its shell so a user's disabled default cannot prevent reads.
    set(format!("features.shell_tool={}", role == Role::Researcher));
    let instructions = match role {
        Role::Brain => include_str!("../../agents/brain.md"),
        Role::Researcher => include_str!("../../agents/researcher.md"),
    };
    set(format!(
        "developer_instructions={}",
        toml_string(instructions)
    ));
    if role == Role::Researcher {
        for feature in [
            "apps",
            "plugins",
            "hooks",
            "skill_mcp_dependency_install",
            "browser_use",
            "browser_use_external",
            "computer_use",
            "image_generation",
            "memories",
        ] {
            set(format!("features.{feature}=false"));
        }
        set("web_search=\"disabled\"".into());
        for name in disabled_mcp {
            set(format!("mcp_servers.{}.enabled=false", toml_string(name)));
        }
    }
    // Startup rejects a collision with this reserved session-only server name.
    set(format!(
        "mcp_servers.ai_profiles_session.command={}",
        toml_string(&executable.to_string_lossy())
    ));
    let mcp_args = ["serve", &session.to_string_lossy(), role.as_str()];
    set(format!(
        "mcp_servers.ai_profiles_session.args={}",
        serde_json::to_string(&mcp_args).expect("string array")
    ));
    set("mcp_servers.ai_profiles_session.required=true".into());
    set(format!(
        "mcp_servers.ai_profiles_session.tool_timeout_sec={}",
        config.timeout_seconds + 150
    ));
    args
}

// JSON basic strings are valid TOML strings for these textual arguments.
fn toml_string(value: &str) -> String {
    serde_json::to_string(value).expect("string")
}

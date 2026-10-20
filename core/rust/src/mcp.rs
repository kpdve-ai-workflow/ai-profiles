pub(crate) mod config;
pub(crate) mod environment;
pub mod runtime;

use std::collections::BTreeMap;

use serde_json::{Map, Value};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct McpName(String);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum McpTransport {
    Local,
    Remote,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum McpAccessMode {
    Restricted,
    Unrestricted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum McpProtocol {
    Legacy,
    Auto,
    Version20260728,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum TimeoutPhase {
    Startup,
    Catalog,
    Execution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PositiveInteger(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum McpTimeout {
    Default(PositiveInteger),
    Phases(BTreeMap<TimeoutPhase, PositiveInteger>),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum McpOAuth {
    Enabled(bool),
    Options(Map<String, Value>),
}

/// Typed view of a registry entry; `raw` retains unrelated fields unchanged.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct McpServerDefinition {
    pub(crate) name: McpName,
    pub(crate) transport: McpTransport,
    pub(crate) command: Option<Vec<String>>,
    pub(crate) url: Option<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) enabled: Option<bool>,
    pub(crate) disabled: Option<bool>,
    pub(crate) codemode: Option<bool>,
    pub(crate) cwd: Option<String>,
    pub(crate) timeout: Option<McpTimeout>,
    pub(crate) oauth: Option<McpOAuth>,
    pub(crate) protocol: Option<McpProtocol>,
    pub(crate) access_mode: Option<McpAccessMode>,
    pub(crate) env_file: Option<String>,
    pub(crate) raw: Value,
}

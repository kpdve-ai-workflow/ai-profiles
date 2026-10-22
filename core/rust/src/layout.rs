use crate::model::Client;

/// Legacy assets are frozen; only exclusive catalogs/configuration are owned.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Layout {
    Legacy,
    Isolated(Client),
}

pub(crate) const STATE: &str = ".ai-profiles/state.json";
pub(crate) const LEGACY_STATE: &str = ".opencode/ai-profile-state.json";
pub(crate) const LEGACY_LOCAL: &str = ".opencode/ai-profile.local.json";

impl Layout {
    pub(crate) fn from_state(client: Client, state: Option<&serde_json::Value>) -> Self {
        if state.and_then(|v| v.get("layout")).and_then(|v| v.as_str()) == Some("legacy") {
            Self::Legacy
        } else {
            Self::Isolated(client)
        }
    }

    pub(crate) fn root(self) -> String {
        match self {
            Self::Legacy => ".opencode/ai-profile".into(),
            Self::Isolated(client) => format!(".ai-profiles/{}", client.as_str()),
        }
    }

    pub(crate) fn plugin(self) -> String {
        format!("{}/plugin", self.root())
    }
    pub(crate) fn skills(self) -> String {
        format!("{}/skills", self.plugin())
    }
    pub(crate) fn scripts(self) -> String {
        format!("{}/scripts", self.root())
    }
    pub(crate) fn readme(self) -> String {
        format!("{}/README.md", self.root())
    }
    pub(crate) fn adapter(self) -> String {
        match self {
            Self::Legacy => ".opencode/ai-profile-mcp".into(),
            Self::Isolated(_) => format!("{}/ai-profile-mcp", self.root()),
        }
    }
    pub(crate) fn local(self) -> String {
        match self {
            Self::Legacy => ".opencode/ai-profile.local.json".into(),
            Self::Isolated(_) => format!("{}/local.json", self.root()),
        }
    }
    pub(crate) fn mcp(self) -> String {
        match self {
            Self::Legacy => ".opencode/mcp".into(),
            Self::Isolated(_) => format!("{}/mcp", self.root()),
        }
    }

    /// Only recognized repository-managed asset paths; never arbitrary strings.
    pub(crate) fn managed_path(self, path: &str) -> String {
        for (prefix, replacement) in [
            (".opencode/ai-profile/", format!("{}/", self.root())),
            (".opencode/mcp/", format!("{}/", self.mcp())),
        ] {
            if let Some(suffix) = path.strip_prefix(prefix) {
                return format!("{replacement}{suffix}");
            }
        }
        if path == ".opencode/ai-profile-mcp" {
            self.adapter()
        } else if path == ".opencode/ai-profile.local.json" {
            self.local()
        } else {
            path.to_owned()
        }
    }

    pub(crate) fn render_text(self, text: &str, client: Client) -> String {
        [
            ("{{AI_PROFILE_ROOT}}", self.root()),
            ("{{AI_PROFILE_MCP}}", self.mcp()),
            ("{{AI_PROFILE_LOCAL}}", self.local()),
            ("{{AI_PROFILE_CLIENT}}", client.as_str().into()),
        ]
        .into_iter()
        .fold(text.to_owned(), |text, (key, value)| {
            text.replace(key, &value)
        })
    }

    pub(crate) fn render(self, text: &str) -> String {
        match self {
            Self::Legacy => text.to_owned(),
            Self::Isolated(client) => self
                .render_text(text, client)
                .replace(".opencode/ai-profile.local.json", &self.local())
                .replace(".opencode/ai-profile-mcp", &self.adapter())
                .replace(".opencode/ai-profile/", &format!("{}/", self.root()))
                .replace(".opencode/mcp/", &format!("{}/", self.mcp())),
        }
    }
}

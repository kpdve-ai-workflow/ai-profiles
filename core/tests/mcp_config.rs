use serde_json::json;

use crate::{
    error::McpConfigError,
    mcp::config::{
        configured_mcp_at, merge, native_client_mcp, render, resolve, resolve_at, validate_servers,
    },
};

#[test]
fn recursive_override_replaces_arrays_and_preserves_other_fields() {
    let base = json!({"server":{"nested":{"a":1,"b":2},"items":[1,2],"keep":true}});
    let override_value = json!({"server":{"nested":{"a":3},"items":[9]}});
    let merged = merge(&base, &override_value);
    assert_eq!(merged["server"]["nested"], json!({"a":3,"b":2}));
    assert_eq!(merged["server"]["items"], json!([9]));
    assert_eq!(merged["server"]["keep"], true);
}

#[test]
fn local_servers_render_native_adapter_and_remote_is_preserved() {
    let effective = json!({
        "local":{"type":"local","command":["tool"],"environment":{"KEY":"secret"},"cwd":"tools"},
        "remote":{"type":"remote","url":"https://example.com/mcp"}
    });
    let rendered = render(effective.as_object().unwrap()).unwrap();
    assert_eq!(
        rendered["local"]["command"],
        json!(["./.opencode/ai-profile-mcp", "local"])
    );
    assert!(rendered["local"].get("environment").is_none());
    assert!(rendered["local"].get("cwd").is_none());
    assert_eq!(rendered["remote"], effective["remote"]);
}

#[test]
fn invalid_registry_and_boolean_timeout_are_rejected() {
    let servers = json!({"x":{"type":"local","command":["run"],"timeout":{"startup":true}}});
    assert!(matches!(
        validate_servers(servers.as_object().unwrap(), None),
        Err(McpConfigError::InvalidTimeout(_))
    ));
}

#[test]
fn legacy_local_override_remains_supported_for_legacy_layout() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join(".opencode")).unwrap();
    std::fs::write(
        project.path().join(".opencode/ai-profile.local.json"),
        br#"{"mcp":{"server":{"environment":{"SOURCE":"legacy"}}}}"#,
    )
    .unwrap();
    let base = json!({"server":{"type":"local","command":["run"]}});
    let resolved = resolve(project.path(), base.as_object().unwrap()).unwrap();
    assert_eq!(
        resolved.servers["server"]["environment"]["SOURCE"],
        "legacy"
    );
}

#[test]
fn native_clients_render_local_remote_disabled_and_timeout_entries() {
    let effective = json!({
        "local":{"type":"local","command":["/project path/.opencode/ai-profile-mcp","local"],"timeout":{"startup":1500,"execution":2500}},
        "figma":{"type":"remote","url":"https://mcp.figma.com/mcp","headers":{"Authorization":"Bearer {env:FIGMA_TOKEN}"}},
        "off":{"type":"local","command":["/adapter","off"],"disabled":true}
    });
    let codex = native_client_mcp(effective.as_object().unwrap(), "codex").unwrap();
    assert_eq!(
        codex["local"]["command"],
        "/project path/.opencode/ai-profile-mcp"
    );
    assert_eq!(codex["local"]["args"], json!(["local"]));
    assert_eq!(codex["local"]["startup_timeout_sec"], 2);
    assert_eq!(codex["local"]["tool_timeout_sec"], 3);
    assert_eq!(
        codex["figma"]["http_headers"]["Authorization"],
        "Bearer ${FIGMA_TOKEN}"
    );
    assert!(codex.get("off").is_none());
    let claude = native_client_mcp(effective.as_object().unwrap(), "claude").unwrap();
    assert_eq!(claude["local"]["type"], "stdio");
    assert_eq!(claude["figma"]["type"], "http");
    assert_eq!(
        claude["figma"]["headers"]["Authorization"],
        "Bearer ${FIGMA_TOKEN}"
    );
}

#[test]
fn playwright_uses_the_shared_adapter_and_keeps_runtime_overrides() {
    let project = tempfile::Builder::new()
        .prefix("project with spaces, ")
        .tempdir()
        .unwrap();
    let definition: serde_json::Value =
        serde_json::from_str(include_str!("../../mcp/servers/playwright.json")).unwrap();
    for client in crate::model::Client::ALL {
        let layout = crate::layout::Layout::Isolated(client);
        let mut client_definition = definition.clone();
        client_definition["command"][0] = json!(format!(
            "{}/mcp/playwright/playwright-launcher",
            layout.root()
        ));
        let base = json!({"playwright":client_definition});
        let base = base.as_object().unwrap();
        std::fs::create_dir_all(project.path().join(layout.root())).unwrap();
        std::fs::write(
            project.path().join(layout.local()),
            serde_json::to_vec(&json!({"mcp":{"playwright":{
                "env_file":".ai-profiles/.env.playwright",
                "environment":{"AI_PROFILE_PLAYWRIGHT_NETWORK":"{env:DEV_NETWORK}"}
            }}}))
            .unwrap(),
        )
        .unwrap();
        let resolved = resolve_at(project.path(), base, &layout.local()).unwrap();
        assert_eq!(
            resolved.servers["playwright"]["command"][0],
            format!("{}/mcp/playwright/playwright-launcher", layout.root())
        );
        assert_eq!(
            resolved.servers["playwright"]["env_file"],
            ".ai-profiles/.env.playwright"
        );
        assert_eq!(
            resolved.servers["playwright"]["environment"]["AI_PROFILE_PLAYWRIGHT_NETWORK"],
            "{env:DEV_NETWORK}"
        );

        let rendered = configured_mcp_at(project.path(), base, &layout.local(), &layout.adapter())
            .unwrap()
            .servers;
        let adapter = project.path().join(layout.adapter());
        assert_eq!(
            rendered["playwright"]["command"],
            json!([adapter.to_str().unwrap(), "playwright"])
        );
        assert!(rendered["playwright"].get("environment").is_none());
        assert!(rendered["playwright"].get("env_file").is_none());
        for native_client in crate::model::Client::ALL {
            let native = native_client_mcp(&rendered, native_client.as_str()).unwrap();
            assert_eq!(native["playwright"]["command"], adapter.to_str().unwrap());
            assert_eq!(native["playwright"]["args"], json!(["playwright"]));
        }
    }
}

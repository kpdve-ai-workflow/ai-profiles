use serde_json::json;

use crate::{
    error::McpConfigError,
    mcp::config::{merge, validate_servers},
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
fn invalid_registry_and_boolean_timeout_are_rejected() {
    let servers = json!({"x":{"type":"local","command":["run"],"timeout":{"startup":true}}});
    assert!(matches!(
        validate_servers(servers.as_object().unwrap(), None),
        Err(McpConfigError::InvalidTimeout(_))
    ));
}

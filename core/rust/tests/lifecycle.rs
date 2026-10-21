use std::{fs, os::unix::fs::PermissionsExt, path::Path};

use serde_json::json;

use crate::{
    codec::{json_bytes, read_json, sha},
    filesystem::check_files,
    model::RepositoryContext,
    profile_management::{
        installation::apply_profile, payload::desired_files, profiles::rename_profile,
    },
};

#[test]
fn client_mcp_ownership_preserves_unrelated_config_and_rejects_manual_changes() {
    use crate::profile_management::native::{configure_claude_mcp, configure_codex_mcp};
    let project = tempfile::tempdir().unwrap();
    let mut desired = serde_json::Map::new();
    desired.insert("figma".into(), json!({"url":"https://mcp.figma.com/mcp","http_headers":{"Authorization":"Bearer ${TOKEN}"}}));
    let codex_input = b"# keep this comment\nmodel = \"gpt-5\"\n[other]\nkeep = true\n";
    let (codex, codex_state) =
        configure_codex_mcp(codex_input, None, &desired, Some(true)).unwrap();
    let codex = String::from_utf8(codex).unwrap();
    assert!(codex.contains("# keep this comment"));
    assert!(codex.contains("[other]"));
    let old = json!({"native":{"mcp_codex":codex_state}});
    let (codex_again, _) =
        configure_codex_mcp(codex.as_bytes(), Some(&old), &desired, Some(true)).unwrap();
    assert_eq!(codex_again, codex.as_bytes());
    let modified = codex.replace("mcp.figma.com", "changed.example");
    assert!(configure_codex_mcp(modified.as_bytes(), Some(&old), &desired, Some(true)).is_err());

    let claude_initial = project.path().join(".mcp.json");
    fs::write(
        &claude_initial,
        br#"{"otherRoot":true,"mcpServers":{"unrelated":{"command":"keep"}}}"#,
    )
    .unwrap();
    let (claude, claude_state) = configure_claude_mcp(project.path(), None, &desired).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&claude).unwrap();
    assert_eq!(parsed["otherRoot"], true);
    assert_eq!(parsed["mcpServers"]["unrelated"]["command"], "keep");
    let old = json!({"native":{"mcp_claude":claude_state}});
    fs::write(&claude_initial, &claude).unwrap();
    let (again, _) = configure_claude_mcp(project.path(), Some(&old), &desired).unwrap();
    assert_eq!(again, claude);
    let changed: serde_json::Value = serde_json::from_slice(&claude).unwrap();
    let mut changed = changed;
    changed["mcpServers"]["figma"]["url"] = json!("https://changed.example");
    fs::write(&claude_initial, crate::codec::json_bytes(&changed).unwrap()).unwrap();
    assert!(configure_claude_mcp(project.path(), Some(&old), &desired).is_err());
}

fn put(root: &Path, relative: &str, contents: &[u8]) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn native_source() -> (tempfile::TempDir, RepositoryContext) {
    let source = tempfile::tempdir().unwrap();
    for directory in ["core/rust/src", "skills", "mcp/servers"] {
        fs::create_dir_all(source.path().join(directory)).unwrap();
    }
    put(
        source.path(),
        "skills/test-gap-discovery/SKILL.md",
        b"---\nname: test-gap-discovery\ndescription: Test inventory\n---\n",
    );
    put(
        source.path(),
        "skills/test-gap-discovery/scripts/inventory-tests.sh",
        b"reference only",
    );
    put(
        source.path(),
        "profiles/wordpress/skills.json",
        br#"{"skills":["test-gap-discovery"]}"#,
    );
    put(
        source.path(),
        "profiles/wordpress/profile.json",
        br#"{"name":"wordpress","version":"1.0.0","description":"WordPress"}"#,
    );
    put(
        source.path(),
        "profiles/wordpress/mcp.json",
        br#"{"servers":["mariadb"]}"#,
    );
    put(
        source.path(),
        "mcp/servers/mariadb.json",
        br#"{"type":"local","command":[".opencode/mcp/mariadb/mariadb-launcher"]}"#,
    );
    put(
        source.path(),
        "mcp/launchers/mariadb/launch.py",
        b"reference only",
    );
    put(
        source.path(),
        "profiles/wordpress/scripts/restore-updraft.py",
        b"reference only",
    );
    put(
        source.path(),
        "profiles/wordpress/scripts/local-guard.php",
        b"<?php // helper",
    );
    for binary in [
        "ai-profile-mcp",
        "mariadb-launcher",
        "restore-updraft",
        "test-inventory",
    ] {
        let relative = format!("core/rust/target/release/{binary}");
        // Synthetic executable bytes are copied and hashed, never executed.
        put(source.path(), &relative, binary.as_bytes());
        fs::set_permissions(
            source.path().join(relative),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
    }
    let context = RepositoryContext::from_sources(source.path().to_owned()).unwrap();
    (source, context)
}

#[test]
fn renamed_wordpress_profile_keeps_its_native_payload() {
    let (_source, context) = native_source();
    let before = desired_files(&context, "wordpress").unwrap();
    rename_profile(&context, "wordpress", "custom-wordpress").unwrap();
    assert_eq!(desired_files(&context, "custom-wordpress").unwrap(), before);
}

#[test]
fn native_install_retires_owned_python_and_preserves_user_files() {
    let (_source, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    let legacy = ".opencode/ai-profile-mcp.py";
    put(project.path(), legacy, b"old managed adapter");
    put(project.path(), "user.py", b"user owned");
    let old_mcp = json!({"mariadb":{"type":"local","command":["python3",legacy,"mariadb"]}});
    let old_state = json!({
        "format":1,"profile":"wordpress","version":"1.0.0",
        "files":{(legacy):sha(b"old managed adapter")},"mcp":old_mcp,
        "had_config":true
    });
    put(
        project.path(),
        ".opencode/ai-profile-state.json",
        &json_bytes(&old_state).unwrap(),
    );
    put(
        project.path(),
        "opencode.json",
        &json_bytes(&json!({"mcp":old_mcp,"theme":"keep"})).unwrap(),
    );
    let canonical_project = project.path().canonicalize().unwrap();
    let marketplace = format!(
        "ai-profiles-{}",
        &sha(canonical_project.to_string_lossy().as_bytes())[..12]
    );
    put(
        project.path(),
        ".claude/settings.json",
        &json_bytes(&json!({"enabledPlugins":{format!("ai-profile@{marketplace}"):false}}))
            .unwrap(),
    );
    put(
        project.path(),
        ".codex/config.toml",
        format!("[plugins.\"ai-profile@{marketplace}\"]\nenabled = false\n").as_bytes(),
    );

    apply_profile(&context, project.path(), "wordpress").unwrap();
    assert!(!project.path().join(legacy).exists());
    assert_eq!(
        fs::read(project.path().join("user.py")).unwrap(),
        b"user owned"
    );
    for (relative, expected) in [
        (
            ".opencode/ai-profile/plugin/skills/test-gap-discovery/scripts/inventory-tests",
            "test-inventory",
        ),
        (".opencode/ai-profile-mcp", "ai-profile-mcp"),
        (".opencode/mcp/mariadb/mariadb-launcher", "mariadb-launcher"),
        (
            ".opencode/ai-profile/scripts/restore-updraft",
            "restore-updraft",
        ),
    ] {
        let path = project.path().join(relative);
        assert_eq!(fs::read(&path).unwrap(), expected.as_bytes());
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o755
        );
    }
    let state_path = project.path().join(".opencode/ai-profile-state.json");
    let state = read_json(&state_path).unwrap();
    assert_eq!(
        state
            .pointer("/native/claude/activation")
            .and_then(serde_json::Value::as_str),
        Some("disabled")
    );
    assert_eq!(
        state
            .pointer("/native/codex/activation")
            .and_then(serde_json::Value::as_str),
        Some("disabled")
    );
    assert!(state["files"]
        .as_object()
        .unwrap()
        .keys()
        .all(|path| !path.ends_with(".py")));
    let config = read_json(&project.path().join("opencode.json")).unwrap();
    assert_eq!(config["theme"], "keep");
    assert_eq!(
        config["mcp"]["mariadb"]["command"],
        json!([
            project
                .path()
                .join(".opencode/ai-profile-mcp")
                .to_str()
                .unwrap(),
            "mariadb"
        ])
    );
    assert_eq!(
        state["mcp_base"]["mariadb"]["command"],
        json!([".opencode/mcp/mariadb/mariadb-launcher"])
    );
    let claude_mcp = read_json(&project.path().join(".mcp.json")).unwrap();
    assert_eq!(claude_mcp["mcpServers"]["mariadb"]["type"], "stdio");
    assert!(claude_mcp["mcpServers"]["mariadb"]["command"]
        .as_str()
        .unwrap()
        .contains(".opencode/ai-profile-mcp"));
    let codex = fs::read_to_string(project.path().join(".codex/config.toml")).unwrap();
    assert!(codex.contains("mcp_servers"));
    assert!(codex.contains("args = [\"mariadb\"]"));
    apply_profile(&context, project.path(), "wordpress").unwrap();
    assert_eq!(read_json(&state_path).unwrap(), state);
    crate::profile_management::uninstall::uninstall(&context, project.path()).unwrap();
    assert!(!project.path().join(".mcp.json").exists());
    let codex = fs::read_to_string(project.path().join(".codex/config.toml")).unwrap();
    assert!(!codex.contains("mcp_servers"));
    assert!(codex.contains(&format!("ai-profile@{marketplace}")));
    assert!(read_json(&project.path().join("opencode.json")).unwrap()["theme"] == "keep");
    assert_eq!(
        fs::read(project.path().join("user.py")).unwrap(),
        b"user owned"
    );
}

#[test]
fn missing_native_binary_stops_install_before_project_writes() {
    let (_source, context) = native_source();
    fs::remove_file(
        context
            .sources
            .join("core/rust/target/release/restore-updraft"),
    )
    .unwrap();
    let project = tempfile::tempdir().unwrap();
    assert!(apply_profile(&context, project.path(), "wordpress").is_err());
    assert_eq!(fs::read_dir(project.path()).unwrap().count(), 0);
}

#[test]
fn changed_legacy_adapter_stops_migration_and_unported_python_is_rejected() {
    let (_source, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    let legacy = ".opencode/ai-profile-mcp.py";
    put(project.path(), legacy, b"local edits");
    let state =
        json!({"format":1,"profile":"wordpress","files":{(legacy):sha(b"original")},"mcp":{}});
    put(
        project.path(),
        ".opencode/ai-profile-state.json",
        &json_bytes(&state).unwrap(),
    );
    assert!(apply_profile(&context, project.path(), "wordpress").is_err());
    assert_eq!(
        fs::read(project.path().join(legacy)).unwrap(),
        b"local edits"
    );
    assert!(!project.path().join(".opencode/ai-profile-mcp").exists());
    put(
        &context.sources,
        "profiles/wordpress/scripts/unported.py",
        b"reference only",
    );
    assert!(desired_files(&context, "wordpress").is_err());
}

#[test]
fn ownership_preflight_rejects_unmanaged_collision_without_writing() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("unmanaged"), b"keep").unwrap();
    assert!(check_files(temp.path(), None, ["unmanaged"]).is_err());
    assert_eq!(
        std::fs::read(temp.path().join("unmanaged")).unwrap(),
        b"keep"
    );
}

use std::{
    fs,
    os::unix::fs::{FileTypeExt, PermissionsExt},
    path::Path,
};

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
        br#"{"servers":["legacy"]}"#,
    );
    put(
        source.path(),
        "mcp/servers/legacy.json",
        br#"{"type":"local","command":[".opencode/ai-profile-mcp","legacy"]}"#,
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
        "playwright-launcher",
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

fn scoped_action(
    context: &RepositoryContext,
    project: &Path,
    action: crate::model::ProfileAction,
    client: crate::model::Client,
    profile: Option<&str>,
) {
    crate::profile_management::scoped::run(
        context,
        project,
        action,
        crate::model::ClientSelection::One(client),
        profile,
    )
    .unwrap();
}

fn novamira_source() -> (tempfile::TempDir, RepositoryContext) {
    let (source, context) = native_source();
    put(
        source.path(),
        "profiles/wordpress/mcp.json",
        br#"{"servers":["novamira"]}"#,
    );
    // Installation requires the complete bundle (mcp/README.md), even when
    // this test only observes ownership of the user-created credentials.
    put(
        source.path(),
        "mcp/servers/novamira.json",
        include_bytes!("../../../mcp/servers/novamira.json"),
    );
    let images = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mcp/images/novamira");
    for filename in [
        "Dockerfile",
        ".dockerignore",
        "compose.yaml",
        "compose.ca.yaml",
        "README.md",
        "novamira.zip",
        "launcher",
        "install-plugin",
    ] {
        put(
            source.path(),
            &format!("mcp/images/novamira/{filename}"),
            &fs::read(images.join(filename)).unwrap(),
        );
    }
    (source, context)
}

#[test]
fn novamira_credentials_are_user_owned_optional_for_install_and_survive_lifecycle() {
    use crate::model::{Client, ProfileAction as A};
    let (source, context) = novamira_source();
    let project = tempfile::tempdir().unwrap();
    put(project.path(), ".gitignore", b"# user rules\nkeep\n");
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::OpenCode,
        Some("wordpress"),
    );
    let env = project.path().join(".env.novamira");
    assert!(!env.exists(), "install must not create a credential file");
    let config = read_json(&project.path().join("opencode.json")).unwrap();
    assert_eq!(config["mcp"]["novamira"]["disabled"], false);
    let secret = b"WP_API_PASSWORD=synthetic-never-store-this\n";
    fs::write(&env, secret).unwrap();
    let before = fs::read(project.path().join("opencode.json")).unwrap();
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::Claude,
        Some("wordpress"),
    );
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::Codex,
        Some("wordpress"),
    );
    assert_eq!(
        fs::read(project.path().join("opencode.json")).unwrap(),
        before
    );
    put(
        project.path(),
        ".ai-profiles/opencode/local.json",
        br#"{"mcp":{"novamira":{"disabled":true,"timeout":45000}}}"#,
    );
    scoped_action(&context, project.path(), A::Update, Client::OpenCode, None);
    let config = read_json(&project.path().join("opencode.json")).unwrap();
    assert_eq!(config["mcp"]["novamira"]["disabled"], true);
    let state = read_json(&project.path().join(".ai-profiles/state.json")).unwrap();
    assert_eq!(
        state["clients"]["opencode"]["mcp_base"]["novamira"]["env_file"],
        ".env.novamira"
    );
    assert!(!state.to_string().contains("synthetic-never-store-this"));
    assert!(!state["clients"]["opencode"]["files"]
        .as_object()
        .unwrap()
        .contains_key(".env.novamira"));
    // Moving away from Novamira must leave this user-owned file untouched.
    put(
        source.path(),
        "profiles/empty/profile.json",
        br#"{"name":"empty","version":"1.0.0","description":"Empty"}"#,
    );
    put(
        source.path(),
        "profiles/empty/skills.json",
        br#"{"skills":[]}"#,
    );
    put(
        source.path(),
        "profiles/empty/mcp.json",
        br#"{"servers":[]}"#,
    );
    fs::remove_file(project.path().join(".ai-profiles/opencode/local.json")).unwrap();
    scoped_action(
        &context,
        project.path(),
        A::Switch,
        Client::OpenCode,
        Some("empty"),
    );
    assert_eq!(fs::read(&env).unwrap(), secret);
    for client in [Client::OpenCode, Client::Claude, Client::Codex] {
        scoped_action(&context, project.path(), A::Uninstall, client, None);
    }
    assert_eq!(fs::read(&env).unwrap(), secret);
    assert_eq!(
        fs::read(project.path().join(".gitignore")).unwrap(),
        b"# user rules\nkeep\n"
    );
}

#[test]
fn novamira_credential_collisions_do_not_block_install_or_get_modified() {
    use crate::model::{Client, ClientSelection, ProfileAction as A};
    let (_source, context) = novamira_source();
    for kind in ["symlink", "dangling", "directory", "fifo"] {
        let project = tempfile::tempdir().unwrap();
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::write(outside.path(), b"unchanged").unwrap();
        fs::create_dir_all(project.path().join(".ai-profiles/opencode/mcp/novamira")).unwrap();
        let path = project.path().join(".env.novamira");
        match kind {
            "symlink" => std::os::unix::fs::symlink(outside.path(), &path).unwrap(),
            "dangling" => {
                std::os::unix::fs::symlink(project.path().join("missing"), &path).unwrap()
            }
            "directory" => fs::create_dir(&path).unwrap(),
            "fifo" => {
                assert!(std::process::Command::new("mkfifo")
                    .arg(&path)
                    .status()
                    .unwrap()
                    .success());
            }
            _ => unreachable!(),
        }
        crate::profile_management::scoped::run(
            &context,
            project.path(),
            A::Install,
            ClientSelection::One(Client::OpenCode),
            Some("wordpress"),
        )
        .unwrap();
        assert!(project.path().join("opencode.json").exists());
        assert!(project.path().join(".ai-profiles/state.json").exists());
        assert_eq!(fs::read(outside.path()).unwrap(), b"unchanged");
        match kind {
            "symlink" | "dangling" => assert!(fs::symlink_metadata(&path)
                .unwrap()
                .file_type()
                .is_symlink()),
            "directory" => assert!(path.is_dir()),
            "fifo" => assert!(fs::symlink_metadata(&path).unwrap().file_type().is_fifo()),
            _ => unreachable!(),
        }
    }
}

#[test]
fn activation_is_scoped_failed_state_is_saved_and_retry_keeps_success_history() {
    use crate::{
        model::{Client, ClientSelection as S, ProfileAction as A},
        process::{CommandOutput, CommandSpec, ProcessRunner},
        profile_management::scoped::run_with_runner,
    };
    use std::{cell::RefCell, os::unix::process::ExitStatusExt};
    struct Runner {
        calls: RefCell<Vec<CommandSpec>>,
        fail: bool,
    }
    impl ProcessRunner for Runner {
        fn run(
            &self,
            spec: CommandSpec,
        ) -> std::result::Result<CommandOutput, crate::error::ProcessError> {
            assert_eq!(spec.program, "claude");
            self.calls.borrow_mut().push(spec);
            Ok(CommandOutput {
                status: std::process::ExitStatus::from_raw(if self.fail { 256 } else { 0 }),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    }
    let (_sources, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    let runner = Runner {
        calls: RefCell::new(Vec::new()),
        fail: false,
    };
    run_with_runner(
        &context,
        project.path(),
        A::Install,
        S::One(Client::OpenCode),
        Some("wordpress"),
        &runner,
    )
    .unwrap();
    assert!(runner.calls.borrow().is_empty());
    run_with_runner(
        &context,
        project.path(),
        A::Install,
        S::One(Client::Claude),
        Some("wordpress"),
        &runner,
    )
    .unwrap();
    assert_eq!(runner.calls.borrow()[0].args[1], "install");
    let successful = read_json(&project.path().join(".ai-profiles/state.json")).unwrap();
    // Older native metadata can record synchronization without installed_before.
    let mut historical = successful.clone();
    historical["clients"]["claude"]["native"]["claude"]
        .as_object_mut()
        .unwrap()
        .remove("installed_before");
    put(
        project.path(),
        ".ai-profiles/state.json",
        &json_bytes(&historical).unwrap(),
    );
    let failed_runner = Runner {
        calls: RefCell::new(Vec::new()),
        fail: true,
    };
    assert!(run_with_runner(
        &context,
        project.path(),
        A::Update,
        S::One(Client::Claude),
        None,
        &failed_runner
    )
    .is_err());
    assert_eq!(failed_runner.calls.borrow()[0].args[1], "update");
    let failed = read_json(&project.path().join(".ai-profiles/state.json")).unwrap();
    assert_eq!(
        failed["clients"]["claude"]["native"]["claude"]["activation"],
        "failed"
    );
    assert_eq!(
        successful["clients"]["claude"]["native"]["claude"]["last_successful_sync"],
        failed["clients"]["claude"]["native"]["claude"]["last_successful_sync"]
    );
    assert_eq!(
        successful["clients"]["opencode"],
        failed["clients"]["opencode"]
    );
    run_with_runner(
        &context,
        project.path(),
        A::Update,
        S::One(Client::Claude),
        None,
        &runner,
    )
    .unwrap();
}

fn snapshot(root: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut result = std::collections::BTreeMap::new();
    if root.is_dir() {
        for entry in fs::read_dir(root).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                result.extend(snapshot(&path));
            } else {
                result.insert(path.clone(), fs::read(path).unwrap());
            }
        }
    }
    result
}

#[test]
fn historical_combined_inspection_is_read_only() {
    let (_sources, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    apply_profile(&context, project.path(), "wordpress").unwrap();
    let state = read_json(&project.path().join(".opencode/ai-profile-state.json")).unwrap();
    let before = snapshot(project.path());
    crate::profile_management::native::current_status(project.path(), &state).unwrap();
    crate::profile_management::native::mcp_configuration_status(project.path(), &state).unwrap();
    assert_eq!(before, snapshot(project.path()));
}

#[test]
fn versions_profiles_empty_selections_and_user_overrides_are_independent() {
    use crate::model::{Client, ProfileAction as A};
    let (sources, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::OpenCode,
        Some("wordpress"),
    );
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::Codex,
        Some("wordpress"),
    );
    put(
        sources.path(),
        "profiles/wordpress/profile.json",
        br#"{"name":"wordpress","version":"2.0.0","description":"WordPress"}"#,
    );
    scoped_action(&context, project.path(), A::Update, Client::OpenCode, None);
    let open = snapshot(&project.path().join(".ai-profiles/opencode"));
    let state = read_json(&project.path().join(".ai-profiles/state.json")).unwrap();
    assert_eq!(state["clients"]["opencode"]["version"], "2.0.0");
    assert_eq!(state["clients"]["codex"]["version"], "1.0.0");
    put(
        sources.path(),
        "profiles/wordpress/profile.json",
        br#"{"name":"wordpress","version":"1.0.0","description":"WordPress"}"#,
    );
    scoped_action(&context, project.path(), A::Update, Client::Codex, None);
    assert!(crate::profile_management::scoped::run(
        &context,
        project.path(),
        A::Update,
        crate::model::ClientSelection::One(Client::OpenCode),
        None
    )
    .is_err());
    put(
        sources.path(),
        "profiles/empty/profile.json",
        br#"{"name":"empty","version":"1.0.0","description":"Empty"}"#,
    );
    put(
        sources.path(),
        "profiles/empty/mcp.json",
        br#"{"servers":[]}"#,
    );
    put(
        sources.path(),
        "profiles/empty/skills.json",
        br#"{"skills":[]}"#,
    );
    put(
        project.path(),
        ".ai-profiles/codex/local.json",
        br#"{"mcp":{}}"#,
    );
    scoped_action(
        &context,
        project.path(),
        A::Switch,
        Client::Codex,
        Some("empty"),
    );
    assert!(!project
        .path()
        .join(".ai-profiles/codex/mcp/legacy")
        .exists());
    assert!(!project
        .path()
        .join(".ai-profiles/codex/ai-profile-mcp")
        .exists());
    assert_eq!(
        open,
        snapshot(&project.path().join(".ai-profiles/opencode"))
    );
    let state = read_json(&project.path().join(".ai-profiles/state.json")).unwrap();
    assert_eq!(state["clients"]["codex"]["profile"], "empty");
    assert_eq!(state["clients"]["opencode"]["version"], "2.0.0");
    scoped_action(&context, project.path(), A::Uninstall, Client::Codex, None);
    assert!(!project.path().join(".ai-profiles/codex/plugin").exists());
    assert_eq!(
        fs::read(project.path().join(".ai-profiles/codex/local.json")).unwrap(),
        br#"{"mcp":{}}"#
    );
}

#[test]
fn combined_legacy_native_migration_partitions_ownership_and_preserves_original_backups() {
    use crate::model::{Client, ProfileAction as A};
    let (_sources, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    put(
        project.path(),
        ".claude/settings.json",
        br#"{"user":"keep"}"#,
    );
    put(
        project.path(),
        ".codex/config.toml",
        b"# original\nmodel = \"keep\"\n",
    );
    apply_profile(&context, project.path(), "wordpress").unwrap();
    put(
        project.path(),
        ".opencode/ai-profile.local.json",
        br#"{"mcp":{"legacy":{"environment":{"CUSTOM":"literal"}}}}"#,
    );
    let imported = crate::project::manifest_for(&context, project.path()).unwrap();
    for client in Client::ALL {
        assert_eq!(imported["clients"][client.as_str()]["layout"], "legacy");
    }
    assert!(!project.path().join(".ai-profiles").exists());
    let frozen = snapshot(&project.path().join(".opencode"));
    let open_config = fs::read(project.path().join("opencode.json")).unwrap();
    let claude_config = fs::read(project.path().join(".claude/settings.json")).unwrap();
    let claude_catalog = fs::read(project.path().join(".claude-plugin/marketplace.json")).unwrap();
    let claude_mcp = fs::read(project.path().join(".mcp.json")).unwrap();
    for action in [A::Update, A::Update, A::Uninstall] {
        scoped_action(&context, project.path(), action, Client::Codex, None);
        assert_eq!(frozen, snapshot(&project.path().join(".opencode")));
        assert_eq!(
            open_config,
            fs::read(project.path().join("opencode.json")).unwrap()
        );
        assert_eq!(
            claude_config,
            fs::read(project.path().join(".claude/settings.json")).unwrap()
        );
        assert_eq!(
            claude_catalog,
            fs::read(project.path().join(".claude-plugin/marketplace.json")).unwrap()
        );
        assert_eq!(
            claude_mcp,
            fs::read(project.path().join(".mcp.json")).unwrap()
        );
        let state = read_json(&project.path().join(".ai-profiles/state.json")).unwrap();
        for client in [Client::OpenCode, Client::Claude] {
            assert_eq!(
                imported["clients"][client.as_str()],
                state["clients"][client.as_str()]
            );
        }
        if action == A::Update {
            assert_eq!(state["clients"]["codex"]["layout"], "isolated");
            assert_eq!(
                state["clients"]["codex"]["native"]["codex"]["plugins_had_table"],
                imported["clients"]["codex"]["native"]["codex"]["plugins_had_table"]
            );
            assert_eq!(
                state["clients"]["codex"]["native"]["codex"]["enabled_previous"],
                imported["clients"]["codex"]["native"]["codex"]["enabled_previous"]
            );
        }
    }
    assert!(project
        .path()
        .join(".ai-profiles/codex/local.json")
        .is_file());
    let remaining = fs::read_to_string(project.path().join(".codex/config.toml")).unwrap();
    assert!(remaining.contains("# original"));
    assert!(remaining.contains("model = \"keep\""));
}

#[test]
fn all_keeps_successful_clients_when_native_activation_fails_later() {
    use crate::{
        model::{ClientSelection, ProfileAction as A},
        process::{CommandOutput, CommandSpec, ProcessRunner},
        profile_management::scoped::run_with_runner,
    };
    use std::{cell::RefCell, os::unix::process::ExitStatusExt};
    struct Runner(RefCell<Vec<std::ffi::OsString>>);
    impl ProcessRunner for Runner {
        fn run(
            &self,
            spec: CommandSpec,
        ) -> std::result::Result<CommandOutput, crate::error::ProcessError> {
            let successful = spec.program == "claude";
            self.0.borrow_mut().push(spec.program);
            Ok(CommandOutput {
                status: std::process::ExitStatus::from_raw(if successful { 0 } else { 256 }),
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    }
    let (_sources, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    let runner = Runner(RefCell::new(Vec::new()));
    assert!(run_with_runner(
        &context,
        project.path(),
        A::Install,
        ClientSelection::All,
        Some("wordpress"),
        &runner
    )
    .is_err());
    let calls: Vec<String> = runner
        .0
        .borrow()
        .iter()
        .map(|s| s.to_string_lossy().into_owned())
        .collect();
    assert_eq!(calls, ["claude", "codex"]);
    let state = read_json(&project.path().join(".ai-profiles/state.json")).unwrap();
    assert_eq!(state["clients"]["opencode"]["profile"], "wordpress");
    assert_eq!(
        state["clients"]["claude"]["native"]["claude"]["activation"],
        "synchronized"
    );
    assert_eq!(
        state["clients"]["codex"]["native"]["codex"]["activation"],
        "failed"
    );
}

#[test]
fn scoped_clients_coexist_and_lifecycle_preserves_foreign_bytes_and_records() {
    use crate::model::{Client, ProfileAction as A};
    let (_sources, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    put(project.path(), ".codex/config.toml", b"malformed = [");
    put(project.path(), ".claude/settings.json", b"not json");
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::OpenCode,
        Some("wordpress"),
    );
    assert!(!project.path().join(".claude-plugin").exists());
    assert!(!project.path().join(".agents").exists());
    assert!(!project
        .path()
        .join(".ai-profiles/opencode/plugin/.codex-plugin")
        .exists());
    let open_payload = snapshot(&project.path().join(".ai-profiles/opencode"));
    let open_config = fs::read(project.path().join("opencode.json")).unwrap();
    let before = read_json(&project.path().join(".ai-profiles/state.json")).unwrap()["clients"]
        ["opencode"]
        .clone();
    fs::remove_file(project.path().join(".codex/config.toml")).unwrap();
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::Codex,
        Some("wordpress"),
    );
    assert_eq!(
        open_payload,
        snapshot(&project.path().join(".ai-profiles/opencode"))
    );
    assert_eq!(
        open_config,
        fs::read(project.path().join("opencode.json")).unwrap()
    );
    assert_eq!(
        before,
        read_json(&project.path().join(".ai-profiles/state.json")).unwrap()["clients"]["opencode"]
    );
    fs::remove_file(project.path().join(".claude/settings.json")).unwrap();
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::Claude,
        Some("wordpress"),
    );
    let claude_payload = snapshot(&project.path().join(".ai-profiles/claude"));
    let claude_config = fs::read(project.path().join(".claude/settings.json")).unwrap();
    let claude_record = read_json(&project.path().join(".ai-profiles/state.json")).unwrap()
        ["clients"]["claude"]
        .clone();
    for action in [A::Update, A::Switch, A::Status, A::Uninstall] {
        scoped_action(
            &context,
            project.path(),
            action,
            Client::Codex,
            (action == A::Switch).then_some("wordpress"),
        );
        assert_eq!(
            open_payload,
            snapshot(&project.path().join(".ai-profiles/opencode"))
        );
        assert_eq!(
            claude_payload,
            snapshot(&project.path().join(".ai-profiles/claude"))
        );
        assert_eq!(
            claude_config,
            fs::read(project.path().join(".claude/settings.json")).unwrap()
        );
        let state = read_json(&project.path().join(".ai-profiles/state.json")).unwrap();
        assert_eq!(before, state["clients"]["opencode"]);
        assert_eq!(claude_record, state["clients"]["claude"]);
    }
}

#[test]
fn native_only_clients_ignore_invalid_opencode_and_other_native_config() {
    use crate::model::{Client, ProfileAction as A};
    for client in [Client::Claude, Client::Codex] {
        let (_sources, context) = native_source();
        let project = tempfile::tempdir().unwrap();
        put(project.path(), "opencode.json", b"invalid");
        put(project.path(), "opencode.jsonc", b"invalid");
        let other = if client == Client::Codex {
            ".claude/settings.json"
        } else {
            ".codex/config.toml"
        };
        put(project.path(), other, b"invalid");
        scoped_action(
            &context,
            project.path(),
            A::Install,
            client,
            Some("wordpress"),
        );
        let before = snapshot(project.path());
        scoped_action(&context, project.path(), A::Status, client, None);
        assert_eq!(before, snapshot(project.path()));
        scoped_action(&context, project.path(), A::Update, client, None);
        scoped_action(&context, project.path(), A::Uninstall, client, None);
        assert_eq!(
            fs::read(project.path().join("opencode.json")).unwrap(),
            b"invalid"
        );
        assert_eq!(fs::read(project.path().join(other)).unwrap(), b"invalid");
    }
}

#[test]
fn all_is_additive_preflights_conflicts_and_keeps_empty_authoritative_state() {
    use crate::model::{Client, ClientSelection, ProfileAction as A};
    use crate::profile_management::scoped::run;
    let (_sources, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    assert!(run(
        &context,
        project.path(),
        A::Update,
        ClientSelection::All,
        None
    )
    .is_err());
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::OpenCode,
        Some("wordpress"),
    );
    let before = snapshot(project.path());
    assert!(run(
        &context,
        project.path(),
        A::Install,
        ClientSelection::All,
        Some("unknown")
    )
    .is_err());
    assert_eq!(before, snapshot(project.path()));
    run(
        &context,
        project.path(),
        A::Install,
        ClientSelection::All,
        Some("wordpress"),
    )
    .unwrap();
    let before = snapshot(project.path());
    run(
        &context,
        project.path(),
        A::Install,
        ClientSelection::All,
        Some("wordpress"),
    )
    .unwrap();
    assert_eq!(before, snapshot(project.path()));
    run(
        &context,
        project.path(),
        A::Uninstall,
        ClientSelection::All,
        None,
    )
    .unwrap();
    assert_eq!(
        read_json(&project.path().join(".ai-profiles/state.json")).unwrap()["clients"],
        json!({})
    );
}

#[test]
fn lazy_legacy_import_and_selected_migration_retain_frozen_assets_and_overrides() {
    use crate::model::{Client, ProfileAction as A};
    let (_sources, context) = native_source();
    let project = tempfile::tempdir().unwrap();
    put(
        project.path(),
        ".opencode/ai-profile/plugin/skills/frozen/SKILL.md",
        b"frozen payload",
    );
    put(
        project.path(),
        ".opencode/ai-profile.local.json",
        br#"{"mcp":{"legacy":{"environment":{"PASSWORD":"do-not-print"}}}}"#,
    );
    let legacy = json!({"format":1,"profile":"wordpress","version":"1.0.0","files":{},"mcp":{},"mcp_base":{"legacy":{"type":"local","command":[".opencode/mcp/legacy/launcher"]}},"native":{"opencode_skill_path_added":true},"had_config":false});
    put(
        project.path(),
        ".opencode/ai-profile-state.json",
        &json_bytes(&legacy).unwrap(),
    );
    let frozen = snapshot(&project.path().join(".opencode"));
    scoped_action(&context, project.path(), A::Status, Client::OpenCode, None);
    assert!(!project.path().join(".ai-profiles").exists());
    scoped_action(
        &context,
        project.path(),
        A::Install,
        Client::Codex,
        Some("wordpress"),
    );
    assert_eq!(frozen, snapshot(&project.path().join(".opencode")));
    assert!(!project
        .path()
        .join(".ai-profiles/codex/local.json")
        .exists());
    scoped_action(&context, project.path(), A::Update, Client::OpenCode, None);
    assert_eq!(frozen, snapshot(&project.path().join(".opencode")));
    assert!(project
        .path()
        .join(".ai-profiles/opencode/local.json")
        .is_file());
    assert_eq!(
        read_json(&project.path().join(".ai-profiles/state.json")).unwrap()["clients"]["codex"]
            ["layout"],
        "isolated"
    );
}

const PLAYWRIGHT_FILES: [&str; 11] = [
    "Dockerfile",
    "entrypoint.sh",
    "basic-auth.cjs",
    "LICENSES.md",
    "README.md",
    ".dockerignore",
    "compose.yaml",
    "compose.host.yaml",
    "compose.ca.yaml",
    "compose.auth.yaml",
    "compose.basic-auth.yaml",
];

fn playwright_source() -> (tempfile::TempDir, RepositoryContext) {
    let (source, context) = native_source();
    put(
        source.path(),
        "profiles/wordpress/mcp.json",
        br#"{"servers":["playwright"]}"#,
    );
    put(
        source.path(),
        "mcp/servers/playwright.json",
        include_bytes!("../../../mcp/servers/playwright.json"),
    );
    let images = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mcp/images/playwright");
    for filename in PLAYWRIGHT_FILES {
        put(
            source.path(),
            &format!("mcp/images/playwright/{filename}"),
            &fs::read(images.join(filename)).unwrap(),
        );
    }
    (source, context)
}

fn disable_native_activation(project: &Path) {
    let canonical = project.canonicalize().unwrap();
    let marketplace = format!(
        "ai-profiles-{}",
        &sha(canonical.to_string_lossy().as_bytes())[..12]
    );
    put(
        project,
        ".claude/settings.json",
        &json_bytes(&json!({"enabledPlugins":{format!("ai-profile@{marketplace}"):false}}))
            .unwrap(),
    );
    put(
        project,
        ".codex/config.toml",
        format!("[plugins.\"ai-profile@{marketplace}\"]\nenabled = false\n").as_bytes(),
    );
}

#[test]
fn playwright_distributes_build_config_only_to_selected_profiles() {
    let (_source, context) = native_source();
    assert!(desired_files(&context, "wordpress")
        .unwrap()
        .iter()
        .all(|file| !file.relative_path.starts_with(".opencode/mcp/playwright/")));

    let (_source, context) = playwright_source();
    let project = tempfile::tempdir().unwrap();
    disable_native_activation(project.path());
    apply_profile(&context, project.path(), "wordpress").unwrap();
    let state = read_json(&project.path().join(".opencode/ai-profile-state.json")).unwrap();
    for filename in PLAYWRIGHT_FILES {
        let relative = format!(".opencode/mcp/playwright/{filename}");
        let contents = fs::read(project.path().join(&relative)).unwrap();
        assert_eq!(state["files"][&relative], sha(&contents));
    }
    assert_eq!(
        fs::read_dir(project.path().join(".opencode/mcp/playwright"))
            .unwrap()
            .count(),
        PLAYWRIGHT_FILES.len() + 1
    );
    assert!(!project.path().join("reports").exists());
    assert!(!project.path().join("node_modules").exists());
    assert!(!project
        .path()
        .join(".opencode/mcp/playwright/tests")
        .exists());
    let opencode = read_json(&project.path().join("opencode.json")).unwrap();
    assert_eq!(
        opencode["mcp"]["playwright"]["command"][0],
        project
            .path()
            .join(".opencode/ai-profile-mcp")
            .to_str()
            .unwrap()
    );
    assert_eq!(
        state["mcp_base"]["playwright"]["command"][0],
        ".opencode/mcp/playwright/playwright-launcher"
    );
    assert_eq!(
        read_json(&project.path().join(".mcp.json")).unwrap()["mcpServers"]["playwright"]["args"],
        json!(["playwright"])
    );
    let before = state.clone();
    apply_profile(&context, project.path(), "wordpress").unwrap();
    assert_eq!(
        read_json(&project.path().join(".opencode/ai-profile-state.json")).unwrap(),
        before
    );
}

#[test]
fn playwright_scoped_lifecycle_owns_launcher_but_not_ignore_rule_or_user_data() {
    use crate::model::{Client, ProfileAction as A};

    let (source, context) = playwright_source();
    let registry_path = source.path().join("mcp/servers/playwright.json");
    let mut registry = read_json(&registry_path).unwrap();
    registry["environment"]["COMPOSE_FILE"] =
        json!(".opencode/mcp/playwright/compose.yaml:.opencode/mcp/playwright/compose.host.yaml");
    put(
        source.path(),
        "mcp/servers/playwright.json",
        &json_bytes(&registry).unwrap(),
    );
    let project = tempfile::tempdir().unwrap();
    disable_native_activation(project.path());
    let original_ignore = b"# user bytes\r\n/private-data\r\n";
    put(project.path(), ".gitignore", original_ignore);
    put(project.path(), "base-auth.json", b"synthetic secret");
    for client in Client::ALL {
        scoped_action(
            &context,
            project.path(),
            A::Install,
            client,
            Some("wordpress"),
        );
    }

    let manifest_path = project.path().join(".ai-profiles/state.json");
    let installed = read_json(&manifest_path).unwrap();
    let mut records_before_update = serde_json::Map::new();
    for client in Client::ALL {
        let record = &installed["clients"][client.as_str()];
        let relative = format!(
            ".ai-profiles/{}/mcp/playwright/playwright-launcher",
            client.as_str()
        );
        let launcher = project.path().join(&relative);
        let bytes = fs::read(&launcher).unwrap();
        assert_eq!(record["files"][&relative], sha(&bytes));
        assert!(fs::metadata(&launcher).unwrap().permissions().mode() & 0o111 != 0);
        assert_eq!(record["mcp_base"]["playwright"]["command"][0], relative);
        let host_overlay = format!(
            ".ai-profiles/{}/mcp/playwright/compose.host.yaml",
            client.as_str()
        );
        let contents = fs::read(project.path().join(&host_overlay)).unwrap();
        assert_eq!(record["files"][&host_overlay], sha(&contents));
        assert_eq!(
            record["mcp_base"]["playwright"]["environment"]["COMPOSE_FILE"],
            format!(
                ".ai-profiles/{}/mcp/playwright/compose.yaml:{host_overlay}",
                client.as_str()
            )
        );
        assert!(!record["files"]
            .as_object()
            .unwrap()
            .contains_key(".gitignore"));
        records_before_update.insert(client.as_str().to_owned(), record.clone());
    }
    let ignore = fs::read(project.path().join(".gitignore")).unwrap();
    assert!(ignore.starts_with(original_ignore));
    assert_eq!(
        ignore
            .windows(b"/base-auth.json".len())
            .filter(|window| *window == b"/base-auth.json")
            .count(),
        1
    );

    scoped_action(&context, project.path(), A::Update, Client::OpenCode, None);
    let updated = read_json(&manifest_path).unwrap();
    for client in [Client::Claude, Client::Codex] {
        assert_eq!(
            updated["clients"][client.as_str()],
            records_before_update[client.as_str()]
        );
    }
    assert_eq!(
        fs::read(project.path().join(".gitignore")).unwrap(),
        ignore,
        "repeat update must not rewrite or duplicate the ignore rule"
    );

    put(
        project.path(),
        "reports/keep.json",
        b"report survives uninstall",
    );
    scoped_action(
        &context,
        project.path(),
        A::Uninstall,
        Client::OpenCode,
        None,
    );
    assert!(!project
        .path()
        .join(".ai-profiles/opencode/mcp/playwright/playwright-launcher")
        .exists());
    assert_eq!(fs::read(project.path().join(".gitignore")).unwrap(), ignore);
    assert_eq!(
        fs::read(project.path().join("reports/keep.json")).unwrap(),
        b"report survives uninstall"
    );
    assert_eq!(
        fs::read(project.path().join("base-auth.json")).unwrap(),
        b"synthetic secret"
    );
}

#[test]
fn playwright_ignore_preflight_preserves_unrelated_profile_and_rejects_bad_destination() {
    use crate::model::{Client, ClientSelection, ProfileAction as A};

    let (_source, context) = native_source();
    let unrelated = tempfile::tempdir().unwrap();
    let original = b"# unrelated profile\r\nkeep\r\n";
    put(unrelated.path(), ".gitignore", original);
    scoped_action(
        &context,
        unrelated.path(),
        A::Install,
        Client::OpenCode,
        Some("wordpress"),
    );
    assert_eq!(
        fs::read(unrelated.path().join(".gitignore")).unwrap(),
        original
    );

    let (_source, context) = playwright_source();
    let blocked = tempfile::tempdir().unwrap();
    disable_native_activation(blocked.path());
    fs::create_dir(blocked.path().join(".gitignore")).unwrap();
    assert!(crate::profile_management::scoped::run(
        &context,
        blocked.path(),
        A::Install,
        ClientSelection::One(Client::OpenCode),
        Some("wordpress"),
    )
    .is_err());
    assert!(!blocked
        .path()
        .join(".ai-profiles/opencode/mcp/playwright/Dockerfile")
        .exists());
    assert!(!blocked.path().join(".ai-profiles/state.json").exists());

    let linked = tempfile::tempdir().unwrap();
    disable_native_activation(linked.path());
    put(linked.path(), ".gitignore-target", b"user-owned\n");
    std::os::unix::fs::symlink(
        linked.path().join(".gitignore-target"),
        linked.path().join(".gitignore"),
    )
    .unwrap();
    assert!(crate::profile_management::scoped::run(
        &context,
        linked.path(),
        A::Install,
        ClientSelection::One(Client::OpenCode),
        Some("wordpress"),
    )
    .is_err());
    assert!(!linked
        .path()
        .join(".ai-profiles/opencode/mcp/playwright/Dockerfile")
        .exists());
    assert_eq!(
        fs::read(linked.path().join(".gitignore-target")).unwrap(),
        b"user-owned\n"
    );
}

#[test]
fn missing_playwright_build_sources_fail_before_project_writes() {
    for filename in PLAYWRIGHT_FILES {
        let (source, context) = playwright_source();
        fs::remove_file(
            source
                .path()
                .join(format!("mcp/images/playwright/{filename}")),
        )
        .unwrap();
        let project = tempfile::tempdir().unwrap();
        assert!(apply_profile(&context, project.path(), "wordpress").is_err());
        assert_eq!(fs::read_dir(project.path()).unwrap().count(), 0);
    }
}

#[test]
fn playwright_update_manages_launcher_and_preserves_project_data() {
    let (_source, context) = playwright_source();
    let project = tempfile::tempdir().unwrap();
    disable_native_activation(project.path());
    apply_profile(&context, project.path(), "wordpress").unwrap();
    let state_path = project.path().join(".opencode/ai-profile-state.json");
    let mut state = read_json(&state_path).unwrap();
    for filename in ["playwright-launcher", "image.json"] {
        let relative = format!(".opencode/mcp/playwright/{filename}");
        put(project.path(), &relative, b"old managed runtime");
        state["files"][&relative] = json!(sha(b"old managed runtime"));
    }
    state["mcp_base"]["playwright"]["command"] =
        json!([".opencode/mcp/playwright/playwright-launcher"]);
    put(
        project.path(),
        ".opencode/ai-profile-state.json",
        &json_bytes(&state).unwrap(),
    );
    put(project.path(), "reports/wcag/report.md", b"keep report");
    put(project.path(), "auth.json", b"keep authentication");

    apply_profile(&context, project.path(), "wordpress").unwrap();
    assert!(project
        .path()
        .join(".opencode/mcp/playwright/playwright-launcher")
        .is_file());
    assert!(!project
        .path()
        .join(".opencode/mcp/playwright/image.json")
        .exists());
    crate::profile_management::uninstall::uninstall(&context, project.path()).unwrap();
    assert!(!project
        .path()
        .join(".opencode/mcp/playwright/Dockerfile")
        .exists());
    assert_eq!(
        fs::read(project.path().join("reports/wcag/report.md")).unwrap(),
        b"keep report"
    );
    assert_eq!(
        fs::read(project.path().join("auth.json")).unwrap(),
        b"keep authentication"
    );
}

#[test]
fn modified_playwright_build_file_blocks_update_without_overwriting_it() {
    let (_source, context) = playwright_source();
    let project = tempfile::tempdir().unwrap();
    disable_native_activation(project.path());
    apply_profile(&context, project.path(), "wordpress").unwrap();
    let state_path = project.path().join(".opencode/ai-profile-state.json");
    let before = fs::read(&state_path).unwrap();
    put(
        project.path(),
        ".opencode/mcp/playwright/Dockerfile",
        b"user change",
    );
    assert!(apply_profile(&context, project.path(), "wordpress").is_err());
    assert_eq!(fs::read(&state_path).unwrap(), before);
    assert_eq!(
        fs::read(project.path().join(".opencode/mcp/playwright/Dockerfile")).unwrap(),
        b"user change"
    );
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
    let old_mcp = json!({"legacy":{"type":"local","command":["python3",legacy,"legacy"]}});
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
    let codex = fs::read_to_string(project.path().join(".codex/config.toml")).unwrap();
    assert!(codex.contains("mcp_servers"));
    assert!(codex.contains("legacy"));
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

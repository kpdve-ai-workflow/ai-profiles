//! Real installed entry points; expected protocol bytes and exit statuses are
//! supplied by the synthetic server, not copied from implementation output.
use crate::manager_fixture::{binaries, copy_executable};
use std::{
    fs,
    path::Path,
    process::{Command, Stdio},
};

fn records(project: &Path) -> Vec<Vec<serde_json::Value>> {
    let mut result = Vec::new();
    for entry in fs::read_dir(project.join(".ai-profiles-logs")).unwrap() {
        let path = entry.unwrap().path();
        if !path.is_dir() {
            continue;
        }
        let mut files: Vec<_> = fs::read_dir(path)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
            .collect();
        files.sort();
        result.push(
            files
                .into_iter()
                .flat_map(|p| {
                    fs::read_to_string(p)
                        .unwrap()
                        .lines()
                        .map(|s| serde_json::from_str(s).unwrap())
                        .collect::<Vec<_>>()
                })
                .collect(),
        );
    }
    result
}

fn adapter(project: &Path, client: &str) -> std::path::PathBuf {
    let directory = project.join(format!(".ai-profiles/{client}"));
    fs::create_dir_all(&directory).unwrap();
    let executable = directory.join("ai-profile-mcp");
    copy_executable(&binaries().join("ai-profile-mcp"), &executable);
    executable
}

#[test]
fn mcp_launch_records_consumed_configuration_and_prepared_environment() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let base = br#"{"format":2,"clients":{"opencode":{"mcp_base":{"probe":{"type":"local","command":["/bin/sh","-c","printf '%s' \"$VALUE:$AI_PROFILE_PROJECT_DIR\""]}}}}}"#;
    fs::create_dir_all(project.path().join(".ai-profiles")).unwrap();
    fs::write(project.path().join(crate::layout::STATE), base).unwrap();
    fs::create_dir_all(project.path().join(".opencode")).unwrap();
    let override_bytes = br#"{"mcp":{"probe":{"env_file":".env.probe","environment":{"VALUE":"{env:FROM_DOTENV}"}}}}"#;
    fs::write(project.path().join(".opencode/ai-profile.local.json"), override_bytes).unwrap();
    let dotenv_bytes = b"FROM_DOTENV='exact value'\n";
    fs::write(project.path().join(".env.probe"), dotenv_bytes).unwrap();
    let executable = adapter(project.path(), "opencode");
    let output = Command::new(executable)
        .arg("probe")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .current_dir(outside.path())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, format!("exact value:{}", project.path().canonicalize().unwrap().display()).as_bytes());
    assert!(output.stderr.is_empty());

    let sessions = records(project.path());
    assert_eq!(sessions.len(), 1);
    let events = &sessions[0];
    let operation_id = events.iter()
        .find(|record| record["fields"]["event"] == "mcp.prepare.started")
        .and_then(|record| record["fields"]["operation_id"].as_str())
        .expect("MCP preparation starts one correlated operation");
    let related: Vec<_> = events.iter().filter(|record| record["fields"]["operation_id"] == operation_id).collect();
    assert!(related.iter().any(|record| record["fields"]["event"] == "mcp.config.consumed"));
    assert!(related.iter().any(|record| record["fields"]["event"] == "mcp.environment.dotenv"));
    assert!(related.iter().any(|record| record["fields"]["event"] == "mcp.environment.prepared"));
    assert!(related.iter().any(|record| record["fields"]["event"] == "mcp.prepare.finished"));
    assert_eq!(related.last().unwrap()["fields"]["outcome"], "handoff");
    let payloads: Vec<_> = fs::read_dir(project.path().join(".ai-profiles-logs"))
        .unwrap().flat_map(|entry| fs::read_dir(entry.unwrap().path()).unwrap())
        .map(|entry| entry.unwrap().path()).filter(|path| path.extension().is_some_and(|ext| ext == "bin"))
        .map(|path| fs::read(path).unwrap()).collect();
    assert!(payloads.iter().any(|payload| payload == base));
    assert!(payloads.iter().any(|payload| payload == override_bytes));
    assert!(payloads.iter().any(|payload| payload == dotenv_bytes));
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn installed_parallel_clients_handoff_preserves_protocol_and_exit_from_foreign_cwd() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    // A worktree .git file must not be traversed or required for identity.
    fs::write(project.path().join(".git"), "gitdir: /not-used\n").unwrap();
    fs::write(project.path().join(".gitignore"), b"# user\r\ncustom").unwrap();
    let state = serde_json::json!({"format":2,"clients":{
        "opencode":{"mcp_base":{"probe":{"type":"local","command":["/bin/sh","-c","printf '{\"ok\":true}\\n'; exit 7"]}}},
        "claude":{"mcp_base":{"probe":{"type":"local","command":["/bin/sh","-c","printf '{\"ok\":true}\\n'; exit 7"]}}},
        "codex":{"mcp_base":{"probe":{"type":"local","command":["/bin/sh","-c","printf '{\"ok\":true}\\n'; exit 7"]}}}
    }});
    fs::create_dir_all(project.path().join(".ai-profiles")).unwrap();
    fs::write(
        project.path().join(crate::layout::STATE),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    let mut children = Vec::new();
    for client in ["opencode", "claude", "codex"] {
        let executable = adapter(project.path(), client);
        for _ in 0..4 {
            children.push(
                Command::new(&executable)
                    .arg("probe")
                    .env_clear()
                    .env("PATH", "/usr/bin:/bin")
                    .current_dir(outside.path())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap(),
            );
        }
    }
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert_eq!(output.stdout, b"{\"ok\":true}\n");
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let sessions = records(project.path());
    assert_eq!(sessions.len(), 12);
    let mut ids = std::collections::HashSet::new();
    for records in sessions {
        assert_eq!(records.len(), 2);
        assert_eq!(records[0]["fields"]["event"], "runtime.started");
        assert_eq!(records[1]["fields"]["event"], "runtime.handoff");
        assert!(ids.insert(records[0]["run_id"].as_str().unwrap().to_owned()));
    }
    assert_eq!(
        fs::read(project.path().join(".gitignore")).unwrap(),
        b"# user\r\ncustom\r\n/.ai-profiles-logs/\r\n"
    );
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn runtime_without_git_and_logger_failure_preserve_usage_and_do_not_fallback() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let executable = adapter(project.path(), "opencode");
    for blocked in [false, true] {
        if blocked {
            fs::remove_dir_all(project.path().join(".ai-profiles-logs")).unwrap();
            fs::write(project.path().join(".ai-profiles-logs"), "blocked").unwrap();
        }
        let output = Command::new(&executable)
            .env_clear()
            .env("HOME", outside.path())
            .current_dir(outside.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.ends_with("Usage: ai-profile-mcp SERVER\n"));
        assert_eq!(
            stderr.matches("WARNING: diagnostics unavailable:").count(),
            usize::from(blocked)
        );
        if !blocked {
            let logs = records(project.path());
            assert_eq!(logs[0].last().unwrap()["fields"]["exit_code"], 2);
        }
    }
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn remaining_entry_points_keep_outputs_and_installed_locations() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    for (binary, relative, expected_code, message) in [
        (
            "playwright-launcher",
            ".ai-profiles/codex/mcp/playwright/playwright-launcher",
            2,
            "playwright: launcher does not accept arguments\n",
        ),
        (
            "restore-updraft",
            ".ai-profiles/claude/scripts/restore-updraft",
            2,
            "BŁĄD: nieznana opcja: --unknown\n",
        ),
        (
            "test-inventory",
            ".ai-profiles/opencode/plugin/skills/test-gap-discovery/scripts/inventory-tests",
            2,
            "Expected directory: --unknown\n",
        ),
    ] {
        let executable = project.path().join(relative);
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        copy_executable(&binaries().join(binary), &executable);
        let output = Command::new(&executable)
            .arg("--unknown")
            .env_clear()
            .current_dir(outside.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(expected_code));
        assert!(output.stdout.is_empty());
        assert_eq!(output.stderr, message.as_bytes());
        let all = records(project.path());
        let logs = all.iter().find(|logs| logs[0]["binary"] == binary).unwrap();
        assert_eq!(logs[0]["fields"]["event"], "runtime.started");
        assert_eq!(logs.last().unwrap()["fields"]["exit_code"], expected_code);
    }
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn shared_configuration_limit_and_alias_stderr_do_not_pollute_output() {
    let project = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let executable = adapter(project.path(), "claude");
    let output_file = fs::File::create(outside.path().join("combined")).unwrap();
    let status = Command::new(&executable)
        .env_clear()
        .env("AI_PROFILE_LOG_SESSION_BYTES", "1")
        .current_dir(outside.path())
        .stdout(output_file.try_clone().unwrap())
        .stderr(output_file)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
    assert_eq!(
        fs::read(outside.path().join("combined")).unwrap(),
        b"Usage: ai-profile-mcp SERVER\n"
    );
    let directory = fs::read_dir(project.path().join(".ai-profiles-logs"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    assert!(directory.join("incomplete").is_file());
    for value in ["0", "-1", "no", "18446744073709551616"] {
        let output = Command::new(&executable)
            .env_clear()
            .env("AI_PROFILE_LOG_SEGMENT_BYTES", value)
            .current_dir(outside.path())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert_eq!(
            stderr.matches("WARNING: diagnostics unavailable:").count(),
            1
        );
        assert!(stderr.contains("invalid AI_PROFILE_LOG_SEGMENT_BYTES"));
        assert!(stderr.ends_with("Usage: ai-profile-mcp SERVER\n"));
    }
}

//! Oracles: manager CLI contract, help.txt, and unchanged CLI user messages.
//! Real child processes only: no global subscriber in this test process.
use crate::manager_fixture::Checkout;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    ffi::{OsStr, OsString},
    fs,
    os::unix::{ffi::OsStrExt, ffi::OsStringExt, process::CommandExt},
    path::Path,
    process::{Command, Output, Stdio},
};

struct Invocation {
    checkout: Checkout,
    outside: tempfile::TempDir,
}

impl Invocation {
    fn new() -> Self {
        let checkout = Checkout::new();
        let outside = tempfile::tempdir().unwrap();
        for name in ["home", "xdg", "tmp", "cwd", "project"] {
            fs::create_dir(outside.path().join(name)).unwrap();
        }
        Self { checkout, outside }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.checkout.executable);
        command
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", self.outside.path().join("home"))
            .env("XDG_STATE_HOME", self.outside.path().join("xdg"))
            .env("XDG_CACHE_HOME", self.outside.path().join("xdg"))
            .env("TMPDIR", self.outside.path().join("tmp"))
            .current_dir(self.outside.path().join("cwd"));
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }

    fn logs(&self) -> Vec<Vec<Value>> {
        let directory = self.checkout.root.path().join(".ai-profiles-logs");
        assert!(
            directory.is_dir(),
            "manager must create source-checkout logs"
        );
        fs::read_dir(directory)
            .unwrap()
            .filter(|entry| entry.as_ref().unwrap().file_type().unwrap().is_dir())
            .map(|entry| {
                let path = entry.unwrap().path();
                let mut segments: Vec<_> = fs::read_dir(path)
                    .unwrap()
                    .map(|e| e.unwrap().path())
                    .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
                    .collect();
                segments.sort();
                segments
                    .into_iter()
                    .flat_map(|p| {
                        fs::read_to_string(p)
                            .unwrap()
                            .lines()
                            .map(|line| serde_json::from_str(line).unwrap())
                            .collect::<Vec<_>>()
                    })
                    .collect()
            })
            .collect()
    }

    fn no_fallback(&self) {
        for name in ["home", "xdg", "tmp", "cwd", "project"] {
            assert_eq!(
                fs::read_dir(self.outside.path().join(name))
                    .unwrap()
                    .count(),
                0,
                "unexpected fallback in {name}"
            );
        }
    }
}

fn lifecycle(records: &[Value], code: i32, error: Option<(&str, &str)>) -> String {
    let context_end = context_structure(records);
    assert_eq!(
        records.len(),
        context_end + if error.is_some() { 3 } else { 2 }
    );
    assert_eq!(records[0]["fields"]["event"], "cli.started");
    let last = records.last().unwrap();
    assert_eq!(last["fields"]["event"], "cli.finished");
    assert_eq!(last["fields"]["exit_code"], code);
    let run_id = records[0]["run_id"].as_str().unwrap();
    assert!(!run_id.is_empty());
    for record in records {
        assert_eq!(record["run_id"], run_id);
        assert_eq!(record["binary"], "ai-profile");
    }
    if let Some((kind, message)) = error {
        assert_eq!(records[context_end + 1]["fields"]["event"], "cli.failed");
        assert_eq!(records[context_end + 1]["fields"]["error_kind"], kind);
        assert_eq!(records[context_end + 1]["fields"]["message"], message);
        assert_eq!(records[context_end + 1]["level"], "ERROR");
    }
    run_id.to_owned()
}

fn lifecycle_with_profile_events(
    records: &[Value],
    code: i32,
    error: Option<(&str, &str)>,
) -> String {
    let cli_records = records
        .iter()
        .filter(|record| {
            !record["fields"]["event"]
                .as_str()
                .is_some_and(|event| event.starts_with("profile.") || event.starts_with("payload."))
        })
        .cloned()
        .collect::<Vec<_>>();
    lifecycle(&cli_records, code, error)
}

fn assert_profile_operation(records: &[Value], action: &str, result: &str) -> String {
    let operation_records = records
        .iter()
        .filter(|record| {
            record["fields"]["event"]
                .as_str()
                .is_some_and(|event| event.starts_with("profile."))
        })
        .collect::<Vec<_>>();
    let starts = operation_records
        .iter()
        .filter(|record| record["fields"]["event"] == "profile.operation.started")
        .collect::<Vec<_>>();
    let terminals = operation_records
        .iter()
        .filter(|record| record["fields"]["event"] == "profile.operation.finished")
        .collect::<Vec<_>>();
    assert_eq!(
        starts.len(),
        1,
        "one profile operation start: {operation_records:?}"
    );
    assert_eq!(
        terminals.len(),
        1,
        "one profile operation terminal: {operation_records:?}"
    );
    assert_eq!(starts[0]["fields"]["action"], action);
    assert_eq!(terminals[0]["fields"]["action"], action);
    assert_eq!(terminals[0]["fields"]["result"], result);
    let operation_id = starts[0]["fields"]["operation_id"].as_str().unwrap();
    assert!(!operation_id.is_empty());
    for record in operation_records {
        assert_eq!(record["fields"]["operation_id"], operation_id);
    }
    operation_id.to_owned()
}

// The versioned framing and contiguous indices detect missing/duplicate records,
// even when the missing value was empty. Only these explicit additions are allowed.
fn context_structure(records: &[Value]) -> usize {
    let start = &records[1]["fields"];
    assert_eq!(start["event"], "cli.context.started");
    assert_eq!(start["schema_version"], 1);
    let argc = start["argc"].as_u64().unwrap() as usize;
    let envc = start["envc"].as_u64().unwrap() as usize;
    for (offset, (event, count)) in [("cli.context.arg", argc), ("cli.context.env", envc)]
        .into_iter()
        .enumerate()
    {
        let base = if offset == 0 { 2 } else { 2 + argc };
        for index in 0..count {
            let fields = &records[base + index]["fields"];
            assert_eq!(fields["event"], event);
            assert_eq!(fields["index"], index);
        }
    }
    assert_eq!(
        records[2 + argc + envc]["fields"]["event"],
        "cli.context.cwd"
    );
    assert_eq!(
        records[3 + argc + envc]["fields"]["event"],
        "cli.context.exe"
    );
    let end = 4 + argc + envc;
    assert_eq!(records[end]["fields"]["event"], "cli.context.finished");
    assert_eq!(records[end]["fields"]["argc"], argc);
    assert_eq!(records[end]["fields"]["envc"], envc);
    end
}

fn decode(fields: &Value, key: &str) -> Vec<u8> {
    let value = fields[key].as_str().unwrap();
    let bytes = match fields[format!("{key}_encoding")].as_str().unwrap() {
        "utf-8" => value.as_bytes().to_vec(),
        "hex" => {
            assert_eq!(value.len() % 2, 0);
            (0..value.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&value[i..i + 2], 16).unwrap())
                .collect()
        }
        other => panic!("unknown encoding: {other}"),
    };
    assert_eq!(fields[format!("{key}_bytes")], bytes.len());
    if let Ok(text) = std::str::from_utf8(&bytes) {
        assert_eq!(fields[format!("{key}_encoding")], "utf-8");
        assert_eq!(value, text, "ordinary UTF-8 must remain readable");
    }
    bytes
}

#[test]
fn manager_logging_context_roundtrips_exact_process_inputs_before_parsing() {
    for invalid in [false, true] {
        let fixture = Invocation::new();
        let cwd = fixture
            .outside
            .path()
            .join(OsString::from_vec(b"cwd-\xff-\n".to_vec()));
        fs::create_dir(&cwd).unwrap();
        let exe = fixture
            .checkout
            .executable
            .with_file_name(OsString::from_vec(b"manager-\xfe-\n".to_vec()));
        crate::manager_fixture::copy_executable(&fixture.checkout.executable, &exe);
        let argv0 = OsString::from_vec(b"custom argv0-\xfd".to_vec());
        let mut args: Vec<OsString> = vec![if invalid {
            OsString::from_vec(b"\xff".to_vec())
        } else {
            "--help".into()
        }];
        args.extend([
            OsString::new(),
            "a b".into(),
            "a".into(),
            "b".into(),
            "\n\"\\żółw🌍".into(),
            "TOKEN=synthetic-secret".into(),
            OsString::from_vec(b"bad\xfe\n".to_vec()),
            "x".repeat(20000).into(),
        ]);
        let env: Vec<(OsString, OsString)> = vec![
            ("EMPTY".into(), OsString::new()),
            ("UNICODE_ż🌍".into(), "\nżółw🌍\n\"\\=".into()),
            ("API_TOKEN".into(), "synthetic-secret-no-redaction".into()),
            (
                OsString::from_vec(b"NAME_\xff\n".to_vec()),
                OsString::from_vec(b"value_\xfe\n=".to_vec()),
            ),
            ("LONG".into(), "q".repeat(20000).into()),
        ];
        let output = Command::new(&exe)
            .arg0(&argv0)
            .args(&args)
            .env_clear()
            .envs(env.iter().cloned())
            .current_dir(&cwd)
            .output()
            .unwrap();
        let message = "argument must be valid UTF-8";
        if invalid {
            assert_eq!(output.status.code(), Some(2));
            assert!(output.stdout.is_empty());
            assert_eq!(output.stderr, format!("usage: ai-profile KOMENDA [NAZWA] [opcje] (pomoc: make help)\nai-profile: error: {message}\n").as_bytes());
        } else {
            assert_eq!(output.status.code(), Some(0));
            assert_eq!(output.stdout, include_bytes!("../help.txt"));
            assert!(output.stderr.is_empty());
        }
        let logs = fixture.logs();
        assert_eq!(logs.len(), 1);
        let records = &logs[0];
        lifecycle(
            records,
            if invalid { 2 } else { 0 },
            invalid.then_some(("arguments", message)),
        );
        let expected_args: Vec<_> = std::iter::once(&argv0)
            .chain(&args)
            .map(|v| v.as_bytes().to_vec())
            .collect();
        let actual_args: Vec<_> = records
            .iter()
            .filter(|r| r["fields"]["event"] == "cli.context.arg")
            .map(|r| decode(&r["fields"], "value"))
            .collect();
        assert_eq!(actual_args, expected_args);
        let actual_env: BTreeMap<_, _> = records
            .iter()
            .filter(|r| r["fields"]["event"] == "cli.context.env")
            .map(|r| (decode(&r["fields"], "name"), decode(&r["fields"], "value")))
            .collect();
        let expected_env: BTreeMap<_, _> = env
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect();
        assert_eq!(records[1]["fields"]["envc"], expected_env.len());
        assert_eq!(actual_env, expected_env);
        for (event, expected) in [
            ("cli.context.cwd", cwd.as_os_str()),
            ("cli.context.exe", exe.as_os_str()),
        ] {
            let fields = &records
                .iter()
                .find(|r| r["fields"]["event"] == event)
                .unwrap()["fields"];
            assert_eq!(fields["status"], "available");
            assert_eq!(decode(fields, "value"), expected.as_bytes());
        }
    }
}

#[test]
fn manager_process_uninstall_reports_removed_state_and_preserves_user_file() {
    for client in crate::model::Client::ALL {
        let checkout = Checkout::with_assets(&crate::manager_fixture::binaries());
        let outside = tempfile::tempdir().unwrap();
        for name in ["home", "xdg", "tmp", "cwd", "project"] {
            fs::create_dir(outside.path().join(name)).unwrap();
        }
        let project = outside.path().join("project");
        let user_file = project.join("context.txt");
        fs::write(&user_file, b"user-owned context").unwrap();
        let command = |action: &str| {
            let mut command = Command::new(&checkout.executable);
            command.arg(action);
            if action == "install" {
                command.arg("wordpress");
            }
            command
                .args([
                    "--client",
                    client.as_str(),
                    "--project",
                    project.to_str().unwrap(),
                ])
                .env_clear()
                .env("PATH", "/usr/bin:/bin")
                .env("HOME", outside.path().join("home"))
                .env("XDG_STATE_HOME", outside.path().join("xdg"))
                .env("XDG_CACHE_HOME", outside.path().join("xdg"))
                .env("TMPDIR", outside.path().join("tmp"))
                .current_dir(outside.path().join("cwd"));
            command
        };
        let installed = command("install").output().unwrap();
        assert_eq!(
            installed.status.code(),
            Some(0),
            "stderr: {:?}",
            installed.stderr
        );
        assert_eq!(installed.stderr, b"");
        let layout = crate::layout::Layout::Isolated(client);
        let managed = project.join(layout.skills());
        assert!(managed.is_dir());

        let removed = command("uninstall").output().unwrap();
        assert_eq!(
            removed.status.code(),
            Some(0),
            "stderr: {:?}",
            removed.stderr
        );
        assert!(String::from_utf8_lossy(&removed.stdout).contains("usunięty"));
        assert_eq!(removed.stderr, b"");
        assert_eq!(fs::read(user_file).unwrap(), b"user-owned context");
        let manifest: Value =
            serde_json::from_slice(&fs::read(project.join(".ai-profiles/state.json")).unwrap())
                .unwrap();
        assert!(manifest["clients"][client.as_str()].is_null());
        assert!(!managed.join("test-gap-discovery/SKILL.md").exists());

        let runs = fs::read_dir(checkout.root.path().join(".ai-profiles-logs"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
        assert_eq!(
            runs.len(),
            2,
            "install and uninstall each start one logged run"
        );
        let uninstall_records = runs
            .iter()
            .flat_map(|run| {
                fs::read_dir(run)
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .filter(|path| {
                        path.extension()
                            .is_some_and(|extension| extension == "jsonl")
                    })
                    .flat_map(|path| {
                        fs::read_to_string(path)
                            .unwrap()
                            .lines()
                            .map(|line| serde_json::from_str::<Value>(line).unwrap())
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|record| record["fields"]["action"] == "uninstall")
            .collect::<Vec<_>>();
        assert!(uninstall_records.iter().any(|record| {
            record["fields"]["event"] == "profile.change.applied"
                && record["fields"]["change"] == "removed"
        }));
        assert!(uninstall_records.iter().any(|record| {
            record["fields"]["event"] == "profile.manifest.saved"
                && record["fields"]["result"] == "saved"
        }));
        assert!(uninstall_records.iter().any(|record| {
            record["fields"]["event"] == "profile.operation.finished"
                && record["fields"]["result"] == "success"
        }));
    }
}

#[test]
fn manager_logging_unavailable_cwd_is_explicit_and_does_not_change_help() {
    let fixture = Invocation::new();
    let cwd = fixture.outside.path().join("deleted");
    fs::create_dir(&cwd).unwrap();
    let output = Command::new("/bin/sh")
        .args([
            OsStr::new("-c"),
            OsStr::new("cd \"$1\" && rmdir \"$1\" && exec \"$2\" --help"),
            OsStr::new("sh"),
            cwd.as_os_str(),
            fixture.checkout.executable.as_os_str(),
        ])
        .env_clear()
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, include_bytes!("../help.txt"));
    assert!(output.stderr.is_empty());
    let logs = fixture.logs();
    lifecycle(&logs[0], 0, None);
    let fields = &logs[0]
        .iter()
        .find(|r| r["fields"]["event"] == "cli.context.cwd")
        .unwrap()["fields"];
    assert_eq!(fields["status"], "unavailable");
    assert_eq!(fields["os_error"], 2);
    assert!(!fields["error"].as_str().unwrap().is_empty());
}

#[test]
fn manager_logging_success_and_operation_error_preserve_cli_and_source_location() {
    let fixture = Invocation::new();
    assert!(!fixture.checkout.root.path().join(".git").exists());
    let success = fixture.run(&["create", "sample"]);
    assert_eq!(success.status.code(), Some(0));
    assert_eq!(
        success.stdout,
        "OK: utworzono profil sample (1.0.0)\n".as_bytes()
    );
    assert!(success.stderr.is_empty());
    let first = lifecycle_with_profile_events(&fixture.logs()[0], 0, None);
    let failure = fixture.run(&["create", "sample"]);
    assert_eq!(failure.status.code(), Some(1));
    assert!(failure.stdout.is_empty());
    let stderr = String::from_utf8(failure.stderr).unwrap();
    let message = stderr
        .strip_prefix("ERROR: ")
        .unwrap()
        .strip_suffix('\n')
        .unwrap();
    assert_eq!(message, "Profile already exists: sample");
    let logs = fixture.logs();
    assert_eq!(logs.len(), 2);
    let failed = logs
        .iter()
        .find(|records| records[0]["run_id"] != first)
        .unwrap();
    lifecycle_with_profile_events(failed, 1, Some(("operation", message)));
    fixture.no_fallback();
}

#[test]
fn manager_profile_maintenance_records_source_changes_and_preserves_cli_results() {
    let fixture = Invocation::new();

    let create = fixture.run(&["create", "sample", "--description", "Example profile"]);
    assert_eq!(create.status.code(), Some(0));
    assert_eq!(create.stdout, b"OK: utworzono profil sample (1.0.0)\n");
    assert!(create.stderr.is_empty());
    let created_metadata = fs::read(
        fixture
            .checkout
            .root
            .path()
            .join("profiles/sample/profile.json"),
    )
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&created_metadata).unwrap(),
        serde_json::json!({
            "name": "sample",
            "version": "1.0.0",
            "description": "Example profile"
        })
    );

    let rename = fixture.run(&["rename", "sample", "--new-name", "renamed"]);
    assert_eq!(rename.status.code(), Some(0));
    assert_eq!(
        rename.stdout,
        "OK: zmieniono nazwę profilu sample -> renamed w repozytorium ai-profiles. Zainstalowane projekty nie zostały zmienione.\n".as_bytes()
    );
    assert!(rename.stderr.is_empty());
    assert!(!fixture
        .checkout
        .root
        .path()
        .join("profiles/sample")
        .exists());
    let renamed_metadata = fs::read(
        fixture
            .checkout
            .root
            .path()
            .join("profiles/renamed/profile.json"),
    )
    .unwrap();
    let mut expected_renamed: Value = serde_json::from_slice(&created_metadata).unwrap();
    expected_renamed["name"] = serde_json::json!("renamed");
    assert_eq!(
        serde_json::from_slice::<Value>(&renamed_metadata).unwrap(),
        expected_renamed
    );

    let bump = fixture.run(&["bump", "renamed", "--part", "minor"]);
    assert_eq!(bump.status.code(), Some(0));
    assert_eq!(bump.stdout, b"OK: renamed: 1.0.0 -> 1.1.0\n");
    assert!(bump.stderr.is_empty());
    let bumped_metadata: Value = serde_json::from_slice(
        &fs::read(
            fixture
                .checkout
                .root
                .path()
                .join("profiles/renamed/profile.json"),
        )
        .unwrap(),
    )
    .unwrap();
    expected_renamed["version"] = serde_json::json!("1.1.0");
    assert_eq!(bumped_metadata, expected_renamed);

    let duplicate = fixture.run(&["create", "renamed"]);
    assert_eq!(duplicate.status.code(), Some(1));
    assert!(duplicate.stdout.is_empty());
    assert_eq!(
        duplicate.stderr,
        b"ERROR: Profile already exists: renamed\n"
    );

    let logs = fixture.logs();
    assert_eq!(logs.len(), 4);
    let records_for = |action: &str, result: &str| {
        logs.iter()
            .find(|records| {
                records.iter().any(|record| {
                    record["fields"]["event"] == "profile.operation.started"
                        && record["fields"]["action"] == action
                }) && records.iter().any(|record| {
                    record["fields"]["event"] == "profile.operation.finished"
                        && record["fields"]["action"] == action
                        && record["fields"]["result"] == result
                })
            })
            .unwrap_or_else(|| panic!("missing {action} {result} operation in {logs:?}"))
    };

    for (action, result) in [
        ("create", "success"),
        ("rename", "success"),
        ("bump", "success"),
        ("create", "error"),
    ] {
        let records = records_for(action, result);
        lifecycle_with_profile_events(
            records,
            if result == "success" { 0 } else { 1 },
            (result == "error").then_some(("operation", "Profile already exists: renamed")),
        );
        assert_profile_operation(records, action, result);
        let source = fixture.checkout.root.path().display().to_string();
        assert!(records.iter().any(|record| {
            record["fields"]["event"] == "profile.operation.started"
                && record["fields"]["source_repository"] == source
                && record["fields"]["target"] == "source_repository"
        }));
    }

    let created = records_for("create", "success");
    assert!(created.iter().any(|record| {
        record["fields"]["event"] == "profile.profile.created"
            && record["fields"]["profile"] == "sample"
            && record["fields"]["version"] == "1.0.0"
            && record["fields"]["description"] == "Example profile"
    }));
    let renamed = records_for("rename", "success");
    assert!(renamed.iter().any(|record| {
        record["fields"]["event"] == "profile.profile.renamed"
            && record["fields"]["previous_name"] == "sample"
            && record["fields"]["profile"] == "renamed"
    }));
    let bumped = records_for("bump", "success");
    assert!(bumped.iter().any(|record| {
        record["fields"]["event"] == "profile.profile.version_bumped"
            && record["fields"]["profile"] == "renamed"
            && record["fields"]["previous_version"] == "1.0.0"
            && record["fields"]["version"] == "1.1.0"
    }));
    let failed_create = records_for("create", "error");
    assert!(failed_create.iter().any(|record| {
        record["fields"]["event"] == "profile.operation.finished"
            && record["fields"]["error"]
                .as_str()
                .unwrap()
                .contains("Profile already exists: renamed")
    }));
    let assert_metadata_payload = |records: &[Value], expected: &[u8]| {
        let event = records
            .iter()
            .find(|record| {
                record["fields"]["event"] == "profile.repository.payload"
                    && record["fields"]["role"] == "profile_metadata"
                    && record["fields"]["path"] == "profile.json"
            })
            .expect("metadata payload event");
        let payload_id = event["fields"]["payload_id"].as_str().unwrap();
        let (run_id, sequence) = payload_id.rsplit_once('-').unwrap();
        let payload_path = fixture
            .checkout
            .root
            .path()
            .join(".ai-profiles-logs")
            .join(format!("run-{run_id}"))
            .join(format!(
                "payload-{:020}-{:020}.bin",
                sequence.parse::<u64>().unwrap(),
                0
            ));
        assert_eq!(fs::read(payload_path).unwrap(), expected);
        assert_eq!(event["fields"]["bytes"], expected.len());
        assert_eq!(event["fields"]["complete"], true);
    };
    assert_metadata_payload(created, &created_metadata);
    assert_metadata_payload(renamed, &renamed_metadata);
    assert_metadata_payload(
        bumped,
        &fs::read(
            fixture
                .checkout
                .root
                .path()
                .join("profiles/renamed/profile.json"),
        )
        .unwrap(),
    );
    fixture.no_fallback();
}

#[test]
fn manager_legacy_migration_records_guard_and_successful_adoption() {
    let fixture = Invocation::new();
    let project = fixture.outside.path().join("project");
    let guarded = fixture
        .command()
        .args(["migrate-legacy", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert_eq!(guarded.status.code(), Some(1));
    assert!(guarded.stdout.is_empty());
    assert_eq!(guarded.stderr, b"ERROR: Migrating a legacy installation requires --accept-existing after reviewing the files\n");
    assert!(!project.join(".opencode/ai-profile-state.json").exists());
    let guard_logs = fixture.logs();
    let guard_records = guard_logs.first().expect("guard diagnostic session");
    lifecycle_with_profile_events(
        guard_records,
        1,
        Some((
            "operation",
            "Migrating a legacy installation requires --accept-existing after reviewing the files",
        )),
    );
    assert_profile_operation(guard_records, "migrate-legacy", "error");

    let assets = Checkout::with_assets(&crate::manager_fixture::binaries());
    // Replace the invocation checkout with a source tree that includes the historical
    // profile name accepted by migrate-legacy.
    let root = &assets.root;
    fs::create_dir_all(root.path().join("profiles/backend-go")).unwrap();
    fs::write(
        root.path().join("profiles/backend-go/mcp.json"),
        br#"{"servers":["postgres"]}"#,
    )
    .unwrap();
    let project = fixture.outside.path().join("project");
    fs::create_dir_all(project.join(".opencode/plugins")).unwrap();
    fs::create_dir_all(project.join(".opencode/skills/implementation-breakdown")).unwrap();
    fs::create_dir_all(project.join(".opencode/skills/refine-plan")).unwrap();
    fs::create_dir_all(project.join(".opencode/skills/go-backend")).unwrap();
    fs::write(project.join(".opencode/workflow-rules.md"), b"rules").unwrap();
    fs::write(project.join(".opencode/plugins/approval-gate.js"), b"gate").unwrap();
    fs::write(
        project.join(".opencode/skills/implementation-breakdown/SKILL.md"),
        b"one",
    )
    .unwrap();
    fs::write(
        project.join(".opencode/skills/refine-plan/SKILL.md"),
        b"two",
    )
    .unwrap();
    fs::write(
        project.join(".opencode/skills/go-backend/SKILL.md"),
        b"three",
    )
    .unwrap();
    fs::write(
        project.join(".opencode/ai-profile-state.json"),
        br#"{"profile":"backend-go","skill":"go-backend","mcp":"postgres"}"#,
    )
    .unwrap();
    fs::write(
        project.join("opencode.json"),
        br#"{"mcp":{"postgres":{"type":"local","command":["docker","run"]}}}"#,
    )
    .unwrap();
    fs::write(
        project.join(".opencode/workflow-approval.json"),
        b"approval",
    )
    .unwrap();

    let fixture = Invocation {
        checkout: assets,
        outside: fixture.outside,
    };
    let output = fixture
        .command()
        .args(["migrate-legacy", "--accept-existing", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "stderr: {:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let state: Value =
        serde_json::from_slice(&fs::read(project.join(".opencode/ai-profile-state.json")).unwrap())
            .unwrap();
    assert_eq!(state["format"], 1);
    assert_eq!(state["profile"], "backend-go");
    assert_eq!(state["mcp"]["postgres"]["command"][0], "docker");
    assert!(!project.join(".opencode/workflow-approval.json").exists());
    let logs = fixture.logs();
    let success = logs.first().expect("successful migration log");
    assert_profile_operation(success, "migrate-legacy", "success");
    assert!(success.iter().any(|record| {
        record["fields"]["event"] == "profile.migration.state_written"
            && record["fields"]["path"] == ".opencode/ai-profile-state.json"
    }));
    assert!(success.iter().any(|record| {
        record["fields"]["event"] == "profile.migration.gate_removed"
            && record["fields"]["result"] == "success"
    }));
    let run_id = success[0]["run_id"].as_str().unwrap();
    let assert_payload = |role: &str, path: &str, expected: &[u8]| {
        let event = success
            .iter()
            .find(|record| {
                record["fields"]["event"] == "profile.repository.payload"
                    && record["fields"]["role"] == role
                    && record["fields"]["path"] == path
            })
            .expect("migration payload event");
        let (_, sequence) = event["fields"]["payload_id"]
            .as_str()
            .unwrap()
            .rsplit_once('-')
            .unwrap();
        let payload = fixture
            .checkout
            .root
            .path()
            .join(".ai-profiles-logs")
            .join(format!("run-{run_id}"))
            .join(format!(
                "payload-{:020}-{:020}.bin",
                sequence.parse::<u64>().unwrap(),
                0
            ));
        assert_eq!(fs::read(payload).unwrap(), expected);
        assert_eq!(event["fields"]["complete"], true);
        assert_eq!(event["fields"]["bytes"], expected.len());
    };
    assert_payload(
        "legacy_state",
        ".opencode/ai-profile-state.json",
        br#"{"profile":"backend-go","skill":"go-backend","mcp":"postgres"}"#,
    );
    assert_payload(
        "legacy_managed_file",
        ".opencode/workflow-rules.md",
        b"rules",
    );
    for name in ["home", "xdg", "tmp", "cwd"] {
        assert_eq!(
            fs::read_dir(fixture.outside.path().join(name))
                .unwrap()
                .count(),
            0
        );
    }
}

#[test]
fn manager_logging_help_and_argument_errors_have_complete_lifecycle() {
    for args in [
        vec![],
        vec!["--help"],
        vec!["help"],
        vec!["--unknown"],
        vec!["install"],
        vec!["create"],
    ] {
        let fixture = Invocation::new();
        let output = fixture.run(&args);
        let error = matches!(args.first(), Some(&"--unknown" | &"install" | &"create"));
        let code = if error { 2 } else { 0 };
        assert_eq!(output.status.code(), Some(code));
        if error {
            assert!(output.stdout.is_empty());
            let message = match args[0] {
                "--unknown" => "unrecognized arguments: --unknown",
                "install" => "Wymagane --client opencode|claude|codex|all",
                "create" => "create wymaga nazwy profilu",
                _ => unreachable!(),
            };
            assert_eq!(output.stderr, format!("usage: ai-profile KOMENDA [NAZWA] [opcje] (pomoc: make help)\nai-profile: error: {message}\n").as_bytes());
            lifecycle(&fixture.logs()[0], code, Some(("arguments", message)));
        } else {
            assert_eq!(output.stdout, include_bytes!("../help.txt"));
            assert!(output.stderr.is_empty());
            lifecycle(&fixture.logs()[0], code, None);
        }
        fixture.no_fallback();
    }
}

#[test]
fn manager_logging_parallel_runs_have_distinct_files_and_ids() {
    let fixture = Invocation::new();
    let children: Vec<_> = (0..24)
        .map(|_| {
            fixture
                .command()
                .arg("--help")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect();
    for child in children {
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, include_bytes!("../help.txt"));
        assert!(output.stderr.is_empty());
    }
    let logs = fixture.logs();
    assert_eq!(logs.len(), 24);
    let ids: HashSet<_> = logs
        .iter()
        .map(|records| lifecycle(records, 0, None))
        .collect();
    assert_eq!(ids.len(), 24);
    fixture.no_fallback();
}

#[test]
fn manager_logging_project_does_not_select_log_directory() {
    for client in crate::model::Client::ALL {
        let fixture = Invocation::new();
        let project = fixture.outside.path().join("project");
        let output = fixture
            .command()
            .args(["status", "--client", client.as_str(), "--project"])
            .arg(&project)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            output.stdout,
            format!("{}: brak profilu\n", client.as_str()).as_bytes()
        );
        assert!(output.stderr.is_empty());
        assert_eq!(fixture.logs().len(), 1);
        lifecycle_with_profile_events(&fixture.logs()[0], 0, None);
        assert_profile_operation(&fixture.logs()[0], "status", "success");
        assert!(!project.join(".ai-profiles-logs").exists());
        fixture.no_fallback();
    }
}

#[test]
fn manager_status_without_installation_records_one_operation_and_each_client() {
    let fixture = Invocation::new();
    let project = fixture.outside.path().join("project");
    let output = fixture
        .command()
        .args(["status", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        b"opencode: brak profilu\nclaude: brak profilu\ncodex: brak profilu\n"
    );
    assert!(output.stderr.is_empty());
    assert!(!project.join(".ai-profiles").exists());

    let logs = fixture.logs();
    assert_eq!(logs.len(), 1);
    lifecycle_with_profile_events(&logs[0], 0, None);
    assert_profile_operation(&logs[0], "status", "success");
    let clients = logs[0]
        .iter()
        .filter(|record| record["fields"]["event"] == "profile.status.client.finished")
        .map(|record| record["fields"]["client"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(clients, ["opencode", "claude", "codex"]);
    assert!(logs[0].iter().any(|record| {
        record["fields"]["event"] == "profile.status.client.finished"
            && record["fields"]["client"] == "opencode"
            && record["fields"]["result"] == "not_installed"
    }));
    fixture.no_fallback();
}

#[test]
fn manager_validate_logs_scope_and_preserves_cli_results() {
    let checkout = Checkout::with_assets(&crate::manager_fixture::binaries());
    let full = Command::new(&checkout.executable)
        .arg("validate")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert_eq!(full.status.code(), Some(0), "stderr: {:?}", full.stderr);
    assert_eq!(
        full.stdout,
        "OK: wszystkie skille i definicje MCP są spójne\n".as_bytes()
    );
    assert!(full.stderr.is_empty());

    let scoped = Command::new(&checkout.executable)
        .args(["validate", "wordpress"])
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert_eq!(scoped.status.code(), Some(0), "stderr: {:?}", scoped.stderr);
    assert_eq!(
        scoped.stdout,
        "OK: profil wordpress, wybrane skille i definicje MCP są spójne\n".as_bytes()
    );
    assert!(scoped.stderr.is_empty());

    let logs = fs::read_dir(checkout.root.path().join(".ai-profiles-logs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    assert_eq!(logs.len(), 2);
    let mut saw_all_scope = false;
    let mut saw_profile_scope = false;
    for run in &logs {
        let records = read_records(run);
        lifecycle_with_profile_events(&records, 0, None);
        assert_profile_operation(&records, "validate", "success");
        let validation = records
            .iter()
            .find(|record| record["fields"]["event"] == "profile.validate.started")
            .unwrap();
        assert_eq!(
            validation["fields"]["source_repository"],
            checkout.root.path().to_str().unwrap()
        );
        assert!(validation["fields"]["project"].is_null());
        match validation["fields"]["scope"].as_str().unwrap() {
            "all_sources" => {
                saw_all_scope = true;
                for check in [
                    "skill_catalog",
                    "mcp_definition",
                    "profile_metadata",
                    "profile_skills",
                    "profile_mcp_definitions",
                ] {
                    assert!(
                        records.iter().any(|record| {
                            record["fields"]["event"] == "profile.validate.check.finished"
                                && record["fields"]["check"] == check
                                && record["fields"]["result"] == "success"
                        }),
                        "missing executed check {check}"
                    );
                }
            }
            "profile" => {
                saw_profile_scope = true;
                for check in ["skill_catalog", "mcp_registry"] {
                    assert!(records.iter().any(|record| {
                        record["fields"]["event"] == "profile.validate.check"
                            && record["fields"]["check"] == check
                            && record["fields"]["result"] == "skipped"
                            && record["fields"]["reason"] == "single_profile_scope"
                    }));
                }
            }
            other => panic!("unexpected validation scope {other}"),
        }
        assert!(records.iter().any(|record| {
            record["fields"]["event"] == "profile.validate.payload"
                && record["fields"]["role"] == "mcp_example"
                && record["fields"]["complete"] == true
        }));
        assert!(read_payloads(run)
            .iter()
            .any(|payload| payload.as_slice() == include_bytes!("../../mcp/example.json")));
    }
    assert!(saw_all_scope && saw_profile_scope);

    let before = fs::read(checkout.root.path().join("mcp/example.json")).unwrap();
    fs::write(
        checkout.root.path().join("mcp/example.json"),
        br#"{"mcp":{"novamira":{"type":"local","command":["incorrect"]}}}"#,
    )
    .unwrap();
    let mismatch = Command::new(&checkout.executable)
        .arg("validate")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .unwrap();
    assert_eq!(mismatch.status.code(), Some(1));
    assert!(mismatch.stdout.is_empty());
    assert_eq!(
        mismatch.stderr,
        b"ERROR: mcp/example.json does not match the MCP definitions selected by the profiles\n"
    );
    let runs = fs::read_dir(checkout.root.path().join(".ai-profiles-logs"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    assert_eq!(runs.len(), 3);
    let (mismatch_run, mismatch_records) = runs
        .iter()
        .find_map(|run| {
            let records = read_records(run);
            records
                .iter()
                .any(|record| record["fields"]["result"] == "error")
                .then_some((run, records))
        })
        .unwrap();
    lifecycle_with_profile_events(
        &mismatch_records,
        1,
        Some((
            "operation",
            "mcp/example.json does not match the MCP definitions selected by the profiles",
        )),
    );
    assert_profile_operation(&mismatch_records, "validate", "error");
    assert!(mismatch_records.iter().any(|record| {
        record["fields"]["event"] == "profile.validate.example_compared"
            && record["fields"]["result"] == "mismatch"
    }));
    assert!(read_payloads(mismatch_run).iter().any(|payload| {
        payload.as_slice() == br#"{"mcp":{"novamira":{"type":"local","command":["incorrect"]}}}"#
    }));
    assert_eq!(before, include_bytes!("../../mcp/example.json"));
}

fn read_records(run: &Path) -> Vec<Value> {
    let mut segments = fs::read_dir(run)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
        })
        .collect::<Vec<_>>();
    segments.sort();
    segments
        .into_iter()
        .flat_map(|path| {
            fs::read_to_string(path)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect::<Vec<Value>>()
        })
        .collect()
}

fn read_payloads(run: &Path) -> Vec<Vec<u8>> {
    fs::read_dir(run)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("payload-")
        })
        .map(|path| fs::read(path).unwrap())
        .collect()
}

#[test]
fn manager_status_result_survives_unavailable_diagnostics() {
    let fixture = Invocation::new();
    let blocked = fixture.checkout.root.path().join(".ai-profiles-logs");
    fs::write(&blocked, b"keep this file").unwrap();
    let project = fixture.outside.path().join("project");
    let output = fixture
        .command()
        .args(["status", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        b"opencode: brak profilu\nclaude: brak profilu\ncodex: brak profilu\n"
    );
    warning(&output.stderr);
    assert_eq!(fs::read(blocked).unwrap(), b"keep this file");
    assert!(!project.join(".ai-profiles").exists());
    fixture.no_fallback();
}

#[test]
fn manager_validate_result_survives_unavailable_diagnostics() {
    let checkout = Checkout::with_assets(&crate::manager_fixture::binaries());
    let outside = tempfile::tempdir().unwrap();
    for name in ["home", "xdg", "tmp", "cwd"] {
        fs::create_dir(outside.path().join(name)).unwrap();
    }
    let blocked = checkout.root.path().join(".ai-profiles-logs");
    fs::write(&blocked, b"keep this file").unwrap();
    let example_before = fs::read(checkout.root.path().join("mcp/example.json")).unwrap();
    let output = Command::new(&checkout.executable)
        .arg("validate")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", outside.path().join("home"))
        .env("XDG_STATE_HOME", outside.path().join("xdg"))
        .env("XDG_CACHE_HOME", outside.path().join("xdg"))
        .env("TMPDIR", outside.path().join("tmp"))
        .current_dir(outside.path().join("cwd"))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        output.stdout,
        "OK: wszystkie skille i definicje MCP są spójne\n".as_bytes()
    );
    warning(&output.stderr);
    assert_eq!(fs::read(blocked).unwrap(), b"keep this file");
    assert_eq!(
        fs::read(checkout.root.path().join("mcp/example.json")).unwrap(),
        example_before
    );
    for name in ["home", "xdg", "tmp", "cwd"] {
        assert_eq!(fs::read_dir(outside.path().join(name)).unwrap().count(), 0);
    }
}

fn warning(stderr: &[u8]) -> &str {
    let stderr = std::str::from_utf8(stderr).unwrap();
    assert_eq!(
        stderr.matches("WARNING: diagnostics unavailable:").count(),
        1
    );
    stderr.lines().next().unwrap()
}

#[test]
fn manager_logging_preparation_failure_preserves_results_without_fallback() {
    let fixture = Invocation::new();
    let blocked = fixture.checkout.root.path().join(".ai-profiles-logs");
    fs::write(&blocked, b"do not replace").unwrap();
    let success = fixture.run(&["create", "sample"]);
    assert_eq!(success.status.code(), Some(0));
    assert_eq!(success.stdout, b"OK: utworzono profil sample (1.0.0)\n");
    assert_eq!(
        std::str::from_utf8(&success.stderr)
            .unwrap()
            .lines()
            .count(),
        1
    );
    warning(&success.stderr);
    let failure = fixture.run(&["create", "sample"]);
    assert_eq!(failure.status.code(), Some(1));
    assert!(failure.stdout.is_empty());
    warning(&failure.stderr);
    assert_eq!(
        std::str::from_utf8(&failure.stderr).unwrap().lines().nth(1),
        Some("ERROR: Profile already exists: sample")
    );
    let help = fixture.run(&["--help"]);
    assert_eq!(help.status.code(), Some(0));
    assert_eq!(help.stdout, include_bytes!("../help.txt"));
    warning(&help.stderr);
    let arguments = fixture.run(&["--unknown"]);
    assert_eq!(arguments.status.code(), Some(2));
    assert!(arguments.stdout.is_empty());
    warning(&arguments.stderr);
    assert!(std::str::from_utf8(&arguments.stderr)
        .unwrap()
        .ends_with("ai-profile: error: unrecognized arguments: --unknown\n"));
    assert_eq!(fs::read(blocked).unwrap(), b"do not replace");
    fixture.no_fallback();
}

#[test]
fn manager_logging_missing_sources_does_not_discover_from_cwd() {
    let fixture = Invocation::new();
    let standalone = fixture.outside.path().join("standalone");
    crate::manager_fixture::copy_executable(&fixture.checkout.executable, &standalone);
    // A valid checkout as CWD must not become a competing source of location.
    let mut command = Command::new(&standalone);
    command
        .env_clear()
        .current_dir(fixture.checkout.root.path());
    let output = command.arg("--help").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, include_bytes!("../help.txt"));
    warning(&output.stderr);
    assert!(!fixture
        .checkout
        .root
        .path()
        .join(".ai-profiles-logs")
        .exists());
    assert!(!Path::new(&standalone)
        .parent()
        .unwrap()
        .join(".ai-profiles-logs")
        .exists());
    fixture.no_fallback();
}

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
            .map(|entry| {
                let path = entry.unwrap().path();
                assert_eq!(path.extension().unwrap(), "jsonl");
                fs::read_to_string(path)
                    .unwrap()
                    .lines()
                    .map(|line| serde_json::from_str(line).unwrap())
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
    let first = lifecycle(&fixture.logs()[0], 0, None);
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
    lifecycle(failed, 1, Some(("operation", message)));
    fixture.no_fallback();
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
    let fixture = Invocation::new();
    let project = fixture.outside.path().join("project");
    let output = fixture
        .command()
        .args(["status", "--client", "opencode", "--project"])
        .arg(&project)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"opencode: brak profilu\n");
    assert!(output.stderr.is_empty());
    assert_eq!(fixture.logs().len(), 1);
    lifecycle(&fixture.logs()[0], 0, None);
    assert!(!project.join(".ai-profiles-logs").exists());
    fixture.no_fallback();
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

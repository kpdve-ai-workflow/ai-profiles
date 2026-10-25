//! Oracles: manager CLI contract, help.txt, and unchanged CLI user messages.
//! Real child processes only: no global subscriber in this test process.
use crate::manager_fixture::Checkout;
use serde_json::Value;
use std::{
    collections::HashSet,
    fs,
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
    assert_eq!(records.len(), if error.is_some() { 3 } else { 2 });
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
        assert_eq!(records[1]["fields"]["event"], "cli.failed");
        assert_eq!(records[1]["fields"]["error_kind"], kind);
        assert_eq!(records[1]["fields"]["message"], message);
        assert_eq!(records[1]["level"], "ERROR");
    }
    run_id.to_owned()
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

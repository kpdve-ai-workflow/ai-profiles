use serde_json::json;

use super::{project_name, restore, shell_quote, RestoreMode};
use crate::{
    logging::{with_session, Config, Session},
    process::{CancellationToken, CommandOutput, CommandSpec, ProcessRunner},
    wordpress::UrlMappings,
};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    path::Path,
    process::{Command, ExitStatus, Stdio},
};

struct PlanRunner {
    project: String,
}

impl ProcessRunner for PlanRunner {
    fn run(&self, spec: CommandSpec) -> Result<CommandOutput, crate::error::ProcessError> {
        let args = spec
            .args
            .iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>();
        let output = if args.first().is_some_and(|arg| arg == "describe") {
            serde_json::json!({
                "name":"fixture-site", "approot":self.project,
                "status":"running", "type":"wordpress", "docroot":"web",
                "urls":["https://local.example"]
            })
            .to_string()
        } else if args.iter().any(|arg| arg == "ai-profile-local-prefix") {
            "wp_".to_owned()
        } else if args.iter().any(|arg| arg.contains("SELECT @@hostname")) {
            "db-host\tdb".to_owned()
        } else if args.iter().any(|arg| arg == "SHOW TABLES") {
            "wp_options\nwp_posts\nwp_users\nwp_usermeta".to_owned()
        } else if args.iter().any(|arg| arg.contains("option_name IN")) {
            "home\thttps://old.example\nsiteurl\thttps://old.example".to_owned()
        } else {
            panic!("unexpected controlled DDEV command: {args:?}");
        };
        Ok(CommandOutput {
            status: ExitStatus::from_raw(0),
            stdout: output.into_bytes(),
            stderr: Vec::new(),
        })
    }
}

fn plan_fixture(project: &Path) -> std::path::PathBuf {
    fs::create_dir_all(project.join(".ddev")).unwrap();
    fs::write(project.join(".ddev/config.yaml"), "name: fixture-site\n").unwrap();
    let scripts = project.join(".opencode/ai-profile/scripts");
    fs::create_dir_all(&scripts).unwrap();
    fs::write(scripts.join("local-guard.php"), "<?php\n").unwrap();
    fs::write(scripts.join("local-safety.php"), "<?php\n").unwrap();
    fs::create_dir_all(project.join("web")).unwrap();
    fs::write(project.join("web/wp-config.php"), "<?php\n").unwrap();
    fs::create_dir_all(project.join("web/wp-content")).unwrap();
    let dump = project.with_extension("sql");
    fs::write(
        &dump,
        b"CREATE TABLE `wp_options` (x int);\nCREATE TABLE `wp_posts` (x int);\nCREATE TABLE `wp_users` (x int);\nCREATE TABLE `wp_usermeta` (x int);\n",
    )
    .unwrap();
    dump
}

fn event_records(session: &Session, root: &Path) -> Vec<serde_json::Value> {
    let directory = root.join(format!("run-{}", session.run_id));
    fs::read_dir(directory)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "jsonl")
        })
        .flat_map(|path| {
            fs::read_to_string(path)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect::<Vec<serde_json::Value>>()
        })
        .collect()
}

fn file_snapshot(root: &Path) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<std::path::PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, files);
            } else {
                files.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut files = BTreeMap::new();
    visit(root, root, &mut files);
    files
}

struct InstalledFixture {
    root: tempfile::TempDir,
    project: std::path::PathBuf,
    executable: std::path::PathBuf,
    state: std::path::PathBuf,
    calls: std::path::PathBuf,
}

impl InstalledFixture {
    fn new() -> Self {
        use flate2::{write::GzEncoder, Compression};
        use std::io::Write as _;

        let root = tempfile::tempdir().unwrap();
        let project = root.path().join("project");
        fs::create_dir(&project).unwrap();
        let dump = plan_fixture(&project);
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&fs::read(dump).unwrap()).unwrap();
        let backup = root.path().join("fixture-backup.gz");
        fs::write(&backup, encoder.finish().unwrap()).unwrap();
        let executable = project.join(".opencode/ai-profile/scripts/restore-updraft");
        crate::manager_fixture::copy_executable(
            &crate::manager_fixture::binaries().join("restore-updraft"),
            &executable,
        );
        let fake_bin = root.path().join("fake-bin");
        fs::create_dir(&fake_bin).unwrap();
        let ddev = fake_bin.join("ddev");
        fs::write(
            &ddev,
            r##"#!/bin/sh
printf '%s\n' "$*" >> "$CALLS"
case "$*" in
  *describe*) printf '{"name":"fixture-site","approot":"%s","status":"running","type":"wordpress","docroot":"web","urls":["https://local.example"]}\n' "$PROJECT" ;;
  *ai-profile-local-prefix*) printf 'wp_\n' ;;
  *'SELECT @@hostname, DATABASE()'*) printf 'db-host\tdb\n' ;;
  *'SHOW TABLES'*) printf 'wp_options\nwp_posts\nwp_users\nwp_usermeta\n' ;;
  *UPDATE*) touch "$STATE/localized" ;;
  *option_name*) if test -f "$STATE/imported" && ! test -f "$STATE/localized"; then printf 'home\thttps://old.example\nsiteurl\thttps://old.example\n'; else printf 'home\thttps://local.example\nsiteurl\thttps://local.example\n'; fi ;;
  *export-db*) if test "$FAIL_EXPORT" = 1; then exit 9; fi; for arg in "$@"; do case "$arg" in --file=*) cp "$BACKUP" "${arg#--file=}";; esac; done ;;
  *import-db*) touch "$STATE/imported"; if test "$FAIL_IMPORT" = 1; then exit 9; fi ;;
  *search-replace*) case "$*" in *--dry-run*) printf '0\n' ;; *) printf '2\n' ;; esac ;;
  *'core is-installed'*) ;;
  *) echo "unexpected ddev command: $*" >&2; exit 88 ;;
esac
"##,
        )
        .unwrap();
        fs::set_permissions(&ddev, fs::Permissions::from_mode(0o755)).unwrap();
        let state = root.path().join("state");
        let calls = root.path().join("ddev.calls");
        Self {
            root,
            project,
            executable,
            state,
            calls,
        }
    }

    fn run(
        &self,
        apply: bool,
        answer: &str,
        fail_export: bool,
        fail_import: bool,
    ) -> std::process::Output {
        use std::io::Write as _;
        let dump = self.project.with_extension("sql");
        let mut command = Command::new(&self.executable);
        command
            .arg(&dump)
            .arg("--project")
            .arg(&self.project)
            .arg("--map")
            .arg("https://old.example=https://local.example");
        if apply {
            command.arg("--apply");
        }
        let mut child = command
            .env_clear()
            .env("HOME", self.root.path().join("home"))
            .env("XDG_STATE_HOME", &self.state)
            .env("PROJECT", &self.project)
            .env("STATE", &self.state)
            .env("BACKUP", self.root.path().join("fixture-backup.gz"))
            .env("CALLS", &self.calls)
            .env("FAIL_EXPORT", if fail_export { "1" } else { "0" })
            .env("FAIL_IMPORT", if fail_import { "1" } else { "0" })
            .env(
                "PATH",
                format!(
                    "{}:/usr/bin:/bin",
                    self.root.path().join("fake-bin").display()
                ),
            )
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(answer.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
}

fn installed_log_records(project: &Path) -> Vec<serde_json::Value> {
    let log_root = project.join(".ai-profiles-logs");
    fs::read_dir(log_root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .flat_map(|entry| {
            fs::read_dir(entry.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|file| {
                    file.path()
                        .extension()
                        .is_some_and(|extension| extension == "jsonl")
                })
                .flat_map(|file| {
                    fs::read_to_string(file.path())
                        .unwrap()
                        .lines()
                        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn project_snapshot(project: &Path) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    file_snapshot(project)
        .into_iter()
        .filter(|(path, _)| {
            path != Path::new(".gitignore") && !path.starts_with(".ai-profiles-logs")
        })
        .collect()
}

#[test]
fn installed_restore_plan_uses_real_cli_without_mutating_project() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    fs::create_dir(&project).unwrap();
    let dump = plan_fixture(&project);
    let executable = project.join(".opencode/ai-profile/scripts/restore-updraft");
    crate::manager_fixture::copy_executable(
        &crate::manager_fixture::binaries().join("restore-updraft"),
        &executable,
    );
    let fake_bin = temporary.path().join("bin");
    fs::create_dir(&fake_bin).unwrap();
    let ddev = fake_bin.join("ddev");
    fs::write(
        &ddev,
        r##"#!/bin/sh
case "$*" in
  *describe*) printf '{"name":"fixture-site","approot":"%s","status":"running","type":"wordpress","docroot":"web","urls":["https://local.example"]}\n' "$PROJECT" ;;
  *ai-profile-local-prefix*) printf 'wp_\n' ;;
  *'SELECT @@hostname, DATABASE()'*) printf 'db-host\tdb\n' ;;
  *'SHOW TABLES'*) printf 'wp_options\nwp_posts\nwp_users\nwp_usermeta\n' ;;
  *option_name*) printf 'home\thttps://local.example\nsiteurl\thttps://local.example\n' ;;
  *) echo "unexpected ddev command: $*" >&2; exit 88 ;;
esac
"##,
    )
    .unwrap();
    fs::set_permissions(&ddev, fs::Permissions::from_mode(0o755)).unwrap();
    let before = file_snapshot(&project)
        .into_iter()
        .filter(|(path, _)| {
            path != Path::new(".gitignore") && path != Path::new(".ai-profiles-logs")
        })
        .collect::<BTreeMap<_, _>>();

    let output = Command::new(&executable)
        .arg(&dump)
        .arg("--project")
        .arg(&project)
        .arg("--map")
        .arg("https://old.example=https://local.example")
        .env_clear()
        .env("HOME", temporary.path().join("home"))
        .env("PROJECT", &project)
        .env("PATH", format!("{}:/usr/bin:/bin", fake_bin.display()))
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .ends_with("PLAN: bez zapisu. Dodaj --apply, aby wykonać import z potwierdzeniem.\n"));
    assert!(output.stderr.is_empty());
    let after = file_snapshot(&project)
        .into_iter()
        .filter(|(path, _)| {
            path != Path::new(".gitignore") && !path.starts_with(".ai-profiles-logs")
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(after, before);
    assert!(!project.join("web/wp-content/mu-plugins").exists());
    let records = fs::read_dir(project.join(".ai-profiles-logs"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .flat_map(|entry| {
            fs::read_dir(entry.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|file| file.path().extension().is_some_and(|e| e == "jsonl"))
                .flat_map(|file| {
                    fs::read_to_string(file.path())
                        .unwrap()
                        .lines()
                        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let started = records
        .iter()
        .find(|record| record["fields"]["event"] == "restore.started")
        .unwrap();
    let terminal = records
        .iter()
        .find(|record| record["fields"]["event"] == "restore.finished")
        .unwrap();
    assert_eq!(started["fields"]["mode"], "plan");
    assert_eq!(terminal["fields"]["result"], "planned");
    assert_eq!(
        started["fields"]["operation_id"],
        terminal["fields"]["operation_id"]
    );
}

#[test]
fn apply_waits_for_exact_confirmation_before_creating_backup_or_mutating_project() {
    let fixture = InstalledFixture::new();
    let before = project_snapshot(&fixture.project);

    let output = fixture.run(true, "no\n", false, false);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        output.stderr,
        "BŁĄD: Cancelled; the database was not changed\n".as_bytes()
    );
    assert_eq!(project_snapshot(&fixture.project), before);
    let calls = fs::read_to_string(&fixture.calls).unwrap();
    assert!(!calls.contains("export-db"));
    assert!(!calls.contains("import-db"));
    assert!(!fixture
        .state
        .join("ai-profiles/wordpress-backups/fixture-site")
        .exists());
    let records = installed_log_records(&fixture.project);
    assert!(records.iter().any(|record| {
        record["fields"]["event"] == "restore.confirmation.rejected"
            && record["fields"]["result"] == "name_mismatch"
    }));
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(
                record["fields"]["event"].as_str(),
                Some("restore.finished" | "restore.failed")
            ))
            .count(),
        1
    );
}

#[test]
fn apply_logs_verified_backup_and_successful_localization() {
    let fixture = InstalledFixture::new();

    let output = fixture.run(true, "fixture-site\n", false, false);

    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}\ncalls: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
        fs::read_to_string(&fixture.calls).unwrap_or_default()
    );
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("OK: baza zaimportowana i URL sprawdzone."));
    assert!(output.stderr.is_empty());
    assert!(fixture
        .project
        .join("web/wp-content/mu-plugins/ai-profile-local-safety.php")
        .is_file());
    let calls = fs::read_to_string(&fixture.calls).unwrap();
    assert!(calls.contains("export-db"));
    assert!(calls.contains("import-db"));
    assert!(calls.contains("search-replace"));
    let backup_root = fixture
        .state
        .join("ai-profiles/wordpress-backups/fixture-site");
    let run = fs::read_dir(&backup_root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .unwrap();
    assert!(run.join("before-import.sql.gz").is_file());
    assert!(!run.join("incoming.sql").exists());
    let records = installed_log_records(&fixture.project);
    assert!(records
        .iter()
        .any(|record| { record["fields"]["event"] == "restore.backup.verified" }));
    assert!(records.iter().any(|record| {
        record["fields"]["event"] == "restore.localization.verified"
            && record["fields"]["result"] == "passed"
    }));
    let terminals: Vec<_> = records
        .iter()
        .filter(|record| record["fields"]["event"] == "restore.finished")
        .collect();
    assert_eq!(terminals.len(), 1);
    assert_eq!(terminals[0]["fields"]["result"], "applied");
}

#[test]
fn apply_failure_before_database_mutation_keeps_original_error_without_recovery_claim() {
    let fixture = InstalledFixture::new();

    let output = fixture.run(true, "fixture-site\n", true, false);

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("DDEV step failed: export-db"));
    let calls = fs::read_to_string(&fixture.calls).unwrap();
    assert!(calls.contains("export-db"));
    assert!(!calls.contains("import-db"));
    let records = installed_log_records(&fixture.project);
    assert!(!records
        .iter()
        .any(|record| { record["fields"]["event"] == "restore.recovery.required" }));
    assert!(records
        .iter()
        .any(|record| { record["fields"]["event"] == "restore.failed" }));
}

#[test]
fn apply_failure_after_import_preserves_backup_and_reports_recovery_cause() {
    let fixture = InstalledFixture::new();

    let output = fixture.run(true, "fixture-site\n", false, true);

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("Import przerwany. Brak automatycznego rollbacku."));
    assert!(stderr.contains("Import aborted. No automatic rollback was performed."));
    let backup_root = fixture
        .state
        .join("ai-profiles/wordpress-backups/fixture-site");
    let run = fs::read_dir(&backup_root)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.is_dir())
        .unwrap();
    assert!(run.join("before-import.sql.gz").is_file());
    assert!(!run.join("incoming.sql").exists());
    let records = installed_log_records(&fixture.project);
    let recovery = records
        .iter()
        .find(|record| record["fields"]["event"] == "restore.recovery.required")
        .unwrap();
    assert_eq!(recovery["fields"]["database_mutation_started"], true);
    assert_eq!(recovery["fields"]["automatic_rollback"], false);
    assert!(recovery["fields"]["cause"]
        .as_str()
        .unwrap()
        .contains("import-db"));
}

#[test]
fn restore_plan_and_apply_are_distinct_states() {
    assert_ne!(RestoreMode::Plan, RestoreMode::Apply);
}

#[test]
fn project_confirmation_and_recovery_command_are_safely_quoted() {
    assert_eq!(
        project_name(&json!({"name":"wordpress-local_2"})).unwrap(),
        "wordpress-local_2"
    );
    assert!(project_name(&json!({"name":"bad;command"})).is_err());
    assert_eq!(
        shell_quote("/tmp/project with ' quote"),
        "'/tmp/project with '\\'' quote'"
    );
}

#[test]
fn restore_plan_has_one_correlated_terminal_and_leaves_project_unchanged() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    fs::create_dir(&project).unwrap();
    let dump = plan_fixture(&project);
    let before = file_snapshot(&project);
    let dump_before = fs::read(&dump).unwrap();
    let runner = PlanRunner {
        project: project.display().to_string(),
    };
    let session_root = temporary.path().join("logs");
    let session = Session::open(&session_root, Config::default(), "restore-test").unwrap();

    with_session(&session, || {
        assert!(restore(
            &project,
            &dump,
            &UrlMappings::default(),
            RestoreMode::Plan,
            &runner,
            CancellationToken::new(),
        )
        .unwrap()
        .is_none());
    });

    let records = event_records(&session, &session_root);
    let starts: Vec<_> = records
        .iter()
        .filter(|record| record["fields"]["event"] == "restore.started")
        .collect();
    let terminals: Vec<_> = records
        .iter()
        .filter(|record| record["fields"]["event"] == "restore.finished")
        .collect();
    assert_eq!(starts.len(), 1, "one operation must start");
    assert_eq!(terminals.len(), 1, "one operation must terminate");
    assert_eq!(starts[0]["fields"]["mode"], "plan");
    assert_eq!(
        starts[0]["fields"]["operation_id"],
        terminals[0]["fields"]["operation_id"]
    );
    assert_eq!(terminals[0]["fields"]["result"], "planned");
    let source_chunk = records
        .iter()
        .find(|record| {
            record["fields"]["event"] == "restore.data.chunk"
                && record["fields"]["role"] == "source_dump"
        })
        .expect("the inspected source dump is retained as a raw payload");
    let payload_id = source_chunk["fields"]["payload_id"].as_str().unwrap();
    let sequence = payload_id
        .strip_prefix(&format!("{}-", session.run_id))
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let payload_file = session_root
        .join(format!("run-{}", session.run_id))
        .join(format!("payload-{sequence:020}-00000000000000000000.bin"));
    assert_eq!(fs::read(payload_file).unwrap(), fs::read(&dump).unwrap());
    assert_eq!(file_snapshot(&project), before);
    assert_eq!(fs::read(dump).unwrap(), dump_before);
}

#[test]
fn rejected_preflight_has_one_failure_terminal_with_the_original_error() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    fs::create_dir(&project).unwrap();
    let dump = temporary.path().join("dump.sql");
    fs::write(&dump, "synthetic input").unwrap();
    let runner = PlanRunner {
        project: project.display().to_string(),
    };
    let session_root = temporary.path().join("logs");
    let session = Session::open(&session_root, Config::default(), "restore-test").unwrap();

    let result = with_session(&session, || {
        restore(
            &project,
            &dump,
            &UrlMappings::default(),
            RestoreMode::Plan,
            &runner,
            CancellationToken::new(),
        )
    });
    assert!(matches!(
        result,
        Err(crate::error::WordPressError::DdevProjectRequired)
    ));

    let records = event_records(&session, &session_root);
    assert_eq!(
        records
            .iter()
            .filter(|record| record["fields"]["event"] == "restore.started")
            .count(),
        1
    );
    let failures: Vec<_> = records
        .iter()
        .filter(|record| record["fields"]["event"] == "restore.failed")
        .collect();
    assert_eq!(failures.len(), 1);
    assert_eq!(
        failures[0]["fields"]["error"],
        "An existing DDEV project is required"
    );
    assert_eq!(
        records
            .iter()
            .filter(|record| matches!(
                record["fields"]["event"].as_str(),
                Some("restore.finished" | "restore.failed")
            ))
            .count(),
        1
    );
}

#[test]
fn unavailable_diagnostic_writer_does_not_turn_plan_into_failure() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    fs::create_dir(&project).unwrap();
    let dump = plan_fixture(&project);
    let runner = PlanRunner {
        project: project.display().to_string(),
    };
    let session_root = temporary.path().join("logs");
    let session = Session::open(
        &session_root,
        Config {
            session_bytes: 1,
            ..Config::default()
        },
        "restore-test",
    )
    .unwrap();

    let result = with_session(&session, || {
        restore(
            &project,
            &dump,
            &UrlMappings::default(),
            RestoreMode::Plan,
            &runner,
            CancellationToken::new(),
        )
    });
    assert!(matches!(result, Ok(None)));
    assert!(!session_is_complete(&session));
}

fn session_is_complete(session: &Session) -> bool {
    session.complete()
}

use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs,
    os::unix::{fs::PermissionsExt, process::ExitStatusExt},
    path::{Path, PathBuf},
    process::ExitStatus,
    time::{Duration, Instant},
};

use crate::{
    error::ProcessError,
    model::Client,
    playwright::runtime::{prepare_launch, prepare_launch_with},
    process::{
        CancellationToken, CommandOutput, CommandSpec, ProcessRunner, StdinMode, StreamMode,
        SystemProcessRunner,
    },
};

fn installed_launcher(root: &Path, client: Client) -> PathBuf {
    let directory = root
        .join(".ai-profiles")
        .join(client.as_str())
        .join("mcp/playwright");
    fs::create_dir_all(&directory).unwrap();
    for name in [
        "compose.yaml",
        "compose.host.yaml",
        "compose.ca.yaml",
        "compose.auth.yaml",
        "compose.basic-auth.yaml",
    ] {
        fs::write(directory.join(name), "services: {}\n").unwrap();
    }
    let launcher = directory.join("playwright-launcher");
    fs::write(&launcher, b"test executable").unwrap();
    fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();
    launcher
}

fn env(entries: &[(&str, &OsStr)]) -> HashMap<OsString, OsString> {
    entries
        .iter()
        .map(|(key, value)| (OsString::from(*key), value.to_os_string()))
        .collect()
}

#[test]
fn launcher_prepares_reports_for_each_isolated_client_independent_of_cwd() {
    for client in Client::ALL {
        let project = tempfile::tempdir().unwrap();
        let launcher = installed_launcher(project.path(), client);
        let metadata = project.path().as_os_str();

        let plan = prepare_launch(&launcher, metadata, &HashMap::new()).unwrap();

        assert_eq!(plan.project, project.path());
        assert!(project.path().join("reports").is_dir());
        assert_eq!(
            plan.environment[OsStr::new("COMPOSE_FILE")],
            OsString::from(format!(
                "{}:{}",
                launcher
                    .parent()
                    .unwrap()
                    .join("compose.host.yaml")
                    .display(),
                launcher.parent().unwrap().join("compose.yaml").display()
            ))
        );
    }
}

#[test]
fn launcher_rejects_project_mismatch_before_creating_reports() {
    let project = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);

    assert!(prepare_launch(&launcher, other.path().as_os_str(), &HashMap::new()).is_err());
    assert!(!project.path().join("reports").exists());
}

#[test]
fn existing_reports_and_files_are_left_untouched() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let report = project.path().join("reports");
    fs::create_dir(&report).unwrap();
    fs::write(report.join("existing.json"), b"keep exactly\r\n").unwrap();

    prepare_launch(&launcher, project.path().as_os_str(), &HashMap::new()).unwrap();

    assert_eq!(
        fs::read(report.join("existing.json")).unwrap(),
        b"keep exactly\r\n"
    );
}

#[test]
fn default_basic_auth_is_optional_and_attached_only_for_a_valid_private_file() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let no_auth = prepare_launch(&launcher, project.path().as_os_str(), &HashMap::new()).unwrap();
    assert!(!no_auth
        .environment
        .contains_key(OsStr::new("AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE")));
    assert!(!no_auth.environment[OsStr::new("COMPOSE_FILE")]
        .to_string_lossy()
        .contains("compose.basic-auth.yaml"));

    let private = project.path().join(".private");
    fs::create_dir(&private).unwrap();
    let credentials = private.join("playwright-basic-auth.json");
    fs::write(
        &credentials,
        br#"{"username":"synthetic","password":"secret","origin":"http://127.0.0.1"}"#,
    )
    .unwrap();
    fs::set_permissions(&credentials, fs::Permissions::from_mode(0o600)).unwrap();

    let with_auth = prepare_launch(&launcher, project.path().as_os_str(), &HashMap::new()).unwrap();
    assert_eq!(
        with_auth.environment[OsStr::new("AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE")],
        credentials.into_os_string()
    );
    assert!(with_auth.environment[OsStr::new("COMPOSE_FILE")]
        .to_string_lossy()
        .contains("compose.basic-auth.yaml"));
}

#[test]
fn explicit_missing_or_unsafe_credentials_do_not_fall_back_to_default() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let private = project.path().join(".private");
    fs::create_dir(&private).unwrap();
    let default = private.join("playwright-basic-auth.json");
    fs::write(&default, b"synthetic").unwrap();
    fs::set_permissions(&default, fs::Permissions::from_mode(0o600)).unwrap();

    let explicit = env(&[(
        "AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE",
        OsStr::new(".private/missing.json"),
    )]);
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &explicit).is_err());

    fs::set_permissions(&default, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &HashMap::new()).is_err());
}

#[test]
fn explicit_credentials_override_default_but_symlinks_and_reports_paths_are_rejected() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let private = project.path().join(".private");
    fs::create_dir(&private).unwrap();
    let default = private.join("playwright-basic-auth.json");
    fs::write(&default, b"synthetic default").unwrap();
    fs::set_permissions(&default, fs::Permissions::from_mode(0o600)).unwrap();
    let explicit = private.join("override.json");
    fs::write(&explicit, b"synthetic explicit").unwrap();
    fs::set_permissions(&explicit, fs::Permissions::from_mode(0o600)).unwrap();
    let inherited = env(&[(
        "AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE",
        OsStr::new(".private/override.json"),
    )]);
    let plan = prepare_launch(&launcher, project.path().as_os_str(), &inherited).unwrap();
    assert_eq!(
        plan.environment[OsStr::new("AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE")],
        explicit.clone().into_os_string()
    );
    let absolute = env(&[(
        "AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE",
        explicit.as_os_str(),
    )]);
    let plan = prepare_launch(&launcher, project.path().as_os_str(), &absolute).unwrap();
    assert_eq!(
        plan.environment[OsStr::new("AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE")],
        explicit.clone().into_os_string()
    );

    let secret_in_reports = project.path().join("reports/private.json");
    fs::write(&secret_in_reports, b"synthetic").unwrap();
    fs::set_permissions(&secret_in_reports, fs::Permissions::from_mode(0o600)).unwrap();
    let inherited = env(&[(
        "AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE",
        OsStr::new("reports/private.json"),
    )]);
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &inherited).is_err());

    let external = project.path().join("outside.json");
    fs::write(&external, b"synthetic").unwrap();
    fs::set_permissions(&external, fs::Permissions::from_mode(0o600)).unwrap();
    fs::remove_file(private.join("playwright-basic-auth.json")).unwrap();
    std::os::unix::fs::symlink(&external, private.join("playwright-basic-auth.json")).unwrap();
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &HashMap::new()).is_err());
}

#[test]
fn symlinked_private_directory_and_unreadable_credentials_are_rejected() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let external = project.path().join("external-private");
    fs::create_dir(&external).unwrap();
    let credentials = external.join("playwright-basic-auth.json");
    fs::write(&credentials, b"synthetic").unwrap();
    fs::set_permissions(&credentials, fs::Permissions::from_mode(0o600)).unwrap();
    std::os::unix::fs::symlink(&external, project.path().join(".private")).unwrap();
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &HashMap::new()).is_err());

    fs::remove_file(project.path().join(".private")).unwrap();
    fs::remove_file(credentials).unwrap();
    fs::remove_dir(&external).unwrap();
    let private = project.path().join(".private");
    fs::create_dir(&private).unwrap();
    fs::write(private.join("playwright-basic-auth.json"), b"synthetic").unwrap();
    fs::set_permissions(
        private.join("playwright-basic-auth.json"),
        fs::Permissions::from_mode(0o200),
    )
    .unwrap();
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &HashMap::new()).is_err());
}

#[test]
fn compose_overlays_precede_and_do_not_reorder_or_duplicate_user_files() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let private = project.path().join(".private");
    fs::create_dir(&private).unwrap();
    let credentials = private.join("playwright-basic-auth.json");
    fs::write(&credentials, b"synthetic").unwrap();
    fs::set_permissions(&credentials, fs::Permissions::from_mode(0o600)).unwrap();
    let state = private.join("storage.json");
    fs::write(&state, b"synthetic storage").unwrap();
    fs::set_permissions(&state, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(project.path().join("public-ca.pem"), b"synthetic public CA").unwrap();
    let original = OsStr::new(".ai-profiles/opencode/mcp/playwright/compose.yaml:custom.yaml:.ai-profiles/opencode/mcp/playwright/compose.basic-auth.yaml");
    let inherited = env(&[
        ("COMPOSE_FILE", original),
        ("AI_PROFILE_PLAYWRIGHT_CA_CERT", OsStr::new("public-ca.pem")),
        (
            "AI_PROFILE_PLAYWRIGHT_STORAGE_STATE",
            OsStr::new(".private/storage.json"),
        ),
    ]);

    let plan = prepare_launch(&launcher, project.path().as_os_str(), &inherited).unwrap();
    let compose = plan.environment[OsStr::new("COMPOSE_FILE")].to_string_lossy();
    let auth_overlay = launcher
        .parent()
        .unwrap()
        .join("compose.auth.yaml")
        .to_string_lossy()
        .into_owned();
    let host_overlay = launcher.parent().unwrap().join("compose.host.yaml");
    let ca_overlay = launcher.parent().unwrap().join("compose.ca.yaml");
    assert!(compose.starts_with(&format!(
        "{}:{}:{auth_overlay}:",
        host_overlay.display(),
        ca_overlay.display()
    )));
    assert!(compose.ends_with(original.to_str().unwrap()));
    assert_eq!(compose.matches("compose.basic-auth.yaml").count(), 1);
}

#[test]
fn host_overlay_tracks_network_mode_and_preserves_explicit_compose_precedence() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    for (network, expected) in [
        ("", true),
        ("host", true),
        ("bridge", false),
        ("none", false),
    ] {
        let inherited = env(&[("AI_PROFILE_PLAYWRIGHT_NETWORK", OsStr::new(network))]);
        let plan = prepare_launch(&launcher, project.path().as_os_str(), &inherited).unwrap();
        assert_eq!(
            plan.environment[OsStr::new("COMPOSE_FILE")]
                .to_string_lossy()
                .contains("compose.host.yaml"),
            expected,
            "network={network}"
        );
    }
    let original = OsStr::new(
        "custom.yaml:.ai-profiles/opencode/mcp/playwright/./compose.host.yaml:final.yaml",
    );
    let inherited = env(&[("COMPOSE_FILE", original)]);
    let plan = prepare_launch(&launcher, project.path().as_os_str(), &inherited).unwrap();
    assert_eq!(plan.environment[OsStr::new("COMPOSE_FILE")], original);
}

#[test]
fn reports_file_or_symlink_is_rejected_before_compose() {
    for as_symlink in [false, true] {
        let project = tempfile::tempdir().unwrap();
        let launcher = installed_launcher(project.path(), Client::OpenCode);
        let report = project.path().join("reports");
        if as_symlink {
            let target = project.path().join("report-target");
            fs::create_dir(&target).unwrap();
            std::os::unix::fs::symlink(&target, &report).unwrap();
        } else {
            fs::write(&report, b"not a directory").unwrap();
        }
        assert!(prepare_launch(&launcher, project.path().as_os_str(), &HashMap::new()).is_err());
        assert!(!report.is_dir() || as_symlink);
    }
}

#[test]
fn caroot_selects_only_the_public_file_and_attaches_ca_before_auth() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let caroot = tempfile::Builder::new()
        .prefix("CA with spaces ")
        .tempdir()
        .unwrap();
    let certificate = caroot.path().join("rootCA.pem");
    fs::write(
        &certificate,
        "synthetic public certificate; parsed inside container",
    )
    .unwrap();
    fs::write(caroot.path().join("rootCA-key.pem"), "DO_NOT_READ_KEY").unwrap();
    fs::set_permissions(
        caroot.path().join("rootCA-key.pem"),
        fs::Permissions::from_mode(0o0),
    )
    .unwrap();
    let inherited = env(&[("CAROOT", caroot.path().as_os_str())]);
    let plan = prepare_launch(&launcher, project.path().as_os_str(), &inherited).unwrap();
    assert_eq!(
        plan.environment[OsStr::new("AI_PROFILE_PLAYWRIGHT_CA_CERT")],
        certificate.as_os_str()
    );
    let directory = launcher.parent().unwrap();
    assert_eq!(
        plan.environment[OsStr::new("COMPOSE_FILE")],
        OsString::from(format!(
            "{}:{}:{}",
            directory.join("compose.host.yaml").display(),
            directory.join("compose.ca.yaml").display(),
            directory.join("compose.yaml").display()
        ))
    );
    assert!(!plan
        .environment
        .values()
        .any(|value| value.to_string_lossy().contains("rootCA-key.pem")));
}

#[test]
fn explicit_ca_wins_and_invalid_explicit_path_does_not_fall_back() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    fs::write(
        project.path().join("custom CA.pem"),
        "synthetic public cert",
    )
    .unwrap();
    let inherited = env(&[
        ("AI_PROFILE_PLAYWRIGHT_CA_CERT", OsStr::new("custom CA.pem")),
        ("CAROOT", OsStr::new("missing")),
    ]);
    let plan = prepare_launch(&launcher, project.path().as_os_str(), &inherited).unwrap();
    assert_eq!(
        plan.environment[OsStr::new("AI_PROFILE_PLAYWRIGHT_CA_CERT")],
        project.path().join("custom CA.pem").as_os_str()
    );
    let inherited = env(&[("AI_PROFILE_PLAYWRIGHT_CA_CERT", OsStr::new("missing.pem"))]);
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &inherited).is_err());
}

#[test]
fn missing_automatic_ca_keeps_public_use_available_and_manual_overlay_is_not_duplicated() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let caroot = tempfile::tempdir().unwrap();
    let inherited = env(&[("CAROOT", caroot.path().as_os_str())]);
    let plan = prepare_launch(&launcher, project.path().as_os_str(), &inherited).unwrap();
    assert!(!plan
        .environment
        .contains_key(OsStr::new("AI_PROFILE_PLAYWRIGHT_CA_CERT")));
    assert!(!plan.environment[OsStr::new("COMPOSE_FILE")]
        .to_string_lossy()
        .contains("compose.ca.yaml"));
    fs::write(caroot.path().join("rootCA.pem"), "synthetic public cert").unwrap();
    let original = OsStr::new(".ai-profiles/opencode/mcp/playwright/compose.yaml:custom.yaml:.ai-profiles/opencode/mcp/playwright/./compose.ca.yaml");
    let inherited = env(&[
        ("CAROOT", caroot.path().as_os_str()),
        ("COMPOSE_FILE", original),
    ]);
    let plan = prepare_launch(&launcher, project.path().as_os_str(), &inherited).unwrap();
    let compose = plan.environment[OsStr::new("COMPOSE_FILE")].to_string_lossy();
    assert_eq!(compose.matches("compose.ca.yaml").count(), 1);
    assert!(compose.ends_with(original.to_str().unwrap()));
    assert!(plan
        .environment
        .contains_key(OsStr::new("AI_PROFILE_PLAYWRIGHT_CA_CERT")));
}

enum DiscoveryReply {
    Output(Vec<u8>, i32),
    Timeout,
    MissingProgram,
}

struct DiscoveryRunner {
    calls: RefCell<Vec<CommandSpec>>,
    reply: DiscoveryReply,
}

impl DiscoveryRunner {
    fn new(reply: DiscoveryReply) -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            reply,
        }
    }
}

impl ProcessRunner for DiscoveryRunner {
    fn run(&self, spec: CommandSpec) -> Result<CommandOutput, ProcessError> {
        self.calls.borrow_mut().push(spec);
        match &self.reply {
            DiscoveryReply::Output(stdout, code) => Ok(CommandOutput {
                status: ExitStatus::from_raw(code << 8),
                stdout: stdout.clone(),
                stderr: Vec::new(),
            }),
            DiscoveryReply::Timeout => Err(ProcessError::Timeout),
            DiscoveryReply::MissingProgram => Err(ProcessError::Spawn {
                source: std::io::Error::from(std::io::ErrorKind::NotFound),
            }),
        }
    }
}

fn discovery_program(root: &Path) -> PathBuf {
    let bin = root.join("bin with spaces");
    fs::create_dir(&bin).unwrap();
    fs::write(bin.join("mkcert"), "not executed by the injected runner").unwrap();
    fs::set_permissions(bin.join("mkcert"), fs::Permissions::from_mode(0o755)).unwrap();
    bin
}

#[test]
fn mkcert_query_is_read_only_bounded_and_does_not_receive_mcp_secrets() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let bin = discovery_program(project.path());
    let caroot = tempfile::Builder::new()
        .prefix("mkcert CA with spaces ")
        .tempdir()
        .unwrap();
    let certificate = caroot.path().join("rootCA.pem");
    fs::write(&certificate, "synthetic public cert").unwrap();
    let runner = DiscoveryRunner::new(DiscoveryReply::Output(
        format!("{}\n", caroot.path().display()).into_bytes(),
        0,
    ));
    let inherited = env(&[
        ("PATH", bin.as_os_str()),
        ("HOME", project.path().as_os_str()),
        ("XDG_DATA_HOME", project.path().as_os_str()),
        ("SECRET_TOKEN", OsStr::new("do-not-pass")),
    ]);
    let plan =
        prepare_launch_with(&launcher, project.path().as_os_str(), &inherited, &runner).unwrap();
    assert_eq!(
        plan.environment[OsStr::new("AI_PROFILE_PLAYWRIGHT_CA_CERT")],
        certificate.as_os_str()
    );
    let calls = runner.calls.borrow();
    assert_eq!(calls.len(), 1);
    let spec = &calls[0];
    assert_eq!(spec.program, bin.join("mkcert"));
    assert_eq!(spec.args, [OsString::from("-CAROOT")]);
    assert_eq!(spec.cwd.as_deref(), Some(project.path()));
    assert_eq!(spec.timeout, Some(Duration::from_secs(5)));
    assert!(spec.clear_env);
    assert!(matches!(spec.stdin, StdinMode::Null));
    assert!(matches!(spec.stdout, StreamMode::CaptureLimited(8192)));
    assert!(matches!(spec.stderr, StreamMode::Null));
    assert_eq!(spec.env.len(), 3);
    assert!(!spec.env.contains_key(OsStr::new("SECRET_TOKEN")));
}

#[test]
fn mkcert_failure_timeout_bad_output_and_absence_do_not_block_public_use() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let bin = discovery_program(project.path());
    let inherited = env(&[("PATH", bin.as_os_str())]);
    let replies = [
        DiscoveryReply::Timeout,
        DiscoveryReply::MissingProgram,
        DiscoveryReply::Output(b"/unavailable".to_vec(), 1),
        DiscoveryReply::Output(Vec::new(), 0),
        DiscoveryReply::Output(b"relative/path\n".to_vec(), 0),
        DiscoveryReply::Output(b"/first\n/second\n".to_vec(), 0),
        DiscoveryReply::Output(b"/bad\0path\n".to_vec(), 0),
        DiscoveryReply::Output(vec![0xff], 0),
        DiscoveryReply::Output(vec![b'/'; 8193], 0),
        DiscoveryReply::Output(b"/missing-ca-directory-t04\n".to_vec(), 0),
    ];
    for reply in replies {
        let runner = DiscoveryRunner::new(reply);
        let plan = prepare_launch_with(&launcher, project.path().as_os_str(), &inherited, &runner)
            .unwrap();
        assert!(!plan
            .environment
            .contains_key(OsStr::new("AI_PROFILE_PLAYWRIGHT_CA_CERT")));
        assert_eq!(runner.calls.borrow().len(), 1);
    }
    let runner = DiscoveryRunner::new(DiscoveryReply::MissingProgram);
    let inherited = env(&[("PATH", project.path().join("missing-bin").as_os_str())]);
    prepare_launch_with(&launcher, project.path().as_os_str(), &inherited, &runner).unwrap();
    assert!(runner.calls.borrow().is_empty());
}

#[test]
fn explicit_ca_and_set_caroot_suppress_mkcert_even_when_auto_file_is_missing() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let bin = discovery_program(project.path());
    let caroot = tempfile::tempdir().unwrap();
    let runner = DiscoveryRunner::new(DiscoveryReply::Timeout);
    let inherited = env(&[
        ("PATH", bin.as_os_str()),
        ("CAROOT", caroot.path().as_os_str()),
    ]);
    prepare_launch_with(&launcher, project.path().as_os_str(), &inherited, &runner).unwrap();
    assert!(runner.calls.borrow().is_empty());
    let certificate = project.path().join("explicit.pem");
    fs::write(&certificate, "synthetic public cert").unwrap();
    let mut inherited = inherited;
    inherited.insert(
        OsString::from("AI_PROFILE_PLAYWRIGHT_CA_CERT"),
        certificate.clone().into_os_string(),
    );
    let plan =
        prepare_launch_with(&launcher, project.path().as_os_str(), &inherited, &runner).unwrap();
    assert_eq!(
        plan.environment[OsStr::new("AI_PROFILE_PLAYWRIGHT_CA_CERT")],
        certificate.as_os_str()
    );
    assert!(runner.calls.borrow().is_empty());
}

#[test]
fn invalid_ca_file_types_and_known_private_key_path_are_rejected() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let caroot = tempfile::tempdir().unwrap();
    let key = caroot.path().join("rootCA-key.pem");
    fs::write(&key, "PRIVATE_KEY_SENTINEL").unwrap();
    let inherited = env(&[("AI_PROFILE_PLAYWRIGHT_CA_CERT", key.as_os_str())]);
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &inherited).is_err());
    let certificate = caroot.path().join("rootCA.pem");
    std::os::unix::fs::symlink(&key, &certificate).unwrap();
    let inherited = env(&[("CAROOT", caroot.path().as_os_str())]);
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &inherited).is_err());
    fs::remove_file(&certificate).unwrap();
    fs::create_dir(&certificate).unwrap();
    assert!(prepare_launch(&launcher, project.path().as_os_str(), &inherited).is_err());
}

#[test]
fn explicit_public_ca_symlink_keeps_existing_compose_file_compatibility() {
    let project = tempfile::tempdir().unwrap();
    let launcher = installed_launcher(project.path(), Client::OpenCode);
    let external = tempfile::tempdir().unwrap();
    let certificate = external.path().join("public.pem");
    fs::write(&certificate, "synthetic public certificate").unwrap();
    std::os::unix::fs::symlink(&certificate, project.path().join("ca.pem")).unwrap();
    let inherited = env(&[("AI_PROFILE_PLAYWRIGHT_CA_CERT", OsStr::new("ca.pem"))]);
    let plan = prepare_launch(&launcher, project.path().as_os_str(), &inherited).unwrap();
    assert_eq!(
        plan.environment[OsStr::new("AI_PROFILE_PLAYWRIGHT_CA_CERT")],
        certificate.as_os_str()
    );
}

#[test]
fn real_process_capture_is_bounded_and_timeout_is_enforced() {
    let spec = || CommandSpec {
        program: OsString::from("/bin/sh"),
        args: vec![
            OsString::from("-c"),
            OsString::from("exec head -c 100000 /dev/zero"),
        ],
        cwd: None,
        env: HashMap::new(),
        clear_env: true,
        stdin: StdinMode::Null,
        stdout: StreamMode::CaptureLimited(8192),
        stderr: StreamMode::Null,
        timeout: Some(Duration::from_secs(5)),
        cancellation: CancellationToken::new(),
    };
    let output = SystemProcessRunner.run(spec()).unwrap();
    assert_eq!(output.stdout.len(), 8193);
    let mut sleep = spec();
    sleep.program = OsString::from("/bin/sleep");
    sleep.args = vec![OsString::from("10")];
    sleep.timeout = Some(Duration::from_millis(20));
    let started = Instant::now();
    assert!(matches!(
        SystemProcessRunner.run(sleep),
        Err(ProcessError::Timeout)
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
    let mut inherited_pipe = spec();
    inherited_pipe.args = vec![OsString::from("-c"), OsString::from("sleep 6 & wait")];
    inherited_pipe.timeout = Some(Duration::from_millis(20));
    let started = Instant::now();
    assert!(matches!(
        SystemProcessRunner.run(inherited_pipe),
        Err(ProcessError::Timeout)
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
    let mut completed_with_inherited_pipe = spec();
    completed_with_inherited_pipe.args = vec![
        OsString::from("-c"),
        OsString::from("sleep 6 & printf '/synthetic-ca-path\\n'"),
    ];
    completed_with_inherited_pipe.timeout = Some(Duration::from_millis(20));
    let started = Instant::now();
    assert!(matches!(
        SystemProcessRunner.run(completed_with_inherited_pipe),
        Err(ProcessError::Timeout)
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
}

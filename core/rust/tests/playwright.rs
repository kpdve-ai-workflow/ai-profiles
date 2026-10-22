use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use crate::{model::Client, playwright::runtime::prepare_launch};

fn installed_launcher(root: &Path, client: Client) -> PathBuf {
    let directory = root
        .join(".ai-profiles")
        .join(client.as_str())
        .join("mcp/playwright");
    fs::create_dir_all(&directory).unwrap();
    for name in [
        "compose.yaml",
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
            launcher
                .parent()
                .unwrap()
                .join("compose.yaml")
                .into_os_string()
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
    let original = OsStr::new(".ai-profiles/opencode/mcp/playwright/compose.yaml:custom.yaml:.ai-profiles/opencode/mcp/playwright/compose.basic-auth.yaml");
    let inherited = env(&[
        ("COMPOSE_FILE", original),
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
    assert!(compose.starts_with(&format!("{auth_overlay}:")));
    assert!(compose.ends_with(original.to_str().unwrap()));
    assert_eq!(compose.matches("compose.basic-auth.yaml").count(), 1);
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

//! Distribution contract: one shared bundle, verified release, no root secrets,
//! no WordPress writes during profile install, and no credential destruction.
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf, process::Command, sync::OnceLock};

fn binaries() -> PathBuf {
    static BUILD: OnceLock<()> = OnceLock::new();
    BUILD.get_or_init(|| {
        assert!(Command::new(env!("CARGO"))
            .args([
                "build",
                "--manifest-path",
                concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
                "--locked",
                "--release",
                "--bins"
            ])
            .status()
            .unwrap()
            .success());
    });
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release")
}

#[test]
fn novamira_install_distributes_shared_docker_bundle_and_verified_plugin_without_touching_wordpress(
) {
    let bin = binaries();
    let project = tempfile::tempdir().unwrap();
    let invoke = |action: &str, client: &str| {
        let mut cmd = Command::new(bin.join("ai-profile"));
        cmd.arg(action);
        if action == "install" {
            cmd.arg("wordpress");
        }
        cmd.args(["--client", client, "--project"])
            .arg(project.path())
            .output()
            .unwrap()
    };
    let result = invoke("install", "opencode");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let root = project.path().join(".ai-profiles/mcp/novamira");
    for name in [
        "Dockerfile",
        "compose.yaml",
        ".dockerignore",
        "launcher",
        "install-plugin",
        "downloads/novamira.zip",
        ".env.novamira",
    ] {
        assert!(root.join(name).is_file(), "missing bundle file {name}");
    }
    assert!(root.join(".private").is_dir());
    assert!(!project.path().join(".env.novamira").exists());
    assert!(!project.path().join("wp-content").exists());
    assert_eq!(
        fs::metadata(root.join(".env.novamira"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        crate::codec::sha(&fs::read(root.join("downloads/novamira.zip")).unwrap()),
        "ad4dac087e9cad073b2206cb75b44bf5224d57880452f1937a038068920caf78"
    );
    assert!(fs::read_to_string(root.join("Dockerfile"))
        .unwrap()
        .contains("@automattic/mcp-wordpress-remote@0.4.0"));
    let config = fs::read(project.path().join("opencode.json")).unwrap();
    fs::write(
        root.join(".env.novamira"),
        "WP_API_PASSWORD=synthetic-private-sentinel\n",
    )
    .unwrap();
    let result = invoke("install", "claude");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(project.path().join("opencode.json")).unwrap(),
        config
    );
    let result = invoke("update", "opencode");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let state = fs::read_to_string(project.path().join(".ai-profiles/state.json")).unwrap();
    assert!(!state.contains("synthetic-private-sentinel"));
    for client in ["opencode", "claude"] {
        assert!(invoke("uninstall", client).status.success());
    }
    assert_eq!(
        fs::read_to_string(root.join(".env.novamira")).unwrap(),
        "WP_API_PASSWORD=synthetic-private-sentinel\n"
    );
    assert!(root.join("downloads/novamira.zip").exists());
    // Exercise the real shell script with a controlled WP-CLI boundary. It
    // must verify local context first and never request force or activation.
    let tools = tempfile::tempdir().unwrap();
    let wp = tools.path().join("wp");
    fs::write(
        &wp,
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" >> "$WP_TEST_CALLS"
case "$*" in
  *eval*) [ "${WP_TEST_REJECT:-}" != yes ] || exit 10; printf novamira-local-install-ready ;;
  *'plugin install'*) exit 0 ;;
  *) exit 8 ;;
esac
"#,
    )
    .unwrap();
    fs::set_permissions(&wp, fs::Permissions::from_mode(0o755)).unwrap();
    let calls = tools.path().join("calls");
    let path = format!(
        "{}:{}",
        tools.path().display(),
        std::env::var("PATH").unwrap()
    );
    let run_plugin = |reject: &str| {
        Command::new(root.join("install-plugin"))
            .arg("--wordpress")
            .arg(project.path())
            .env("PATH", &path)
            .env("WP_TEST_CALLS", &calls)
            .env("WP_TEST_REJECT", reject)
            .output()
            .unwrap()
    };
    let result = run_plugin("yes");
    assert!(!result.status.success());
    assert!(!fs::read_to_string(&calls)
        .unwrap()
        .contains("plugin install"));
    fs::write(&calls, "").unwrap();
    assert!(run_plugin("no").status.success());
    let log = fs::read_to_string(&calls).unwrap();
    assert!(log.contains("plugin install"));
    assert!(!log.contains("--force") && !log.contains("--activate"));
    fs::write(&calls, "").unwrap();
    fs::write(root.join("downloads/novamira.zip"), "corrupt archive").unwrap();
    assert!(!run_plugin("no").status.success());
    assert!(
        fs::read_to_string(&calls).unwrap().is_empty(),
        "bad ZIP must fail before WordPress is started"
    );
}

#[test]
fn novamira_local_settings_cannot_redirect_runtime_or_secret_paths() {
    let project = tempfile::tempdir().unwrap();
    fs::create_dir_all(project.path().join(".ai-profiles/opencode")).unwrap();
    let definition: serde_json::Value =
        serde_json::from_slice(include_bytes!("../../../mcp/servers/novamira.json")).unwrap();
    let base = serde_json::json!({"novamira":definition});
    for entry in [
        serde_json::json!({"env_file":"outside.env"}),
        serde_json::json!({"command":["arbitrary"]}),
        serde_json::json!({"cwd":"/outside"}),
    ] {
        fs::write(
            project.path().join(".ai-profiles/opencode/local.json"),
            serde_json::json!({"mcp":{"novamira":entry}}).to_string(),
        )
        .unwrap();
        assert!(crate::mcp::config::resolve_at(
            project.path(),
            base.as_object().unwrap(),
            ".ai-profiles/opencode/local.json"
        )
        .is_err());
    }
}

#[test]
fn plugin_script_installs_verified_release_in_real_disposable_wordpress_without_activation() {
    // Real WP-CLI, PHP and MariaDB run only inside the required VM's Docker.
    // No application is exposed over HTTP and no real project/credentials exist.
    struct Sandbox {
        network: String,
    }
    impl Drop for Sandbox {
        fn drop(&mut self) {
            let _ = Command::new("docker")
                .args([
                    "rm",
                    "-f",
                    &format!("{}-cli", self.network),
                    &format!("{}-db", self.network),
                ])
                .output();
            let _ = Command::new("docker")
                .args(["network", "rm", &self.network])
                .output();
        }
    }
    let project = tempfile::tempdir().unwrap();
    let root = project.path().join(".ai-profiles/mcp/novamira");
    crate::profile_management::credentials::prepare(project.path()).unwrap();
    let unique = project
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .trim_start_matches('.');
    let sandbox = Sandbox {
        network: format!("ai-profiles-wp-{unique}"),
    };
    let docker = |args: &[&str]| {
        Command::new("timeout")
            .args(["180", "docker"])
            .args(args)
            .output()
            .unwrap()
    };
    assert!(docker(&["network", "create", &sandbox.network])
        .status
        .success());
    let db = docker(&[
        "run",
        "-d",
        "--rm",
        "--name",
        &format!("{}-db", sandbox.network),
        "--network",
        &sandbox.network,
        "--network-alias",
        "db",
        "-e",
        "MARIADB_ROOT_PASSWORD=synthetic-root-only",
        "-e",
        "MARIADB_DATABASE=wordpress",
        "-e",
        "MARIADB_USER=wordpress",
        "-e",
        "MARIADB_PASSWORD=synthetic-database-only",
        "mariadb:11.4",
    ]);
    assert!(
        db.status.success(),
        "{}",
        String::from_utf8_lossy(&db.stderr)
    );
    let tools = tempfile::tempdir().unwrap();
    let wrapper = tools.path().join("wp");
    fs::write(&wrapper, r#"#!/bin/sh
set -eu
exec timeout 180 docker run --rm --name "$WP_TEST_NETWORK-cli" --network "$WP_TEST_NETWORK" --user 1000:1000 \
  --volume "$WP_TEST_PROJECT:$WP_TEST_PROJECT" --workdir "$WP_TEST_PROJECT" \
  -e "WP_CLI_CACHE_DIR=$WP_TEST_PROJECT/.ai-profiles/mcp/novamira/.private/wp-cli-cache" \
  wordpress:cli-php8.3 php -d memory_limit=512M /usr/local/bin/wp "$@"
"#).unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    let wp = |args: &[&str]| {
        Command::new(&wrapper)
            .args(args)
            .env("WP_TEST_NETWORK", &sandbox.network)
            .env("WP_TEST_PROJECT", project.path())
            .output()
            .unwrap()
    };
    let downloaded = wp(&["core", "download", "--version=6.9"]);
    assert!(
        downloaded.status.success(),
        "{}",
        String::from_utf8_lossy(&downloaded.stderr)
    );
    let config = wp(&[
        "config",
        "create",
        "--dbname=wordpress",
        "--dbuser=wordpress",
        "--dbpass=synthetic-database-only",
        "--dbhost=db",
        "--skip-check",
    ]);
    assert!(
        config.status.success(),
        "{}",
        String::from_utf8_lossy(&config.stderr)
    );
    assert!(wp(&[
        "config",
        "set",
        "WP_ENVIRONMENT_TYPE",
        "local",
        "--type=constant"
    ])
    .status
    .success());
    let mut installed = false;
    for _ in 0..30 {
        let result = wp(&[
            "core",
            "install",
            "--url=http://novamira-fixture.test",
            "--title=Novamira fixture",
            "--admin_user=fixture-admin",
            "--admin_password=synthetic-admin-only",
            "--admin_email=admin@example.test",
            "--skip-email",
        ]);
        if result.status.success() {
            installed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    assert!(
        installed,
        "could not initialize disposable WordPress database"
    );
    let run_plugin = || {
        Command::new(root.join("install-plugin"))
            .arg("--wordpress")
            .arg(project.path())
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    tools.path().display(),
                    std::env::var("PATH").unwrap()
                ),
            )
            .env("WP_TEST_NETWORK", &sandbox.network)
            .env("WP_TEST_PROJECT", project.path())
            .output()
            .unwrap()
    };
    let result = run_plugin();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let version = wp(&["plugin", "get", "novamira", "--field=version"]);
    assert!(version.status.success());
    assert_eq!(String::from_utf8(version.stdout).unwrap().trim(), "1.12.12");
    assert!(!wp(&["plugin", "is-active", "novamira"]).status.success());
    assert!(
        !run_plugin().status.success(),
        "existing plugin must not be overwritten"
    );
}

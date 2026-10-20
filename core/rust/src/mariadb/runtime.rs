use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs::{self, OpenOptions},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::Command,
    time::Duration,
};

use fs2::FileExt;

use crate::{
    mariadb::{
        archive::{fetch_source, HttpSourceDownloader, SourceDownloader},
        MariaDbError, COMMIT, CONTAINER_ENV, IMAGE, PLATFORM, SOURCE_LABEL,
    },
    process::{
        CancellationToken, CommandSpec, ProcessRunner, StdinMode, StreamMode, SystemProcessRunner,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct DatabasePort(u16);

impl DatabasePort {
    pub(crate) fn get(self) -> u16 {
        self.0
    }
}

/// Validated settings retain environment values for the later runtime stage.
/// Deliberately does not implement `Debug`, because it contains credentials.
pub(crate) struct ValidatedEnvironment {
    pub(crate) port: DatabasePort,
    pub(crate) read_only: bool,
    pub(crate) block_sensitive_show: bool,
    pub(crate) values: HashMap<String, String>,
}

pub(crate) fn validate_environment(
    environment: &HashMap<String, String>,
) -> Result<ValidatedEnvironment, MariaDbError> {
    for key in super::REQUIRED {
        if !environment.get(key).is_some_and(|value| !value.is_empty()) {
            return Err(MariaDbError::MissingSetting(key));
        }
    }

    let port_value = environment
        .get("DB_PORT")
        .ok_or(MariaDbError::MissingSetting("DB_PORT"))?;
    if !port_value.is_ascii() || !port_value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(MariaDbError::InvalidPort);
    }
    let port = port_value
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .map(DatabasePort)
        .ok_or(MariaDbError::InvalidPort)?;

    let read_only = parse_bool(environment, "MCP_READ_ONLY")?;
    let block_sensitive_show = parse_bool(environment, "MCP_BLOCK_SENSITIVE_SHOW")?;
    Ok(ValidatedEnvironment {
        port,
        read_only,
        block_sensitive_show,
        values: environment.clone(),
    })
}

fn parse_bool(
    environment: &HashMap<String, String>,
    key: &'static str,
) -> Result<bool, MariaDbError> {
    match environment.get(key).map(String::as_str).unwrap_or("false") {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(MariaDbError::InvalidBoolean(key)),
    }
}

pub(crate) fn docker_environment(environment: &HashMap<String, String>) -> HashMap<String, String> {
    const ALLOWED: [&str; 14] = [
        "PATH",
        "HOME",
        "XDG_RUNTIME_DIR",
        "XDG_CONFIG_HOME",
        "TMPDIR",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "NO_PROXY",
        "http_proxy",
        "https_proxy",
        "no_proxy",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "SSH_AUTH_SOCK",
    ];
    environment
        .iter()
        .filter(|(key, _)| ALLOWED.contains(&key.as_str()) || key.starts_with("DOCKER_"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

pub(crate) fn image_available(environment: &HashMap<String, String>) -> Result<bool, MariaDbError> {
    image_available_with(environment, &SystemProcessRunner)
}

fn image_available_with(
    environment: &HashMap<String, String>,
    runner: &impl ProcessRunner,
) -> Result<bool, MariaDbError> {
    let format = format!("{{{{ index .Config.Labels \"{SOURCE_LABEL}\" }}}}");
    let output = run_docker(
        runner,
        ["image", "inspect", "--format", format.as_str(), IMAGE],
        environment,
        Duration::from_secs(30),
        StreamMode::Capture,
        StreamMode::Capture,
    )?;
    Ok(output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == COMMIT)
}

pub(crate) fn ensure_image(environment: &HashMap<String, String>) -> Result<(), MariaDbError> {
    ensure_image_with(environment, &SystemProcessRunner, &HttpSourceDownloader)
}

pub(crate) fn ensure_image_with(
    environment: &HashMap<String, String>,
    runner: &impl ProcessRunner,
    downloader: &impl SourceDownloader,
) -> Result<(), MariaDbError> {
    let build_environment = docker_environment(environment);
    if image_available_with(&build_environment, runner)? {
        return Ok(());
    }

    let daemon = run_docker(
        runner,
        ["info", "--format", "{{.OSType}}"],
        &build_environment,
        Duration::from_secs(30),
        StreamMode::Capture,
        StreamMode::Capture,
    )?;
    if !daemon.status.success() || String::from_utf8_lossy(&daemon.stdout).trim() != "linux" {
        return Err(MariaDbError::DockerUnavailable);
    }

    let cache = cache_directory(environment)?
        .join("ai-profiles")
        .join("mariadb");
    fs::create_dir_all(&cache).map_err(|source| MariaDbError::Io {
        path: cache.clone(),
        source,
    })?;
    let lock_path = cache.join(format!("{COMMIT}.lock"));
    let lock = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&lock_path)
        .map_err(|source| MariaDbError::Io {
            path: lock_path.clone(),
            source,
        })?;
    FileExt::lock_exclusive(&lock).map_err(|source| MariaDbError::Io {
        path: lock_path,
        source,
    })?;
    if image_available_with(&build_environment, runner)? {
        return Ok(());
    }

    eprintln!(
        "MariaDB MCP: building pinned image; first start can take several minutes and several GB of disk space. Progress is on stderr."
    );
    let temporary = tempfile::Builder::new()
        .prefix("ai-profiles-mariadb-")
        .tempdir()
        .map_err(|source| MariaDbError::Io {
            path: env::temp_dir(),
            source,
        })?;
    fetch_source(temporary.path(), downloader)?;

    let label = format!("{SOURCE_LABEL}={COMMIT}");
    let build_args = vec![
        OsString::from("build"),
        OsString::from("--platform"),
        OsString::from(PLATFORM),
        OsString::from("--label"),
        OsString::from(label),
        OsString::from("--tag"),
        OsString::from(IMAGE),
        temporary.path().as_os_str().to_owned(),
    ];
    let built = run_docker_os(
        runner,
        build_args,
        &build_environment,
        Duration::from_secs(3600),
        StreamMode::RedirectToStderr,
        StreamMode::Inherit,
    )?;
    if !built.status.success() || !image_available_with(&build_environment, runner)? {
        return Err(MariaDbError::ImageBuildFailed);
    }
    Ok(())
}

pub(crate) fn launch(environment: HashMap<String, String>) -> Result<(), MariaDbError> {
    launch_with(environment, &SystemProcessRunner, &HttpSourceDownloader)
}

fn launch_with(
    environment: HashMap<String, String>,
    runner: &impl ProcessRunner,
    downloader: &impl SourceDownloader,
) -> Result<(), MariaDbError> {
    let mut settings = validate_environment(&environment)?;
    ensure_image_with(&settings.values, runner, downloader)?;

    let read_only_default = settings.read_only.to_string();
    settings
        .values
        .entry("MCP_READ_ONLY".to_owned())
        .or_insert(read_only_default);
    let block_sensitive_default = settings.block_sensitive_show.to_string();
    settings
        .values
        .entry("MCP_BLOCK_SENSITIVE_SHOW".to_owned())
        .or_insert(block_sensitive_default);

    let network = settings
        .values
        .get("MARIADB_MCP_NETWORK")
        .map(String::as_str)
        .unwrap_or("host");
    let mut args = vec![
        OsString::from("run"),
        OsString::from("--interactive"),
        OsString::from("--rm"),
        OsString::from("--platform"),
        OsString::from(PLATFORM),
        OsString::from(format!("--network={network}")),
    ];
    for key in CONTAINER_ENV {
        if settings.values.contains_key(key) {
            args.extend([OsString::from("--env"), OsString::from(key)]);
        }
    }
    args.extend([
        OsString::from(IMAGE),
        OsString::from("python"),
        OsString::from("src/server.py"),
        OsString::from("--transport"),
        OsString::from("stdio"),
    ]);

    let mut command = Command::new("docker");
    command.args(args).envs(&settings.values);
    let _exec_error = command.exec();
    Err(MariaDbError::Exec)
}

fn run_docker<const N: usize>(
    runner: &impl ProcessRunner,
    args: [&str; N],
    environment: &HashMap<String, String>,
    timeout: Duration,
    stdout: StreamMode,
    stderr: StreamMode,
) -> Result<crate::process::CommandOutput, MariaDbError> {
    run_docker_os(
        runner,
        args.into_iter().map(OsString::from).collect(),
        environment,
        timeout,
        stdout,
        stderr,
    )
}

fn run_docker_os(
    runner: &impl ProcessRunner,
    args: Vec<OsString>,
    environment: &HashMap<String, String>,
    timeout: Duration,
    stdout: StreamMode,
    stderr: StreamMode,
) -> Result<crate::process::CommandOutput, MariaDbError> {
    let process_environment = environment
        .iter()
        .map(|(key, value)| (OsString::from(key), OsString::from(value)))
        .collect();
    runner
        .run(CommandSpec {
            program: OsString::from("docker"),
            args,
            cwd: None,
            env: process_environment,
            clear_env: true,
            stdin: StdinMode::Null,
            stdout,
            stderr,
            timeout: Some(timeout),
            cancellation: CancellationToken::new(),
        })
        .map_err(|_| MariaDbError::DockerCommand)
}

fn cache_directory(environment: &HashMap<String, String>) -> Result<PathBuf, MariaDbError> {
    if let Some(path) = environment
        .get("XDG_CACHE_HOME")
        .filter(|path| !path.is_empty())
    {
        return Ok(PathBuf::from(path));
    }
    environment
        .get("HOME")
        .filter(|home| !home.is_empty())
        .map(|home| PathBuf::from(home).join(".cache"))
        .ok_or(MariaDbError::CacheUnavailable)
}

pub fn main() -> i32 {
    let environment = env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    match launch(environment) {
        Ok(()) => 0,
        Err(error) if is_generic_failure(&error) => {
            eprintln!(
                "MariaDB MCP startup failed: check Docker, network access, cache and disk space"
            );
            1
        }
        Err(error) => {
            eprintln!("MariaDB MCP startup failed: {error}");
            1
        }
    }
}

fn is_generic_failure(error: &MariaDbError) -> bool {
    matches!(
        error,
        MariaDbError::Download
            | MariaDbError::InvalidArchive
            | MariaDbError::DockerCommand
            | MariaDbError::CacheUnavailable
            | MariaDbError::Exec
            | MariaDbError::Io { .. }
    )
}

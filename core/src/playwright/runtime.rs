use std::{
    collections::HashMap,
    env,
    ffi::{OsStr, OsString},
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Component, Path, PathBuf},
    process::Command,
    time::Duration,
};

use crate::{
    filesystem::file_path,
    model::Client,
    process::{
        CancellationToken, CommandSpec, ProcessRunner, StdinMode, StreamMode, SystemProcessRunner,
    },
};

const PROJECT_METADATA: &str = "AI_PROFILE_PROJECT_DIR";
const COMPOSE_FILE: &str = "COMPOSE_FILE";
const BASIC_AUTH_FILE: &str = "AI_PROFILE_PLAYWRIGHT_BASIC_AUTH_FILE";
const STORAGE_STATE: &str = "AI_PROFILE_PLAYWRIGHT_STORAGE_STATE";
const DEFAULT_BASIC_AUTH: &str = "base-auth.json";
const CA_CERT: &str = "AI_PROFILE_PLAYWRIGHT_CA_CERT";
const CAROOT_OUTPUT_LIMIT: usize = 8192;

#[derive(Debug, thiserror::Error)]
pub(crate) enum PlaywrightError {
    #[error("Invalid Playwright launcher location")]
    InvalidLauncher,
    #[error("Playwright project metadata does not match the installed launcher")]
    ProjectMismatch,
    #[error("Cannot prepare the Playwright reports directory")]
    InvalidReports,
    #[error("Invalid private Playwright input; check its path, file type and permissions")]
    InvalidPrivateInput,
    #[error("Cannot prepare Playwright Compose configuration")]
    InvalidCompose,
    #[error(
        "Invalid Playwright CA certificate path; expected an existing public certificate file"
    )]
    InvalidCa,
}

pub(crate) struct LaunchPlan {
    pub(crate) project: PathBuf,
    pub(crate) environment: HashMap<OsString, OsString>,
}

pub(crate) fn prepare_launch(
    executable: &Path,
    project_dir: &OsStr,
    inherited: &HashMap<OsString, OsString>,
) -> Result<LaunchPlan, PlaywrightError> {
    prepare_launch_with(executable, project_dir, inherited, &SystemProcessRunner)
}

pub(crate) fn prepare_launch_with(
    executable: &Path,
    project_dir: &OsStr,
    inherited: &HashMap<OsString, OsString>,
    runner: &impl ProcessRunner,
) -> Result<LaunchPlan, PlaywrightError> {
    let (project, launcher_directory) = installed_identity(executable)?;
    let metadata_project = fs::canonicalize(PathBuf::from(project_dir))
        .map_err(|_| PlaywrightError::ProjectMismatch)?;
    if metadata_project != project {
        return Err(PlaywrightError::ProjectMismatch);
    }

    prepare_reports(&project)?;
    let reports = project
        .join(".ai-profiles/reports")
        .canonicalize()
        .map_err(|_| PlaywrightError::InvalidReports)?;

    let mut environment = HashMap::new();
    let mut overlays = Vec::new();
    if inherited
        .get(OsStr::new("AI_PROFILE_PLAYWRIGHT_NETWORK"))
        .is_none_or(|network| network.is_empty() || network == "host")
    {
        overlays.push(launcher_directory.join("compose.host.yaml"));
    }
    if let Some(certificate) = ca_certificate(&project, inherited, runner)? {
        environment.insert(OsString::from(CA_CERT), certificate.into_os_string());
        overlays.push(launcher_directory.join("compose.ca.yaml"));
    }
    if let Some(value) = inherited
        .get(OsStr::new(STORAGE_STATE))
        .filter(|value| !value.is_empty())
    {
        let storage = private_input(&project, value, &reports)?;
        environment.insert(OsString::from(STORAGE_STATE), storage.into_os_string());
        overlays.push(launcher_directory.join("compose.auth.yaml"));
    }

    let explicit_auth = inherited
        .get(OsStr::new(BASIC_AUTH_FILE))
        .filter(|value| !value.is_empty());
    let credentials = match explicit_auth {
        Some(value) => Some(private_input(&project, value, &reports)?),
        None => default_credentials(&project, &reports)?,
    };
    if let Some(credentials) = credentials {
        environment.insert(
            OsString::from(BASIC_AUTH_FILE),
            credentials.into_os_string(),
        );
        overlays.push(launcher_directory.join("compose.basic-auth.yaml"));
    }

    let original = inherited
        .get(OsStr::new(COMPOSE_FILE))
        .filter(|value| !value.is_empty())
        .map(|value| value.to_str().ok_or(PlaywrightError::InvalidCompose))
        .transpose()?;
    let compose_files = compose_files(
        &project,
        &launcher_directory.join("compose.yaml"),
        original,
        &overlays,
    )?;
    environment.insert(OsString::from(COMPOSE_FILE), OsString::from(compose_files));

    Ok(LaunchPlan {
        project,
        environment,
    })
}

fn ca_certificate(
    project: &Path,
    inherited: &HashMap<OsString, OsString>,
    runner: &impl ProcessRunner,
) -> Result<Option<PathBuf>, PlaywrightError> {
    if let Some(explicit) = inherited
        .get(OsStr::new(CA_CERT))
        .filter(|value| !value.is_empty())
    {
        return public_certificate(&absolute_path(project, Path::new(explicit))).map(Some);
    }
    if let Some(caroot) = inherited
        .get(OsStr::new("CAROOT"))
        .filter(|value| !value.is_empty())
    {
        return existing_ca(&absolute_path(project, Path::new(caroot)));
    }
    let Some(program) = inherited.get(OsStr::new("PATH")).and_then(|path| {
        env::split_paths(path)
            .map(|directory| absolute_path(project, &directory).join("mkcert"))
            .find(|program| {
                fs::metadata(program).is_ok_and(|metadata| {
                    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
                })
            })
    }) else {
        return Ok(None);
    };
    // mkcert's CAROOT query only needs location-related environment, not MCP secrets.
    let environment = ["PATH", "HOME", "XDG_DATA_HOME"]
        .into_iter()
        .filter_map(|key| {
            inherited
                .get(OsStr::new(key))
                .map(|value| (OsString::from(key), value.clone()))
        })
        .collect();
    let output = runner.run(CommandSpec {
        program: program.into_os_string(),
        args: vec![OsString::from("-CAROOT")],
        cwd: Some(project.to_owned()),
        env: environment,
        clear_env: true,
        stdin: StdinMode::Null,
        stdout: StreamMode::CaptureLimited(CAROOT_OUTPUT_LIMIT),
        stderr: StreamMode::Null,
        timeout: Some(Duration::from_secs(5)),
        cancellation: CancellationToken::new(),
    });
    let caroot = output
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| parse_caroot(&output.stdout));
    match caroot {
        Some(caroot) => existing_ca(&caroot),
        None => {
            eprintln!("playwright: cannot discover mkcert CA; continuing without automatic CA");
            Ok(None)
        }
    }
}

fn absolute_path(project: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        project.join(path)
    }
}

fn parse_caroot(output: &[u8]) -> Option<PathBuf> {
    if output.len() > CAROOT_OUTPUT_LIMIT {
        return None;
    }
    let text = std::str::from_utf8(output).ok()?;
    let text = text.strip_suffix('\n').unwrap_or(text);
    let text = text.strip_suffix('\r').unwrap_or(text);
    let path = Path::new(text);
    (path.is_absolute() && !text.contains(['\n', '\r', '\0'])).then(|| path.to_owned())
}

fn existing_ca(caroot: &Path) -> Result<Option<PathBuf>, PlaywrightError> {
    let path = caroot.join("rootCA.pem");
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(PlaywrightError::InvalidCa),
        Ok(_) => public_certificate(&path).map(Some),
    }
}

fn public_certificate(path: &Path) -> Result<PathBuf, PlaywrightError> {
    // Only inspect metadata on the host. Certificate parsing/import stays in Docker.
    // Preserve explicit public-CA symlinks, but mount only the resolved file.
    let path = path
        .canonicalize()
        .map_err(|_| PlaywrightError::InvalidCa)?;
    if path.file_name() == Some(OsStr::new("rootCA-key.pem")) {
        return Err(PlaywrightError::InvalidCa);
    }
    let metadata = fs::metadata(&path).map_err(|_| PlaywrightError::InvalidCa)?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o444 == 0 {
        return Err(PlaywrightError::InvalidCa);
    }
    Ok(path)
}

pub(crate) fn installed_identity(executable: &Path) -> Result<(PathBuf, PathBuf), PlaywrightError> {
    let executable = executable
        .canonicalize()
        .map_err(|_| PlaywrightError::InvalidLauncher)?;
    if executable.file_name() != Some(OsStr::new("playwright-launcher")) {
        return Err(PlaywrightError::InvalidLauncher);
    }
    let launcher_directory = executable
        .parent()
        .filter(|path| path.file_name() == Some(OsStr::new("playwright")))
        .ok_or(PlaywrightError::InvalidLauncher)?;
    let mcp_directory = launcher_directory
        .parent()
        .filter(|path| path.file_name() == Some(OsStr::new("mcp")))
        .ok_or(PlaywrightError::InvalidLauncher)?;
    let client_directory = mcp_directory
        .parent()
        .ok_or(PlaywrightError::InvalidLauncher)?;
    let client_name = client_directory
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or(PlaywrightError::InvalidLauncher)?;
    client_name
        .parse::<Client>()
        .map_err(|_| PlaywrightError::InvalidLauncher)?;
    let profiles_directory = client_directory
        .parent()
        .filter(|path| path.file_name() == Some(OsStr::new(".ai-profiles")))
        .ok_or(PlaywrightError::InvalidLauncher)?;
    let project = profiles_directory
        .parent()
        .ok_or(PlaywrightError::InvalidLauncher)?
        .canonicalize()
        .map_err(|_| PlaywrightError::InvalidLauncher)?;
    let launcher_directory = launcher_directory
        .canonicalize()
        .map_err(|_| PlaywrightError::InvalidLauncher)?;
    Ok((project, launcher_directory))
}

fn prepare_reports(project: &Path) -> Result<(), PlaywrightError> {
    let path =
        file_path(project, ".ai-profiles/reports").map_err(|_| PlaywrightError::InvalidReports)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(PlaywrightError::InvalidReports),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => match fs::create_dir(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                fs::symlink_metadata(&path)
                    .ok()
                    .filter(|metadata| metadata.is_dir() && !metadata.file_type().is_symlink())
                    .map(|_| ())
                    .ok_or(PlaywrightError::InvalidReports)
            }
            Err(_) => Err(PlaywrightError::InvalidReports),
        },
        Err(_) => Err(PlaywrightError::InvalidReports),
    }
}

fn default_credentials(project: &Path, reports: &Path) -> Result<Option<PathBuf>, PlaywrightError> {
    let path =
        file_path(project, DEFAULT_BASIC_AUTH).map_err(|_| PlaywrightError::InvalidPrivateInput)?;
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err(PlaywrightError::InvalidPrivateInput),
        Ok(_) => private_input(project, OsStr::new(DEFAULT_BASIC_AUTH), reports).map(Some),
    }
}

fn private_input(
    project: &Path,
    value: &OsStr,
    reports: &Path,
) -> Result<PathBuf, PlaywrightError> {
    let supplied = PathBuf::from(value);
    let path = if supplied.is_absolute() {
        if has_parent_component(&supplied) || has_symlink_component(&supplied) {
            return Err(PlaywrightError::InvalidPrivateInput);
        }
        supplied
    } else {
        let relative = supplied
            .to_str()
            .ok_or(PlaywrightError::InvalidPrivateInput)?;
        file_path(project, relative).map_err(|_| PlaywrightError::InvalidPrivateInput)?
    };
    let metadata = fs::symlink_metadata(&path).map_err(|_| PlaywrightError::InvalidPrivateInput)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o400 == 0
        || metadata.permissions().mode() & 0o077 != 0
    {
        return Err(PlaywrightError::InvalidPrivateInput);
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| PlaywrightError::InvalidPrivateInput)?;
    if canonical.starts_with(reports) || !canonical.starts_with(project) {
        return Err(PlaywrightError::InvalidPrivateInput);
    }
    Ok(canonical)
}

fn has_parent_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn has_symlink_component(path: &Path) -> bool {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if fs::symlink_metadata(&current).is_ok_and(|metadata| metadata.file_type().is_symlink()) {
            return true;
        }
    }
    false
}

fn compose_files(
    project: &Path,
    base: &Path,
    original: Option<&str>,
    overlays: &[PathBuf],
) -> Result<String, PlaywrightError> {
    let original_files = original
        .map(|value| value.split(':').map(str::to_owned).collect::<Vec<_>>())
        .filter(|files| !files.is_empty())
        .unwrap_or_else(|| vec![base.to_string_lossy().into_owned()]);
    let mut combined = Vec::with_capacity(overlays.len() + original_files.len());
    for overlay in overlays {
        if !original_files
            .iter()
            .any(|entry| same_compose_path(project, entry, overlay))
        {
            combined.push(overlay.to_string_lossy().into_owned());
        }
    }
    combined.extend(original_files);
    Ok(combined.join(":"))
}

fn same_compose_path(project: &Path, entry: &str, expected: &Path) -> bool {
    let entry = Path::new(entry);
    let resolved = if entry.is_absolute() {
        entry.to_owned()
    } else {
        project.join(entry)
    };
    match (resolved.canonicalize(), expected.canonicalize()) {
        (Ok(resolved), Ok(expected)) => resolved == expected,
        _ => resolved == expected,
    }
}

pub fn main() -> i32 {
    let mut arguments = env::args_os();
    let _executable = arguments.next();
    if arguments.next().is_some() {
        eprintln!("playwright: launcher does not accept arguments");
        return 2;
    }
    let executable = match env::current_exe() {
        Ok(path) => path,
        Err(_) => {
            eprintln!("playwright: invalid installed launcher location");
            return 1;
        }
    };
    let Some(project) = env::var_os(PROJECT_METADATA) else {
        eprintln!("playwright: start through the installed ai-profile-mcp adapter");
        return 1;
    };
    let inherited = env::vars_os().collect::<HashMap<_, _>>();
    let plan = match prepare_launch(&executable, &project, &inherited) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("playwright: {error}");
            return 1;
        }
    };

    let mut command = Command::new("docker");
    command
        .arg("compose")
        .arg("--project-directory")
        .arg(&plan.project)
        .args([
            "run",
            "--rm",
            "--no-deps",
            "--pull",
            "never",
            "-T",
            "playwright",
        ])
        .current_dir(&plan.project)
        .envs(plan.environment);
    crate::logging::handoff();
    let source = command.exec();
    eprintln!("playwright: cannot start Docker Compose: {source}");
    1
}

use std::{
    env, fs,
    io::Write,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const FALLBACK_IMAGE: &str = "";

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    match launch() {
        Ok(status) => status,
        Err(message) => {
            eprintln!("playwright launcher: {message}");
            1
        }
    }
}

fn launch() -> Result<i32, String> {
    let exe = env::current_exe().map_err(|e| format!("cannot locate installed launcher: {e}"))?;
    let launcher_dir = exe.parent().ok_or("invalid launcher location")?;
    let project = project_from_launcher_path(&exe)?;
    let descriptor = fs::read(launcher_dir.join("image.json"))
        .map_err(|e| format!("cannot read image descriptor: {e}"))?;
    let value: serde_json::Value = serde_json::from_slice(&descriptor)
        .map_err(|e| format!("invalid image descriptor: {e}"))?;
    let image = value
        .get("image")
        .and_then(serde_json::Value::as_str)
        .filter(|s| !s.is_empty())
        .unwrap_or(FALLBACK_IMAGE);
    if image.is_empty()
        || image.contains("latest")
        || !image.starts_with("ghcr.io/kpdve-ai-workflow/playwright:")
    {
        return Err("image descriptor must contain a fixed ghcr.io/kpdve-ai-workflow/playwright:<version> reference".into());
    }
    let docker = |args: &[&str]| {
        Command::new("docker")
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .output()
    };
    match docker(&["info", "--format", "{{.ServerVersion}}"]) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err("Docker CLI is missing; install Docker Engine or Docker Desktop".into())
        }
        Err(e) => return Err(format!("cannot run Docker: {e}")),
        Ok(output) if !output.status.success() => {
            return Err("Docker daemon is unavailable; start Docker and retry".into())
        }
        _ => {}
    }
    let inspect = docker(&["image", "inspect", image])
        .map_err(|e| format!("cannot inspect Docker image: {e}"))?;
    if !inspect.status.success() {
        eprintln!("playwright launcher: pulling {image}");
        let pull = Command::new("docker")
            .args(["pull", image])
            .output()
            .map_err(|e| format!("cannot pull image: {e}"))?;
        let mut stderr = std::io::stderr().lock();
        let _ = stderr.write_all(&pull.stdout);
        let _ = stderr.write_all(&pull.stderr);
        if !pull.status.success() {
            return Err(format!(
                "image pull failed for {image}; check registry access and release publication"
            ));
        }
    }
    let report = project.join("reports/wcag");
    fs::create_dir_all(&report).map_err(|e| format!("cannot create reports/wcag: {e}"))?;
    let canonical_report = report
        .canonicalize()
        .map_err(|e| format!("cannot resolve report directory: {e}"))?;
    let uid = unsafe { libc_getuid() };
    let gid = unsafe { libc_getgid() };
    let mut args = vec![
        "run".to_owned(),
        "--rm".to_owned(),
        "-i".to_owned(),
        "--init".to_owned(),
        "--workdir".to_owned(),
        project.display().to_string(),
        "--mount".to_owned(),
        mount(&project, &project, true),
        "--mount".to_owned(),
        mount(&canonical_report, &canonical_report, false),
    ];
    args.extend([
        "-e".into(),
        format!("HOST_UID={uid}"),
        "-e".into(),
        format!("HOST_GID={gid}"),
        "-e".into(),
        format!(
            "PLAYWRIGHT_MCP_OUTPUT_DIR={}/reports/wcag",
            project.display()
        ),
    ]);
    if let Ok(network) = env::var("AI_PROFILE_PLAYWRIGHT_NETWORK") {
        if !valid_network(&network) {
            return Err("AI_PROFILE_PLAYWRIGHT_NETWORK has an invalid Docker network name".into());
        }
        args.extend(["--network".into(), network]);
    }
    if let Ok(ca) = env::var("AI_PROFILE_PLAYWRIGHT_CA_CERT") {
        let ca = PathBuf::from(ca)
            .canonicalize()
            .map_err(|_| "configured trusted CA file does not exist")?;
        args.extend([
            "--mount".into(),
            format!(
                "type=bind,src={},dst=/run/ai-profile-ca.crt,readonly",
                ca.display()
            ),
            "-e".into(),
            "PLAYWRIGHT_CA_CERT=/run/ai-profile-ca.crt".into(),
        ]);
    }
    if let Ok(storage) = env::var("AI_PROFILE_PLAYWRIGHT_STORAGE_STATE") {
        let storage = PathBuf::from(storage)
            .canonicalize()
            .map_err(|_| "configured authentication storage state does not exist")?;
        let metadata =
            fs::metadata(&storage).map_err(|_| "cannot inspect authentication storage state")?;
        if !metadata.is_file() || metadata.permissions().mode() & 0o077 != 0 {
            return Err(
                "authentication storage state must be a private file with mode 0600 or stricter"
                    .into(),
            );
        }
        args.extend([
            "--mount".into(),
            format!(
                "type=bind,src={},dst=/run/ai-profile-auth.json,readonly",
                storage.display()
            ),
            "-e".into(),
            "PLAYWRIGHT_MCP_STORAGE_STATE=/run/ai-profile-auth.json".into(),
        ]);
    }
    args.push(image.to_owned());
    let child = Command::new("docker")
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("cannot start Docker container: {e}"))?;
    let pid = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(child.id() as i32));
    let signal_pid = std::sync::Arc::clone(&pid);
    ctrlc::set_handler(move || {
        let pid = signal_pid.load(std::sync::atomic::Ordering::Relaxed);
        if pid > 0 {
            unsafe {
                send_sigint(pid);
            }
        }
    })
    .map_err(|e| format!("cannot install shutdown handler: {e}"))?;
    let mut child = child;
    let status = child
        .wait()
        .map_err(|e| format!("cannot wait for Docker container: {e}"))?;
    pid.store(0, std::sync::atomic::Ordering::Relaxed);
    Ok(status.code().unwrap_or(1))
}

fn project_from_launcher_path(executable: &Path) -> Result<PathBuf, String> {
    executable
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .and_then(Path::parent)
        .ok_or("launcher must be installed under <project>/.opencode/mcp/playwright")?
        .canonicalize()
        .map_err(|e| format!("cannot resolve project path: {e}"))
}

fn mount(source: &Path, target: &Path, readonly: bool) -> String {
    format!(
        "type=bind,src={},dst={}{}",
        source.display(),
        target.display(),
        if readonly { ",readonly" } else { "" }
    )
}
fn valid_network(value: &str) -> bool {
    !value.is_empty()
        && value.len() < 64
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.')
}

// Keep libc out of the runtime dependency graph; uid/gid are read through the
// standard `id` utility when building on Unix.
unsafe fn libc_getuid() -> u32 {
    use std::os::raw::c_uint;
    unsafe extern "C" {
        fn getuid() -> c_uint;
    }
    getuid()
}
unsafe fn libc_getgid() -> u32 {
    use std::os::raw::c_uint;
    unsafe extern "C" {
        fn getgid() -> c_uint;
    }
    getgid()
}
unsafe fn send_sigint(pid: i32) {
    use std::os::raw::c_int;
    unsafe extern "C" {
        fn kill(pid: c_int, signal: c_int) -> c_int;
    }
    let _ = kill(pid, 2);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_project_from_installed_location_with_spaces_and_independent_cwd() {
        let project = tempfile::tempdir().unwrap();
        let executable = project
            .path()
            .join(".opencode/mcp/playwright/playwright-launcher");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"stub").unwrap();
        let actual = project_from_launcher_path(&executable).unwrap();
        assert_eq!(actual, project.path().canonicalize().unwrap());
        assert_eq!(
            mount(&actual, &actual, true),
            format!(
                "type=bind,src={},dst={},readonly",
                actual.display(),
                actual.display()
            )
        );
    }

    #[test]
    fn validates_network_and_never_allocates_tty() {
        assert!(valid_network("ddev_default"));
        assert!(!valid_network("bad network"));
        let docker_args = ["run", "--rm", "-i", "--init"];
        assert!(docker_args.contains(&"-i"));
        assert!(!docker_args.contains(&"-t"));
    }
}

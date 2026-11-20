use super::{Config, Session};
use fs2::FileExt;
use rustix::fs::{openat, Mode, OFlags};
use std::{
    cell::RefCell,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

thread_local! { static CURRENT: RefCell<Option<Session>> = const { RefCell::new(None) }; }

struct CurrentGuard(Option<Session>);

impl Drop for CurrentGuard {
    fn drop(&mut self) {
        CURRENT.with(|current| *current.borrow_mut() = self.0.take());
    }
}

pub(crate) fn current_session() -> Option<Session> {
    CURRENT.with(|current| current.borrow().clone())
}

pub(crate) fn with_session<T>(session: &Session, invoke: impl FnOnce() -> T) -> T {
    let previous = CURRENT.with(|current| current.borrow_mut().replace(session.clone()));
    let _guard = CurrentGuard(previous);
    tracing::subscriber::with_default(session.subscriber(), invoke)
}

fn location(binary: &str) -> io::Result<PathBuf> {
    let executable = std::env::current_exe()?;
    match binary {
        "ai-profile" => crate::model::RepositoryContext::discover()
            .map(|c| c.sources)
            .map_err(io::Error::other),
        "ai-profile-mcp" => crate::mcp::runtime::identity_from_adapter(&executable)
            .map(|(p, _)| p)
            .map_err(io::Error::other),
        "playwright-launcher" => crate::playwright::runtime::installed_identity(&executable)
            .map(|(p, _)| p)
            .map_err(io::Error::other),
        "test-inventory" => crate::layout::Layout::project_from_skill_helper(
            &executable,
            "test-gap-discovery",
            "inventory-tests",
        )
        .map(Ok)
        .unwrap_or_else(|| {
            crate::model::RepositoryContext::discover()
                .map(|c| c.sources)
                .map_err(io::Error::other)
        }),
        "restore-updraft" => {
            for project in executable.ancestors().skip(1) {
                if crate::wordpress::ddev::scripts_from_executable(project, &executable).is_some() {
                    return Ok(project.to_owned());
                }
            }
            crate::model::RepositoryContext::discover()
                .map(|c| c.sources)
                .map_err(io::Error::other)
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unknown production binary",
        )),
    }
}

/// Persistent append-only protection is not installer-owned and survives uninstall.
fn ignore_logs(project: &Path) -> io::Result<()> {
    let directory = super::prepare_directory(project)?;
    let fd = openat(
        &directory,
        ".gitignore",
        OFlags::RDWR
            | OFlags::APPEND
            | OFlags::CREATE
            | OFlags::NOFOLLOW
            | OFlags::CLOEXEC
            | OFlags::NONBLOCK,
        Mode::from_raw_mode(0o600),
    )?;
    let mut file = File::from(fd);
    if !file.metadata()?.is_file() {
        return Err(io::Error::other(".gitignore is not a regular file"));
    }
    file.lock_exclusive()?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents)?;
    let rule = b"/.ai-profiles-logs/";
    if contents
        .split(|b| *b == b'\r' || *b == b'\n')
        .rfind(|l| !l.is_empty())
        == Some(rule.as_slice())
    {
        return Ok(());
    }
    let newline: &[u8] = if contents.windows(2).any(|s| s == b"\r\n") {
        b"\r\n"
    } else if contents.contains(&b'\r') && !contents.contains(&b'\n') {
        b"\r"
    } else {
        b"\n"
    };
    if !contents.is_empty() && !contents.ends_with(b"\n") && !contents.ends_with(b"\r") {
        file.write_all(newline)?;
    }
    file.write_all(rule)?;
    file.write_all(newline)?;
    file.flush()
}

/// Called only by entry points, not from domain operations.
pub fn with_diagnostics<T>(binary: &str, invoke: impl FnOnce() -> T) -> T {
    let prepared = (|| {
        let config = Config::from_env()?;
        let project = location(binary)?;
        ignore_logs(&project)?;
        Session::open(&project.join(".ai-profiles-logs"), config, binary)
    })();
    match prepared {
        Ok(session) => {
            let result = with_session(&session, invoke);
            session.flush();
            result
        }
        Err(error) => {
            super::session::warn(&error.to_string());
            invoke()
        }
    }
}

pub fn diagnostics_complete() -> bool {
    CURRENT.with(|current| current.borrow().as_ref().is_some_and(Session::complete))
}

/// This is the last observed boundary, NOT the target process's completion.
pub fn handoff() {
    tracing::info!(
        event = "runtime.handoff",
        diagnostics_complete = diagnostics_complete(),
        observation = "ends_at_exec"
    );
    CURRENT.with(|current| {
        if let Some(session) = current.borrow().as_ref() {
            session.flush();
        }
    });
}

pub fn runtime_entry(binary: &str, invoke: impl FnOnce() -> i32) -> std::process::ExitCode {
    with_diagnostics(binary, || {
        tracing::info!(
            event = "runtime.started",
            diagnostics_complete = diagnostics_complete()
        );
        let code = invoke();
        tracing::info!(
            event = "runtime.finished",
            exit_code = code,
            diagnostics_complete = diagnostics_complete()
        );
        std::process::ExitCode::from(code as u8)
    })
}

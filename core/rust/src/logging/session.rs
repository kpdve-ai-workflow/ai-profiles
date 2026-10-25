//! Session ownership is the retention unit. All filesystem traversal is anchored
//! to nofollow directory descriptors; flock protects lifecycle across processes.
use fs2::FileExt;
use rustix::fs::{fchmod, mkdirat, openat, renameat, unlinkat, AtFlags, Mode, OFlags};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    os::fd::{AsRawFd, OwnedFd},
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tracing_subscriber::fmt::MakeWriter;

#[derive(Clone, Debug)]
pub struct Config {
    pub segment_bytes: u64,
    pub session_bytes: u64,
    pub retain_bytes: u64,
    pub retain_seconds: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            segment_bytes: 8 * 1024 * 1024,
            session_bytes: 1024 * 1024 * 1024,
            retain_bytes: 2 * 1024 * 1024 * 1024,
            retain_seconds: 7 * 86400,
        }
    }
}

impl Config {
    pub fn from_env() -> io::Result<Self> {
        let mut config = Self::default();
        for (name, value) in [
            ("AI_PROFILE_LOG_SEGMENT_BYTES", &mut config.segment_bytes),
            ("AI_PROFILE_LOG_SESSION_BYTES", &mut config.session_bytes),
            ("AI_PROFILE_LOG_RETAIN_BYTES", &mut config.retain_bytes),
            ("AI_PROFILE_LOG_RETAIN_SECONDS", &mut config.retain_seconds),
        ] {
            if let Some(raw) = std::env::var_os(name) {
                *value = raw
                    .to_str()
                    .filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
                    .and_then(|s| s.parse().ok())
                    .filter(|v| *v > 0)
                    .ok_or_else(|| {
                        io::Error::new(io::ErrorKind::InvalidInput, format!("invalid {name}"))
                    })?;
            }
        }
        Ok(config)
    }
}

#[derive(Clone)]
pub struct Session {
    inner: Arc<Mutex<State>>,
    prefix: String,
    pub run_id: String,
}
struct State {
    directory: OwnedFd,
    _active: File,
    config: Config,
    file: File,
    segment: u64,
    segment_length: u64,
    total: u64,
    payload_sequence: u64,
    failure: Option<String>,
    writer_failed: bool,
}

#[derive(Debug)]
pub struct PayloadSummary {
    pub payload_id: String,
    pub complete: bool,
    pub length: u64,
    pub sha256: String,
}

fn new_file(directory: &OwnedFd, name: &str) -> io::Result<File> {
    let file = openat(
        directory,
        name,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )?;
    fchmod(&file, Mode::from_raw_mode(0o600))?;
    Ok(file.into())
}

fn lock_file(directory: &OwnedFd, name: &str) -> io::Result<File> {
    let fd = openat(
        directory,
        name,
        OFlags::RDWR | OFlags::CREATE | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
        Mode::from_raw_mode(0o600),
    )?;
    let file = File::from(fd);
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("diagnostic lock is not a file"));
    }
    fchmod(&file, Mode::from_raw_mode(0o600))?;
    Ok(file)
}

fn directory_at(parent: &OwnedFd, name: &str) -> io::Result<OwnedFd> {
    Ok(openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )?)
}

/// Warning is deliberately suppressed if stderr aliases protocol stdout.
/// The best-effort incomplete marker is the remaining local channel.
pub(super) fn warn(message: &str) {
    let distinct = rustix::fs::fstat(std::io::stdout())
        .ok()
        .zip(rustix::fs::fstat(std::io::stderr()).ok())
        .is_some_and(|(a, b)| {
            a.st_dev != b.st_dev || a.st_ino != b.st_ino || a.st_rdev != b.st_rdev
        });
    if distinct {
        let _ = writeln!(
            io::stderr().lock(),
            "WARNING: diagnostics unavailable: {message}"
        );
    }
}

impl State {
    fn degrade(&mut self, error: impl std::fmt::Display) {
        if self.failure.is_none() {
            let message = error.to_string();
            if let Ok(mut marker) = new_file(&self.directory, "incomplete") {
                let _ = marker.write_all(message.as_bytes());
            }
            warn(&message);
            self.failure = Some(message);
        }
    }
    fn reserve(&self, count: usize) -> io::Result<()> {
        if self.total.saturating_add(count as u64) > self.config.session_bytes {
            Err(io::Error::other(
                "diagnostic session byte limit reached; logs incomplete",
            ))
        } else {
            Ok(())
        }
    }
    fn record(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.writer_failed {
            return Ok(());
        }
        self.reserve(bytes.len())?;
        if self.segment_length > 0
            && self.segment_length.saturating_add(bytes.len() as u64) > self.config.segment_bytes
        {
            self.file.flush()?;
            self.segment += 1;
            self.file = new_file(
                &self.directory,
                &format!("events-{:020}.jsonl", self.segment),
            )?;
            self.segment_length = 0;
        }
        self.file.write_all(bytes)?;
        self.segment_length += bytes.len() as u64;
        self.total += bytes.len() as u64;
        Ok(())
    }
}

struct RecordWriter {
    state: Arc<Mutex<State>>,
    record: Vec<u8>,
}
impl Write for RecordWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.record.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
impl Drop for RecordWriter {
    fn drop(&mut self) {
        if let Ok(mut state) = self.state.lock() {
            if let Err(error) = state.record(&self.record) {
                state.writer_failed = true;
                state.degrade(error);
            }
        }
    }
}
#[derive(Clone)]
struct SessionWriter(Arc<Mutex<State>>);
impl<'a> MakeWriter<'a> for SessionWriter {
    type Writer = RecordWriter;
    fn make_writer(&'a self) -> Self::Writer {
        RecordWriter {
            state: Arc::clone(&self.0),
            record: Vec::new(),
        }
    }
}

impl Session {
    pub fn open(directory: &Path, config: Config, binary: &str) -> io::Result<Self> {
        if [
            config.segment_bytes,
            config.session_bytes,
            config.retain_bytes,
            config.retain_seconds,
        ]
        .contains(&0)
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "diagnostic limits must be positive",
            ));
        }
        let root = super::prepare_directory(directory)?;
        if rustix::fs::fstat(&root)?.st_mode & 0o777 != 0o700 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "diagnostic directory must have mode 0700",
            ));
        }
        let global = lock_file(&root, ".lock")?;
        global.lock_exclusive()?;
        retain(&root, &config)?;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        for sequence in 0u64.. {
            let run_id = format!("{}-{timestamp}-{sequence}", std::process::id());
            let name = format!("run-{run_id}");
            match mkdirat(&root, name.as_str(), Mode::from_raw_mode(0o700)) {
                Err(rustix::io::Errno::EXIST) => continue,
                result => result?,
            }
            let directory = directory_at(&root, &name)?;
            fchmod(&directory, Mode::from_raw_mode(0o700))?;
            let active = lock_file(&directory, "active")?;
            active.lock_exclusive()?;
            let mut marker = new_file(&directory, "format")?;
            marker.write_all(b"ai-profiles-diagnostics-v1\n")?;
            let file = new_file(&directory, "events-00000000000000000000.jsonl")?;
            let mut prefix = serde_json::to_string(&super::RunMetadata {
                binary,
                version: env!("CARGO_PKG_VERSION"),
                run_id: &run_id,
            })?;
            prefix.pop();
            prefix.push(',');
            return Ok(Self {
                inner: Arc::new(Mutex::new(State {
                    directory,
                    _active: active,
                    config,
                    file,
                    segment: 0,
                    segment_length: 0,
                    total: 0,
                    payload_sequence: 0,
                    failure: None,
                    writer_failed: false,
                })),
                prefix,
                run_id,
            });
        }
        Err(io::Error::other("exhausted diagnostic run identifiers"))
    }
    /// Reads at most 64 KiB at once, raw payload chunks at most 1 MiB.
    /// A short/failed write never claims completeness or changes the input stream.
    pub fn payload(&self, mut input: impl Read) -> PayloadSummary {
        let sequence = {
            let mut state = self.inner.lock().unwrap();
            let sequence = state.payload_sequence;
            state.payload_sequence += 1;
            sequence
        };
        let payload_id = format!("{}-{sequence}", self.run_id);
        let mut digest = Sha256::new();
        let mut length = 0u64;
        let mut complete = true;
        tracing::subscriber::with_default(self.subscriber(), || {
            tracing::info!(event = "payload.started", payload_id, encoding = "raw");
            let mut buffer = [0u8; 65536];
            let mut index = 0u64;
            let mut eof = false;
            while !eof && complete {
                let name = format!("payload-{sequence:020}-{index:020}.bin");
                let opened = new_file(&self.inner.lock().unwrap().directory, &name);
                let mut file = match opened {
                    Ok(file) => file,
                    Err(error) => {
                        self.degrade(error);
                        complete = false;
                        break;
                    }
                };
                let offset = length;
                let mut chunk_digest = Sha256::new();
                let mut chunk_length = 0u64;
                while chunk_length < 1024 * 1024 {
                    let remaining = (1024 * 1024 - chunk_length) as usize;
                    let capacity = buffer.len().min(remaining);
                    let count = match input.read(&mut buffer[..capacity]) {
                        Ok(0) => {
                            eof = true;
                            break;
                        }
                        Ok(count) => count,
                        Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                        Err(error) => {
                            self.degrade(error);
                            complete = false;
                            break;
                        }
                    };
                    let mut written = 0;
                    let result = (|| -> io::Result<()> {
                        let mut state = self.inner.lock().unwrap();
                        state.reserve(count)?;
                        while written < count {
                            match file.write(&buffer[written..count]) {
                                Ok(0) => {
                                    return Err(io::Error::new(
                                        io::ErrorKind::WriteZero,
                                        "payload writer stopped",
                                    ))
                                }
                                Ok(size) => {
                                    written += size;
                                    state.total += size as u64;
                                }
                                Err(error) if error.kind() == io::ErrorKind::Interrupted => {
                                    continue
                                }
                                Err(error) => return Err(error),
                            }
                        }
                        Ok(())
                    })();
                    digest.update(&buffer[..written]);
                    chunk_digest.update(&buffer[..written]);
                    length += written as u64;
                    chunk_length += written as u64;
                    if let Err(error) = result {
                        self.degrade(error);
                        complete = false;
                        break;
                    }
                }
                if let Err(error) = file.flush() {
                    self.degrade(error);
                    complete = false;
                }
                let sha256 = format!("{:x}", chunk_digest.finalize());
                tracing::info!(
                    event = "payload.chunk",
                    payload_id,
                    index,
                    offset,
                    file = name,
                    length = chunk_length,
                    sha256,
                    status = if complete { "complete" } else { "incomplete" }
                );
                complete &= self.complete();
                index += 1;
            }
        });
        let sha256 = format!("{:x}", digest.finalize());
        complete &= self.complete();
        tracing::subscriber::with_default(self.subscriber(), || {
            tracing::info!(
                event = "payload.finished",
                payload_id,
                length,
                sha256,
                status = if complete { "complete" } else { "incomplete" }
            );
        });
        PayloadSummary {
            payload_id,
            complete: complete && self.complete(),
            length,
            sha256,
        }
    }
    fn degrade(&self, error: impl std::fmt::Display) {
        self.inner.lock().unwrap().degrade(error);
    }
    pub fn complete(&self) -> bool {
        self.inner.lock().unwrap().failure.is_none()
    }
    pub fn flush(&self) {
        let mut state = self.inner.lock().unwrap();
        if let Err(error) = state.file.flush() {
            state.degrade(error);
        }
    }
    pub fn subscriber(&self) -> impl tracing::Subscriber + Send + Sync + 'static {
        tracing_subscriber::fmt()
            .json()
            .event_format(super::WithRunMetadata {
                prefix: self.prefix.clone(),
                json: tracing_subscriber::fmt::format().json(),
            })
            .with_max_level(tracing::Level::TRACE)
            .with_writer(SessionWriter(Arc::clone(&self.inner)))
            .finish()
    }
}

// /proc is used only to enumerate an already opened directory, never to resolve
// a destination from ambient CWD or follow user-supplied symlinks.
fn entries(directory: &OwnedFd) -> io::Result<Vec<std::ffi::OsString>> {
    fs::read_dir(format!("/proc/self/fd/{}", directory.as_raw_fd()))?
        .map(|e| e.map(|e| e.file_name()))
        .collect()
}

fn recognized_file(name: &str) -> bool {
    if matches!(name, "active" | "format" | "incomplete") {
        return true;
    }
    if let Some(index) = name
        .strip_prefix("events-")
        .and_then(|s| s.strip_suffix(".jsonl"))
    {
        return index.len() == 20 && index.bytes().all(|b| b.is_ascii_digit());
    }
    name.strip_prefix("payload-")
        .and_then(|s| s.strip_suffix(".bin"))
        .is_some_and(|s| {
            s.len() == 41
                && s.as_bytes()[20] == b'-'
                && s.bytes()
                    .enumerate()
                    .all(|(i, b)| i == 20 || b.is_ascii_digit())
        })
}

fn retain(root: &OwnedFd, config: &Config) -> io::Result<()> {
    let now = SystemTime::now();
    let mut sessions = Vec::new();
    let mut total = 0u64;
    for name in entries(root)? {
        let Some(text) = name.to_str() else {
            continue;
        };
        if !text
            .strip_prefix("run-")
            .or_else(|| text.strip_prefix("retired-"))
            .is_some_and(|s| {
                s.split('-').count() == 3
                    && s.split('-')
                        .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            })
        {
            continue;
        }
        let Ok(directory) = directory_at(root, text) else {
            continue;
        };
        let Ok(fd) = openat(
            &directory,
            "format",
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        ) else {
            continue;
        };
        let marker = File::from(fd);
        if !marker.metadata()?.is_file() {
            continue;
        }
        let mut bytes = Vec::new();
        marker.take(64).read_to_end(&mut bytes)?;
        if bytes != b"ai-profiles-diagnostics-v1\n" {
            continue;
        }
        let names = entries(&directory)?;
        let mut size = 0u64;
        let mut recognized = true;
        let mut modified = UNIX_EPOCH;
        for entry in &names {
            if !entry.to_str().is_some_and(recognized_file) {
                recognized = false;
                break;
            }
            let stat = rustix::fs::statat(&directory, entry, AtFlags::SYMLINK_NOFOLLOW)?;
            if rustix::fs::FileType::from_raw_mode(stat.st_mode)
                != rustix::fs::FileType::RegularFile
            {
                recognized = false;
                break;
            }
            size = size.saturating_add(stat.st_size.max(0) as u64);
            modified = modified
                .max(UNIX_EPOCH + std::time::Duration::from_secs(stat.st_mtime.max(0) as u64));
        }
        if !recognized {
            continue;
        }
        total = total.saturating_add(size);
        sessions.push((modified, name, directory, names, size));
    }
    sessions.sort_by_key(|s| s.0);
    for (modified, name, directory, mut names, size) in sessions {
        let retired = name.to_str().unwrap().starts_with("retired-");
        if !retired
            && total <= config.retain_bytes
            && now.duration_since(modified).unwrap_or_default().as_secs() <= config.retain_seconds
        {
            continue;
        }
        let active = lock_file(&directory, "active")?;
        if active.try_lock_exclusive().is_err() {
            continue;
        }
        // A prior interrupted GC may already have removed the activity inode.
        // lock_file recreated it; include it in this deletion transaction too.
        if !names.iter().any(|name| name == "active") {
            names.push("active".into());
        }
        // Remove a complete bundle from the visible namespace atomically before
        // unlinking any records or payloads. Interrupted GC is resumed next open.
        let destination = if retired {
            name.clone()
        } else {
            let destination =
                std::ffi::OsString::from(name.to_str().unwrap().replacen("run-", "retired-", 1));
            if entries(root)?.contains(&destination) {
                continue;
            }
            renameat(root, &name, root, &destination)?;
            destination
        };
        // Keep the format marker until last so interrupted cleanup is retryable.
        names.sort_by_key(|n| n == "format");
        for entry in names {
            unlinkat(&directory, entry, AtFlags::empty())?;
        }
        unlinkat(root, destination, AtFlags::REMOVEDIR)?;
        total = total.saturating_sub(size);
    }
    Ok(())
}

#[cfg(test)]
mod io_tests {
    use super::*;

    #[test]
    fn exact_segment_and_session_byte_boundaries() {
        let root = tempfile::tempdir().unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let session = Session::open(
            root.path(),
            Config {
                segment_bytes: 6,
                session_bytes: 9,
                ..Config::default()
            },
            "test",
        )
        .unwrap();
        let mut state = session.inner.lock().unwrap();
        state.record(b"{}\n").unwrap();
        state.record(b"{}\n").unwrap();
        assert_eq!(state.segment, 0);
        state.record(b"{}\n").unwrap();
        assert_eq!(state.segment, 1);
        assert!(state.record(b"{}\n").is_err());
        assert_eq!(state.total, 9);
    }

    #[test]
    fn kernel_enospc_after_successful_open_marks_incomplete() {
        let root = tempfile::tempdir().unwrap();
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
        let session = Session::open(root.path(), Config::default(), "test").unwrap();
        session.inner.lock().unwrap().file = File::options().write(true).open("/dev/full").unwrap();
        tracing::subscriber::with_default(session.subscriber(), || {
            tracing::info!(event = "disk_full");
            assert!(!session.complete());
            tracing::info!(event = "not_retried");
        });
        assert!(session
            .inner
            .lock()
            .unwrap()
            .failure
            .as_ref()
            .unwrap()
            .contains("space"));
        let directory = fs::read_dir(root.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| p.is_dir())
            .unwrap();
        assert!(directory.join("incomplete").is_file());
    }
}

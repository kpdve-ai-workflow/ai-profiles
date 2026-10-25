//! Synchronous, local JSONL logging. The caller chooses the destination and
//! installs the returned subscriber at its entry point (or scopes a dispatcher).
//!
//! Each event has UTC `timestamp`, `level`, `binary`, `version`, and `run_id`.
//! The standard tracing-subscriber JSON schema retains event values in `fields`
//! and context in `span`/`spans`, so event fields cannot overwrite run metadata.
//! All levels, including TRACE, are enabled, independently of RUST_LOG.
//!
//! Files are create-new only; callers must supply a fresh single-component name.
//! Directory components are opened relative to directory descriptors without
//! following symlinks. Parent (`..`) components are rejected. Existing directory
//! permissions are left alone; newly created directories/files have 0700/0600.
//!
//! There is no queue, background thread, or shutdown flush requirement. A mutex
//! serializes whole records within this subscriber. Successful writes reach the
//! OS synchronously, not necessarily stable storage. Runtime I/O failures use
//! tracing-subscriber's stderr reporting; this API returns preparation failures,
//! not delivery acknowledgements. Disk-full/crash durability and a richer
//! incompleteness reporting channel are outside the low-level `build_subscriber`
//! contract. Production entry points use `Session` and `with_diagnostics`, which
//! add common configuration, payloads, rotation, retention and delivery status.

use std::{
    fmt,
    fs::File,
    io,
    path::{Component, Path},
    sync::Mutex,
};

use rustix::fs::{fchmod, mkdirat, open, openat, Mode, OFlags};
use serde::Serialize;
use tracing::{Event, Subscriber};
use tracing_subscriber::{
    fmt::{
        format::{Format, Json, JsonFields, Writer},
        FmtContext, FormatEvent,
    },
    registry::LookupSpan,
};

mod session;
pub use session::{Config, PayloadSummary, Session};
mod entry;
pub use entry::{diagnostics_complete, handoff, runtime_entry, with_diagnostics};

/// Run identity supplied by the binary, not inferred from paths or environment.
#[derive(Clone, Copy, Debug, Serialize)]
pub struct RunMetadata<'a> {
    pub binary: &'a str,
    pub version: &'a str,
    pub run_id: &'a str,
}

/// Prepare a private, new file and build a subscriber without installing it.
///
/// Returns an error on invalid paths, symlinks, existing files, or any filesystem
/// preparation failure. It never retries in another location or truncates a file.
pub fn build_subscriber(
    directory: &Path,
    filename: &str,
    metadata: RunMetadata<'_>,
) -> io::Result<impl Subscriber + Send + Sync + 'static> {
    let mut components = Path::new(filename).components();
    if !matches!(components.next(), Some(Component::Normal(_)))
        || components.next().is_some()
        || filename.contains('/')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "log filename must be a single normal path component",
        ));
    }
    let directory = prepare_directory(directory)?;
    let file = openat(
        &directory,
        filename,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )?;
    fchmod(&file, Mode::from_raw_mode(0o600))?;

    // Only the common fields are ours; tracing-subscriber formats everything
    // else, including typed event values, escaping, timestamps and span updates.
    let mut prefix = serde_json::to_string(&metadata)?;
    prefix.pop(); // the closing brace of the metadata object
    prefix.push(',');
    Ok(tracing_subscriber::fmt()
        .json()
        .event_format(WithRunMetadata {
            prefix,
            json: tracing_subscriber::fmt::format().json(),
        })
        .with_max_level(tracing::Level::TRACE)
        .with_writer(Mutex::new(File::from(file)))
        .finish())
}

fn prepare_directory(path: &Path) -> io::Result<std::os::fd::OwnedFd> {
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "log directory must be nonempty and contain no parent components",
        ));
    }
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut directory = open(
        if path.is_absolute() { "/" } else { "." },
        flags,
        Mode::empty(),
    )?;
    for component in path.components() {
        if let Component::Normal(name) = component {
            let created = match mkdirat(&directory, name, Mode::from_raw_mode(0o700)) {
                Ok(()) => true,
                Err(rustix::io::Errno::EXIST) => false,
                Err(error) => return Err(error.into()),
            };
            // Every lookup is relative to an already opened directory, avoiding
            // check-then-open symlink races in ancestors as well as the leaf.
            let next = openat(&directory, name, flags, Mode::empty())?;
            if created {
                fchmod(&next, Mode::from_raw_mode(0o700))?;
            }
            directory = next;
        }
    }
    Ok(directory)
}

struct WithRunMetadata {
    prefix: String,
    json: Format<Json>,
}

impl<S> FormatEvent<S, JsonFields> for WithRunMetadata
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn format_event(
        &self,
        context: &FmtContext<'_, S, JsonFields>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut record = String::new();
        self.json
            .format_event(context, Writer::new(&mut record), event)?;
        let body = record.strip_prefix('{').ok_or(fmt::Error)?;
        writer.write_str(&self.prefix)?;
        writer.write_str(body)
    }
}

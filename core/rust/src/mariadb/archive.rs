use std::{
    fs,
    io::{self, Cursor, Read, Write},
    path::{Component, Path, PathBuf},
    time::Duration,
};

use flate2::read::GzDecoder;
use tar::Archive;

use crate::{
    codec::sha,
    mariadb::{
        MariaDbError, ARCHIVE_SHA256, ARCHIVE_TIMEOUT_SECONDS, ARCHIVE_URL, COMMIT,
        MAX_ARCHIVE_BYTES,
    },
};

pub(crate) trait SourceDownloader {
    fn download(
        &self,
        url: &str,
        timeout: Duration,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, MariaDbError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HttpSourceDownloader;

impl SourceDownloader for HttpSourceDownloader {
    fn download(
        &self,
        url: &str,
        timeout: Duration,
        maximum_bytes: usize,
    ) -> Result<Vec<u8>, MariaDbError> {
        let config = ureq::Agent::config_builder()
            .timeout_global(Some(timeout))
            .build();
        let agent: ureq::Agent = config.into();
        let response = agent.get(url).call().map_err(|_| MariaDbError::Download)?;
        let reader = response.into_body().into_reader();
        let limit = u64::try_from(maximum_bytes)
            .unwrap_or(u64::MAX)
            .saturating_add(1);
        let mut bytes = Vec::new();
        reader
            .take(limit)
            .read_to_end(&mut bytes)
            .map_err(|_| MariaDbError::Download)?;
        Ok(bytes)
    }
}

pub(crate) fn fetch_source(
    destination: &Path,
    downloader: &impl SourceDownloader,
) -> Result<(), MariaDbError> {
    let archive = downloader.download(
        ARCHIVE_URL,
        Duration::from_secs(ARCHIVE_TIMEOUT_SECONDS),
        MAX_ARCHIVE_BYTES,
    )?;
    if archive.len() > MAX_ARCHIVE_BYTES {
        return Err(MariaDbError::ChecksumMismatch);
    }
    if sha(&archive) != ARCHIVE_SHA256 {
        return Err(MariaDbError::ChecksumMismatch);
    }
    extract_verified_archive(&archive, destination)
}

fn extract_verified_archive(archive_bytes: &[u8], destination: &Path) -> Result<(), MariaDbError> {
    let decoder = GzDecoder::new(Cursor::new(archive_bytes));
    let mut archive = Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|_| MariaDbError::InvalidArchive)?;
    let expected_root = format!("mcp-{COMMIT}");

    for entry in entries {
        let mut entry = entry.map_err(|_| MariaDbError::InvalidArchive)?;
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err(MariaDbError::UnsafeArchive);
        }
        let entry_path = entry
            .path()
            .map_err(|_| MariaDbError::UnsafeArchive)?
            .into_owned();
        let parts = safe_archive_parts(&entry_path, &expected_root)?;
        let mut target = destination.to_owned();
        for part in parts.iter().skip(1) {
            target.push(part);
        }

        if entry_type.is_dir() {
            fs::create_dir_all(&target).map_err(|source| MariaDbError::Io {
                path: target,
                source,
            })?;
        } else {
            let parent = target
                .parent()
                .ok_or(MariaDbError::UnsafeArchive)?
                .to_owned();
            fs::create_dir_all(&parent).map_err(|source| MariaDbError::Io {
                path: parent,
                source,
            })?;
            let mut output = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&target)
                .map_err(|source| MariaDbError::Io {
                    path: target.clone(),
                    source,
                })?;
            io::copy(&mut entry, &mut output).map_err(|source| MariaDbError::Io {
                path: target.clone(),
                source,
            })?;
            output.flush().map_err(|source| MariaDbError::Io {
                path: target,
                source,
            })?;
        }
    }

    if !destination.join("Dockerfile").is_file() {
        return Err(MariaDbError::MissingDockerfile);
    }
    Ok(())
}

fn safe_archive_parts(path: &Path, expected_root: &str) -> Result<Vec<PathBuf>, MariaDbError> {
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::Prefix(_)))
    {
        return Err(MariaDbError::UnsafeArchive);
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(PathBuf::from(part)),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(MariaDbError::UnsafeArchive);
            }
        }
    }
    if parts.first().and_then(|part| part.to_str()) != Some(expected_root)
        || parts.iter().any(|part| part.as_os_str().is_empty())
    {
        return Err(MariaDbError::UnsafeArchive);
    }
    Ok(parts)
}

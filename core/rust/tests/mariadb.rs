use std::time::Duration;

use crate::mariadb::{
    archive::{fetch_source, SourceDownloader},
    runtime::validate_environment,
    MariaDbError, ARCHIVE_SHA256, ARCHIVE_URL,
};

struct FixtureDownloader;

impl SourceDownloader for FixtureDownloader {
    fn download(
        &self,
        _url: &str,
        _timeout: Duration,
        _maximum_bytes: usize,
    ) -> Result<Vec<u8>, MariaDbError> {
        Ok(b"not an archive".to_vec())
    }
}

#[test]
fn archive_identity_is_pinned_and_invalid_archive_fails_closed() {
    assert!(!ARCHIVE_URL.is_empty());
    assert_eq!(ARCHIVE_SHA256.len(), 64);
    let temp = tempfile::tempdir().unwrap();
    assert!(fetch_source(temp.path(), &FixtureDownloader).is_err());
}

#[test]
fn database_settings_reject_invalid_port() {
    let values = std::collections::HashMap::from([
        ("DB_HOST".to_owned(), "localhost".to_owned()),
        ("DB_PORT".to_owned(), "65536".to_owned()),
        ("DB_NAME".to_owned(), "db".to_owned()),
        ("DB_USER".to_owned(), "user".to_owned()),
        ("DB_PASSWORD".to_owned(), "secret".to_owned()),
    ]);
    assert!(validate_environment(&values).is_err());
}

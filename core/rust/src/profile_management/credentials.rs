//! One-time, user-owned Novamira template; never part of managed payload/state.
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

const TEMPLATE: &str = "# Adres endpointu Novamiry, np. https://twoja-witryna/wp-json/mcp/novamira\nWP_API_URL=\n# Login konta WordPress, które wygenerowało hasło aplikacyjne\nWP_API_USERNAME=\n# Hasło aplikacyjne WordPressa — NIE hasło do panelu wp-admin\nWP_API_PASSWORD=\"\"\n";

pub(super) fn check(project: &Path) -> io::Result<()> {
    match fs::symlink_metadata(project.join(".env.novamira")) {
        Ok(metadata) if metadata.file_type().is_file() => Ok(()),
        Ok(_) => Err(io::Error::other(
            ".env.novamira: wymagany zwykły plik, bez symlinków",
        )),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(super) fn prepare(project: &Path) -> io::Result<()> {
    // O_EXCL/create_new rejects even dangling symlinks atomically; mode is
    // private at creation, not repaired after writing. Never open an existing
    // file for read/write or hash its contents.
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(project.join(".env.novamira"))
    {
        Ok(mut file) => file.write_all(TEMPLATE.as_bytes()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => check(project),
        Err(error) => Err(error),
    }
}

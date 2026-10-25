//! Shared, immutable Novamira bootstrap. Secrets are never read, hashed or owned.
use std::{
    fs::{self, OpenOptions},
    io::{self, Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) const ROOT: &str = ".ai-profiles/mcp/novamira";
pub(crate) const ENV_FILE: &str = ".ai-profiles/mcp/novamira/.env.novamira";
pub(crate) const ARCHIVE_SHA: &str = "ad4dac087e9cad073b2206cb75b44bf5224d57880452f1937a038068920caf78";
const ARCHIVE_URL: &str = "https://github.com/use-novamira/novamira/releases/download/v1.12.12/novamira-1.12.12.zip";
const ARCHIVE: &str = ".ai-profiles/mcp/novamira/downloads/novamira.zip";
const MAX_ARCHIVE: u64 = 8 * 1024 * 1024;
const FILES: &[(&str, &[u8], u32)] = &[
    ("Dockerfile", include_bytes!("../../../../mcp/images/novamira/Dockerfile"), 0o644),
    (".dockerignore", include_bytes!("../../../../mcp/images/novamira/.dockerignore"), 0o644),
    ("compose.yaml", include_bytes!("../../../../mcp/images/novamira/compose.yaml"), 0o644),
    ("compose.ca.yaml", include_bytes!("../../../../mcp/images/novamira/compose.ca.yaml"), 0o644),
    ("launcher", include_bytes!("../../../../mcp/images/novamira/launcher"), 0o755),
    ("install-plugin", include_bytes!("../../../../mcp/images/novamira/install-plugin"), 0o755),
    ("README.md", include_bytes!("../../../../mcp/images/novamira/README.md"), 0o644),
];

const TEMPLATE: &str = "# Adres endpointu Novamiry, np. https://twoja-witryna/wp-json/mcp/novamira\nWP_API_URL=\n# Login konta WordPress, które wygenerowało hasło aplikacyjne\nWP_API_USERNAME=\n# Hasło aplikacyjne WordPressa — NIE hasło do panelu wp-admin\nWP_API_PASSWORD=\"\"\n";

pub(super) fn check(project: &Path) -> io::Result<()> {
    for relative in [".ai-profiles", ".ai-profiles/mcp", ROOT, ".ai-profiles/mcp/novamira/.private", ".ai-profiles/mcp/novamira/downloads"] {
        let path = safe_path(project, relative)?;
        if fs::symlink_metadata(path).is_ok_and(|m| !m.is_dir()) {
            return Err(io::Error::other("Novamira: kolizja katalogu bundle"));
        }
    }
    check_regular(&safe_path(project, ENV_FILE)?)?;
    for (name, expected, _) in FILES {
        let path = safe_path(project, &format!("{ROOT}/{name}"))?;
        check_regular(&path)?;
        if path.exists() && fs::read(&path)? != *expected {
            return Err(io::Error::other(format!("Novamira: zmieniony plik bundle {name}; nie zostanie nadpisany")));
        }
    }
    let archive = safe_path(project, ARCHIVE)?;
    check_regular(&archive)?;
    if archive.exists() {
        if fs::metadata(&archive)?.len() > MAX_ARCHIVE || crate::codec::sha(&fs::read(archive)?) != ARCHIVE_SHA {
            return Err(io::Error::other("Novamira: niezgodna suma ZIP-a; nie zostanie nadpisany"));
        }
    }
    Ok(())
}

fn safe_path(project: &Path, relative: &str) -> io::Result<PathBuf> {
    crate::filesystem::file_path(project, relative).map_err(|_| io::Error::other("Novamira: niedozwolona ścieżka lub symlink"))
}

fn check_regular(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_file() => Ok(()),
        Ok(_) => Err(io::Error::other("Novamira: wymagany zwykły plik, bez symlinków")),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

pub(crate) fn archive_for(project: &Path) -> io::Result<Vec<u8>> {
    check(project)?;
    if project.join(ARCHIVE).exists() { return Ok(Vec::new()); }
    download_verified(ARCHIVE_URL, ARCHIVE_SHA)
}

pub(crate) fn download_verified(url: &str, sha: &str) -> io::Result<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(60))).build().into();
    let response = agent.get(url).call().map_err(|_| io::Error::other("Novamira: nie udało się pobrać oficjalnej paczki ZIP"))?;
    let mut bytes = Vec::new();
    response.into_body().into_reader().take(MAX_ARCHIVE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ARCHIVE || crate::codec::sha(&bytes) != sha {
        return Err(io::Error::other("Novamira: niezgodna suma SHA-256 paczki ZIP"));
    }
    Ok(bytes)
}

fn seed(path: &Path, bytes: &[u8], mode: u32) -> io::Result<()> {
    if path.exists() { return Ok(()); }
    let mut file = tempfile::NamedTempFile::new_in(path.parent().ok_or_else(|| io::Error::other("missing parent"))?)?;
    file.write_all(bytes)?;
    file.as_file().set_permissions(fs::Permissions::from_mode(mode))?;
    file.as_file().sync_all()?;
    file.persist_noclobber(path).map_err(|e| e.error)?;
    Ok(())
}

pub(crate) fn prepare(project: &Path, archive: &[u8]) -> io::Result<()> {
    check(project)?;
    let root = safe_path(project, ROOT)?;
    fs::create_dir_all(&root)?;
    let private = safe_path(project, ".ai-profiles/mcp/novamira/.private")?;
    if !private.exists() { fs::DirBuilder::new().mode(0o700).create(&private)?; }
    fs::create_dir_all(safe_path(project, ".ai-profiles/mcp/novamira/downloads")?)?;
    if !project.join(ARCHIVE).exists() {
        if crate::codec::sha(archive) != ARCHIVE_SHA { return Err(io::Error::other("Novamira: brak zweryfikowanej paczki ZIP")); }
        seed(&safe_path(project, ARCHIVE)?, archive, 0o644)?;
    }
    for (name, bytes, mode) in FILES { seed(&root.join(name), bytes, *mode)?; }
    // O_EXCL/create_new rejects even dangling symlinks atomically; mode is
    // private at creation, not repaired after writing. Never open an existing
    // file for read/write or hash its contents.
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(safe_path(project, ENV_FILE)?)
    {
        Ok(mut file) => file.write_all(TEMPLATE.as_bytes()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => check(project),
        Err(error) => Err(error),
    }
}

pub(crate) mod archive;
pub mod runtime;

pub(crate) const COMMIT: &str = "64876fb1a4cb81bf51564c46eb60401097c067cc";
pub(crate) const ARCHIVE_URL: &str =
    "https://codeload.github.com/MariaDB/mcp/tar.gz/64876fb1a4cb81bf51564c46eb60401097c067cc";
pub(crate) const ARCHIVE_SHA256: &str =
    "0730bd3dd546bce18fb3cb4f940fd79980d6114b89909c72da1ebefc1d5fbbc0";
pub(crate) const IMAGE: &str = "ai-profiles/mariadb-mcp:64876fb1a4cb81bf51564c46eb60401097c067cc";
pub(crate) const SOURCE_LABEL: &str = "ai-profiles.mariadb.source";
pub(crate) const PLATFORM: &str = "linux/amd64";
pub(crate) const MAX_ARCHIVE_BYTES: usize = 16 * 1024 * 1024;
pub(crate) const ARCHIVE_TIMEOUT_SECONDS: u64 = 60;
pub(crate) const REQUIRED: [&str; 5] = ["DB_HOST", "DB_PORT", "DB_NAME", "DB_USER", "DB_PASSWORD"];
pub(crate) const CONTAINER_ENV: [&str; 12] = [
    "DB_HOST",
    "DB_PORT",
    "DB_NAME",
    "DB_USER",
    "DB_PASSWORD",
    "DB_CHARSET",
    "DB_SSL",
    "DB_SSL_VERIFY_CERT",
    "DB_SSL_VERIFY_IDENTITY",
    "MCP_READ_ONLY",
    "MCP_BLOCK_SENSITIVE_SHOW",
    "MCP_MAX_POOL_SIZE",
];

#[derive(Debug, thiserror::Error)]
pub(crate) enum MariaDbError {
    #[error("Missing required MariaDB setting: {0}")]
    MissingSetting(&'static str),
    #[error("DB_PORT must be a number between 1 and 65535")]
    InvalidPort,
    #[error("{0} must be true or false")]
    InvalidBoolean(&'static str),
    #[error("MariaDB MCP source checksum mismatch; image was not built")]
    ChecksumMismatch,
    #[error("Unsafe MariaDB MCP source archive")]
    UnsafeArchive,
    #[error("MariaDB MCP source archive has no Dockerfile")]
    MissingDockerfile,
    #[error("Could not download MariaDB MCP source archive")]
    Download,
    #[error("Invalid MariaDB MCP source archive")]
    InvalidArchive,
    #[error("A running Linux Docker daemon is required")]
    DockerUnavailable,
    #[error("MariaDB MCP image build failed; check Docker build progress on stderr")]
    ImageBuildFailed,
    #[error("MariaDB MCP Docker command failed")]
    DockerCommand,
    #[error("MariaDB MCP could not determine a writable cache directory")]
    CacheUnavailable,
    #[error("MariaDB MCP Docker exec failed")]
    Exec,
    #[error("Błąd systemu plików dla {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
}

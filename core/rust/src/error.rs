use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ModelError {
    #[error("Nieprawidłowa nazwa profilu: {0:?}")]
    InvalidProfileName(String),
    #[error("Nieprawidłowa wersja MAJOR.MINOR.PATCH: {0:?}")]
    InvalidVersion(String),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CodecError {
    #[error("Nie mogę odczytać JSON z {path}: {source}")]
    ReadJson {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Nieprawidłowy JSON w {path}: {source}")]
    ParseJson {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Nie mogę serializować JSON: {0}")]
    SerializeJson(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FilesystemError {
    #[error("Nieprawidłowa ścieżka zarządzana: {0}")]
    InvalidManagedPath(String),
    #[error("Odmowa operacji przez symlink: {0}")]
    Symlink(PathBuf),
    #[error("Konflikt: zarządzany plik zmieniono/usunięto: {relative}. Zrób backup i przywróć zarządzaną wersję przed update/remove.")]
    ManagedFileChanged { relative: String },
    #[error("Konflikt z istniejącym niezarządzanym plikiem: {0}")]
    UnmanagedFileExists(String),
    #[error("Błąd systemu plików dla {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProcessError {
    #[error("Nie można uruchomić procesu: {source}")]
    Spawn {
        #[source]
        source: std::io::Error,
    },
    #[error("Proces przekroczył limit czasu")]
    Timeout,
    #[error("Proces został anulowany")]
    Cancelled,
    #[error("Błąd komunikacji z procesem: {source}")]
    Io {
        #[source]
        source: std::io::Error,
    },
    #[error("Wątek obsługujący strumień procesu uległ awarii")]
    StreamThread,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RepositoryError {
    #[error("Nie znaleziono checkoutu ai-profiles w ścieżce {0}")]
    CheckoutNotFound(PathBuf),
    #[error("Nieprawidłowy katalog źródeł profili: {0}")]
    InvalidSourceRoot(PathBuf),
    #[error("Nieprawidłowy profile.json: {0}")]
    InvalidMetadata(String),
    #[error("Błąd systemu plików dla {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Process(#[from] ProcessError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProjectError {
    #[error("Projekt nie istnieje: {0}")]
    ProjectNotFound(PathBuf),
    #[error("Nie instaluj profilu wewnątrz repozytorium ai-profiles")]
    ProjectInsideSources,
    #[error("Nieobsługiwany stan profilu. Nie nadpisuję obcych plików: {0}")]
    UnsupportedState(PathBuf),
    #[error("Wykryto opencode.jsonc. Żeby nie zgubić komentarzy ani zmieszać dwóch konfiguracji, najpierw scal go z opencode.json.")]
    JsoncPresent,
    #[error("opencode.json musi zawierać obiekt JSON")]
    ConfigMustBeObject,
    #[error("opencode.json: mcp musi być obiektem")]
    McpMustBeObject,
    #[error("opencode.json: instructions musi być tablicą")]
    InstructionsMustBeArray,
    #[error("Błąd systemu plików dla {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Filesystem(#[from] FilesystemError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProfileManagementError {
    #[error("Nazwa profilu musi zawierać małe litery/cyfry rozdzielone pojedynczymi myślnikami")]
    InvalidProfileName,
    #[error("Opis profilu nie może być pusty")]
    EmptyDescription,
    #[error("Profil już istnieje: {0}")]
    ProfileAlreadyExists(String),
    #[error("Nieznany profil: {0}")]
    UnknownProfile(String),
    #[error("Nie udało się utworzyć profilu: {destination}; sprawdź jego zawartość")]
    CreateFailed { destination: PathBuf },
    #[error("Błąd systemu plików dla {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Nie udało się zapisać tymczasowych metadanych profilu: {0}")]
    Persist(String),
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Filesystem(#[from] FilesystemError),
    #[error(transparent)]
    Model(#[from] ModelError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum McpConfigError {
    #[error("Nieprawidłowa nazwa definicji MCP; użyj małych liter/cyfr i pojedynczych myślników")]
    InvalidName,
    #[error("Brak definicji MCP: {name} (mcp/servers/{name}.json)")]
    MissingDefinition { name: String },
    #[error("Nieprawidłowa MCP dla {profile}: oczekiwano listy servers z nazwami z mcp/servers/")]
    InvalidManifest { profile: String },
    #[error("Powielona nazwa MCP w profilu {profile}: {name}")]
    DuplicateName { profile: String, name: String },
    #[error("Invalid MCP type: {0}")]
    InvalidType(String),
    #[error("MCP command must be a nonempty string array: {0}")]
    InvalidCommand(String),
    #[error("Missing remote MCP URL: {0}")]
    MissingRemoteUrl(String),
    #[error("access_mode is supported only for local postgres MCP")]
    AccessModeUnsupported,
    #[error("PostgreSQL access_mode must be restricted or unrestricted")]
    InvalidAccessMode,
    #[error("Missing PostgreSQL --access-mode value")]
    MissingCommandAccessMode,
    #[error("Duplicate PostgreSQL --access-mode arguments; specify exactly one mode")]
    DuplicateCommandAccessMode,
    #[error("PostgreSQL --access-mode must be restricted or unrestricted")]
    InvalidCommandAccessMode,
    #[error("MCP {field} must map names to strings: {name}")]
    InvalidStringMap { field: &'static str, name: String },
    #[error("Invalid environment variable name: {0}")]
    InvalidEnvironmentName(String),
    #[error("MCP {field} must be boolean: {name}")]
    InvalidBoolean { field: &'static str, name: String },
    #[error("Invalid MCP cwd: {0}")]
    InvalidCwd(String),
    #[error("Invalid MCP timeout: {0}")]
    InvalidTimeout(String),
    #[error("Invalid MCP OAuth settings: {0}")]
    InvalidOAuth(String),
    #[error("Invalid MCP protocol: {0}")]
    InvalidProtocol(String),
    #[error("env_file is supported only for local MCP: {0}")]
    RemoteEnvFile(String),
    #[error("Expected a relative path inside the project")]
    InvalidProjectPath,
    #[error("Symlinks are not supported for MCP configuration")]
    ProjectSymlink,
    #[error("ai-profile.local.json must contain an mcp object")]
    InvalidLocalManifest,
    #[error("Local override targets an unmanaged or removed MCP: {0}")]
    UnmanagedOverride(String),
    #[error("Invalid local MCP fields: {0}")]
    InvalidOverrideFields(String),
    #[error("Local override cannot change MCP transport type: {0}")]
    OverrideTransportChange(String),
    #[error("Konflikt: konfiguracja MCP '{0}' była zmieniona. Nie nadpisuję jej automatycznie.")]
    ManagedMcpChanged(String),
    #[error("MCP '{0}' już istnieje i nie należy do managera.")]
    UnmanagedMcpExists(String),
    #[error("Nieprawidłowy obiekt MCP: {0}")]
    InvalidMcpObject(String),
    #[error("Błąd systemu plików dla {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Filesystem(#[from] FilesystemError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

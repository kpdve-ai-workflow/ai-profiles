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

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
pub(crate) enum PayloadError {
    #[error("Brak katalogu skills: {0}")]
    MissingSkills(PathBuf),
    #[error("Nieprawidłowy katalog skilla: {0}")]
    InvalidSkillDirectory(PathBuf),
    #[error("Brak lub nieprawidłowy SKILL.md w {0}")]
    MissingSkillDocument(PathBuf),
    #[error("Nieprawidłowy YAML frontmatter: {0}")]
    InvalidFrontmatter(PathBuf),
    #[error("SKILL.md musi mieć name={name} i description: {path}")]
    InvalidSkillMetadata { name: String, path: PathBuf },
    #[error("Skill używa symlinka: {0}")]
    SkillSymlink(PathBuf),
    #[error("Duplikat pliku skilla: {0}")]
    DuplicateSkillFile(String),
    #[error("Brak zarządzanego pliku launchera MCP: {0}")]
    MissingManagedLauncher(String),
    #[error("Nieprawidłowy katalog launchera MCP: {0}")]
    InvalidLauncherDirectory(String),
    #[error("Launcher MCP używa symlinka: {0}")]
    LauncherSymlink(PathBuf),
    #[error("Nieprawidłowy katalog scripts: {0}")]
    InvalidScriptsDirectory(String),
    #[error("Skrypt profilu używa symlinka: {0}")]
    ScriptSymlink(PathBuf),
    #[error("Nieprawidłowa ścieżka payloadu: {0}")]
    InvalidPayloadPath(PathBuf),
    #[error("Błąd systemu plików dla {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Filesystem(#[from] FilesystemError),
    #[error(transparent)]
    Mcp(#[from] McpConfigError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InstallationError {
    #[error("Downgrade niedozwolony: zainstalowana wersja jest wyższa od dostępnej")]
    Downgrade,
    #[error("Konflikt: ścieżka nie jest plikiem: {0}")]
    DestinationNotFile(String),
    #[error("Konflikt: ścieżka nie jest katalogiem: {0}")]
    ParentNotDirectory(PathBuf),
    #[error("Nie można odczytać .gitignore jako UTF-8")]
    GitignoreRead,
    #[error("Nieprawidłowy stan profilu instalacji")]
    InvalidState,
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
    Mcp(#[from] McpConfigError),
    #[error(transparent)]
    Payload(#[from] PayloadError),
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Model(#[from] ModelError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum UninstallError {
    #[error("Brak zainstalowanego profilu w tym projekcie")]
    NoInstalledProfile,
    #[error("Nieprawidłowy stan profilu instalacji")]
    InvalidState,
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
    Mcp(#[from] McpConfigError),
    #[error(transparent)]
    Project(#[from] ProjectError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StatusError {
    #[error("Nieprawidłowy stan profilu")]
    InvalidState,
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
    Mcp(#[from] McpConfigError),
    #[error(transparent)]
    Payload(#[from] PayloadError),
    #[error(transparent)]
    Project(#[from] ProjectError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Model(#[from] ModelError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ValidationError {
    #[error("Nieznany profil: {0}")]
    UnknownProfile(String),
    #[error("Nieprawidłowy mcp/example.json")]
    InvalidExample,
    #[error("Nieprawidłowa nazwa definicji MCP: {0}")]
    InvalidRegistryName(PathBuf),
    #[error("mcp/example.json różni się od MCP wybranych przez profil {0}")]
    ScopedExampleMismatch(String),
    #[error("mcp/example.json różni się od definicji MCP wybranych przez profile")]
    FullExampleMismatch,
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
    Mcp(#[from] McpConfigError),
    #[error(transparent)]
    Payload(#[from] PayloadError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum WordPressError {
    #[error("Mapowanie wymaga origin URL, np. https://example.pl (bez ścieżki)")]
    InvalidOrigin,
    #[error("Użyj --map https://zrodlo.pl=https://projekt.ddev.site")]
    InvalidMappingSyntax,
    #[error("Powielone lub identyczne mapowanie domen")]
    DuplicateOrIdentityMapping,
    #[error("Jeden host źródłowy nie może wskazywać różnych hostów lokalnych")]
    SourceHostConflict,
    #[error("Wymagane są niełańcuchowe, jawne mapowania domen")]
    MissingOrChainedMappings,
    #[error("Nie łącz różnych witryn pod jednym hostem w tym importerze")]
    TargetHostCollision,
    #[error(
        "Backup zawiera URL witryny bez jawnego mapowania; baza wymaga odtworzenia lub dokończenia"
    )]
    UnmappedUrl,
    #[error("Brak lub pusty plik backupu")]
    EmptyDump,
    #[error("Obsługiwane są tylko nieszyfrowane .sql lub .gz, nie ZIP/.crypt")]
    UnsupportedDumpType,
    #[error("Dump zawiera niedozwolone dyrektywy baz/użytkowników lub klienta SQL")]
    ForbiddenDumpDirective,
    #[error("Nie rozpoznano jednej kompletnej bazy WP; sprawdź pełny dump i prefiks")]
    IncompleteDatabase,
    #[error("Niekompletny backup multisite")]
    IncompleteMultisite,
    #[error("Błąd odczytu dumpa: {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Błąd kroku DDEV: {step}; przerwano (szczegóły nie są wypisywane z uwagi na sekrety)")]
    DdevStepFailed { step: String },
    #[error(
        "WP-CLI zgłosił ostrzeżenie/błąd search-replace; nie uznaję częściowej zamiany za sukces"
    )]
    SearchReplaceWarning,
    #[error("Nie rozpoznano JSON ddev describe")]
    InvalidDdevDescribe,
    #[error("Nieprawidłowy wynik zapytania WP-CLI")]
    InvalidQueryOutput,
    #[error("Nieprawidłowy identyfikator bloga")]
    InvalidBlogId,
    #[error("Brak home/siteurl dla jednej z witryn")]
    MissingSiteOptions,
    #[error("Nie można uruchomić DDEV")]
    DdevUnavailable,
    #[error("Wymagany jest istniejący projekt DDEV")]
    DdevProjectRequired,
    #[error("Najpierw zainstaluj/zaktualizuj profil WordPress w tym projekcie")]
    MissingWordPressHelpers,
    #[error("Cel musi być wskazanym, uruchomionym projektem DDEV typu wordpress")]
    InvalidDdevProject,
    #[error("Wersja 1 obsługuje standardowy WordPress z wp-config.php w docroot")]
    MissingWpConfig,
    #[error("Brak standardowego katalogu wp-content")]
    MissingWpContent,
    #[error("Nieprawidłowe URL w konfiguracji DDEV")]
    InvalidDdevUrls,
    #[error(
        "Każdy docelowy host musi występować w ddev describe; nie zmieniamy konfiguracji DDEV"
    )]
    UnknownDdevTargetHost,
    #[error("Nieprawidłowy prefiks tabel lokalnej bazy")]
    InvalidLocalTablePrefix,
    #[error("WordPress i DDEV nie wskazują tej samej lokalnej bazy db")]
    DatabaseMismatch,
    #[error("Dump i lokalny WordPress muszą mieć ten sam prefiks i tryb single/multisite")]
    DumpTopologyMismatch,
    #[error("Lokalny plik ochrony jest inny; nie nadpisuję go")]
    SafetyFileConflict,
    #[error("Nieprawidłowe pola ddev describe")]
    InvalidDdevFields,
    #[error("Brak mapowania hosta bloga multisite")]
    MissingBlogHostMapping,
    #[error("Brak mapowania lub nieprawidłowy identyfikator sieci multisite")]
    InvalidNetworkMapping,
    #[error("Search-replace nie zastąpił wszystkich żądanych URL")]
    SearchReplaceIncomplete,
    #[error("Zmieniła się liczba blogów podczas importu")]
    SiteCountChanged,
    #[error("Weryfikacja URL po imporcie nie powiodła się")]
    SiteVerificationFailed,
    #[error("Weryfikacja domen sieci multisite nie powiodła się")]
    NetworkVerificationFailed,
    #[error("Brak witryn do lokalizacji")]
    NoSitesToLocalize,
    #[error("Nieprawidłowa nazwa projektu DDEV")]
    InvalidDdevName,
    #[error("Katalog backupów XDG_STATE_HOME musi być poza projektem i docroot")]
    BackupDirectoryInsideProject,
    #[error("Inny import tego projektu już działa")]
    RestoreAlreadyRunning,
    #[error("Źródło lub cel zmieniły się po potwierdzeniu; uruchom plan ponownie")]
    RestorePlanChanged,
    #[error("Plik źródłowy zmienił się podczas kopiowania")]
    IncomingChanged,
    #[error("Backup lokalnej bazy nie odpowiada potwierdzonemu celowi")]
    BackupMismatch,
    #[error("Plik ochrony zmienił się po potwierdzeniu; nie nadpisuję go")]
    SafetyChanged,
    #[error("Nieprawidłowa topologia zaimportowanej bazy")]
    ImportedTopologyMismatch,
    #[error("Anulowano; baza nie została zmieniona")]
    RestoreCancelled,
    #[error("Nie można przygotować prywatnego katalogu backupu")]
    BackupDirectory,
    #[error("Nie można zablokować katalogu backupu")]
    BackupLock,
    #[error("Nie można przygotować plików restore")]
    RestoreIo,
    #[error("Brak danych do potwierdzenia restore")]
    ConfirmationInput,
    #[error("Import przerwany. Brak automatycznego rollbacku. Przywróć backup ręcznie.")]
    RecoveryRequired,
    #[error(transparent)]
    Filesystem(#[from] crate::error::FilesystemError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MigrationError {
    #[error("Migracja starej instalacji wymaga --accept-existing po przeglądzie plików")]
    AcceptanceRequired,
    #[error("Brak starego profilu")]
    MissingLegacyState,
    #[error("Nie wykryto starego formatu profilu (zainstalowany nowy lub niepoprawny)")]
    NotLegacyFormat,
    #[error("Nieznany format starego profilu")]
    UnknownLegacyFormat,
    #[error("Nie odnaleziono dotychczasowej konfiguracji MCP")]
    MissingMcpConfiguration,
    #[error("Brak pliku dotychczasowego profilu: {0}")]
    MissingLegacyFile(String),
    #[error("Nieprawidłowy stary stan profilu")]
    InvalidLegacyState,
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
    Mcp(#[from] McpConfigError),
    #[error(transparent)]
    Project(#[from] ProjectError),
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
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Filesystem(#[from] FilesystemError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum EnvironmentError {
    #[error("Cannot read env file: {filename}")]
    ReadEnvFile { filename: String },
    #[error("Invalid env entry at line {line}")]
    InvalidEntry { line: usize },
    #[error("Invalid env quoting at line {line}")]
    InvalidQuoting { line: usize },
    #[error("Invalid env value at line {line}")]
    InvalidValue { line: usize },
    #[error("Required environment variable is missing or empty: {0}")]
    MissingVariable(String),
    #[error("Invalid MCP environment mapping")]
    InvalidMapping,
    #[error(transparent)]
    McpConfig(#[from] McpConfigError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum McpRuntimeError {
    #[error("MCP startup failed: check working directory and executable availability.")]
    ProjectLocation,
    #[error("Cannot read valid JSON: ai-profile-state.json")]
    StateRead,
    #[error("Missing or invalid mcp_base in profile state; run ai-profiles update")]
    InvalidState,
    #[error("Unknown local MCP server")]
    UnknownLocalServer,
    #[error("MCP server is disabled locally")]
    Disabled,
    #[error("MCP startup failed: check working directory and executable availability.")]
    Exec,
    #[error(transparent)]
    Environment(#[from] EnvironmentError),
    #[error(transparent)]
    Config(#[from] McpConfigError),
}

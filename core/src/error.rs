use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ModelError {
    #[error("Invalid profile name: {0:?}")]
    InvalidProfileName(String),
    #[error("Invalid MAJOR.MINOR.PATCH version: {0:?}")]
    InvalidVersion(String),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CodecError {
    #[error("Failed to read JSON from {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid JSON in {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("Failed to serialize JSON: {0}")]
    Serialize(#[from] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum FilesystemError {
    #[error("Invalid managed path: {0}")]
    InvalidManagedPath(String),
    #[error("Operation refused because of symlink: {0}")]
    Symlink(PathBuf),
    #[error("Conflict: managed file was changed or removed: {relative}. Back up and restore the managed version before updating or removing it.")]
    ManagedFileChanged { relative: String },
    #[error("Conflict with existing unmanaged file: {0}")]
    UnmanagedFileExists(String),
    #[error("Filesystem error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProcessError {
    #[error("Failed to start process: {source}")]
    Spawn {
        #[source]
        source: std::io::Error,
    },
    #[error("Process timed out")]
    Timeout,
    #[error("Process was cancelled")]
    Cancelled,
    #[error("Process communication error: {source}")]
    Io {
        #[source]
        source: std::io::Error,
    },
    #[error("Process stream handling thread panicked")]
    StreamThread,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum RepositoryError {
    #[error("Could not find an ai-profiles checkout at {0}")]
    CheckoutNotFound(PathBuf),
    #[error("Invalid profile source directory: {0}")]
    InvalidSourceRoot(PathBuf),
    #[error("Invalid profile.json: {0}")]
    InvalidMetadata(String),
    #[error("Filesystem error for {path}: {source}")]
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
    #[error("Project does not exist: {0}")]
    ProjectNotFound(PathBuf),
    #[error("Profiles cannot be installed inside the ai-profiles repository")]
    ProjectInsideSources,
    #[error("Unsupported profile state. Will not overwrite files owned by others: {0}")]
    UnsupportedState(PathBuf),
    #[error("Found opencode.jsonc. Merge it into opencode.json first to preserve comments and avoid combining two configurations.")]
    JsoncPresent,
    #[error(".gitignore exists but is not a regular file")]
    GitignoreNotFile,
    #[error("opencode.json must contain a JSON object")]
    ConfigMustBeObject,
    #[error("opencode.json: mcp must be an object")]
    McpMustBeObject,
    #[error("opencode.json: instructions must be an array")]
    InstructionsMustBeArray,
    #[error("Filesystem error for {path}: {source}")]
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
    #[error("The new profile name must differ from the current name")]
    SameProfileName,
    #[error("Failed to save metadata ({source}) and failed to restore the directory name ({rollback}). The profile remains at {destination}; restore it manually to {original}.")]
    RenameRollback {
        original: PathBuf,
        destination: PathBuf,
        #[source]
        source: std::io::Error,
        rollback: std::io::Error,
    },
    #[error("Profile names must contain lowercase letters or digits separated by single hyphens")]
    InvalidProfileName,
    #[error("Profile description cannot be empty")]
    EmptyDescription,
    #[error("Profile already exists: {0}")]
    ProfileAlreadyExists(String),
    #[error("Unknown profile: {0}")]
    UnknownProfile(String),
    #[error("Failed to create profile at {destination}; check its contents")]
    CreateFailed { destination: PathBuf },
    #[error("Filesystem error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Failed to save temporary profile metadata: {0}")]
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
    #[error("Profile {profile}: failed to read skill manifest {path}: {source}")]
    SkillManifestRead {
        profile: String,
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Profile {profile}: invalid skill manifest {path}")]
    InvalidSkillManifest { profile: String, path: PathBuf },
    #[error("Profile {profile}: invalid skill name {name:?} in {path}")]
    InvalidSelectedSkill {
        profile: String,
        path: PathBuf,
        name: String,
    },
    #[error("Profile {profile}: duplicate skill {name:?} in {path}")]
    DuplicateSelectedSkill {
        profile: String,
        path: PathBuf,
        name: String,
    },
    #[error("Profile {profile}: unknown skill {name:?} in {path}")]
    UnknownSelectedSkill {
        profile: String,
        path: PathBuf,
        name: String,
    },
    #[error("Profile {profile}: symlink found in skill manifest or path {path}")]
    SelectedSkillSymlink { profile: String, path: PathBuf },
    #[error("Native executable {0} is missing; run make rust-build")]
    MissingNativeBinary(PathBuf),
    #[error("Python is not allowed in the installed payload: {0}")]
    PythonPayload(PathBuf),
    #[error("Invalid skill directory: {0}")]
    InvalidSkillDirectory(PathBuf),
    #[error("Missing or invalid SKILL.md in {0}")]
    MissingSkillDocument(PathBuf),
    #[error("Invalid YAML front matter: {0}")]
    InvalidFrontmatter(PathBuf),
    #[error("SKILL.md must define name={name} and description: {path}")]
    InvalidSkillMetadata { name: String, path: PathBuf },
    #[error("Skill uses a symlink: {0}")]
    SkillSymlink(PathBuf),
    #[error("Duplicate skill file: {0}")]
    DuplicateSkillFile(String),
    #[error("Managed MCP launcher file is missing: {0}")]
    MissingManagedLauncher(String),
    #[error("Invalid MCP launcher directory: {0}")]
    InvalidLauncherDirectory(String),
    #[error("MCP launcher uses a symlink: {0}")]
    LauncherSymlink(PathBuf),
    #[error("Invalid scripts directory: {0}")]
    InvalidScriptsDirectory(String),
    #[error("Profile script uses a symlink: {0}")]
    ScriptSymlink(PathBuf),
    #[error("Invalid payload path: {0}")]
    InvalidPayloadPath(PathBuf),
    #[error("Filesystem error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Filesystem(#[from] FilesystemError),
    #[error(transparent)]
    Codec(#[from] CodecError),
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Mcp(#[from] McpConfigError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum InstallationError {
    #[error("Downgrade is not allowed: the installed version is newer than the available version")]
    Downgrade,
    #[error("Conflict: path is not a file: {0}")]
    DestinationNotFile(String),
    #[error("Conflict: path is not a directory: {0}")]
    ParentNotDirectory(PathBuf),
    #[error("Failed to read .gitignore as UTF-8")]
    #[cfg(test)]
    GitignoreRead,
    #[error("Invalid installed profile state")]
    InvalidState,
    #[error("Native plugin configuration error: {0}")]
    Native(String),
    #[error("Filesystem error for {path}: {source}")]
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
    #[error("No profile is installed in this project")]
    NoInstalledProfile,
    #[error("Invalid installed profile state")]
    InvalidState,
    #[error("Native plugin configuration error: {0}")]
    Native(String),
    #[error("Filesystem error for {path}: {source}")]
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
    #[error("Invalid profile state")]
    InvalidState,
    #[error("Native plugin configuration error: {0}")]
    Native(String),
    #[error("Filesystem error for {path}: {source}")]
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
    #[error("Unknown profile: {0}")]
    UnknownProfile(String),
    #[error("Invalid mcp/example.json")]
    InvalidExample,
    #[error("Invalid MCP definition name: {0}")]
    InvalidRegistryName(PathBuf),
    #[error("mcp/example.json does not match the MCP servers selected by profile {0}")]
    ScopedExampleMismatch(String),
    #[error("mcp/example.json does not match the MCP definitions selected by the profiles")]
    FullExampleMismatch,
    #[error("Filesystem error for {path}: {source}")]
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
    #[error("Mapping requires an origin URL, for example https://example.com (without a path)")]
    InvalidOrigin,
    #[error("Use --map https://source.example=https://project.ddev.site")]
    InvalidMappingSyntax,
    #[error("Duplicate or identical domain mapping")]
    DuplicateOrIdentityMapping,
    #[error("A source host cannot map to multiple local hosts")]
    SourceHostConflict,
    #[error("Explicit, non-chained domain mappings are required")]
    MissingOrChainedMappings,
    #[error("This importer cannot combine different sites under one host")]
    TargetHostCollision,
    #[error(
        "Backup contains a site URL without an explicit mapping; the database must be restored or the operation completed"
    )]
    UnmappedUrl,
    #[error("Backup file is missing or empty")]
    EmptyDump,
    #[error("Only unencrypted .sql or .gz files are supported; ZIP and .crypt files are not")]
    UnsupportedDumpType,
    #[error("Dump contains forbidden database or user directives, or SQL client commands")]
    ForbiddenDumpDirective,
    #[error(
        "Could not identify one complete WordPress database; check the full dump and table prefix"
    )]
    IncompleteDatabase,
    #[error("Incomplete multisite backup")]
    IncompleteMultisite,
    #[error("Failed to read database dump {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("DDEV step failed: {step}; operation aborted (details are hidden to protect secrets)")]
    DdevStepFailed { step: String },
    #[error(
        "WP-CLI reported a search-replace warning or error; a partial replacement is not considered successful"
    )]
    SearchReplaceWarning,
    #[error("Could not parse ddev describe JSON")]
    InvalidDdevDescribe,
    #[error("Invalid WP-CLI query result")]
    InvalidQueryOutput,
    #[error("Invalid blog ID")]
    InvalidBlogId,
    #[error("A site is missing home or siteurl")]
    MissingSiteOptions,
    #[error("Failed to start DDEV")]
    DdevUnavailable,
    #[error("An existing DDEV project is required")]
    DdevProjectRequired,
    #[error("Install or update the WordPress profile in this project first")]
    MissingWordPressHelpers,
    #[error("The target must be the specified, running DDEV project of type wordpress")]
    InvalidDdevProject,
    #[error("Version 1 supports standard WordPress with wp-config.php in the document root")]
    MissingWpConfig,
    #[error("Standard wp-content directory is missing")]
    MissingWpContent,
    #[error("Invalid URLs in DDEV configuration")]
    InvalidDdevUrls,
    #[error(
        "Each target host must appear in ddev describe; DDEV configuration will not be changed"
    )]
    UnknownDdevTargetHost,
    #[error("Invalid local database table prefix")]
    InvalidLocalTablePrefix,
    #[error("WordPress and DDEV do not point to the same local db database")]
    DatabaseMismatch,
    #[error("The dump and local WordPress must use the same table prefix and single-site or multisite mode")]
    DumpTopologyMismatch,
    #[error("Local safety file differs; it will not be overwritten")]
    SafetyFileConflict,
    #[error("Invalid ddev describe fields")]
    InvalidDdevFields,
    #[error("Missing host mapping for a multisite blog")]
    MissingBlogHostMapping,
    #[error("Missing mapping or invalid multisite network ID")]
    InvalidNetworkMapping,
    #[error("Search-replace did not replace all requested URLs")]
    SearchReplaceIncomplete,
    #[error("The number of blogs changed during import")]
    SiteCountChanged,
    #[error("Post-import URL verification failed")]
    SiteVerificationFailed,
    #[error("Multisite network domain verification failed")]
    NetworkVerificationFailed,
    #[error("No sites to localize")]
    NoSitesToLocalize,
    #[error("Invalid DDEV project name")]
    InvalidDdevName,
    #[error("The XDG_STATE_HOME backup directory must be outside the project and document root")]
    BackupDirectoryInsideProject,
    #[error("Another import for this project is already running")]
    RestoreAlreadyRunning,
    #[error("The source or target changed after confirmation; run the plan again")]
    RestorePlanChanged,
    #[error("The source file changed during copying")]
    IncomingChanged,
    #[error("The local database backup does not match the confirmed target")]
    BackupMismatch,
    #[error("The safety file changed after confirmation; it will not be overwritten")]
    SafetyChanged,
    #[error("Invalid topology in the imported database")]
    ImportedTopologyMismatch,
    #[error("Cancelled; the database was not changed")]
    RestoreCancelled,
    #[error("Failed to prepare the private backup directory")]
    BackupDirectory,
    #[error("Failed to lock the backup directory")]
    BackupLock,
    #[error("Failed to prepare restore files")]
    RestoreIo,
    #[error("Missing input to confirm the restore")]
    ConfirmationInput,
    #[error("Import aborted. No automatic rollback was performed. Restore the backup manually.")]
    RecoveryRequired,
    #[error(transparent)]
    Filesystem(#[from] crate::error::FilesystemError),
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum MigrationError {
    #[error(
        "Migrating a legacy installation requires --accept-existing after reviewing the files"
    )]
    AcceptanceRequired,
    #[error("Legacy profile is missing")]
    MissingLegacyState,
    #[error(
        "Legacy profile format was not detected (the profile may be newly installed or invalid)"
    )]
    NotLegacyFormat,
    #[error("Unknown legacy profile format")]
    UnknownLegacyFormat,
    #[error("Could not find the existing MCP configuration")]
    MissingMcpConfiguration,
    #[error("Legacy profile file is missing: {0}")]
    MissingLegacyFile(String),
    #[error("Invalid legacy profile state")]
    InvalidLegacyState,
    #[error("Filesystem error for {path}: {source}")]
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
    #[error(
        "Invalid MCP definition name; use lowercase letters or digits separated by single hyphens"
    )]
    InvalidName,
    #[error("MCP definition is missing: {name} (mcp/servers/{name}.json)")]
    MissingDefinition { name: String },
    #[error(
        "Invalid MCP manifest for {profile}: expected a servers list with names from mcp/servers/"
    )]
    InvalidManifest { profile: String },
    #[error("Duplicate MCP name in profile {profile}: {name}")]
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
    #[error("MCP {server} cannot be represented for {client}: unsupported field {field}")]
    UnsupportedClientField {
        server: String,
        client: String,
        field: String,
    },
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
    #[error(
        "Conflict: MCP configuration '{0}' has changed. It will not be overwritten automatically."
    )]
    ManagedMcpChanged(String),
    #[error("MCP '{0}' already exists and is not managed by this tool.")]
    UnmanagedMcpExists(String),
    #[error("Invalid MCP object: {0}")]
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
    #[error("Cannot read env file: {filename}. Create it in the project root with the required MCP values.")]
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
    #[error("Novamira initialization rejected (safe MCP error already sent)")]
    NovamiraRejected,
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

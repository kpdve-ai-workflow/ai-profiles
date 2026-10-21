use std::{
    env,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    sync::mpsc,
    thread,
    time::Duration,
};

use fs2::FileExt;
use serde_json::Value;

use crate::{
    error::WordPressError,
    process::{CancellationToken, ProcessRunner, SystemProcessRunner},
    wordpress::{
        ddev::{topology, Ddev},
        dump::{file_hash, inspect_dump},
        localize::localize,
        mapping::mappings,
        preflight::{inside, preflight, PreflightResult},
        DumpCompression, SiteMode, UrlMappings,
    },
};

#[cfg(test)]
#[path = "../../tests/wordpress_restore.rs"]
mod restore_tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RestoreMode {
    Plan,
    Apply,
}

#[derive(Debug)]
struct RunDirectory(PathBuf);

static RUN_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(crate) fn restore(
    project: &Path,
    dump: &Path,
    mappings: &UrlMappings,
    mode: RestoreMode,
    runner: &dyn ProcessRunner,
    cancellation: CancellationToken,
) -> Result<Option<PathBuf>, WordPressError> {
    let mut ddev = Ddev::new(project, runner, cancellation.clone());
    let initial = preflight(project, dump, mappings, &mut ddev)?;
    let name = project_name(&initial.info)?;
    print_plan(project, dump, mappings, &name, &initial);
    if mode == RestoreMode::Plan {
        println!("PLAN: bez zapisu. Dodaj --apply, aby wykonać import z potwierdzeniem.");
        return Ok(None);
    }

    let mut answer = String::new();
    print!("Lokalna baza zostanie ZASTĄPIONA. Wpisz nazwę projektu {name}: ");
    io::stdout()
        .flush()
        .map_err(|_| WordPressError::ConfirmationInput)?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = io::stdin().read_line(&mut answer);
        let _ = sender.send((result, answer));
    });
    let answer = loop {
        if cancellation.is_cancelled() {
            return Err(WordPressError::RestoreCancelled);
        }
        match receiver.recv_timeout(Duration::from_millis(100)) {
            Ok((Ok(_), answer)) => break answer,
            Ok((Err(_), _)) => return Err(WordPressError::ConfirmationInput),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(WordPressError::ConfirmationInput)
            }
        }
    };
    if answer.trim() != name {
        return Err(WordPressError::RestoreCancelled);
    }
    if cancellation_was_requested(&cancellation) {
        return Err(WordPressError::RestoreCancelled);
    }

    let state_home = state_home()?;
    if state_home == project || state_home.starts_with(project) {
        return Err(WordPressError::BackupDirectoryInsideProject);
    }
    let backup_root = state_home.join("ai-profiles/wordpress-backups").join(&name);
    create_private_dir_all(&backup_root)?;
    let lock_path = backup_root.join("restore.lock");
    let lock = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&lock_path)
        .map_err(|_| WordPressError::BackupLock)?;
    lock.set_permissions(fs::Permissions::from_mode(0o600))
        .map_err(|_| WordPressError::BackupLock)?;
    lock.try_lock_exclusive()
        .map_err(|_| WordPressError::RestoreAlreadyRunning)?;

    let confirmed = preflight(project, dump, mappings, &mut ddev)?;
    if confirmed.source != initial.source
        || confirmed.info.get("name") != initial.info.get("name")
        || confirmed.current != initial.current
    {
        return Err(WordPressError::RestorePlanChanged);
    }
    if cancellation_was_requested(&cancellation) {
        return Err(WordPressError::RestoreCancelled);
    }

    let run_dir = create_run_dir(&backup_root)?;
    let backup = run_dir.0.join("before-import.sql.gz");
    let extension = if initial.source.compression == DumpCompression::Gzip {
        "gz"
    } else {
        "sql"
    };
    let incoming = run_dir.0.join(format!("incoming.{extension}"));
    let mut incoming_cleanup = IncomingCleanup(Some(incoming.clone()));

    copy_private(dump, &incoming)?;
    if file_hash(&incoming)? != initial.source.sha256 {
        return Err(WordPressError::IncomingChanged);
    }

    ddev.run(&[
        "export-db".to_owned(),
        "--database=db".to_owned(),
        "--gzip".to_owned(),
        format!("--file={}", backup.display()),
    ])?;
    fs::set_permissions(&backup, fs::Permissions::from_mode(0o600))
        .map_err(|_| WordPressError::RestoreIo)?;
    let verified_backup = inspect_dump(&backup)?;
    if verified_backup.prefix != initial.source.prefix
        || verified_backup.site_mode != initial.current.site_mode
    {
        return Err(WordPressError::BackupMismatch);
    }
    println!("Backup lokalny: {}", backup.display());

    let safety = safety_path(project, ddev.docroot())?;
    let payload_path = inside(project, ".opencode/ai-profile/scripts/local-safety.php")?;
    let payload = fs::read(&payload_path).map_err(|_| WordPressError::RestoreIo)?;
    let safety_existed = safety.exists();
    if safety_existed && fs::read(&safety).map_err(|_| WordPressError::RestoreIo)? != payload {
        return Err(WordPressError::SafetyChanged);
    }

    let mut import_started = false;
    let operation = (|| {
        if !safety_existed {
            let parent = safety.parent().ok_or(WordPressError::RestoreIo)?;
            if !parent.exists() {
                fs::create_dir(parent).map_err(|_| WordPressError::RestoreIo)?;
                fs::set_permissions(parent, fs::Permissions::from_mode(0o755))
                    .map_err(|_| WordPressError::RestoreIo)?;
            }
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o644)
                .open(&safety)
                .map_err(|_| WordPressError::RestoreIo)?;
            output
                .write_all(&payload)
                .map_err(|_| WordPressError::RestoreIo)?;
            output
                .set_permissions(fs::Permissions::from_mode(0o644))
                .map_err(|_| WordPressError::RestoreIo)?;
        }
        import_started = true;
        ddev.run(&[
            "import-db".to_owned(),
            "--database=db".to_owned(),
            format!("--file={}", incoming.display()),
        ])?;
        let imported = topology(&ddev, &initial.source.prefix)?;
        if imported.site_mode != initial.source.site_mode {
            return Err(WordPressError::ImportedTopologyMismatch);
        }
        localize(&ddev, &initial.source.prefix, mappings, &imported)?;
        Ok(())
    })();

    if operation.is_err() && import_started {
        eprintln!("Import przerwany. Brak automatycznego rollbacku. Przywróć backup:");
        eprintln!("cd {}", shell_quote(&project.display().to_string()));
        eprintln!(
            "ddev import-db --database=db --file={}",
            shell_quote(&backup.display().to_string())
        );
        return Err(WordPressError::RecoveryRequired);
    }
    operation?;
    fs::remove_file(&incoming).map_err(|_| WordPressError::RestoreIo)?;
    incoming_cleanup.0 = None;
    println!("OK: baza zaimportowana i URL sprawdzone. Ochrona lokalna mail/HTTP/cron pozostaje aktywna.");
    println!("Sprawdź oba hosty multisite, cache, AI Abilities Novamiry i odtwórz lokalne hasło aplikacyjne, jeśli import je zastąpił.");
    Ok(Some(backup))
}

pub fn cli_main() -> i32 {
    // SAFETY: this synchronous CLI sets the process-wide umask before creating
    // state or starting any worker threads, so new backup files default to 0600.
    unsafe { umask(0o077) };
    let mut dump = None;
    let mut project = None;
    let mut mapping_values = Vec::new();
    let mut apply = false;
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Lokalny import pełnej bazy UpdraftPlus do DDEV; domyślnie tylko plan.\n\nUsage: restore-updraft <dump.sql|dump.gz> --project <PROJECT> --map SOURCE=TARGET [--map SOURCE=TARGET ...] [--apply]");
        return 0;
    }
    let mut index = 0;
    while index < args.len() {
        let Some(value) = args[index].to_str() else {
            eprintln!("BŁĄD: nieprawidłowy argument CLI");
            return 2;
        };
        match value {
            "--project" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    eprintln!("BŁĄD: brak wartości dla --project");
                    return 2;
                };
                project = Some(expand_cli_path(Path::new(value)));
            }
            "--map" => {
                index += 1;
                let Some(value) = args.get(index).and_then(|value| value.to_str()) else {
                    eprintln!("BŁĄD: brak lub nieprawidłowa wartość dla --map");
                    return 2;
                };
                mapping_values.push(value.to_owned());
            }
            "--apply" => apply = true,
            option if option.starts_with('-') => {
                eprintln!("BŁĄD: nieznana opcja: {option}");
                return 2;
            }
            _ if dump.is_none() => dump = Some(expand_cli_path(Path::new(&args[index]))),
            _ => {
                eprintln!("BŁĄD: nieoczekiwany argument: {value}");
                return 2;
            }
        }
        index += 1;
    }
    let Some(dump) = dump else {
        eprintln!("BŁĄD: wymagany jest plik dumpa");
        return 2;
    };
    let Some(project) = project else {
        eprintln!("BŁĄD: opcja --project jest wymagana");
        return 2;
    };
    if mapping_values.is_empty() {
        eprintln!("BŁĄD: opcja --map jest wymagana");
        return 2;
    }
    let project = match fs::canonicalize(project) {
        Ok(path) => path,
        Err(_) => {
            eprintln!("BŁĄD: projekt nie istnieje lub nie jest dostępny");
            return 1;
        }
    };
    let dump = normalize_cli_path(&dump);
    let mappings = match mappings(&mapping_values) {
        Ok(mappings) => mappings,
        Err(error) => {
            eprintln!("BŁĄD: {error}");
            return 1;
        }
    };
    let cancellation = CancellationToken::new();
    let signal_token = cancellation.clone();
    if ctrlc::set_handler(move || signal_token.cancel()).is_err() {
        eprintln!("BŁĄD: nie można zainstalować obsługi przerwania");
        return 1;
    }
    let runner = SystemProcessRunner;
    let mode = if apply {
        RestoreMode::Apply
    } else {
        RestoreMode::Plan
    };
    match restore(
        &project,
        &dump,
        &mappings,
        mode,
        &runner,
        cancellation.clone(),
    ) {
        Ok(_) => 0,
        Err(WordPressError::RestoreCancelled) if cancellation.is_cancelled() => {
            eprintln!("Przerwano przez użytkownika");
            1
        }
        Err(error) => {
            eprintln!("BŁĄD: {error}");
            1
        }
    }
}

fn expand_cli_path(path: &Path) -> PathBuf {
    if path == Path::new("~") {
        return env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    }
    if let Some(value) = path.to_str() {
        if let Some(rest) = value.strip_prefix("~/") {
            return env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join(rest);
        }
    }
    path.to_owned()
}

fn normalize_cli_path(path: &Path) -> PathBuf {
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        env::current_dir().unwrap_or_default().join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn print_plan(
    project: &Path,
    dump: &Path,
    mappings: &UrlMappings,
    name: &str,
    plan: &PreflightResult,
) {
    println!(
        "Projekt: {name}\nKatalog: {}\nCel: lokalna usługa DDEV db / baza db",
        project.display()
    );
    println!(
        "Backup źródłowy: {}\nSHA-256: {}\nPrefiks: {}; multisite: {}",
        dump.display(),
        plan.source.sha256.as_str(),
        plan.source.prefix.as_str(),
        plan.source.site_mode == SiteMode::Multisite
    );
    for mapping in mappings.entries() {
        println!(
            "  {} → {}",
            mapping.source.as_str(),
            mapping.target.as_str()
        );
    }
    println!("Plan: backup lokalnej bazy → ochrona mail/HTTP/cron → zastąpienie bazy → mapowanie URL → weryfikacja.");
}

fn project_name(info: &Value) -> Result<String, WordPressError> {
    let name = info
        .get("name")
        .and_then(Value::as_str)
        .ok_or(WordPressError::InvalidDdevName)?;
    if name.is_empty()
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(WordPressError::InvalidDdevName);
    }
    Ok(name.to_owned())
}

fn state_home() -> Result<PathBuf, WordPressError> {
    let value = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_default()
                .join(".local/state")
        });
    let expanded = if value == Path::new("~") {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or(WordPressError::RestoreIo)?
    } else if let Ok(text) = value.to_str().ok_or(WordPressError::RestoreIo) {
        if let Some(rest) = text.strip_prefix("~/") {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .ok_or(WordPressError::RestoreIo)?
                .join(rest)
        } else {
            value
        }
    } else {
        return Err(WordPressError::RestoreIo);
    };
    resolve_without_creating(&normalize_cli_path(&expanded))
}

fn resolve_without_creating(path: &Path) -> Result<PathBuf, WordPressError> {
    let mut ancestor = path;
    let mut missing = Vec::new();
    while !ancestor.exists() {
        let name = ancestor
            .file_name()
            .ok_or(WordPressError::BackupDirectory)?
            .to_owned();
        missing.push(name);
        ancestor = ancestor.parent().ok_or(WordPressError::BackupDirectory)?;
    }
    let mut resolved = fs::canonicalize(ancestor).map_err(|_| WordPressError::BackupDirectory)?;
    for component in missing.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

fn create_private_dir_all(path: &Path) -> Result<(), WordPressError> {
    fs::create_dir_all(path).map_err(|_| WordPressError::BackupDirectory)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|_| WordPressError::BackupDirectory)
}

fn create_run_dir(parent: &Path) -> Result<RunDirectory, WordPressError> {
    for _ in 0..100 {
        let suffix = RUN_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!("restore-{}-{suffix}", std::process::id()));
        match fs::create_dir(&path) {
            Ok(()) => {
                fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                    .map_err(|_| WordPressError::BackupDirectory)?;
                return Ok(RunDirectory(path));
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => return Err(WordPressError::BackupDirectory),
        }
    }
    Err(WordPressError::BackupDirectory)
}

fn copy_private(source: &Path, destination: &Path) -> Result<(), WordPressError> {
    let mut input = File::open(source).map_err(|_| WordPressError::RestoreIo)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(destination)
        .map_err(|_| WordPressError::RestoreIo)?;
    io::copy(&mut input, &mut output).map_err(|_| WordPressError::RestoreIo)?;
    output.sync_all().map_err(|_| WordPressError::RestoreIo)
}

fn safety_path(project: &Path, docroot: &str) -> Result<PathBuf, WordPressError> {
    let relative = Path::new(docroot).join("wp-content/mu-plugins/ai-profile-local-safety.php");
    inside(project, &relative.to_string_lossy())
}

struct IncomingCleanup(Option<PathBuf>);

impl Drop for IncomingCleanup {
    fn drop(&mut self) {
        if let Some(path) = &self.0 {
            let _ = fs::remove_file(path);
        }
    }
}

fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_owned();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn cancellation_was_requested(token: &CancellationToken) -> bool {
    token.is_cancelled()
}

unsafe extern "C" {
    fn umask(mask: u32) -> u32;
}

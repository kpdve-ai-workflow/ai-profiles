//! Oracles: stage-1 contract in todos/logger.md; byte identity and SHA-256.
use crate::logging::{Config, Session};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{self, Read},
    path::Path,
};

fn private_root() -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    root
}

#[test]
fn refuses_public_diagnostic_directory_without_changing_its_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Session::open(root.path(), Config::default(), "test").is_err());
    assert_eq!(
        fs::metadata(root.path()).unwrap().permissions().mode() & 0o777,
        0o755
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn interrupted_retention_resumes_a_retired_bundle_without_active_file() {
    let root = private_root();
    let session = Session::open(root.path(), Config::default(), "test").unwrap();
    assert!(session.payload(&b"payload"[..]).complete);
    drop(session);
    let directory = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let retired = root.path().join(
        directory
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .replacen("run-", "retired-", 1),
    );
    fs::rename(directory, &retired).unwrap();
    fs::remove_file(retired.join("active")).unwrap();
    let _next = Session::open(root.path(), Config::default(), "test").unwrap();
    assert!(!retired.exists());
}

fn events(root: &Path) -> Vec<serde_json::Value> {
    let mut files: Vec<_> = fs::read_dir(root)
        .unwrap()
        .filter_map(|e| {
            let p = e.unwrap().path();
            (p.extension().is_some_and(|e| e == "jsonl")).then_some(p)
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .flat_map(|p| {
            fs::read_to_string(p)
                .unwrap()
                .lines()
                .map(|s| serde_json::from_str(s).unwrap())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn payload_binary_rotation_and_exact_reconstruction() {
    let root = private_root();
    let session = Session::open(
        root.path(),
        Config {
            segment_bytes: 512,
            ..Config::default()
        },
        "test",
    )
    .unwrap();
    let data: Vec<u8> = (0..2_100_123).map(|i| (i % 256) as u8).collect();
    let result = tracing::subscriber::with_default(session.subscriber(), || {
        session.payload(ShortReads(&data))
    });
    assert!(result.complete);
    assert_eq!(result.length, data.len() as u64);
    assert_eq!(result.sha256, format!("{:x}", Sha256::digest(&data)));
    let dir = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let records = events(&dir);
    let mut restored = Vec::new();
    for (index, record) in records
        .iter()
        .filter(|r| r["fields"]["event"] == "payload.chunk")
        .enumerate()
    {
        let f = &record["fields"];
        assert_eq!(f["index"], index);
        assert_eq!(f["offset"], restored.len());
        let chunk = fs::read(dir.join(f["file"].as_str().unwrap())).unwrap();
        assert!(
            chunk.len() <= 1024 * 1024,
            "short reads must not overshoot the chunk boundary"
        );
        assert_eq!(f["length"], chunk.len());
        assert_eq!(f["sha256"], format!("{:x}", Sha256::digest(&chunk)));
        restored.extend(chunk);
    }
    assert_eq!(restored, data);
    assert!(
        records
            .iter()
            .any(|r| r["fields"]["event"] == "payload.finished"
                && r["fields"]["status"] == "complete")
    );
    assert!(
        fs::read_dir(dir)
            .unwrap()
            .filter(|e| e
                .as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|s| s == "jsonl"))
            .count()
            > 1
    );
}

struct BrokenReader;
struct ShortReads<'a>(&'a [u8]);
impl Read for ShortReads<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let length = self.0.len().min(buffer.len()).min(997);
        buffer[..length].copy_from_slice(&self.0[..length]);
        self.0 = &self.0[length..];
        Ok(length)
    }
}
impl Read for BrokenReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("synthetic read failure"))
    }
}

#[test]
fn writer_failure_after_initialization_is_reported_once_and_never_follows_symlink() {
    use std::os::unix::fs::symlink;
    let root = private_root();
    let session = Session::open(
        root.path(),
        Config {
            segment_bytes: 1,
            ..Config::default()
        },
        "test",
    )
    .unwrap();
    let dir = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let victim = root.path().join("victim");
    fs::write(&victim, "do not touch").unwrap();
    symlink(&victim, dir.join("events-00000000000000000001.jsonl")).unwrap();
    tracing::subscriber::with_default(session.subscriber(), || {
        tracing::info!(event = "first");
        assert!(session.complete());
        tracing::info!(event = "second");
        assert!(!session.complete());
        tracing::info!(event = "third");
    });
    assert_eq!(fs::read_to_string(victim).unwrap(), "do not touch");
    assert!(dir.join("incomplete").is_file());
}

#[test]
fn retention_refuses_unknown_files_and_symlinks() {
    use std::os::unix::fs::symlink;
    let root = private_root();
    let config = Config {
        retain_bytes: 1,
        ..Config::default()
    };
    let first = Session::open(root.path(), config.clone(), "test").unwrap();
    let dir = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    fs::write(dir.join("unknown"), "user").unwrap();
    drop(first);
    let second = Session::open(root.path(), config.clone(), "test").unwrap();
    assert!(dir.join("unknown").is_file());
    fs::remove_file(dir.join("unknown")).unwrap();
    symlink(
        root.path().join("victim"),
        dir.join("payload-00000000000000000000-00000000000000000000.bin"),
    )
    .unwrap();
    let _third = Session::open(root.path(), config, "test").unwrap();
    assert!(dir.is_dir());
    drop(second);
}

#[test]
fn empty_payload_and_rotation_boundary_keep_records_whole() {
    let root = private_root();
    let session = Session::open(
        root.path(),
        Config {
            segment_bytes: 1,
            ..Config::default()
        },
        "test",
    )
    .unwrap();
    assert!(session.payload(io::empty()).complete);
    let dir = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let records = events(&dir);
    assert_eq!(records.len(), 3);
    assert_eq!(records[1]["fields"]["length"], 0);
    assert_eq!(
        records[2]["fields"]["sha256"],
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    for entry in fs::read_dir(dir).unwrap() {
        assert_eq!(
            entry.unwrap().metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}

#[test]
fn payload_error_and_session_limit_are_explicit() {
    let root = private_root();
    let session = Session::open(
        root.path(),
        Config {
            session_bytes: 3000,
            ..Config::default()
        },
        "test",
    )
    .unwrap();
    tracing::subscriber::with_default(session.subscriber(), || {
        assert!(!session.payload(BrokenReader).complete);
        assert!(!session.complete());
        assert!(!session.payload(&vec![0xff; 10000][..]).complete);
    });
    let dir = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    assert!(dir.join("incomplete").is_file());
}

#[test]
fn retention_keeps_active_sessions_and_unknown_resources() {
    let root = private_root();
    fs::write(root.path().join("user.jsonl"), "keep").unwrap();
    let config = Config {
        retain_bytes: 1,
        ..Config::default()
    };
    let a = Session::open(root.path(), config.clone(), "a").unwrap();
    assert!(a.payload(&b"retained payload"[..]).complete);
    let subscriber = a.subscriber();
    let active = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let b = Session::open(root.path(), config.clone(), "b").unwrap();
    assert!(active.is_dir());
    drop(a);
    let _c = Session::open(root.path(), config.clone(), "c").unwrap();
    assert!(
        active.is_dir(),
        "subscriber still owns the active session and its payloads"
    );
    drop(subscriber);
    let _d = Session::open(root.path(), config, "d").unwrap();
    assert!(!active.exists());
    assert!(root.path().join("user.jsonl").is_file());
    drop(b);
}

#[test]
fn age_retention_ignores_active_even_with_old_timestamps() {
    let root = private_root();
    let config = Config {
        retain_seconds: 1,
        ..Config::default()
    };
    let a = Session::open(root.path(), config.clone(), "a").unwrap();
    let directory = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    for entry in fs::read_dir(&directory).unwrap() {
        fs::File::options()
            .write(true)
            .open(entry.unwrap().path())
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH))
            .unwrap();
    }
    let _b = Session::open(root.path(), config.clone(), "b").unwrap();
    assert!(directory.is_dir());
    drop(a);
    let _c = Session::open(root.path(), config, "c").unwrap();
    assert!(!directory.exists());
}

#[test]
fn retention_coordinates_with_another_live_process() {
    use std::{
        io::{BufRead, Write},
        process::{Command, Stdio},
    };
    if let Some(root) = std::env::var_os("LOGGER_RETENTION_TEST_ROOT") {
        let session = Session::open(Path::new(&root), Config::default(), "child").unwrap();
        assert!(session.payload(&b"child payload"[..]).complete);
        println!("LOGGER_READY");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_exact(&mut [0]).unwrap();
        return;
    }
    let root = private_root();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "logging_session_tests::retention_coordinates_with_another_live_process",
            "--nocapture",
        ])
        .env("LOGGER_RETENTION_TEST_ROOT", root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut output = std::io::BufReader::new(child.stdout.take().unwrap());
    loop {
        let mut line = String::new();
        assert!(
            output.read_line(&mut line).unwrap() > 0,
            "child failed before readiness"
        );
        if line.trim() == "LOGGER_READY" {
            break;
        }
    }
    let directory = fs::read_dir(root.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.is_dir())
        .unwrap();
    let config = Config {
        retain_bytes: 1,
        ..Config::default()
    };
    let _other = Session::open(root.path(), config.clone(), "parent").unwrap();
    assert!(directory.is_dir());
    assert!(fs::read_dir(&directory).unwrap().any(|e| e
        .unwrap()
        .path()
        .extension()
        .is_some_and(|s| s == "bin")));
    child.stdin.take().unwrap().write_all(b"x").unwrap();
    assert!(child.wait().unwrap().success());
    let _cleanup = Session::open(root.path(), config, "parent").unwrap();
    assert!(!directory.exists());
}

//! Oracles: the requested JSONL contract, JSON syntax, and Unix permission bits.
//! No global dispatcher, environment mutation, real HOME, or mocked filesystem.
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::Path,
    sync::{Arc, Barrier},
};

use serde_json::Value;

use crate::logging::{build_subscriber, RunMetadata};

fn metadata() -> RunMetadata<'static> {
    RunMetadata {
        binary: "synthetic-cli",
        version: "1.2.3-test",
        run_id: "synthetic-run-42",
    }
}

fn records(path: &Path) -> Vec<Value> {
    let bytes = fs::read(path).unwrap();
    assert!(!bytes.is_empty(), "events must be written synchronously");
    assert_eq!(bytes.last(), Some(&b'\n'), "complete JSONL terminator");
    let text = std::str::from_utf8(&bytes).unwrap();
    text.lines()
        .map(|line| {
            let record: Value = serde_json::from_str(line).expect("one JSON object per line");
            assert!(record.is_object());
            record
        })
        .collect()
}

#[test]
fn jsonl_preserves_metadata_levels_typed_fields_and_nested_spans() {
    let temp = tempfile::tempdir().unwrap();
    let subscriber = build_subscriber(temp.path(), "events.jsonl", metadata()).unwrap();
    tracing::subscriber::with_default(subscriber, || {
        let root = tracing::info_span!(
            "operation",
            project = "synthetic",
            revision = tracing::field::Empty
        );
        root.record("revision", 7);
        let _root = root.enter();
        let child = tracing::debug_span!("stage", client = "fixture");
        let _child = child.enter();
        tracing::trace!(event = "trace.fixture", count = 17_u64, enabled = true);
        tracing::debug!(event = "debug.fixture");
        tracing::info!(event = "info.fixture");
        tracing::warn!(event = "warn.fixture");
        tracing::error!(event = "error.fixture");
        // Read before the subscriber is dropped: no shutdown/flush dependency.
        let rows = records(&temp.path().join("events.jsonl"));
        assert_eq!(rows.len(), 5);
        let utc = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$").unwrap();
        for (row, level) in rows.iter().zip(["TRACE", "DEBUG", "INFO", "WARN", "ERROR"]) {
            assert_eq!(row["level"], level);
            assert_eq!(row["binary"], "synthetic-cli");
            assert_eq!(row["version"], "1.2.3-test");
            assert_eq!(row["run_id"], "synthetic-run-42");
            let timestamp = row["timestamp"].as_str().unwrap();
            assert!(utc.is_match(timestamp), "UTC RFC3339: {timestamp}");
            assert_eq!(row["spans"][0]["name"], "operation");
            assert_eq!(row["spans"][0]["project"], "synthetic");
            assert_eq!(row["spans"][0]["revision"], 7);
            assert_eq!(row["spans"][1]["name"], "stage");
            assert_eq!(row["spans"][1]["client"], "fixture");
            assert_eq!(
                row["fields"]["event"],
                format!("{}.fixture", level.to_lowercase())
            );
        }
        assert_eq!(rows[0]["fields"]["count"], 17);
        assert_eq!(rows[0]["fields"]["enabled"], true);
    });
}

#[test]
fn synthetic_values_are_not_redacted_truncated_or_split_into_lines() {
    let temp = tempfile::tempdir().unwrap();
    let subscriber = build_subscriber(temp.path(), "values.jsonl", metadata()).unwrap();
    let payload = format!(
        "token=synthetic-secret \"żółć 🔬\"\n\r\t\\{}END",
        "λ".repeat(8192)
    );
    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("payload", value = payload.as_str());
        let _entered = span.enter();
        tracing::info!(
            event = "payload.fixture",
            value = payload.as_str(),
            run_id = "event-run",
            binary = "event-binary",
            message = "line one\nline two"
        );
    });
    let rows = records(&temp.path().join("values.jsonl"));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["fields"]["value"], payload);
    assert_eq!(rows[0]["spans"][0]["value"], payload);
    assert_eq!(rows[0]["fields"]["message"], "line one\nline two");
    assert_eq!(rows[0]["fields"]["run_id"], "event-run");
    assert_eq!(rows[0]["fields"]["binary"], "event-binary");
    assert_eq!(rows[0]["run_id"], "synthetic-run-42");
}

#[test]
fn parallel_records_are_complete_unique_and_not_interleaved() {
    let temp = tempfile::tempdir().unwrap();
    let dispatch = tracing::Dispatch::new(
        build_subscriber(temp.path(), "parallel.jsonl", metadata()).unwrap(),
    );
    let barrier = Arc::new(Barrier::new(8));
    std::thread::scope(|scope| {
        for worker in 0..8 {
            let dispatch = dispatch.clone();
            let barrier = Arc::clone(&barrier);
            scope.spawn(move || {
                tracing::dispatcher::with_default(&dispatch, || {
                    let span = tracing::info_span!("worker", worker);
                    let _entered = span.enter();
                    barrier.wait();
                    for sequence in 0..80 {
                        let payload = format!("{worker}:{sequence}:{}", "\"雪\n".repeat(1024));
                        tracing::trace!(event = "parallel.fixture", worker, sequence, payload);
                    }
                })
            });
        }
    });
    let rows = records(&temp.path().join("parallel.jsonl"));
    assert_eq!(rows.len(), 640);
    let mut seen = BTreeSet::new();
    for row in rows {
        let worker = row["fields"]["worker"].as_u64().unwrap();
        let sequence = row["fields"]["sequence"].as_u64().unwrap();
        assert!(worker < 8 && sequence < 80);
        assert!(seen.insert((worker, sequence)), "duplicate record");
        assert_eq!(row["spans"][0]["worker"], worker);
        assert_eq!(
            row["fields"]["payload"],
            format!("{worker}:{sequence}:{}", "\"雪\n".repeat(1024))
        );
    }
}

#[test]
fn creates_private_directories_and_file_without_changing_existing_parent() {
    let temp = tempfile::tempdir().unwrap();
    fs::set_permissions(temp.path(), fs::Permissions::from_mode(0o750)).unwrap();
    let directory = temp.path().join("new/nested");
    let _subscriber = build_subscriber(&directory, "private.jsonl", metadata()).unwrap();
    for path in [temp.path().join("new"), directory.clone()] {
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
    assert_eq!(
        fs::metadata(directory.join("private.jsonl"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(temp.path()).unwrap().permissions().mode() & 0o777,
        0o750
    );
}

#[test]
fn existing_file_is_neither_truncated_nor_appended() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("user.jsonl");
    fs::write(&path, b"synthetic user content\n").unwrap();
    assert!(build_subscriber(temp.path(), "user.jsonl", metadata()).is_err());
    assert_eq!(fs::read(path).unwrap(), b"synthetic user content\n");
}

#[test]
fn rejects_file_directory_and_ancestor_symlinks_without_touching_targets() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("user"), b"keep").unwrap();
    symlink(target.join("user"), temp.path().join("linked.jsonl")).unwrap();
    assert!(build_subscriber(temp.path(), "linked.jsonl", metadata()).is_err());
    symlink(&target, temp.path().join("linked-dir")).unwrap();
    assert!(build_subscriber(&temp.path().join("linked-dir"), "events.jsonl", metadata()).is_err());
    assert!(build_subscriber(
        &temp.path().join("linked-dir/new/nested"),
        "events.jsonl",
        metadata()
    )
    .is_err());
    symlink(target.join("absent"), temp.path().join("dangling.jsonl")).unwrap();
    assert!(build_subscriber(temp.path(), "dangling.jsonl", metadata()).is_err());
    assert_eq!(fs::read(target.join("user")).unwrap(), b"keep");
    assert_eq!(fs::read_dir(&target).unwrap().count(), 1);
}

#[test]
fn preparation_errors_are_returned_without_alternative_output() {
    let temp = tempfile::tempdir().unwrap();
    let blocked = temp.path().join("not-a-directory");
    fs::write(&blocked, b"keep").unwrap();
    assert!(build_subscriber(&blocked.join("logs"), "events.jsonl", metadata()).is_err());
    assert!(build_subscriber(temp.path(), "missing/events.jsonl", metadata()).is_err());
    assert!(build_subscriber(temp.path(), "../escape.jsonl", metadata()).is_err());
    assert_eq!(fs::read(blocked).unwrap(), b"keep");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
}

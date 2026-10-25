//! Read-only test-file inventory; never executes tests or inspected files.
use std::{env, fs, path::Path, process::ExitCode};

use regex::Regex;
use serde::Serialize;

const EXCLUDED: &[&str] = &[
    ".git",
    "node_modules",
    ".venv",
    "venv",
    "target",
    "dist",
    "build",
    ".next",
    ".tox",
    "vendor",
    "coverage",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
];

#[derive(Default, Serialize)]
struct Hints {
    mock_files: usize,
    snapshot_files: usize,
    sleep_or_retry_files: usize,
}

#[derive(Serialize)]
struct Inventory<'a> {
    root: &'a Path,
    test_file_count: usize,
    test_files: Vec<String>,
    heuristic_file_hints: Hints,
    disclaimer: &'static str,
}

fn walk(root: &Path, directory: &Path, pattern: &Regex, files: &mut Vec<String>) {
    // Like os.walk, unreadable directories are skipped and links are not followed.
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    let mut directories = Vec::new();
    let mut names = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if !EXCLUDED.contains(&entry.file_name().to_string_lossy().as_ref())
                && !entry.file_type().is_ok_and(|kind| kind.is_symlink())
            {
                directories.push(path);
            }
        } else {
            names.push(path);
        }
    }
    names.sort();
    directories.sort();
    for path in names {
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let Ok(relative) = path.strip_prefix(root) else {
            continue;
        };
        let in_tests = relative.parent().is_some_and(|parent| {
            parent.components().any(|part| {
                matches!(
                    part.as_os_str().to_str(),
                    Some("tests" | "test" | "__tests__" | "spec" | "specs")
                )
            })
        });
        let supported_extension =
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    matches!(
                        ext,
                        "py" | "go" | "ts" | "tsx" | "js" | "jsx" | "rs" | "rb" | "java" | "cs"
                    )
                });
        if pattern.is_match(name) || (in_tests && supported_extension) {
            files.push(relative.to_string_lossy().into_owned());
        }
    }
    for directory in directories {
        walk(root, &directory, pattern, files);
    }
}

fn main() -> ExitCode {
    ai_profiles_core::logging::runtime_entry("test-inventory", invoke)
}

fn invoke() -> i32 {
    let requested = env::args_os().nth(1).unwrap_or_else(|| ".".into());
    let root = match fs::canonicalize(&requested) {
        Ok(root) if root.is_dir() => root,
        _ => {
            eprintln!("Expected directory: {}", Path::new(&requested).display());
            return 2;
        }
    };
    let pattern = Regex::new(concat!(
        r"^(?:.*_test\.go|(?:test_.*|.*_test)\.py|.*\.(?:test|spec)\.[cm]?[jt]sx?|",
        r"test_.*\.rs|.*_test\.rs|.*(?:Test|Tests)\.java|.*Tests?\.cs|.*_spec\.rb|.*\.feature)$"
    ))
    .expect("fixed test-file pattern");
    let mut files = Vec::new();
    walk(&root, &root, &pattern, &mut files);
    let mock = Regex::new(r"(?i)\b(mock|mockk|jest\.fn|vi\.fn|monkeypatch|sinon|stub)\b")
        .expect("fixed mock pattern");
    let snapshot = Regex::new(r"(?i)\b(snapshot|toMatchSnapshot|snapshots)\b")
        .expect("fixed snapshot pattern");
    let retry =
        Regex::new(r"(?i)\b(sleep|setTimeout|retry|retries)\b").expect("fixed retry pattern");
    let mut hints = Hints::default();
    for file in &files {
        if let Ok(bytes) = fs::read(root.join(file)) {
            let data: String = String::from_utf8_lossy(&bytes)
                .chars()
                .take(200_000)
                .collect();
            hints.mock_files += usize::from(mock.is_match(&data));
            hints.snapshot_files += usize::from(snapshot.is_match(&data));
            hints.sleep_or_retry_files += usize::from(retry.is_match(&data));
        }
    }
    let inventory = Inventory {
        root: &root,
        test_file_count: files.len(),
        test_files: files,
        heuristic_file_hints: hints,
        disclaimer: "File name and text heuristics only; not coverage, quality judgment, or detected defects.",
    };
    match serde_json::to_string_pretty(&inventory) {
        Ok(output) => {
            println!("{output}");
            0
        }
        Err(error) => {
            eprintln!("Cannot serialize inventory: {error}");
            1
        }
    }
}

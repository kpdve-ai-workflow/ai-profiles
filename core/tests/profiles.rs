use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use crate::{
    codec::{json_bytes, read_json},
    model::{RepositoryContext, VersionBump},
    profile_management::profiles::{bump_profile, create_profile, rename_profile},
};

fn source() -> (tempfile::TempDir, RepositoryContext) {
    let temp = tempfile::tempdir().unwrap();
    for path in ["core/src", "profiles", "skills", "mcp/servers"] {
        std::fs::create_dir_all(temp.path().join(path)).unwrap();
    }
    let context = RepositoryContext::from_sources(PathBuf::from(temp.path())).unwrap();
    (temp, context)
}

fn validation_source() -> (tempfile::TempDir, RepositoryContext) {
    let (temp, context) = source();
    for (relative, contents) in [
        (
            "profiles/demo/profile.json",
            br#"{"name":"demo","version":"1.0.0","description":"Demo profile"}"#.as_slice(),
        ),
        ("profiles/demo/skills.json", br#"{"skills":[]}"#.as_slice()),
        (
            "profiles/demo/mcp.json",
            br#"{"servers":["sample"]}"#.as_slice(),
        ),
        (
            "mcp/servers/sample.json",
            br#"{"type":"remote","url":"https://example.test/mcp"}"#.as_slice(),
        ),
        (
            "mcp/example.json",
            br#"{"mcp":{"sample":{"type":"remote","url":"https://example.test/mcp"}}}"#.as_slice(),
        ),
    ] {
        let path = temp.path().join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let native_binary = temp.path().join("core/target/release/ai-profile-mcp");
    fs::create_dir_all(native_binary.parent().unwrap()).unwrap();
    fs::write(native_binary, b"synthetic native binary").unwrap();
    fs::set_permissions(
        temp.path().join("core/target/release/ai-profile-mcp"),
        fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    (temp, context)
}

fn tree_snapshot(root: &Path) -> std::collections::BTreeMap<PathBuf, Vec<u8>> {
    fn visit(
        root: &Path,
        current: &Path,
        files: &mut std::collections::BTreeMap<PathBuf, Vec<u8>>,
    ) {
        for entry in fs::read_dir(current).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), files);
            } else if entry.file_type().unwrap().is_file() {
                files.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    visit(root, root, &mut files);
    files
}

#[test]
fn validate_sources_accepts_full_and_scoped_fixture_without_mutating_sources() {
    let (temp, context) = validation_source();
    let before = tree_snapshot(temp.path());

    crate::profile_management::validation::validate_sources(&context, None).unwrap();
    crate::profile_management::validation::validate_sources(&context, Some("demo")).unwrap();

    assert_eq!(tree_snapshot(temp.path()), before);
}

#[test]
fn validate_sources_reports_unknown_profile_and_example_mismatch() {
    let (temp, context) = validation_source();
    let before = tree_snapshot(temp.path());

    let unknown =
        crate::profile_management::validation::validate_sources(&context, Some("missing"))
            .unwrap_err();
    assert_eq!(unknown.to_string(), "Unknown profile: missing");

    fs::write(
        temp.path().join("mcp/example.json"),
        br#"{"mcp":{"sample":{"type":"remote","url":"https://wrong.example/mcp"}}}"#,
    )
    .unwrap();
    let mismatch =
        crate::profile_management::validation::validate_sources(&context, None).unwrap_err();
    assert_eq!(
        mismatch.to_string(),
        "mcp/example.json does not match the MCP definitions selected by the profiles"
    );
    assert_ne!(
        tree_snapshot(temp.path()),
        before,
        "only the controlled example fixture changed"
    );
}

#[test]
fn rename_changes_only_directory_and_metadata_name() {
    let (_temp, context) = source();
    create_profile(&context, "old-profile", Some("Description old-profile")).unwrap();
    create_profile(&context, "other-profile", None).unwrap();
    let original = context.sources.join("profiles/old-profile");
    let metadata_path = original.join("profile.json");
    let mut metadata = read_json(&metadata_path).unwrap();
    metadata["custom"] = serde_json::json!({"reference":"old-profile"});
    fs::write(&metadata_path, json_bytes(&metadata).unwrap()).unwrap();
    fs::set_permissions(&metadata_path, fs::Permissions::from_mode(0o640)).unwrap();
    fs::write(original.join("README.md"), b"old-profile documentation").unwrap();
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("ai-profile-state.json"),
        br#"{"profile":"old-profile"}"#,
    )
    .unwrap();
    let other = fs::read(context.sources.join("profiles/other-profile/profile.json")).unwrap();
    let mcp = fs::read(original.join("mcp.json")).unwrap();

    rename_profile(&context, "old-profile", "new-profile").unwrap();

    assert!(!original.exists());
    let renamed = context.sources.join("profiles/new-profile");
    metadata["name"] = serde_json::json!("new-profile");
    assert_eq!(read_json(&renamed.join("profile.json")).unwrap(), metadata);
    assert_eq!(
        fs::metadata(renamed.join("profile.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    assert_eq!(fs::read(renamed.join("mcp.json")).unwrap(), mcp);
    assert_eq!(
        fs::read(renamed.join("README.md")).unwrap(),
        b"old-profile documentation"
    );
    assert_eq!(
        read_json(&renamed.join("skills.json")).unwrap(),
        serde_json::json!({"skills": []})
    );
    assert_eq!(
        fs::read(context.sources.join("profiles/other-profile/profile.json")).unwrap(),
        other
    );
    assert_eq!(
        fs::read(project.path().join("ai-profile-state.json")).unwrap(),
        br#"{"profile":"old-profile"}"#
    );
    assert_eq!(
        fs::read_dir(context.sources.join("profiles"))
            .unwrap()
            .count(),
        2
    );
}

#[test]
fn rename_rejects_collisions_invalid_names_and_missing_profile_without_changes() {
    let (_temp, context) = source();
    create_profile(&context, "original", None).unwrap();
    fs::create_dir(context.sources.join("profiles/occupied")).unwrap();
    let metadata = context.sources.join("profiles/original/profile.json");
    let before = fs::read(&metadata).unwrap();
    for (old, new) in [
        ("original", "occupied"),
        ("original", "original"),
        ("original", "../escape"),
        ("original", "Bad_Name"),
        ("original", ""),
        ("../original", "new"),
        ("missing", "new"),
    ] {
        assert!(
            rename_profile(&context, old, new).is_err(),
            "{old} -> {new}"
        );
        assert_eq!(fs::read(&metadata).unwrap(), before);
        assert_eq!(
            fs::read_dir(context.sources.join("profiles"))
                .unwrap()
                .count(),
            2
        );
    }
}

#[test]
fn rename_rejects_symlinked_profile_and_metadata() {
    use std::os::unix::fs::symlink;
    let (_temp, context) = source();
    create_profile(&context, "original", None).unwrap();
    symlink("original", context.sources.join("profiles/linked")).unwrap();
    assert!(rename_profile(&context, "linked", "new").is_err());
    assert!(rename_profile(&context, "original", "linked").is_err());
    let metadata = context.sources.join("profiles/original/profile.json");
    let backup = context.sources.join("profile-backup.json");
    fs::rename(&metadata, &backup).unwrap();
    symlink(&backup, &metadata).unwrap();
    assert!(rename_profile(&context, "original", "new").is_err());
    assert!(!context.sources.join("profiles/new").exists());
    assert_eq!(read_json(&backup).unwrap()["name"], "original");
}

#[test]
fn create_and_bump_touch_only_requested_profile() {
    let (_temp, context) = source();
    create_profile(&context, "new-profile", Some("Description")).unwrap();
    assert_eq!(
        read_json(&context.sources.join("profiles/new-profile/skills.json")).unwrap(),
        serde_json::json!({"skills": []})
    );
    assert_eq!(
        bump_profile(&context, "new-profile", VersionBump::Minor).unwrap(),
        "1.1.0"
    );
    assert!(create_profile(&context, "new-profile", None).is_err());
}

use std::{os::unix::fs::symlink, str::FromStr};

use crate::{
    codec::{json_bytes, sha},
    filesystem::file_path,
    model::{ProfileName, Version, VersionBump},
};

#[test]
fn names_versions_and_decimal_bumps_preserve_python_domain() {
    assert!(ProfileName::from_str("wordpress-local").is_ok());
    assert!(ProfileName::from_str("Bad--name").is_err());
    let older = Version::from_str("1.9.0").unwrap();
    let newer = Version::from_str("1.10.0").unwrap();
    assert!(older < newer);
    let huge = Version::from_str("999999999999999999999999999999.0.9").unwrap();
    assert_eq!(
        huge.bump(VersionBump::Patch).as_string(),
        "999999999999999999999999999999.0.10"
    );
    assert!(Version::from_str("01.0.0").is_err());
}

#[test]
fn json_is_utf8_indented_and_terminated_and_hash_is_sha256() {
    let value = serde_json::json!({"name":"żółć", "n":12345678901234567890_u64});
    let bytes = json_bytes(&value).unwrap();
    assert!(bytes.ends_with(b"\n"));
    assert!(std::str::from_utf8(&bytes).unwrap().contains("żółć"));
    assert_eq!(
        sha(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn managed_paths_reject_parent_and_symlink_components() {
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    symlink(&outside, temp.path().join("link")).unwrap();
    assert!(file_path(temp.path(), "../outside/file").is_err());
    assert!(file_path(temp.path(), "link/file").is_err());
}

use std::path::PathBuf;

use crate::{
    model::{RepositoryContext, VersionBump},
    profile_management::profiles::{bump_profile, create_profile},
};

fn source() -> (tempfile::TempDir, RepositoryContext) {
    let temp = tempfile::tempdir().unwrap();
    for path in ["core/rust/src", "profiles", "common/skills", "mcp/servers"] {
        std::fs::create_dir_all(temp.path().join(path)).unwrap();
    }
    let context = RepositoryContext::from_sources(PathBuf::from(temp.path())).unwrap();
    (temp, context)
}

#[test]
fn create_and_bump_touch_only_requested_profile() {
    let (_temp, context) = source();
    create_profile(&context, "new-profile", Some("Description")).unwrap();
    assert!(context
        .sources
        .join("profiles/new-profile/skills/.gitkeep")
        .exists());
    assert_eq!(
        bump_profile(&context, "new-profile", VersionBump::Minor).unwrap(),
        "1.1.0"
    );
    assert!(create_profile(&context, "new-profile", None).is_err());
}

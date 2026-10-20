use crate::wordpress::preflight::inside;

#[test]
fn project_relative_paths_reject_symlink_escape() {
    let temp = tempfile::tempdir().unwrap();
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::os::unix::fs::symlink(&outside, temp.path().join("linked")).unwrap();
    assert!(inside(temp.path(), "linked/config.yaml").is_err());
}

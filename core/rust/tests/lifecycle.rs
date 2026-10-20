use crate::filesystem::check_files;

#[test]
fn ownership_preflight_rejects_unmanaged_collision_without_writing() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("unmanaged"), b"keep").unwrap();
    assert!(check_files(temp.path(), None, ["unmanaged"]).is_err());
    assert_eq!(
        std::fs::read(temp.path().join("unmanaged")).unwrap(),
        b"keep"
    );
}

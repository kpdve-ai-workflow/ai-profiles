use crate::wordpress::{dump::inspect_dump, SiteMode};

#[test]
fn sql_dump_prefix_and_incomplete_database_are_classified() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("backup.sql");
    std::fs::write(&path, b"CREATE TABLE `wp_options` (x int);\nCREATE TABLE `wp_posts` (x int);\nCREATE TABLE `wp_users` (x int);\nCREATE TABLE `wp_usermeta` (x int);\n").unwrap();
    let dump = inspect_dump(&path).unwrap();
    assert_eq!(dump.prefix.as_str(), "wp_");
    assert_eq!(dump.site_mode, SiteMode::SingleSite);
    std::fs::write(&path, b"CREATE TABLE `wp_options` (x int);").unwrap();
    assert!(inspect_dump(&path).is_err());
}

#[test]
fn forbidden_database_switch_and_corrupt_gzip_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let sql = temp.path().join("unsafe.sql");
    std::fs::write(&sql, b"USE other_db;\n").unwrap();
    assert!(inspect_dump(&sql).is_err());
    let gzip = temp.path().join("broken.gz");
    std::fs::write(&gzip, [0x1f, 0x8b, 0, 1, 2]).unwrap();
    assert!(inspect_dump(&gzip).is_err());
}

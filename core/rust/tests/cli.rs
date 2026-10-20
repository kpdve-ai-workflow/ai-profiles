use std::ffi::OsString;

use std::ffi::OsString;

use super::parse_args;

#[test]
fn cli_parser_rejects_unknown_arguments_and_accepts_paths_with_spaces() {
    assert!(parse_args(vec![OsString::from("--unknown")]).is_err());
    assert!(parse_args(vec![
        "status".into(),
        "--project".into(),
        "/tmp/project with spaces".into()
    ])
    .is_ok());
}

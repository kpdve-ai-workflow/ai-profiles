use std::ffi::OsString;

use super::{parse_args, validate_option_combinations, ParseResult};

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

#[test]
fn rename_requires_new_name_and_rejects_project_or_unrelated_options() {
    for (args, valid) in [
        (vec!["rename", "old", "--new-name", "new"], true),
        (vec!["rename", "old", "--new-name=new"], true),
        (vec!["rename", "old"], false),
        (vec!["rename", "old", "--new-name="], false),
        (
            vec![
                "rename",
                "old",
                "--new-name",
                "new",
                "--project",
                "/tmp/project",
            ],
            false,
        ),
        (
            vec!["rename", "old", "--new-name", "new", "--part", "patch"],
            false,
        ),
        (vec!["bump", "old", "--new-name", "new"], false),
    ] {
        let parsed = parse_args(args.into_iter().map(OsString::from).collect()).unwrap();
        let ParseResult::Options(options) = parsed else {
            panic!("expected options")
        };
        assert_eq!(validate_option_combinations(&options).is_ok(), valid);
    }
}

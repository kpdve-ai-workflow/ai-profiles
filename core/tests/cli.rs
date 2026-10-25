use std::ffi::OsString;

use super::{parse_args, validate_option_combinations, ParseResult};

#[test]
fn client_selection_is_explicit_closed_and_not_repeatable() {
    for args in [
        vec!["install", "react"],
        vec!["update"],
        vec!["switch", "react"],
        vec!["uninstall"],
        vec!["install", "react", "--client="],
        vec!["update", "--client", "other"],
        vec!["update", "--client", "codex", "--client=codex"],
        vec!["create", "react", "--client=opencode"],
        vec!["validate", "--client=all"],
        vec!["rename", "react", "--new-name=x", "--client=claude"],
        vec!["bump", "react", "--client=codex"],
    ] {
        let result = parse_args(args.iter().map(OsString::from).collect());
        assert!(
            match result {
                Err(_) => true,
                Ok(ParseResult::Options(options)) =>
                    validate_option_combinations(&options).is_err(),
                Ok(ParseResult::Help) => false,
            },
            "{args:?}"
        );
    }
    for client in ["opencode", "claude", "codex", "all"] {
        for args in [
            vec!["update".to_owned(), "--client".into(), client.into()],
            vec!["update".into(), format!("--client={client}")],
        ] {
            let Ok(ParseResult::Options(options)) =
                parse_args(args.into_iter().map(OsString::from).collect())
            else {
                panic!("expected client options")
            };
            assert!(validate_option_combinations(&options).is_ok());
        }
    }
}

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

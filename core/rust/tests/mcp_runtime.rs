use std::{collections::HashMap, path::Path};

use serde_json::json;

use crate::mcp::environment::{dotenv, environment_for};

#[test]
fn dotenv_is_literal_and_does_not_interpolate_shell_variables() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join(".env");
    std::fs::write(
        &path,
        b"\xef\xbb\xbf# comment\nexport A='one'\nB=\"$A value\" # tail\n",
    )
    .unwrap();
    let parsed = dotenv(&path).unwrap();
    assert_eq!(parsed.get("A").map(String::as_str), Some("one"));
    assert_eq!(parsed.get("B").map(String::as_str), Some("$A value"));
    let inherited = HashMap::from([
        ("A".to_owned(), "inherited".to_owned()),
        ("OLD".to_owned(), "yes".to_owned()),
    ]);
    let entry = json!({"env_file":".env", "environment":{"FROM_A":"{env:A}", "FROM_B":"{env:B}"}});
    let effective = environment_for(temp.path(), entry.as_object().unwrap(), &inherited).unwrap();
    assert_eq!(effective.get("A").map(String::as_str), Some("one"));
    assert_eq!(effective.get("FROM_A").map(String::as_str), Some("one"));
    assert_eq!(
        effective.get("FROM_B").map(String::as_str),
        Some("$A value")
    );
    let invalid = json!({"environment":{"MISSING":"{env:NOPE}"}});
    assert!(environment_for(
        Path::new("/nonexistent"),
        invalid.as_object().unwrap(),
        &inherited
    )
    .is_err());
}

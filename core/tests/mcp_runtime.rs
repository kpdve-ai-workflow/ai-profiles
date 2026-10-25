use std::{collections::HashMap, path::Path};

use serde_json::json;

use crate::mcp::environment::{dotenv, environment_for};

#[test]
fn project_location_follows_the_installed_adapter_not_its_invocation_path() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("installed project,with spaces");
    std::fs::create_dir_all(project.join(".opencode")).unwrap();
    let executable = project.join(".opencode/ai-profile-mcp");
    std::fs::write(&executable, b"adapter").unwrap();
    let alias = temp.path().join("adapter-alias");
    std::os::unix::fs::symlink(&executable, &alias).unwrap();
    assert_eq!(
        crate::mcp::runtime::project_from_adapter(&alias).unwrap(),
        project.canonicalize().unwrap()
    );
    let unrelated = temp.path().join("not-an-installed-adapter");
    std::fs::write(&unrelated, b"adapter").unwrap();
    assert!(crate::mcp::runtime::project_from_adapter(&unrelated).is_err());
}

#[test]
fn isolated_adapters_and_local_overrides_use_their_own_client() {
    use crate::{layout::Layout, model::Client};
    let project = tempfile::tempdir().unwrap();
    for client in Client::ALL {
        let layout = Layout::Isolated(client);
        let adapter = project.path().join(layout.adapter());
        std::fs::create_dir_all(adapter.parent().unwrap()).unwrap();
        std::fs::write(&adapter, b"adapter").unwrap();
        std::fs::write(
            project.path().join(layout.local()),
            format!(
                r#"{{"mcp":{{"server":{{"environment":{{"CLIENT":"{}"}}}}}}}}"#,
                client.as_str()
            ),
        )
        .unwrap();
        assert_eq!(
            crate::mcp::runtime::project_from_adapter(&adapter).unwrap(),
            project.path().canonicalize().unwrap()
        );
        let base = json!({"server":{"type":"local","command":["example"]}});
        let resolved = crate::mcp::config::resolve_at(
            project.path(),
            base.as_object().unwrap(),
            &layout.local(),
        )
        .unwrap();
        assert_eq!(
            resolved.servers["server"]["environment"]["CLIENT"],
            client.as_str()
        );
    }
}

#[test]
fn project_metadata_overrides_every_environment_source_only_in_child() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().canonicalize().unwrap();
    let before = std::env::var_os("AI_PROFILE_PROJECT_DIR");
    for source in ["inherited", "dotenv", "mapping"] {
        std::fs::write(
            project.join(".env"),
            "AI_PROFILE_PROJECT_DIR=/wrong/dotenv\n",
        )
        .unwrap();
        let inherited =
            HashMap::from([("AI_PROFILE_PROJECT_DIR".into(), "/wrong/inherited".into())]);
        let entry = match source {
            "dotenv" => json!({"env_file":".env"}),
            "mapping" => {
                json!({"env_file":".env", "environment":{"AI_PROFILE_PROJECT_DIR":"/wrong/mapping"}})
            }
            _ => json!({}),
        };
        let effective = environment_for(&project, entry.as_object().unwrap(), &inherited).unwrap();
        let mut child = std::process::Command::new("printenv");
        child.arg("AI_PROFILE_PROJECT_DIR").current_dir("/");
        crate::mcp::runtime::apply_environment(&mut child, &project, effective);
        let output = child.output().unwrap();
        assert!(output.status.success());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            project.to_str().unwrap()
        );
        assert_eq!(std::env::var_os("AI_PROFILE_PROJECT_DIR"), before);
    }
}

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
    let missing = json!({"env_file":".env.missing"});
    let error = environment_for(temp.path(), missing.as_object().unwrap(), &inherited)
        .unwrap_err()
        .to_string();
    assert!(error.contains(".env.missing"));
    assert!(error.contains("Create it in the project root"));
    std::fs::write(temp.path().join(".env.bad"), "not-an-assignment\n").unwrap();
    let malformed = json!({"env_file":".env.bad"});
    assert!(environment_for(temp.path(), malformed.as_object().unwrap(), &inherited).is_err());
    let invalid = json!({"environment":{"MISSING":"{env:NOPE}"}});
    assert!(environment_for(
        Path::new("/nonexistent"),
        invalid.as_object().unwrap(),
        &inherited
    )
    .is_err());
}

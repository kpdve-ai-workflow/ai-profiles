use serde_json::json;

use super::{project_name, shell_quote, RestoreMode};

#[test]
fn restore_plan_and_apply_are_distinct_states() {
    assert_ne!(RestoreMode::Plan, RestoreMode::Apply);
}

#[test]
fn project_confirmation_and_recovery_command_are_safely_quoted() {
    assert_eq!(
        project_name(&json!({"name":"wordpress-local_2"})).unwrap(),
        "wordpress-local_2"
    );
    assert!(project_name(&json!({"name":"bad;command"})).is_err());
    assert_eq!(
        shell_quote("/tmp/project with ' quote"),
        "'/tmp/project with '\\'' quote'"
    );
}

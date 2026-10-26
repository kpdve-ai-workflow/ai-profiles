use std::{
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{logging, model::ProfileAction};

static NEXT_OPERATION: AtomicU64 = AtomicU64::new(1);

pub(super) fn action_name(action: ProfileAction) -> &'static str {
    match action {
        ProfileAction::Install => "install",
        ProfileAction::Update => "update",
        ProfileAction::Switch => "switch",
        ProfileAction::Uninstall => "uninstall",
        ProfileAction::Status => "status",
        ProfileAction::Help => "help",
        ProfileAction::Create => "create",
        ProfileAction::Rename => "rename",
        ProfileAction::Validate => "validate",
        ProfileAction::MigrateLegacy => "migrate-legacy",
        ProfileAction::Bump => "bump",
    }
}

pub(super) fn operation<T>(
    action: ProfileAction,
    project: &Path,
    invoke: impl FnOnce(&str) -> Result<T, Box<dyn std::error::Error>>,
) -> Result<T, Box<dyn std::error::Error>> {
    let action = action_name(action);
    let run_id = logging::current_session().map(|session| session.run_id);
    let sequence = NEXT_OPERATION.fetch_add(1, Ordering::Relaxed);
    let operation_id = run_id
        .as_deref()
        .map(|run_id| format!("{run_id}-operation-{sequence}"))
        .unwrap_or_else(|| format!("operation-{sequence}"));
    let span = tracing::info_span!(
        "profile.operation",
        operation_id = %operation_id,
        action,
        project = %project.display(),
        run_id = run_id.as_deref().unwrap_or("unavailable"),
    );
    let _guard = span.enter();
    tracing::info!(
        event = "profile.operation.started",
        operation_id = %operation_id,
        action,
        project = %project.display(),
    );
    if let Some(session) = logging::current_session() {
        tracing::subscriber::with_default(session.subscriber(), || invoke(&operation_id))
    } else {
        invoke(&operation_id)
    }
}

pub(super) fn error_chain(error: &(dyn std::error::Error + 'static)) -> String {
    let mut message = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        message.push_str(": ");
        message.push_str(&cause.to_string());
        source = cause.source();
    }
    message
}

pub(super) fn failed(operation_id: &str, action: &str, project: &Path) {
    tracing::error!(
        event = "profile.operation.finished",
        operation_id,
        action,
        project = %project.display(),
        result = "error",
        coverage = "selection_and_preparation",
    );
}

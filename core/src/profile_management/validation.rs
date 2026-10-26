use std::{fs, io::Cursor};

use serde_json::{Map, Value};

use crate::{
    codec::json_bytes,
    error::ValidationError,
    filesystem::file_path,
    mcp::config::{definitions_to_map, desired_mcp, mcp_definition},
    model::RepositoryContext,
    profile_management::payload::{desired_files, validate_skill_catalog},
    repository::{profile_metadata, profile_names},
};

pub(crate) fn validate_sources(
    context: &RepositoryContext,
    selected_profile: Option<&str>,
) -> Result<(), ValidationError> {
    super::diagnostics::source_operation(&context.sources, |operation_id| {
        validate_sources_inner(context, selected_profile, operation_id)
    })
}

fn validate_sources_inner(
    context: &RepositoryContext,
    selected_profile: Option<&str>,
    operation_id: &str,
) -> Result<(), ValidationError> {
    let scope = if selected_profile.is_some() {
        "profile"
    } else {
        "all_sources"
    };
    tracing::info!(
        event = "profile.validate.started",
        operation_id,
        scope,
        profile = selected_profile.unwrap_or(""),
        source_repository = %context.sources.display(),
    );
    if let Some(profile) = selected_profile {
        let names = validation_check(operation_id, scope, "profile_inventory", profile, || {
            profile_names(context).map_err(ValidationError::from)
        })?;
        if !names.iter().any(|name| name == profile) {
            return Err(ValidationError::UnknownProfile(profile.to_owned()));
        }
    }

    if selected_profile.is_none() {
        validation_check(operation_id, scope, "skill_catalog", "", || {
            validate_skill_catalog(context).map_err(ValidationError::from)
        })?;
        let registry = file_path(&context.sources, "mcp/servers")?;
        let mut definitions = fs::read_dir(&registry)
            .map_err(|source| ValidationError::Io {
                path: registry.clone(),
                source,
            })?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|source| ValidationError::Io {
                        path: registry.clone(),
                        source,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        definitions.sort();
        for path in definitions {
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                let name = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| ValidationError::InvalidRegistryName(path.clone()))?;
                validation_check(operation_id, scope, "mcp_definition", name, || {
                    mcp_definition(context, name).map_err(ValidationError::from)
                })?;
            }
        }
    } else {
        for check in ["skill_catalog", "mcp_registry"] {
            tracing::info!(
                event = "profile.validate.check",
                operation_id,
                scope,
                check,
                result = "skipped",
                reason = "single_profile_scope"
            );
        }
    }

    let profiles = match selected_profile {
        Some(profile) => vec![profile.to_owned()],
        None => profile_names(context)?,
    };
    let mut all_mcp = Map::new();
    for profile in profiles {
        let metadata = validation_check(operation_id, scope, "profile_metadata", &profile, || {
            profile_metadata(context, &profile).map_err(ValidationError::from)
        })?;
        if let Ok(bytes) = crate::codec::json_bytes(&metadata) {
            record_payload(operation_id, "profile_metadata", &profile, &bytes);
        }
        let files = validation_check(operation_id, scope, "profile_skills", &profile, || {
            desired_files(context, &profile).map_err(ValidationError::from)
        })?;
        for file in &files {
            record_payload(
                operation_id,
                "profile_file",
                &file.relative_path,
                &file.contents,
            );
        }
        let definitions = validation_check(
            operation_id,
            scope,
            "profile_mcp_definitions",
            &profile,
            || desired_mcp(context, &profile).map_err(ValidationError::from),
        )?;
        for definition in &definitions {
            if let Ok(bytes) = json_bytes(&definition.raw) {
                record_payload(
                    operation_id,
                    "mcp_definition",
                    definition.name.as_str(),
                    &bytes,
                );
            }
        }
        for (name, definition) in definitions_to_map(&definitions) {
            all_mcp.insert(name, definition);
        }
    }

    let example_path = context.sources.join("mcp/example.json");
    let (example, example_bytes) = crate::codec::read_json_with_bytes(&example_path)?;
    record_payload(
        operation_id,
        "mcp_example",
        "mcp/example.json",
        &example_bytes,
    );
    let Some(example_object) = example.as_object() else {
        return Err(ValidationError::InvalidExample);
    };
    if example_object.len() != 1 {
        return Err(ValidationError::InvalidExample);
    }
    let Some(example_mcp) = example_object.get("mcp").and_then(Value::as_object) else {
        return Err(ValidationError::InvalidExample);
    };
    if example_mcp.values().any(|entry| !entry.is_object()) {
        return Err(ValidationError::InvalidExample);
    }

    if let Some(profile) = selected_profile {
        let matches = !all_mcp
            .iter()
            .any(|(name, entry)| example_mcp.get(name) != Some(entry));
        tracing::info!(
            event = "profile.validate.example_compared",
            operation_id,
            scope,
            profile,
            result = if matches { "matched" } else { "mismatch" },
            checked_definitions = all_mcp.len(),
        );
        if !matches {
            return Err(ValidationError::ScopedExampleMismatch(profile.to_owned()));
        }
        println!("OK: profil {profile}, wybrane skille i definicje MCP są spójne");
        return Ok(());
    }

    let matches = example_mcp == &all_mcp;
    tracing::info!(
        event = "profile.validate.example_compared",
        operation_id,
        scope,
        result = if matches { "matched" } else { "mismatch" },
        checked_definitions = all_mcp.len(),
    );
    if !matches {
        return Err(ValidationError::FullExampleMismatch);
    }
    println!("OK: wszystkie skille i definicje MCP są spójne");
    Ok(())
}

fn validation_check<T>(
    operation_id: &str,
    scope: &str,
    check: &str,
    item: &str,
    invoke: impl FnOnce() -> Result<T, ValidationError>,
) -> Result<T, ValidationError> {
    tracing::info!(
        event = "profile.validate.check.started",
        operation_id,
        scope,
        check,
        item,
    );
    let result = invoke();
    match &result {
        Ok(_) => tracing::info!(
            event = "profile.validate.check.finished",
            operation_id,
            scope,
            check,
            item,
            result = "success",
        ),
        Err(error) => tracing::error!(
            event = "profile.validate.check.finished",
            operation_id,
            scope,
            check,
            item,
            result = "error",
            error = %super::diagnostics::error_chain(error),
        ),
    }
    result
}

fn record_payload(operation_id: &str, role: &str, path: &str, bytes: &[u8]) {
    let Some(session) = crate::logging::current_session() else {
        return;
    };
    let summary = session.payload(Cursor::new(bytes));
    tracing::info!(
        event = "profile.validate.payload",
        operation_id,
        role,
        path,
        payload_id = summary.payload_id,
        bytes = summary.length,
        complete = summary.complete,
    );
}

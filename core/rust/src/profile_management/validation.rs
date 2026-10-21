use std::fs;

use serde_json::{Map, Value};

use crate::{
    codec::read_json,
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
    if let Some(profile) = selected_profile {
        if !profile_names(context)?.iter().any(|name| name == profile) {
            return Err(ValidationError::UnknownProfile(profile.to_owned()));
        }
    }

    if selected_profile.is_none() {
        validate_skill_catalog(context)?;
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
                mcp_definition(context, name)?;
            }
        }
    }

    let profiles = match selected_profile {
        Some(profile) => vec![profile.to_owned()],
        None => profile_names(context)?,
    };
    let mut all_mcp = Map::new();
    for profile in profiles {
        profile_metadata(context, &profile)?;
        let _ = desired_files(context, &profile)?;
        let definitions = desired_mcp(context, &profile)?;
        for (name, definition) in definitions_to_map(&definitions) {
            all_mcp.insert(name, definition);
        }
    }

    let example_path = context.sources.join("mcp/example.json");
    let example = read_json(&example_path)?;
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
        if all_mcp
            .iter()
            .any(|(name, entry)| example_mcp.get(name) != Some(entry))
        {
            return Err(ValidationError::ScopedExampleMismatch(profile.to_owned()));
        }
        println!("OK: profil {profile}, wybrane skille i definicje MCP są spójne");
        return Ok(());
    }

    if example_mcp != &all_mcp {
        return Err(ValidationError::FullExampleMismatch);
    }
    println!("OK: wszystkie skille i definicje MCP są spójne");
    Ok(())
}

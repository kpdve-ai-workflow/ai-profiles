use std::collections::HashMap;

use crate::{
    error::WordPressError,
    wordpress::{Origin, UrlMapping, UrlMappings},
};

pub(crate) fn origin(value: &str) -> Result<Origin, WordPressError> {
    let normalized = urlsplit_input(value);
    let value = normalized.as_str();
    let Some((scheme, remainder)) = value.split_once("://") else {
        return Err(WordPressError::InvalidOrigin);
    };
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(WordPressError::InvalidOrigin);
    }

    let authority_end = remainder.find(['/', '?', '#']).unwrap_or(remainder.len());
    let authority = &remainder[..authority_end];
    let suffix = &remainder[authority_end..];
    let path_end = suffix.find(['?', '#']).unwrap_or(suffix.len());
    let path = &suffix[..path_end];
    let query_fragment = &suffix[path_end..];
    let query_fragment_values_nonempty = query_fragment
        .split(['?', '#'])
        .skip(1)
        .any(|value| !value.is_empty());

    if authority.is_empty()
        || !valid_authority(authority)
        || (path != "" && path != "/")
        || query_fragment_values_nonempty
    {
        return Err(WordPressError::InvalidOrigin);
    }

    let authority = authority.to_ascii_lowercase();
    Ok(Origin {
        value: format!("{scheme}://{authority}"),
        authority,
    })
}

fn valid_authority(authority: &str) -> bool {
    let mut parts = authority.split(':');
    let host = parts.next().unwrap_or_default();
    if host.is_empty()
        || !host
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'.' || byte == b'-')
    {
        return false;
    }
    match parts.next() {
        None => true,
        Some(port) => {
            !port.is_empty()
                && port.bytes().all(|byte| byte.is_ascii_digit())
                && parts.next().is_none()
        }
    }
}

pub(crate) fn mappings(values: &[String]) -> Result<UrlMappings, WordPressError> {
    let mut result = Vec::with_capacity(values.len());
    let mut seen_origins = HashMap::new();
    let mut host_targets = HashMap::new();

    for value in values {
        let Some((old, new)) = value.split_once('=') else {
            return Err(WordPressError::InvalidMappingSyntax);
        };
        let source = origin(old)?;
        let target = origin(new)?;
        if seen_origins.contains_key(source.as_str()) || source == target {
            return Err(WordPressError::DuplicateOrIdentityMapping);
        }
        if host_targets
            .get(source.authority())
            .is_some_and(|previous| previous != target.authority())
        {
            return Err(WordPressError::SourceHostConflict);
        }
        host_targets.insert(source.authority().to_owned(), target.authority().to_owned());
        seen_origins.insert(source.as_str().to_owned(), ());
        result.push(UrlMapping { source, target });
    }

    if result.is_empty()
        || result.iter().any(|mapping| {
            result
                .iter()
                .any(|candidate| candidate.target.as_str().contains(mapping.source.as_str()))
        })
    {
        return Err(WordPressError::MissingOrChainedMappings);
    }
    let unique_target_hosts = host_targets
        .values()
        .map(String::as_str)
        .collect::<std::collections::HashSet<_>>();
    if unique_target_hosts.len() != host_targets.len() {
        return Err(WordPressError::TargetHostCollision);
    }

    // Stable ordering preserves input order among equal-length origins.
    result.sort_by(|left, right| right.source.as_str().len().cmp(&left.source.as_str().len()));
    Ok(UrlMappings(result))
}

pub(crate) fn mapped_url(value: &str, mapping: &UrlMappings) -> Result<String, WordPressError> {
    let source = urlsplit_origin(value);
    let Some(rule) = mapping
        .entries()
        .iter()
        .find(|rule| rule.source.as_str() == source)
    else {
        return Err(WordPressError::UnmappedUrl);
    };
    let suffix = value
        .get(source.len()..)
        .ok_or(WordPressError::UnmappedUrl)?;
    Ok(format!("{}{suffix}", rule.target.as_str()))
}

/// Match `urllib.parse.urlsplit` for the scheme/authority pair used by mapped URLs,
/// without normalizing the authority (origin configuration intentionally does).
fn urlsplit_origin(value: &str) -> String {
    let normalized = urlsplit_input(value);
    let value = normalized.as_str();
    if let Some(network_path) = value.strip_prefix("//") {
        let end = network_path
            .find(['/', '?', '#'])
            .unwrap_or(network_path.len());
        return format!("://{}", &network_path[..end]);
    }
    let Some(colon) = value.find(':') else {
        return "://".to_owned();
    };
    let raw_scheme = &value[..colon];
    if !valid_scheme(raw_scheme) {
        return "://".to_owned();
    }
    let scheme = raw_scheme.to_ascii_lowercase();
    let after_scheme = &value[colon + 1..];
    if let Some(authority_and_path) = after_scheme.strip_prefix("//") {
        let end = authority_and_path
            .find(['/', '?', '#'])
            .unwrap_or(authority_and_path.len());
        format!("{scheme}://{}", &authority_and_path[..end])
    } else {
        format!("{scheme}://")
    }
}

fn urlsplit_input(value: &str) -> String {
    let without_leading_controls =
        value.trim_start_matches(|character: char| character <= '\u{20}');
    without_leading_controls
        .chars()
        .filter(|character| !matches!(character, '\t' | '\r' | '\n'))
        .collect()
}

fn valid_scheme(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'a'..=b'z'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'.' | b'-'))
}

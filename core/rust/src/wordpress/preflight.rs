use std::{fs, path::Path};

use regex::Regex;
use serde_json::Value;

use crate::{
    error::WordPressError,
    filesystem::file_path,
    wordpress::{
        ddev::{topology, Ddev},
        dump::inspect_dump,
        DumpInfo, UrlMappings, WordPressTopology,
    },
};

#[derive(Clone, Debug)]
pub(crate) struct PreflightResult {
    pub(crate) info: Value,
    pub(crate) source: DumpInfo,
    pub(crate) current: WordPressTopology,
}

pub(crate) fn inside(project: &Path, relative: &str) -> Result<std::path::PathBuf, WordPressError> {
    file_path(project, relative).map_err(WordPressError::Filesystem)
}

pub(crate) fn preflight(
    project: &Path,
    dump: &Path,
    mappings: &UrlMappings,
    ddev: &mut Ddev<'_>,
) -> Result<PreflightResult, WordPressError> {
    if !inside(project, ".ddev/config.yaml")?.is_file() {
        return Err(WordPressError::DdevProjectRequired);
    }
    for name in ["local-guard.php", "local-safety.php"] {
        if !inside(project, &format!(".opencode/ai-profile/scripts/{name}"))?.is_file() {
            return Err(WordPressError::MissingWordPressHelpers);
        }
    }

    let info = ddev.describe()?;
    let approot = info
        .get("approot")
        .and_then(Value::as_str)
        .ok_or(WordPressError::InvalidDdevFields)?;
    let approot = fs::canonicalize(approot).map_err(|source| WordPressError::Io {
        path: approot.into(),
        source,
    })?;
    let project_root = fs::canonicalize(project).map_err(|source| WordPressError::Io {
        path: project.to_owned(),
        source,
    })?;
    if approot != project_root
        || info.get("status").and_then(Value::as_str) != Some("running")
        || info.get("type").and_then(Value::as_str) != Some("wordpress")
    {
        return Err(WordPressError::InvalidDdevProject);
    }

    let docroot = info
        .get("docroot")
        .and_then(Value::as_str)
        .unwrap_or_default();
    ddev.set_docroot(docroot);
    inside(project, docroot)?;
    if !inside(
        project,
        &Path::new(docroot).join("wp-config.php").to_string_lossy(),
    )?
    .is_file()
    {
        return Err(WordPressError::MissingWpConfig);
    }
    if !inside(
        project,
        &Path::new(docroot).join("wp-content").to_string_lossy(),
    )?
    .is_dir()
    {
        return Err(WordPressError::MissingWpContent);
    }

    let urls = info
        .get("urls")
        .and_then(Value::as_array)
        .ok_or(WordPressError::InvalidDdevUrls)?;
    let mut hosts = Vec::new();
    for value in urls.iter().chain(
        ["primary_url", "httpurl", "httpsurl"]
            .iter()
            .filter_map(|key| info.get(*key)),
    ) {
        if let Some(value) = value.as_str().filter(|value| !value.is_empty()) {
            if let Ok(parsed) = url::Url::parse(value) {
                if let Some(host) = parsed.host_str() {
                    hosts.push(format!(
                        "{}{}",
                        host.to_ascii_lowercase(),
                        parsed
                            .port()
                            .map(|port| format!(":{port}"))
                            .unwrap_or_default()
                    ));
                }
            }
        }
    }
    if urls.iter().any(|value| !value.is_string()) {
        return Err(WordPressError::InvalidDdevUrls);
    }
    for mapping in mappings.entries() {
        if !hosts.iter().any(|host| host == mapping.target.authority()) {
            return Err(WordPressError::UnknownDdevTargetHost);
        }
    }

    let prefix = ddev.wp(&["ai-profile-local-prefix".to_owned()])?;
    let prefix_pattern = Regex::new(r"^[A-Za-z0-9_]+$").expect("fixed prefix regex is valid");
    if !prefix_pattern.is_match(&prefix) {
        return Err(WordPressError::InvalidLocalTablePrefix);
    }
    let connection = ddev.query("SELECT @@hostname, DATABASE()")?;
    let native = ddev.run(&[
        "mysql".to_owned(),
        "--database=db".to_owned(),
        "--batch".to_owned(),
        "--skip-column-names".to_owned(),
        "--execute=SELECT @@hostname, DATABASE()".to_owned(),
    ])?;
    if connection
        .first()
        .is_none_or(|row| row.last().map(String::as_str) != Some("db"))
        || connection
            .first()
            .is_none_or(|row| row.join("\t") != native)
    {
        return Err(WordPressError::DatabaseMismatch);
    }

    let table_prefix = crate::wordpress::TablePrefix::from_validated(prefix);
    let current = topology(ddev, &table_prefix)?;
    let source = inspect_dump(dump)?;
    if source.prefix.as_str() != table_prefix.as_str() || source.site_mode != current.site_mode {
        return Err(WordPressError::DumpTopologyMismatch);
    }

    let safety_path = Path::new(docroot).join("wp-content/mu-plugins/ai-profile-local-safety.php");
    let guard = inside(project, &safety_path.to_string_lossy())?;
    let safety = fs::read(inside(
        project,
        ".opencode/ai-profile/scripts/local-safety.php",
    )?)
    .map_err(|source| WordPressError::Io {
        path: project.join(".opencode/ai-profile/scripts/local-safety.php"),
        source,
    })?;
    if guard.exists()
        && fs::read(&guard).map_err(|source| WordPressError::Io {
            path: guard.clone(),
            source,
        })? != safety
    {
        return Err(WordPressError::SafetyFileConflict);
    }
    Ok(PreflightResult {
        info,
        source,
        current,
    })
}

use std::{fs, path::Path};

use regex::Regex;
use serde_json::Value;

use crate::{
    error::WordPressError,
    filesystem::file_path,
    wordpress::{
        ddev::{topology, Ddev},
        dump::inspect_dump_with_diagnostics,
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
    operation_id: &str,
) -> Result<PreflightResult, WordPressError> {
    tracing::info!(event = "restore.preflight.started", operation_id, project = %project.display(), dump = %dump.display(), mapping_count = mappings.entries().len());
    let result = preflight_inner(project, dump, mappings, ddev, operation_id);
    match &result {
        Ok(plan) => tracing::info!(
            event = "restore.preflight.finished",
            operation_id,
            result = "passed",
            project_name = plan.info.get("name").and_then(|value| value.as_str()).unwrap_or(""),
            dump_sha256 = plan.source.sha256.as_str(),
            table_prefix = plan.source.prefix.as_str(),
            site_mode = ?plan.source.site_mode,
        ),
        Err(error) => tracing::error!(
            event = "restore.preflight.rejected",
            operation_id,
            error = %error,
            source = ?std::error::Error::source(error),
        ),
    }
    result
}

fn preflight_inner(
    project: &Path,
    dump: &Path,
    mappings: &UrlMappings,
    ddev: &mut Ddev<'_>,
    operation_id: &str,
) -> Result<PreflightResult, WordPressError> {
    if !inside(project, ".ddev/config.yaml")?.is_file() {
        return Err(WordPressError::DdevProjectRequired);
    }
    tracing::info!(event = "restore.preflight.check", operation_id, check = "ddev_config", result = "passed", path = %project.join(".ddev/config.yaml").display());
    for name in ["local-guard.php", "local-safety.php"] {
        if !inside(
            project,
            &format!("{}/{name}", super::ddev::scripts_for(project)?),
        )?
        .is_file()
        {
            return Err(WordPressError::MissingWordPressHelpers);
        }
    }
    tracing::info!(
        event = "restore.preflight.check",
        operation_id,
        check = "wordpress_helpers",
        result = "passed"
    );

    let info = ddev.describe()?;
    tracing::info!(
        event = "restore.preflight.check",
        operation_id,
        check = "ddev_describe",
        result = "passed",
        status = info
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or(""),
        project_type = info
            .get("type")
            .and_then(|value| value.as_str())
            .unwrap_or("")
    );
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
    tracing::info!(event = "restore.preflight.check", operation_id, check = "project_identity_and_state", result = "passed", approot = %approot.display());

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
    tracing::info!(
        event = "restore.preflight.check",
        operation_id,
        check = "wordpress_files",
        result = "passed",
        docroot
    );

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
    tracing::info!(event = "restore.preflight.check", operation_id, check = "mapping_targets", result = "passed", mapping_count = mappings.entries().len(), available_hosts = ?hosts);

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
    tracing::info!(
        event = "restore.preflight.check",
        operation_id,
        check = "database_identity",
        result = "passed",
        host = connection
            .first()
            .and_then(|row| row.first())
            .map(String::as_str)
            .unwrap_or(""),
        database = "db"
    );

    let table_prefix = crate::wordpress::TablePrefix::from_validated(prefix);
    let current = topology(ddev, &table_prefix)?;
    let source = inspect_dump_with_diagnostics(dump, operation_id, "source_dump")?;
    if source.prefix.as_str() != table_prefix.as_str() || source.site_mode != current.site_mode {
        return Err(WordPressError::DumpTopologyMismatch);
    }
    tracing::info!(event = "restore.preflight.check", operation_id, check = "dump_topology", result = "passed", dump_sha256 = source.sha256.as_str(), table_prefix = source.prefix.as_str(), site_mode = ?source.site_mode, current_site_mode = ?current.site_mode);

    let safety_path = Path::new(docroot).join("wp-content/mu-plugins/ai-profile-local-safety.php");
    let guard = inside(project, &safety_path.to_string_lossy())?;
    let safety_source = inside(
        project,
        &format!("{}/local-safety.php", super::ddev::scripts_for(project)?),
    )?;
    let safety = fs::read(&safety_source).map_err(|source| WordPressError::Io {
        path: safety_source,
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
    tracing::info!(event = "restore.preflight.check", operation_id, check = "local_safety_guard", result = "passed", path = %guard.display(), existed = guard.exists());
    Ok(PreflightResult {
        info,
        source,
        current,
    })
}

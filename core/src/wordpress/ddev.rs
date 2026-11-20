use std::{collections::HashMap, ffi::OsString, path::Path, sync::OnceLock};

use regex::Regex;
use serde_json::Value;

use crate::{
    error::WordPressError,
    process::{CancellationToken, CommandSpec, ProcessRunner, StdinMode, StreamMode},
    wordpress::{SiteMode, TablePrefix, WordPressNetwork, WordPressSite, WordPressTopology},
};

const CONTAINER_ROOT: &str = "/var/www/html";
const SCRIPTS: &str = ".opencode/ai-profile/scripts";

/// The invoked helper selects its own payload; never scan installed clients.
pub(crate) fn scripts_for(project: &Path) -> Result<String, WordPressError> {
    let executable =
        std::env::current_exe().map_err(|_| WordPressError::MissingWordPressHelpers)?;
    if let Some(scripts) = scripts_from_executable(project, &executable) {
        return Ok(scripts);
    }
    if executable.ancestors().any(|p| {
        p.file_name()
            .is_some_and(|s| s == ".ai-profiles" || s == ".opencode")
    }) {
        return Err(WordPressError::MissingWordPressHelpers);
    }
    // Source-tree/standalone tools retain the historical explicitly OpenCode layout.
    Ok(SCRIPTS.into())
}

pub(crate) fn scripts_from_executable(project: &Path, executable: &Path) -> Option<String> {
    use crate::{layout::Layout, model::Client};
    let scripts = executable.parent()?;
    for client in Client::ALL {
        let relative = Layout::Isolated(client).scripts();
        if scripts == project.join(&relative) {
            return Some(relative);
        }
    }
    if scripts == project.join(SCRIPTS) {
        return Some(SCRIPTS.into());
    }
    None
}

pub(crate) struct Ddev<'a> {
    project: &'a Path,
    docroot: String,
    runner: &'a dyn ProcessRunner,
    cancellation: CancellationToken,
}

impl<'a> Ddev<'a> {
    pub(crate) fn new(
        project: &'a Path,
        runner: &'a dyn ProcessRunner,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            project,
            docroot: String::new(),
            runner,
            cancellation,
        }
    }

    pub(crate) fn set_docroot(&mut self, docroot: &str) {
        self.docroot = docroot.to_owned();
    }

    pub(crate) fn docroot(&self) -> &str {
        &self.docroot
    }

    pub(crate) fn run(&self, args: &[String]) -> Result<String, WordPressError> {
        let step = args.first().map(String::as_str).unwrap_or("");
        let output = self
            .runner
            .run(CommandSpec {
                program: OsString::from("ddev"),
                args: args.iter().cloned().map(OsString::from).collect(),
                cwd: Some(self.project.to_owned()),
                env: HashMap::new(),
                clear_env: false,
                stdin: StdinMode::Null,
                stdout: StreamMode::Capture,
                stderr: StreamMode::Capture,
                timeout: None,
                cancellation: self.cancellation.clone(),
            })
            .map_err(|_| WordPressError::DdevUnavailable)?;
        if !output.status.success() {
            return Err(WordPressError::DdevStepFailed {
                step: step.to_owned(),
            });
        }
        let stderr =
            String::from_utf8(output.stderr).map_err(|_| WordPressError::DdevUnavailable)?;
        if args.iter().any(|argument| argument == "search-replace")
            && has_search_replace_warning(&stderr)
        {
            return Err(WordPressError::SearchReplaceWarning);
        }
        let stdout =
            String::from_utf8(output.stdout).map_err(|_| WordPressError::DdevUnavailable)?;
        Ok(stdout.trim().to_owned())
    }

    pub(crate) fn describe(&self) -> Result<Value, WordPressError> {
        let output = self.run(&["describe".to_owned(), "--json-output".to_owned()])?;
        let mut messages = match serde_json::from_str::<Value>(&output) {
            Ok(message) => vec![message],
            Err(_) => output
                .lines()
                .filter(|line| line.starts_with('{'))
                .map(serde_json::from_str::<Value>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| WordPressError::InvalidDdevDescribe)?,
        };
        while let Some(message) = messages.pop() {
            let value = message
                .as_object()
                .and_then(|object| object.get("raw"))
                .unwrap_or(&message);
            if value
                .as_object()
                .is_some_and(|object| object.contains_key("approot"))
            {
                return Ok(value.clone());
            }
        }
        Err(WordPressError::InvalidDdevDescribe)
    }

    pub(crate) fn wp(&self, args: &[String]) -> Result<String, WordPressError> {
        let directory = if self.docroot.is_empty() {
            CONTAINER_ROOT.to_owned()
        } else {
            format!("{CONTAINER_ROOT}/{}", self.docroot)
        };
        let mut command = vec![
            "exec".to_owned(),
            format!("--dir={directory}"),
            "--raw".to_owned(),
            "--".to_owned(),
            "wp".to_owned(),
            format!(
                "--require={CONTAINER_ROOT}/{}/local-guard.php",
                scripts_for(self.project)?
            ),
            "--skip-plugins".to_owned(),
            "--skip-themes".to_owned(),
        ];
        command.extend(args.iter().cloned());
        self.run(&command)
    }

    pub(crate) fn query(&self, sql: &str) -> Result<Vec<Vec<String>>, WordPressError> {
        let output = self.wp(&[
            "db".to_owned(),
            "query".to_owned(),
            sql.to_owned(),
            "--batch".to_owned(),
            "--skip-column-names".to_owned(),
        ])?;
        Ok(output
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| line.split('\t').map(str::to_owned).collect())
            .collect())
    }
}

fn has_search_replace_warning(stderr: &str) -> bool {
    static WARNING: OnceLock<Regex> = OnceLock::new();
    WARNING
        .get_or_init(|| {
            Regex::new(r"(?i)\b(?:warning|error)\b")
                .expect("fixed WP-CLI diagnostic regex is valid")
        })
        .is_match(stderr)
}

pub(crate) fn literal(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut hex = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    format!("CONVERT(0x{hex} USING utf8mb4)")
}

pub(crate) fn topology(
    ddev: &Ddev<'_>,
    prefix: &TablePrefix,
) -> Result<WordPressTopology, WordPressError> {
    let prefix = prefix.as_str();
    let tables = ddev
        .query("SHOW TABLES")?
        .into_iter()
        .map(|row| {
            row.first()
                .cloned()
                .ok_or(WordPressError::InvalidQueryOutput)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let table_set = tables.iter().collect::<std::collections::HashSet<_>>();
    let is_multisite = table_set.contains(&format!("{prefix}blogs"));

    let blog_rows = if is_multisite {
        ddev.query(&format!(
            "SELECT blog_id,domain,path FROM `{prefix}blogs` ORDER BY blog_id"
        ))?
    } else {
        vec![vec!["1".to_owned(), String::new(), "/".to_owned()]]
    };
    let mut sites = Vec::with_capacity(blog_rows.len());
    for row in blog_rows {
        let [blog_id, domain, path] = row.as_slice() else {
            return Err(WordPressError::InvalidQueryOutput);
        };
        if blog_id.is_empty() || !blog_id.chars().all(char::is_numeric) {
            return Err(WordPressError::InvalidBlogId);
        }
        let options_table = format!(
            "{prefix}{}options",
            if blog_id == "1" {
                String::new()
            } else {
                format!("{blog_id}_")
            }
        );
        let option_rows = ddev.query(&format!(
            "SELECT option_name,option_value FROM `{options_table}` WHERE option_name IN ('home','siteurl')"
        ))?;
        let mut options = HashMap::new();
        for row in option_rows {
            let [key, value] = row.as_slice() else {
                return Err(WordPressError::InvalidQueryOutput);
            };
            options.insert(key.clone(), value.clone());
        }
        if options.len() != 2 || !options.contains_key("home") || !options.contains_key("siteurl") {
            return Err(WordPressError::MissingSiteOptions);
        }
        let home = options
            .remove("home")
            .ok_or(WordPressError::MissingSiteOptions)?;
        let site_url = options
            .remove("siteurl")
            .ok_or(WordPressError::MissingSiteOptions)?;
        sites.push(WordPressSite {
            id: blog_id.clone(),
            domain: domain.clone(),
            path: path.clone(),
            options_table,
            home,
            site_url,
        });
    }

    let networks = if table_set.contains(&format!("{prefix}site")) {
        ddev.query(&format!(
            "SELECT id,domain,path FROM `{prefix}site` ORDER BY id"
        ))?
        .into_iter()
        .map(|row| {
            let [id, domain, path] = row.as_slice() else {
                return Err(WordPressError::InvalidQueryOutput);
            };
            Ok(WordPressNetwork {
                id: id.clone(),
                domain: domain.clone(),
                path: path.clone(),
            })
        })
        .collect::<Result<Vec<_>, WordPressError>>()?
    } else {
        Vec::new()
    };

    Ok(WordPressTopology {
        sites,
        networks,
        site_mode: if is_multisite {
            SiteMode::Multisite
        } else {
            SiteMode::SingleSite
        },
    })
}

use std::{
    collections::HashSet,
    ffi::OsStr,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

use crate::{
    error::PayloadError, filesystem::file_path, mcp::config::desired_mcp, model::RepositoryContext,
};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct SkillName(String);

impl SkillName {
    fn parse(value: &str) -> Result<Self, ()> {
        static NAME: OnceLock<Regex> = OnceLock::new();
        let valid = NAME.get_or_init(|| {
            Regex::new(r"^[a-z0-9]+(?:-[a-z0-9]+)*$").expect("fixed skill-name regex is valid")
        });
        if valid.is_match(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(())
        }
    }
    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PayloadKind {
    Data,
    Executable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PayloadFile {
    pub(crate) relative_path: String,
    pub(crate) contents: Vec<u8>,
    pub(crate) kind: PayloadKind,
}

pub(crate) fn desired_files(
    context: &RepositoryContext,
    profile: &str,
) -> Result<Vec<PayloadFile>, PayloadError> {
    let mut files = Vec::new();
    let selected = selected_skills(context, profile)?;
    for name in selected {
        let folder = file_path(&context.sources, &format!("skills/{}", name.as_str()))?;
        collect_skill(context, &name, &folder, &mut files)?;
    }

    // Both native compatibility manifests expose the same project-local skill tree.
    let metadata = crate::repository::profile_metadata(context, profile)?;
    let description = metadata
        .get("description")
        .and_then(Value::as_str)
        .unwrap_or("");
    let version = metadata
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("1.0.0");
    let manifest = serde_json::json!({"name":"ai-profile","version":version,"description":description,"skills":["./skills/"]});
    let manifest_bytes = crate::codec::json_bytes(&manifest)?;
    files.push(PayloadFile {
        relative_path: ".opencode/ai-profile/plugin/.claude-plugin/plugin.json".into(),
        contents: manifest_bytes.clone(),
        kind: PayloadKind::Data,
    });
    files.push(PayloadFile {
        relative_path: ".opencode/ai-profile/plugin/.codex-plugin/plugin.json".into(),
        contents: manifest_bytes,
        kind: PayloadKind::Data,
    });
    // A real empty directory is ensured by installation when selection is empty.

    insert_native(
        context,
        &mut files,
        "ai-profile-mcp",
        ".opencode/ai-profile-mcp",
    )?;

    for definition in desired_mcp(context, profile)? {
        let name = definition.name.as_str();
        if name == "mariadb" {
            insert_native(
                context,
                &mut files,
                "mariadb-launcher",
                ".opencode/mcp/mariadb/mariadb-launcher",
            )?;
        }
        if name == "playwright" {
            // Distribute a self-contained build context and standard Docker config.
            for filename in [
                "Dockerfile",
                "entrypoint.sh",
                "basic-auth.cjs",
                "LICENSES.md",
                "README.md",
                ".dockerignore",
                "compose.yaml",
                "compose.ca.yaml",
                "compose.auth.yaml",
                "compose.basic-auth.yaml",
            ] {
                let source = file_path(
                    &context.sources,
                    &format!("mcp/images/playwright/{filename}"),
                )?;
                insert_file(
                    &mut files,
                    &format!(".opencode/mcp/playwright/{filename}"),
                    &source,
                    true,
                )?;
            }
        }
        let launcher = file_path(&context.sources, &format!("mcp/launchers/{name}"))?;
        if launcher.exists() {
            if !launcher.is_dir() {
                return Err(PayloadError::InvalidLauncherDirectory(name.to_owned()));
            }
            for source in sorted_tree(&launcher)? {
                if source_is_symlink(&source)? {
                    return Err(PayloadError::LauncherSymlink(source));
                }
                let relative = source
                    .strip_prefix(&launcher)
                    .map_err(|_| PayloadError::InvalidPayloadPath(source.clone()))?;
                if contains_component(relative, "__pycache__") || has_extension(&source, "pyc") {
                    continue;
                }
                if name == "mariadb" && relative == Path::new("launch.py") {
                    continue; // Preserved reference; distributed as mariadb-launcher.
                }
                if source.is_file() {
                    let relative_path =
                        payload_path(&[".opencode/mcp", name, &path_to_posix(relative)?]);
                    insert_file(&mut files, &relative_path, &source, false)?;
                }
            }
        }

        for argument in definition.command.iter().flatten() {
            if argument.starts_with(".opencode/mcp/")
                && !files.iter().any(|file| file.relative_path == *argument)
            {
                return Err(PayloadError::MissingManagedLauncher(argument.clone()));
            }
        }
    }

    let readme = file_path(&context.sources, &format!("profiles/{profile}/README.md"))?;
    let scripts = file_path(&context.sources, &format!("profiles/{profile}/scripts"))?;
    if scripts.join("restore-updraft.py").is_file() {
        insert_native(
            context,
            &mut files,
            "restore-updraft",
            ".opencode/ai-profile/scripts/restore-updraft",
        )?;
    }
    if scripts.exists() {
        if !scripts.is_dir() {
            return Err(PayloadError::InvalidScriptsDirectory(profile.to_owned()));
        }
        for source in sorted_tree(&scripts)? {
            if source_is_symlink(&source)? {
                return Err(PayloadError::ScriptSymlink(source));
            }
            let relative = source
                .strip_prefix(&scripts)
                .map_err(|_| PayloadError::InvalidPayloadPath(source.clone()))?;
            if contains_component(relative, "__pycache__") || has_extension(&source, "pyc") {
                continue;
            }
            if relative == Path::new("restore-updraft.py") {
                continue; // Preserved reference; distributed as restore-updraft.
            }
            if source.is_file() {
                let relative_path =
                    payload_path(&[".opencode/ai-profile/scripts", &path_to_posix(relative)?]);
                insert_file(&mut files, &relative_path, &source, false)?;
            }
        }
    }
    if readme.is_file() {
        insert_file(&mut files, ".opencode/ai-profile/README.md", &readme, false)?;
    }
    Ok(files)
}

fn selected_skills(
    context: &RepositoryContext,
    profile: &str,
) -> Result<Vec<SkillName>, PayloadError> {
    let relative = format!("profiles/{profile}/skills.json");
    let manifest_path = match file_path(&context.sources, &relative) {
        Ok(path) => path,
        Err(crate::error::FilesystemError::Symlink(path)) => {
            return Err(PayloadError::SelectedSkillSymlink {
                profile: profile.to_owned(),
                path,
            });
        }
        Err(error) => return Err(error.into()),
    };
    let bytes = fs::read(&manifest_path).map_err(|source| PayloadError::SkillManifestRead {
        profile: profile.to_owned(),
        path: manifest_path.clone(),
        source,
    })?;
    let manifest: Value =
        serde_json::from_slice(&bytes).map_err(|_| PayloadError::InvalidSkillManifest {
            profile: profile.to_owned(),
            path: manifest_path.clone(),
        })?;
    let Some(object) = manifest.as_object() else {
        return Err(PayloadError::InvalidSkillManifest {
            profile: profile.to_owned(),
            path: manifest_path,
        });
    };
    if object.len() != 1 {
        return Err(PayloadError::InvalidSkillManifest {
            profile: profile.to_owned(),
            path: manifest_path,
        });
    }
    let Some(entries) = object.get("skills").and_then(Value::as_array) else {
        return Err(PayloadError::InvalidSkillManifest {
            profile: profile.to_owned(),
            path: manifest_path,
        });
    };
    let mut seen = HashSet::with_capacity(entries.len());
    let mut names = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(raw) = entry.as_str() else {
            return Err(PayloadError::InvalidSkillManifest {
                profile: profile.to_owned(),
                path: manifest_path.clone(),
            });
        };
        let name = SkillName::parse(raw).map_err(|_| PayloadError::InvalidSelectedSkill {
            profile: profile.to_owned(),
            path: manifest_path.clone(),
            name: raw.to_owned(),
        })?;
        if !seen.insert(name.clone()) {
            return Err(PayloadError::DuplicateSelectedSkill {
                profile: profile.to_owned(),
                path: manifest_path.clone(),
                name: raw.to_owned(),
            });
        }
        let skill_relative = format!("skills/{}", name.as_str());
        let skill_dir = match file_path(&context.sources, &skill_relative) {
            Ok(path) => path,
            Err(crate::error::FilesystemError::Symlink(path)) => {
                return Err(PayloadError::SelectedSkillSymlink {
                    profile: profile.to_owned(),
                    path,
                });
            }
            Err(error) => return Err(error.into()),
        };
        if !skill_dir.is_dir() {
            return Err(PayloadError::UnknownSelectedSkill {
                profile: profile.to_owned(),
                path: manifest_path.clone(),
                name: raw.to_owned(),
            });
        }
        names.push(name);
    }
    Ok(names)
}

pub(crate) fn validate_skill_catalog(context: &RepositoryContext) -> Result<(), PayloadError> {
    let root = file_path(&context.sources, "skills")?;
    let mut entries = read_directory(&root)?;
    entries.sort();
    for entry in entries {
        if entry.file_name().and_then(OsStr::to_str) == Some("README.md") {
            continue;
        }
        let metadata = fs::symlink_metadata(&entry).map_err(|source| PayloadError::Io {
            path: entry.clone(),
            source,
        })?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(PayloadError::InvalidSkillDirectory(entry));
        }
        let raw = entry
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or_else(|| PayloadError::InvalidPayloadPath(entry.clone()))?;
        let name = SkillName::parse(raw)
            .map_err(|_| PayloadError::InvalidSkillDirectory(entry.clone()))?;
        collect_skill(context, &name, &entry, &mut Vec::new())?;
    }
    Ok(())
}

fn collect_skill(
    context: &RepositoryContext,
    name: &SkillName,
    folder: &Path,
    files: &mut Vec<PayloadFile>,
) -> Result<(), PayloadError> {
    let skill_name = name.as_str();
    let skill_file = folder.join("SKILL.md");
    let document_type = fs::symlink_metadata(&skill_file).ok();
    if !document_type
        .as_ref()
        .is_some_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
    {
        return Err(PayloadError::MissingSkillDocument(skill_file));
    }
    let frontmatter = fs::read_to_string(&skill_file).map_err(|source| PayloadError::Io {
        path: skill_file.clone(),
        source,
    })?;
    if !frontmatter.starts_with("---\n") {
        return Err(PayloadError::InvalidFrontmatter(skill_file));
    }
    let Some((metadata, _rest)) = frontmatter[4..].split_once("\n---") else {
        return Err(PayloadError::InvalidFrontmatter(skill_file));
    };
    let Some((declared_name, has_description)) = skill_metadata(metadata) else {
        return Err(PayloadError::InvalidSkillMetadata {
            name: skill_name.to_owned(),
            path: skill_file,
        });
    };
    if declared_name != skill_name || !has_description {
        return Err(PayloadError::InvalidSkillMetadata {
            name: skill_name.to_owned(),
            path: skill_file,
        });
    }

    for source in tree_in_filesystem_order(folder)? {
        if source_is_symlink(&source)? {
            return Err(PayloadError::SkillSymlink(source));
        }
        if source.is_file() {
            let relative = source
                .strip_prefix(folder)
                .map_err(|_| PayloadError::InvalidPayloadPath(source.clone()))?;
            if skill_name == "test-gap-discovery"
                && relative == Path::new("scripts/inventory-tests.sh")
            {
                insert_native(
                    context,
                    files,
                    "test-inventory",
                    ".opencode/ai-profile/plugin/skills/test-gap-discovery/scripts/inventory-tests",
                )?;
                continue;
            }
            let relative_path = payload_path(&[
                ".opencode/ai-profile/plugin/skills",
                skill_name,
                &path_to_posix(relative)?,
            ]);
            insert_file(files, &relative_path, &source, true)?;
        }
    }
    Ok(())
}

fn skill_metadata(metadata: &str) -> Option<(&str, bool)> {
    static NAME: OnceLock<Regex> = OnceLock::new();
    static DESCRIPTION: OnceLock<Regex> = OnceLock::new();
    let name = NAME.get_or_init(|| {
        Regex::new(r#"(?m)^name:\s*["']?([a-z0-9]+(?:-[a-z0-9]+)*)["']?\s*$"#)
            .expect("fixed skill-name regex is valid")
    });
    let description = DESCRIPTION.get_or_init(|| {
        Regex::new(r"(?m)^description:\s*.+$").expect("fixed description regex is valid")
    });
    Some((
        name.captures(metadata)?.get(1)?.as_str(),
        description.is_match(metadata),
    ))
}

fn read_directory(path: &Path) -> Result<Vec<PathBuf>, PayloadError> {
    fs::read_dir(path)
        .map_err(|source| PayloadError::Io {
            path: path.to_owned(),
            source,
        })?
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|source| PayloadError::Io {
                    path: path.to_owned(),
                    source,
                })
        })
        .collect()
}

fn tree_in_filesystem_order(root: &Path) -> Result<Vec<PathBuf>, PayloadError> {
    let mut entries = Vec::new();
    collect_tree(root, &mut entries)?;
    Ok(entries)
}

fn sorted_tree(root: &Path) -> Result<Vec<PathBuf>, PayloadError> {
    let mut entries = tree_in_filesystem_order(root)?;
    entries.sort();
    Ok(entries)
}

fn collect_tree(root: &Path, entries: &mut Vec<PathBuf>) -> Result<(), PayloadError> {
    for path in read_directory(root)? {
        entries.push(path.clone());
        let metadata = fs::symlink_metadata(&path).map_err(|source| PayloadError::Io {
            path: path.clone(),
            source,
        })?;
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            collect_tree(&path, entries)?;
        }
    }
    Ok(())
}

fn source_is_symlink(path: &Path) -> Result<bool, PayloadError> {
    fs::symlink_metadata(path)
        .map(|metadata| metadata.file_type().is_symlink())
        .map_err(|source| PayloadError::Io {
            path: path.to_owned(),
            source,
        })
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension() == Some(OsStr::new(extension))
}

fn contains_component(path: &Path, expected: &str) -> bool {
    path.components()
        .any(|component| component.as_os_str() == expected)
}

fn path_to_posix(path: &Path) -> Result<String, PayloadError> {
    path.components()
        .map(|component| {
            component
                .as_os_str()
                .to_str()
                .map(str::to_owned)
                .ok_or_else(|| PayloadError::InvalidPayloadPath(path.to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join("/"))
}

fn payload_path(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|part| part.trim_matches('/'))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn insert_file(
    files: &mut Vec<PayloadFile>,
    relative_path: &str,
    source: &Path,
    reject_duplicate: bool,
) -> Result<(), PayloadError> {
    if ["py", "pyc", "pyo"]
        .iter()
        .any(|extension| has_extension(source, extension))
    {
        return Err(PayloadError::PythonPayload(source.to_owned()));
    }
    if let Some(existing) = files
        .iter_mut()
        .find(|file| file.relative_path == relative_path)
    {
        if reject_duplicate || existing.kind == PayloadKind::Executable {
            return Err(PayloadError::DuplicateSkillFile(relative_path.to_owned()));
        }
        existing.contents = fs::read(source).map_err(|source_error| PayloadError::Io {
            path: source.to_owned(),
            source: source_error,
        })?;
        return Ok(());
    }
    let contents = fs::read(source).map_err(|source_error| PayloadError::Io {
        path: source.to_owned(),
        source: source_error,
    })?;
    files.push(PayloadFile {
        relative_path: relative_path.to_owned(),
        contents,
        kind: PayloadKind::Data,
    });
    Ok(())
}

fn insert_native(
    context: &RepositoryContext,
    files: &mut Vec<PayloadFile>,
    binary: &str,
    relative_path: &str,
) -> Result<(), PayloadError> {
    let source = file_path(
        context.sources.as_path(),
        &format!("core/rust/target/release/{binary}"),
    )?;
    let metadata =
        fs::metadata(&source).map_err(|_| PayloadError::MissingNativeBinary(source.clone()))?;
    if !metadata.is_file() || metadata.permissions().mode() & 0o111 == 0 {
        return Err(PayloadError::MissingNativeBinary(source));
    }
    if files.iter().any(|file| file.relative_path == relative_path) {
        return Err(PayloadError::DuplicateSkillFile(relative_path.to_owned()));
    }
    let contents = fs::read(&source).map_err(|source_error| PayloadError::Io {
        path: source,
        source: source_error,
    })?;
    files.push(PayloadFile {
        relative_path: relative_path.to_owned(),
        contents,
        kind: PayloadKind::Executable,
    });
    Ok(())
}

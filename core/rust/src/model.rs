use std::{cmp::Ordering, path::PathBuf, str::FromStr};

use crate::error::ModelError;

/// A profile identifier in lowercase kebab-case.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct ProfileName(String);

impl ProfileName {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for ProfileName {
    type Err = ModelError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let valid = !value.is_empty()
            && value
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
            && value.split('-').all(|part| {
                !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_alphanumeric())
            });
        if !valid {
            return Err(ModelError::InvalidProfileName(value.to_owned()));
        }
        Ok(Self(value.to_owned()))
    }
}

/// A semantic version whose decimal segments are not limited to machine integers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Version {
    major: String,
    minor: String,
    patch: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VersionBump {
    Major,
    Minor,
    Patch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProfileAction {
    Help,
    Create,
    Rename,
    Install,
    Update,
    Switch,
    Uninstall,
    Status,
    Validate,
    MigrateLegacy,
    Bump,
}

/// Explicit location of source assets, independent of the caller's working directory.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RepositoryContext {
    pub(crate) sources: PathBuf,
}

impl RepositoryContext {
    pub(crate) fn from_sources(sources: PathBuf) -> Result<Self, crate::error::RepositoryError> {
        crate::repository::validate_source_root(&sources)?;
        let sources = std::fs::canonicalize(&sources).map_err(|source| {
            crate::error::RepositoryError::Io {
                path: sources,
                source,
            }
        })?;
        Ok(Self { sources })
    }

    pub(crate) fn discover() -> Result<Self, crate::error::RepositoryError> {
        let executable =
            std::env::current_exe().map_err(|source| crate::error::RepositoryError::Io {
                path: PathBuf::from("<current executable>"),
                source,
            })?;
        let start = executable
            .parent()
            .ok_or_else(|| crate::error::RepositoryError::CheckoutNotFound(executable.clone()))?;
        for candidate in start.ancestors() {
            if crate::repository::is_source_root(candidate) {
                return Self::from_sources(candidate.to_owned());
            }
        }
        Err(crate::error::RepositoryError::CheckoutNotFound(executable))
    }
}

impl Version {
    pub(crate) fn bump(&self, part: VersionBump) -> Self {
        let mut next = self.clone();
        match part {
            VersionBump::Major => {
                next.major = increment_decimal(&next.major);
                next.minor = "0".to_owned();
                next.patch = "0".to_owned();
            }
            VersionBump::Minor => {
                next.minor = increment_decimal(&next.minor);
                next.patch = "0".to_owned();
            }
            VersionBump::Patch => next.patch = increment_decimal(&next.patch),
        }
        next
    }

    pub(crate) fn as_string(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    fn segments(&self) -> [&str; 3] {
        [&self.major, &self.minor, &self.patch]
    }
}

impl FromStr for Version {
    type Err = ModelError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let mut segments = value.split('.');
        let parsed: Vec<&str> = segments.by_ref().collect();
        if parsed.len() != 3 || parsed.iter().any(|part| !valid_decimal(part)) {
            return Err(ModelError::InvalidVersion(value.to_owned()));
        }
        Ok(Self {
            major: parsed[0].to_owned(),
            minor: parsed[1].to_owned(),
            patch: parsed[2].to_owned(),
        })
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        self.segments()
            .into_iter()
            .zip(other.segments())
            .map(|(left, right)| compare_decimal(left, right))
            .find(|ordering| *ordering != Ordering::Equal)
            .unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn valid_decimal(part: &str) -> bool {
    !part.is_empty()
        && part.bytes().all(|byte| byte.is_ascii_digit())
        && (part == "0" || !part.starts_with('0'))
}

fn compare_decimal(left: &str, right: &str) -> Ordering {
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

fn increment_decimal(value: &str) -> String {
    let mut digits = value.as_bytes().to_vec();
    for digit in digits.iter_mut().rev() {
        if *digit < b'9' {
            *digit += 1;
            return String::from_utf8(digits).expect("ASCII decimal digits remain UTF-8");
        }
        *digit = b'0';
    }
    let mut result = String::with_capacity(value.len() + 1);
    result.push('1');
    result.push_str(std::str::from_utf8(&digits).expect("ASCII decimal digits remain UTF-8"));
    result
}

pub(crate) mod ddev;
pub(crate) mod dump;
pub(crate) mod localize;
pub(crate) mod mapping;
pub(crate) mod preflight;
pub mod restore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Origin {
    value: String,
    authority: String,
}

impl Origin {
    pub(crate) fn as_str(&self) -> &str {
        &self.value
    }

    pub(crate) fn authority(&self) -> &str {
        &self.authority
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UrlMapping {
    pub(crate) source: Origin,
    pub(crate) target: Origin,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct UrlMappings(Vec<UrlMapping>);

impl UrlMappings {
    pub(crate) fn entries(&self) -> &[UrlMapping] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TablePrefix(String);

impl TablePrefix {
    pub(crate) fn from_validated(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SiteMode {
    SingleSite,
    Multisite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DumpCompression {
    Plain,
    Gzip,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Sha256Digest(String);

impl Sha256Digest {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DumpInfo {
    pub(crate) prefix: TablePrefix,
    pub(crate) site_mode: SiteMode,
    pub(crate) compression: DumpCompression,
    pub(crate) sha256: Sha256Digest,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WordPressSite {
    pub(crate) id: String,
    pub(crate) domain: String,
    pub(crate) path: String,
    pub(crate) options_table: String,
    pub(crate) home: String,
    pub(crate) site_url: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WordPressNetwork {
    pub(crate) id: String,
    pub(crate) domain: String,
    pub(crate) path: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WordPressTopology {
    pub(crate) sites: Vec<WordPressSite>,
    pub(crate) networks: Vec<WordPressNetwork>,
    pub(crate) site_mode: SiteMode,
}

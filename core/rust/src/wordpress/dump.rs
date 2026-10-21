use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufRead, BufReader, Read},
    path::Path,
    sync::OnceLock,
};

use flate2::read::MultiGzDecoder;
use regex::bytes::{Regex, RegexBuilder};
use sha2::{Digest, Sha256};

use crate::{
    error::WordPressError,
    wordpress::{DumpCompression, DumpInfo, Sha256Digest, SiteMode, TablePrefix},
};

pub(crate) fn inspect_dump(path: &Path) -> Result<DumpInfo, WordPressError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(WordPressError::EmptyDump);
        }
        Err(source) => {
            return Err(WordPressError::Io {
                path: path.to_owned(),
                source,
            });
        }
    };
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(WordPressError::EmptyDump);
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if !extension.eq_ignore_ascii_case("sql") && !extension.eq_ignore_ascii_case("gz") {
        return Err(WordPressError::UnsupportedDumpType);
    }

    let mut source = File::open(path).map_err(|source| WordPressError::Io {
        path: path.to_owned(),
        source,
    })?;
    let mut magic = [0_u8; 2];
    let magic_size = source
        .read(&mut magic)
        .map_err(|source| WordPressError::Io {
            path: path.to_owned(),
            source,
        })?;
    let compressed = magic_size == magic.len() && magic == [0x1f, 0x8b];
    drop(source);

    let tables = if compressed {
        let file = File::open(path).map_err(|source| WordPressError::Io {
            path: path.to_owned(),
            source,
        })?;
        scan_sql(path, MultiGzDecoder::new(file))?
    } else {
        let file = File::open(path).map_err(|source| WordPressError::Io {
            path: path.to_owned(),
            source,
        })?;
        scan_sql(path, file)?
    };

    let prefixes = tables
        .iter()
        .filter_map(|table| {
            let prefix = table.strip_suffix("options")?;
            ["posts", "users", "usermeta"]
                .iter()
                .all(|suffix| tables.contains(&format!("{prefix}{suffix}")))
                .then(|| prefix.to_owned())
        })
        .collect::<Vec<_>>();
    if prefixes.len() != 1 {
        return Err(WordPressError::IncompleteDatabase);
    }
    let prefix = prefixes
        .into_iter()
        .next()
        .ok_or(WordPressError::IncompleteDatabase)?;
    let multisite = tables.contains(&format!("{prefix}blogs"));
    if multisite
        && (!tables.contains(&format!("{prefix}site"))
            || !tables.contains(&format!("{prefix}sitemeta")))
    {
        return Err(WordPressError::IncompleteMultisite);
    }

    Ok(DumpInfo {
        prefix: TablePrefix(prefix),
        site_mode: if multisite {
            SiteMode::Multisite
        } else {
            SiteMode::SingleSite
        },
        compression: if compressed {
            DumpCompression::Gzip
        } else {
            DumpCompression::Plain
        },
        sha256: file_hash(path)?,
    })
}

pub(crate) fn file_hash(path: &Path) -> Result<Sha256Digest, WordPressError> {
    let mut source = File::open(path).map_err(|source| WordPressError::Io {
        path: path.to_owned(),
        source,
    })?;
    let mut digest = Sha256::new();
    let mut chunk = [0_u8; 1024 * 1024];
    loop {
        let count = source
            .read(&mut chunk)
            .map_err(|source| WordPressError::Io {
                path: path.to_owned(),
                source,
            })?;
        if count == 0 {
            break;
        }
        digest.update(&chunk[..count]);
    }
    let digest = digest.finalize();
    let mut value = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut value, "{byte:02x}").expect("writing to String cannot fail");
    }
    Ok(Sha256Digest(value))
}

fn scan_sql(reader_path: &Path, reader: impl Read) -> Result<HashSet<String>, WordPressError> {
    static FORBIDDEN: OnceLock<Regex> = OnceLock::new();
    static CREATE_TABLE: OnceLock<Regex> = OnceLock::new();
    let forbidden = FORBIDDEN.get_or_init(|| {
        RegexBuilder::new(
            r"\s*(?:USE\s|(?:CREATE|DROP)\s+DATABASE\b|GRANT\s|CREATE\s+USER\b|SOURCE\s|\\[!])",
        )
        .case_insensitive(true)
        .build()
        .expect("fixed forbidden-SQL regex is valid")
    });
    let create_table = CREATE_TABLE.get_or_init(|| {
        RegexBuilder::new(r"\s*CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?`([A-Za-z0-9_]+)`")
            .case_insensitive(true)
            .build()
            .expect("fixed CREATE TABLE regex is valid")
    });

    let mut tables = HashSet::new();
    let mut source = BufReader::new(reader);
    let mut line = Vec::new();
    loop {
        line.clear();
        let count = source
            .read_until(b'\n', &mut line)
            .map_err(|source| WordPressError::Io {
                path: reader_path.to_owned(),
                source,
            })?;
        if count == 0 {
            break;
        }
        if forbidden.is_match(&line) {
            return Err(WordPressError::ForbiddenDumpDirective);
        }
        if let Some(capture) = create_table.captures(&line) {
            if let Some(name) = capture.get(1) {
                tables.insert(String::from_utf8_lossy(name.as_bytes()).into_owned());
            }
        }
    }
    Ok(tables)
}

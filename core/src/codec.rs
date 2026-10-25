use std::{fs, path::Path};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::CodecError;

pub(crate) fn sha(content: &[u8]) -> String {
    let digest = Sha256::digest(content);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut hex, "{byte:02x}").expect("writing to String cannot fail");
    }
    hex
}

/// Serialize JSON with the Python implementation's two-space indentation, UTF-8,
/// insertion order, and final line feed.
pub(crate) fn json_bytes<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>, CodecError> {
    let mut bytes = serde_json::to_vec_pretty(value)?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub(crate) fn read_json(path: &Path) -> Result<Value, CodecError> {
    let contents = fs::read(path).map_err(|source| CodecError::Read {
        path: path.to_owned(),
        source,
    })?;
    serde_json::from_slice(&contents).map_err(|source| CodecError::Parse {
        path: path.to_owned(),
        source,
    })
}

//! Invocation context for the manager only. Read OS values without changing the
//! arguments, environment or working directory subsequently used by the CLI.
use std::{borrow::Cow, env, ffi::OsStr, io, os::unix::ffi::OsStrExt, path::PathBuf};

struct Encoded<'a> {
    encoding: &'static str,
    value: Cow<'a, str>,
    bytes: usize,
}

impl<'a> Encoded<'a> {
    fn new(value: &'a OsStr) -> Self {
        let bytes = value.as_bytes();
        let (encoding, value) = match std::str::from_utf8(bytes) {
            Ok(text) => ("utf-8", Cow::Borrowed(text)),
            Err(_) => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let mut hex = String::with_capacity(bytes.len() * 2);
                for &byte in bytes {
                    hex.push(HEX[(byte >> 4) as usize] as char);
                    hex.push(HEX[(byte & 15) as usize] as char);
                }
                ("hex", Cow::Owned(hex))
            }
        };
        Self {
            encoding,
            value,
            bytes: bytes.len(),
        }
    }
}

pub(super) fn record() {
    let args = env::args_os();
    let argc = args.len();
    let environment: Vec<_> = env::vars_os().collect();
    let envc = environment.len();
    tracing::info!(
        event = "cli.context.started",
        schema_version = 1,
        argc,
        envc
    );
    for (index, arg) in args.enumerate() {
        let value = Encoded::new(&arg);
        tracing::info!(event = "cli.context.arg", index,
            value_encoding = value.encoding, value = %value.value, value_bytes = value.bytes);
    }
    for (index, (name, value)) in environment.iter().enumerate() {
        let name = Encoded::new(name);
        let value = Encoded::new(value);
        tracing::info!(event = "cli.context.env", index,
            name_encoding = name.encoding, name = %name.value, name_bytes = name.bytes,
            value_encoding = value.encoding, value = %value.value, value_bytes = value.bytes);
    }
    record_path("cli.context.cwd", env::current_dir());
    record_path("cli.context.exe", env::current_exe());
    tracing::info!(event = "cli.context.finished", argc, envc);
}

fn record_path(event: &'static str, result: io::Result<PathBuf>) {
    match result {
        Ok(path) => {
            let value = Encoded::new(path.as_os_str());
            tracing::info!(event, status = "available",
                value_encoding = value.encoding, value = %value.value, value_bytes = value.bytes);
        }
        Err(error) => {
            tracing::info!(event, status = "unavailable", error = %error,
                os_error = error.raw_os_error());
        }
    }
}

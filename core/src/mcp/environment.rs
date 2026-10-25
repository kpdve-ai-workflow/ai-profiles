use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

use regex::Regex;
use serde_json::{Map, Value};

use crate::{error::EnvironmentError, mcp::config::project_path};

pub(crate) fn dotenv(path: &Path) -> Result<HashMap<String, String>, EnvironmentError> {
    let contents = fs::read_to_string(path).map_err(|_| EnvironmentError::ReadEnvFile {
        filename: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
    })?;
    let contents = contents.strip_prefix('\u{feff}').unwrap_or(&contents);
    let mut values = HashMap::new();
    for (index, line) in split_lines(contents).into_iter().enumerate() {
        let line_number = index + 1;
        let mut line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(exported) = line.strip_prefix("export ") {
            line = exported.trim_start();
        }
        let Some(separator) = line.find('=') else {
            return Err(EnvironmentError::InvalidEntry { line: line_number });
        };
        let key = line[..separator].trim();
        let mut value = line[separator + 1..].trim().to_owned();
        if !valid_environment_name(key) {
            return Err(EnvironmentError::InvalidEntry { line: line_number });
        }
        if value.starts_with(['\'', '"']) {
            let words = parse_quoted_words(&value)
                .map_err(|()| EnvironmentError::InvalidQuoting { line: line_number })?;
            if words.len() != 1 {
                return Err(EnvironmentError::InvalidValue { line: line_number });
            }
            value = words.into_iter().next().unwrap_or_default();
        } else {
            value = if value.starts_with('#') {
                String::new()
            } else {
                value
                    .split_once(" #")
                    .map_or(value.as_str(), |(before_comment, _)| before_comment)
                    .trim_end()
                    .to_owned()
            };
        }
        values.insert(key.to_owned(), value);
    }
    Ok(values)
}

pub(crate) fn environment_for(
    project: &Path,
    entry: &Map<String, Value>,
    inherited: &HashMap<String, String>,
) -> Result<HashMap<String, String>, EnvironmentError> {
    let mut environment = inherited.clone();
    if let Some(env_file) = entry.get("env_file") {
        let Some(env_file) = env_file.as_str() else {
            return Err(EnvironmentError::InvalidMapping);
        };
        let path = project_path(project, env_file)?;
        environment.extend(dotenv(&path)?);
    }

    let mappings = match entry.get("environment") {
        None => None,
        Some(value) => Some(value.as_object().ok_or(EnvironmentError::InvalidMapping)?),
    };
    let mut mapped = HashMap::new();
    if let Some(mappings) = mappings {
        for (key, value) in mappings {
            let Some(value) = value.as_str() else {
                return Err(EnvironmentError::InvalidMapping);
            };
            mapped.insert(key.clone(), substitute_environment(value, &environment)?);
        }
    }
    environment.extend(mapped);
    Ok(environment)
}

fn substitute_environment(
    template: &str,
    environment: &HashMap<String, String>,
) -> Result<String, EnvironmentError> {
    static REFERENCE: OnceLock<Regex> = OnceLock::new();
    let pattern = REFERENCE.get_or_init(|| {
        Regex::new(r"\{env:([A-Za-z_][A-Za-z0-9_]*)\}")
            .expect("the fixed environment-reference regex is valid")
    });
    let mut output = String::with_capacity(template.len());
    let mut previous_end = 0;
    for capture in pattern.captures_iter(template) {
        let complete = capture.get(0).expect("capture 0 is always present");
        let name = capture
            .get(1)
            .expect("the expression has one capture")
            .as_str();
        output.push_str(&template[previous_end..complete.start()]);
        let Some(value) = environment.get(name).filter(|value| !value.is_empty()) else {
            return Err(EnvironmentError::MissingVariable(name.to_owned()));
        };
        output.push_str(value);
        previous_end = complete.end();
    }
    output.push_str(&template[previous_end..]);
    Ok(output)
}

fn valid_environment_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Matches Python `str.splitlines()` line boundaries, including CR-only files.
fn split_lines(contents: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut chars = contents.char_indices().peekable();
    while let Some((index, character)) = chars.next() {
        if !is_line_boundary(character) {
            continue;
        }
        lines.push(&contents[start..index]);
        let mut end = index + character.len_utf8();
        if character == '\r' && chars.peek().is_some_and(|(_, next)| *next == '\n') {
            let (next_index, next) = chars.next().expect("peeked line feed exists");
            end = next_index + next.len_utf8();
        }
        start = end;
    }
    if start < contents.len() {
        lines.push(&contents[start..]);
    }
    lines
}

fn is_line_boundary(character: char) -> bool {
    matches!(
        character,
        '\n' | '\r'
            | '\u{000b}'
            | '\u{000c}'
            | '\u{001c}'
            | '\u{001d}'
            | '\u{001e}'
            | '\u{0085}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

/// Tokenize the quoted value like the limited POSIX `shlex.split(..., comments=True)`
/// use in Python, without invoking a shell or expanding variables.
fn parse_quoted_words(value: &str) -> Result<Vec<String>, ()> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut quote = None;
    let mut chars = value.chars().peekable();

    while let Some(character) = chars.next() {
        if let Some(quote_character) = quote {
            if character == quote_character {
                quote = None;
                started = true;
            } else if character == '\\' && quote_character == '"' {
                match chars.peek().copied() {
                    Some('"' | '\\') => {
                        word.push(chars.next().ok_or(())?);
                    }
                    _ => word.push('\\'),
                }
                started = true;
            } else {
                word.push(character);
                started = true;
            }
            continue;
        }

        match character {
            '\'' | '"' => {
                quote = Some(character);
                started = true;
            }
            '#' => break,
            '\\' => {
                word.push(chars.next().ok_or(())?);
                started = true;
            }
            character if character.is_whitespace() => {
                if started {
                    words.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            _ => {
                word.push(character);
                started = true;
            }
        }
    }
    if quote.is_some() {
        return Err(());
    }
    if started {
        words.push(word);
    }
    Ok(words)
}

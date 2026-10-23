//! Novamira-only stdio guard for the pinned WordPress bridge's degraded handshake.
//! Never diagnose from tools count or forward upstream error text/logs.
use std::{
    io::{self, BufRead, BufReader, Write},
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use serde_json::{json, Value};
use std::os::unix::process::CommandExt;

const DIAGNOSTIC_ID: &str = "ai-profiles/novamira/init-diagnostic";
const GENERAL: &str = "Novamira: nie udało się połączyć z WordPressem. Sprawdź adres witryny i konfigurację połączenia.";
const MAX_MESSAGE: usize = 16 * 1024 * 1024;

fn authentication_message(env_file: &str) -> String {
    format!("Novamira: WordPress odrzucił dane aplikacyjne (HTTP 401). Sprawdź WP_API_USERNAME i WP_API_PASSWORD w {env_file}. Użyj hasła aplikacyjnego WordPressa, nie hasła do panelu.")
}

fn connection_message(env_file: &str) -> String {
    format!("{GENERAL} Konfiguracja: {env_file}.")
}

fn failure(id: &Value, message: &str) {
    let response = json!({"jsonrpc":"2.0","id":id,"error":{"code":-32603,"message":message}});
    let _ = writeln!(io::stdout().lock(), "{response}");
}

pub(super) fn configuration_failure(env_file: &str) {
    let message = format!("Novamira: uzupełnij WP_API_URL, WP_API_USERNAME i WP_API_PASSWORD w {env_file}. Użyj hasła aplikacyjnego WordPressa, nie hasła do panelu. Sprawdź plik i konfigurację połączenia.");
    initial_failure(&message);
}

fn initial_failure(message: &str) {
    if let Ok(Some(line)) = read_message(&mut io::stdin().lock()) {
        if let Ok(request) = serde_json::from_slice::<Value>(&line) {
            if request["method"] == "initialize" {
                failure(&request["id"], message);
            }
        }
    }
}

fn read_message(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            return Ok((!line.is_empty()).then_some(line));
        }
        let end = available.iter().position(|b| *b == b'\n').map(|i| i + 1);
        let count = end.unwrap_or(available.len());
        if line.len() + count > MAX_MESSAGE {
            return Err(io::Error::other("MCP message too large"));
        }
        line.extend_from_slice(&available[..count]);
        reader.consume(count);
        if end.is_some() {
            return Ok(Some(line));
        }
    }
}

enum Event {
    Message(Vec<u8>),
    Closed,
}

struct Bridge(Child);
impl Drop for Bridge {
    fn drop(&mut self) {
        // npx can own a shell and a Node grandchild. Terminate only this guard's
        // private process group, including on EOF/error, and reap the direct child.
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", self.0.id())])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) fn run(process: &mut Command, env_file: &str) -> bool {
    let general = connection_message(env_file);
    let Ok(Some(first)) = read_message(&mut io::stdin().lock()) else {
        return false;
    };
    let Ok(request) = serde_json::from_slice::<Value>(&first) else {
        return false;
    };
    if request["method"] != "initialize" || request.get("id").is_none() {
        return false;
    }
    let id = &request["id"];
    // Upstream logs raw HTTP bodies even at ERROR. Do not capture or forward them,
    // and disable its optional file sink regardless of inherited debug settings.
    process
        .env_remove("LOG_FILE")
        .env("LOG_LEVEL", "-1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0);
    let Ok(child) = process.spawn() else {
        failure(id, &general);
        return false;
    };
    let mut bridge = Bridge(child);
    let input = Arc::new(Mutex::new(bridge.0.stdin.take().expect("piped stdin")));
    if input.lock().expect("stdin lock").write_all(&first).is_err() {
        failure(id, &general);
        return false;
    }
    let stdout = bridge.0.stdout.take().expect("piped stdout");
    let (tx, rx) = mpsc::sync_channel(8);
    let signal_events = tx.clone();
    if ctrlc::set_handler(move || {
        let _ = signal_events.try_send(Event::Closed);
    })
    .is_err()
    {
        failure(id, &general);
        return false;
    }
    let output_events = tx.clone();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        while let Ok(Some(line)) = read_message(&mut reader) {
            if output_events.send(Event::Message(line)).is_err() {
                return;
            }
        }
        let _ = output_events.send(Event::Closed);
    });
    let child_input = input.clone();
    thread::spawn(move || {
        let mut reader = io::stdin().lock();
        while let Ok(Some(line)) = read_message(&mut reader) {
            if child_input
                .lock()
                .expect("stdin lock")
                .write_all(&line)
                .is_err()
            {
                break;
            }
        }
        let _ = tx.send(Event::Closed);
    });
    let mut initialized = false;
    while let Ok(event) = rx.recv() {
        let Event::Message(line) = event else {
            if !initialized {
                failure(id, &general);
            }
            return initialized;
        };
        let Ok(response) = serde_json::from_slice::<Value>(&line) else {
            if !initialized {
                failure(id, &general);
            }
            return false;
        };
        if !initialized && response.get("method").is_none() && response.get("id") == Some(id) {
            if response.get("error").is_some() {
                failure(id, &general);
                return false;
            }
            if response
                .pointer("/result/capabilities/experimental")
                .and_then(Value::as_object)
                .is_some_and(|e| e.contains_key("connectionFailed"))
            {
                // Upstream 0.4.0 holds the real cause in session state. tools/list
                // after failed init reads it locally; it does NOT retry HTTP.
                // The marker alone (often {}) proves no particular HTTP status.
                let diagnostic = json!({"jsonrpc":"2.0","id":DIAGNOSTIC_ID,"method":"tools/list"});
                let _ = writeln!(input.lock().expect("stdin lock"), "{diagnostic}");
                let deadline = Instant::now() + Duration::from_secs(2);
                let mut auth = false;
                while let Ok(Event::Message(line)) =
                    rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))
                {
                    let Ok(detail) = serde_json::from_slice::<Value>(&line) else {
                        break;
                    };
                    if detail["id"] == DIAGNOSTIC_ID {
                        // Fixed prefix generated by wpRequest for a non-OK HTTP
                        // response, not a substring in an untrusted server body.
                        auth = detail.pointer("/error/data/reason").and_then(Value::as_str)
                            == Some("failed")
                            && detail
                                .pointer("/error/data/detail")
                                .and_then(Value::as_str)
                                .is_some_and(|text| {
                                    text.starts_with("WordPress API error (401): ")
                                });
                        break;
                    }
                }
                let message = if auth {
                    authentication_message(env_file)
                } else {
                    general.clone()
                };
                failure(id, &message);
                return false;
            }
            initialized = true;
        }
        let mut output = io::stdout().lock();
        if output
            .write_all(&line)
            .and_then(|()| output.flush())
            .is_err()
        {
            return false;
        }
    }
    initialized
}

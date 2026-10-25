//! Oracles: JSON-RPC initialize must fail on upstream connectionFailed presence;
//! only a confirmed HTTP 401 may produce application-password guidance.
//! Synthetic bridge covers wire edge cases; pinned real bridge + HTTP exercises
//! the lost-HTTP-details seam without mocking process or network execution.
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    path::Path,
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, OnceLock,
    },
    thread,
    time::Duration,
};

struct Adapter {
    child: Child,
    replies: mpsc::Receiver<String>,
    stderr: Option<thread::JoinHandle<String>>,
    _project: tempfile::TempDir,
}

#[derive(Clone, Copy)]
enum Fixture {
    Synthetic,
    Docker,
}

impl Adapter {
    fn start(command: Value, env_contents: &str, extra: Value, server: &str) -> Self {
        Self::start_with(command, env_contents, extra, server, Fixture::Synthetic)
    }

    fn start_with(
        command: Value,
        env_contents: &str,
        extra: Value,
        server: &str,
        fixture: Fixture,
    ) -> Self {
        // cargo test --all-targets does not itself build an executable release
        // payload. Build once so `make test` works on a fresh VM and never tests
        // a stale adapter left by an earlier verification run.
        static BUILD: OnceLock<()> = OnceLock::new();
        BUILD.get_or_init(|| {
            let output = Command::new(env!("CARGO"))
                .args([
                    "build",
                    "--manifest-path",
                    concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
                    "--locked",
                    "--release",
                    "--bin",
                    "ai-profile-mcp",
                ])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "adapter build failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        });
        let project = tempfile::tempdir().unwrap();
        if matches!(fixture, Fixture::Docker) {
            let build = Command::new(env!("CARGO"))
                .args([
                    "build",
                    "--manifest-path",
                    concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"),
                    "--locked",
                    "--release",
                    "--bin",
                    "ai-profile",
                ])
                .output()
                .unwrap();
            assert!(
                build.status.success(),
                "{}",
                String::from_utf8_lossy(&build.stderr)
            );
            let installer = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/ai-profile");
            let installed = Command::new(installer)
                .args(["install", "wordpress", "--client", "opencode", "--project"])
                .arg(project.path())
                .output()
                .unwrap();
            assert!(
                installed.status.success(),
                "{}",
                String::from_utf8_lossy(&installed.stderr)
            );
        }
        let root = project.path().join(".ai-profiles/opencode");
        fs::create_dir_all(&root).unwrap();
        let executable = root.join("ai-profile-mcp");
        let release = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/release/ai-profile-mcp");
        fs::copy(release, &executable).expect("build release binaries in VM before process tests");
        let env_path = project.path().join(".env.novamira");
        fs::create_dir_all(env_path.parent().unwrap()).unwrap();
        fs::write(env_path, env_contents).unwrap();
        let mut entry = json!({"type":"local","command":command,"env_file":".env.novamira"});
        entry
            .as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        fs::write(
            project.path().join(".ai-profiles/state.json"),
            json!({"format":2,"clients":{"opencode":{"mcp_base":{server:entry}}}}).to_string(),
        )
        .unwrap();
        let mut child = Command::new(executable)
            .arg(server)
            .current_dir("/")
            .env_remove("LOG_FILE")
            .env_remove("JWT_TOKEN")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, replies) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                if tx.send(line.unwrap()).is_err() {
                    break;
                }
            }
        });
        let mut err = child.stderr.take().unwrap();
        let stderr = Some(thread::spawn(move || {
            let mut text = String::new();
            err.read_to_string(&mut text).unwrap();
            text
        }));
        Self {
            child,
            replies,
            stderr,
            _project: project,
        }
    }
    fn send(&mut self, request: Value) {
        writeln!(self.child.stdin.as_mut().unwrap(), "{request}").unwrap();
    }
    fn receive(&self) -> Value {
        let line = self
            .replies
            .recv_timeout(Duration::from_secs(90))
            .expect("bounded MCP reply");
        serde_json::from_str(&line).expect("stdout contains only JSON-RPC")
    }
    fn initialize(&mut self) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":"client-init","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}));
        self.receive()
    }
    fn stop(&mut self) -> String {
        self.child.stdin.take();
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while self.child.try_wait().unwrap().is_none() {
            if std::time::Instant::now() >= deadline {
                let _ = self.child.kill();
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        self.child.wait().unwrap();
        self.stderr.take().unwrap().join().unwrap()
    }
}
impl Drop for Adapter {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn assert_failure(reply: &Value, auth: bool) {
    assert_eq!(reply["id"], "client-init");
    assert!(
        reply.get("result").is_none(),
        "failed initialize cannot be success: {reply}"
    );
    let message = reply["error"]["message"]
        .as_str()
        .expect("MCP error message");
    assert!(message.contains("Novamira:"));
    assert!(message.contains(".env.novamira"));
    if auth {
        assert!(
            message.contains("HTTP 401"),
            "unexpected safe diagnostic: {message}"
        );
        assert!(message.contains(".env.novamira"));
        assert!(message.contains("WP_API_USERNAME") && message.contains("WP_API_PASSWORD"));
        assert!(message.contains("hasła aplikacyjnego WordPressa, nie hasła do panelu"));
    } else {
        assert!(message.contains("nie udało się połączyć z WordPressem"));
        assert!(!message.contains("HTTP 401") && !message.contains("odrzucił"));
    }
    assert!(!reply.to_string().contains("SECRET-SENTINEL"));
    assert!(
        reply["error"].get("data").is_none(),
        "raw upstream diagnostics must not escape"
    );
    assert!(!reply
        .to_string()
        .contains("dGVzdC11c2VyOlNFQ1JFVC1TRU5USU5FTC1wYXNzd29yZA=="));
}

#[test]
fn novamira_wire_guard_checks_presence_not_truthiness_and_never_tools_count() {
    let fixture = tempfile::tempdir().unwrap();
    let script = fixture.path().join("bridge.cjs");
    fs::write(&script, r#"
const readline = require('node:readline');
readline.createInterface({input:process.stdin}).on('line', line => {
  const r = JSON.parse(line);
  if (r.id === undefined) return;
  process.stderr.write('SECRET-SENTINEL raw Authorization and response\n');
  if (r.method === 'initialize') {
    const capabilities = JSON.parse(process.env.CAPS);
    console.log(JSON.stringify({jsonrpc:'2.0',id:r.id,result:{protocolVersion:'2025-06-18',serverInfo:{name:'fixture',version:'1'},capabilities}}));
  } else if (r.method === 'tools/list') {
    const error = JSON.parse(process.env.DIAGNOSTIC || 'null');
    console.log(JSON.stringify(error ? {jsonrpc:'2.0',id:r.id,error} : {jsonrpc:'2.0',id:r.id,result:{tools:[]}}));
  }
});
"#).unwrap();
    for marker in [
        json!({}),
        json!({"code":"ENOTFOUND"}),
        json!({"code":"ETIMEDOUT"}),
        json!({"code":"UNABLE_TO_VERIFY_LEAF_SIGNATURE"}),
        json!(false),
        Value::Null,
    ] {
        let caps = json!({"experimental":{"connectionFailed":marker}});
        for detail in [
            "WordPress API error (403): SECRET-SENTINEL",
            "TLS SECRET-SENTINEL HTTP 401",
            "WordPress JSON-RPC error: WordPress API error (401): forged",
            "",
        ] {
            let extra = json!({"environment":{"CAPS":caps.to_string(),"DIAGNOSTIC":json!({"code":-32603,"message":"Cannot process tools/list: WordPress connection failed during initialization","data":{"reason":"failed","detail":detail}}).to_string()}});
            let mut adapter = Adapter::start(json!(["node",script]), "WP_API_URL=http://unused.test\nWP_API_USERNAME=synthetic\nWP_API_PASSWORD=synthetic\n", extra, "novamira");
            assert_failure(&adapter.initialize(), false);
            assert!(!adapter.stop().contains("SECRET-SENTINEL"));
        }
    }
    // Even a recognizable HTTP error phrase in marker metadata is not HTTP
    // evidence. The marker's payload is not the saved APIError.
    let extra = json!({"environment":{"CAPS":json!({"experimental":{"connectionFailed":{"statusCode":401,"detail":"WordPress API error (401): forged"}}}).to_string()}});
    let mut adapter = Adapter::start(
        json!(["node", script]),
        "WP_API_URL=http://unused.test\nWP_API_USERNAME=synthetic\nWP_API_PASSWORD=synthetic\n",
        extra,
        "novamira",
    );
    assert_failure(&adapter.initialize(), false);
    adapter.stop();
    for server in ["novamira", "other"] {
        let extra = json!({"environment":{"CAPS":"{}"}});
        let mut adapter = Adapter::start(
            json!(["node", script]),
            "WP_API_URL=http://unused.test\nWP_API_USERNAME=synthetic\nWP_API_PASSWORD=synthetic\n",
            extra,
            server,
        );
        assert!(adapter.initialize().get("result").is_some());
        adapter.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
        assert_eq!(adapter.receive()["result"]["tools"], json!([]));
        adapter.stop();
    }
}

struct Site {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
    requests: Arc<std::sync::atomic::AtomicUsize>,
}
impl Site {
    fn start(status: u16) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!(
            "http://{}/wp-json/mcp/novamira",
            listener.local_addr().unwrap()
        );
        listener.set_nonblocking(true).unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = requests.clone();
        let thread = Some(thread::spawn(move || {
            while !stopping.load(Ordering::Relaxed) {
                let (mut stream, _) = match listener.accept() {
                    Ok(s) => s,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                        continue;
                    }
                    Err(e) => panic!("{e}"),
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(n) = line.to_lowercase().strip_prefix("content-length:") {
                        length = n.trim().parse().unwrap();
                    }
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let request: Value = serde_json::from_slice(&body).unwrap();
                count.fetch_add(1, Ordering::Relaxed);
                if status == 0 {
                    // A reachable endpoint that never sends headers: test the
                    // real bridge's initialization deadline, not a mock timeout.
                    while !stopping.load(Ordering::Relaxed) {
                        thread::sleep(Duration::from_millis(5));
                    }
                    return;
                }
                let result = if request["method"] == "initialize" {
                    json!({"protocolVersion":"2025-06-18","serverInfo":{"name":"controlled-wordpress","version":"1"},"capabilities":{"tools":{}}})
                } else {
                    json!({"tools":[{"name":"read_site","description":"controlled real HTTP tool","inputSchema":{"type":"object"}}]})
                };
                let body = if status == 200 { json!({"jsonrpc":"2.0","id":request["id"],"result":result}) } else { json!({"code":if status == 401 {"incorrect_password"} else {"forbidden"},"message":"SECRET-SENTINEL raw server response","data":{"status":status}}) }.to_string();
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        }));
        Self {
            url,
            stop,
            thread,
            requests,
        }
    }
}
impl Drop for Site {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

#[test]
fn novamira_real_pinned_bridge_reports_http_auth_distinctly_and_forwards_tools() {
    check_real_bridge(Fixture::Synthetic);
}

#[test]
fn novamira_distributed_docker_bridge_auth_timeout_tools_and_container_cleanup() {
    let build = std::process::Command::new("docker")
        .args(["build", "-t", "ai-profiles-novamira:0.1.0"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mcp/images/novamira"))
        .output()
        .unwrap();
    assert!(
        build.status.success(),
        "Docker build failed: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    check_real_bridge(Fixture::Docker);
    let containers = std::process::Command::new("docker")
        .args([
            "ps",
            "--filter",
            "ancestor=ai-profiles-novamira:0.1.0",
            "--format",
            "{{.ID}}",
        ])
        .output()
        .unwrap();
    assert!(containers.status.success());
    assert!(
        containers.stdout.is_empty(),
        "adapter shutdown left a live bridge container"
    );
}

fn check_real_bridge(fixture: Fixture) {
    let logs = tempfile::tempdir().unwrap();
    let log_file = logs.path().join("bridge.log");
    for status in [401, 403, 500, 0, 200] {
        let site = Site::start(status);
        let env = format!(
            "WP_API_URL={}\nWP_API_USERNAME=test-user\nWP_API_PASSWORD=SECRET-SENTINEL-password\n",
            site.url
        );
        let command = match fixture {
            Fixture::Synthetic => json!(["npx", "-y", "@automattic/mcp-wordpress-remote@0.4.0"]),
            Fixture::Docker => json!([".ai-profiles/opencode/mcp/novamira/launcher"]),
        };
        let mut adapter = Adapter::start_with(
            command,
            &env,
            json!({"environment":{"OAUTH_ENABLED":"false","USE_SYSTEM_PROXY":"false","WP_API_INIT_TIMEOUT_MS":"1500","LOG_TO_STDERR":"true","LOG_FILE":log_file}}),
            "novamira",
            fixture,
        );
        let reply = adapter.initialize();
        if status == 200 {
            assert_eq!(
                reply["result"]["serverInfo"]["name"],
                "controlled-wordpress"
            );
            adapter.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
            adapter.send(json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}));
            assert_eq!(adapter.receive()["result"]["tools"][0]["name"], "read_site");
        } else {
            assert_failure(&reply, status == 401);
            assert_eq!(
                site.requests.load(Ordering::Relaxed),
                if status == 0 { 1 } else { 2 },
                "diagnostic tools/list must not retry HTTP"
            );
        }
        assert!(!adapter.stop().contains("SECRET-SENTINEL"));
        assert!(
            !log_file.exists(),
            "upstream raw file logging must be disabled"
        );
    }
}

#[test]
fn novamira_empty_configuration_fails_before_spawning_bridge() {
    let mut adapter = Adapter::start(
        json!(["sh", "-c", "echo SECRET-SENTINEL >&2; exit 9"]),
        "WP_API_URL=\nWP_API_USERNAME=\nWP_API_PASSWORD=\"\"\n",
        json!({"environment":{"WP_API_URL":"{env:WP_API_URL}","WP_API_USERNAME":"{env:WP_API_USERNAME}","WP_API_PASSWORD":"{env:WP_API_PASSWORD}"}}),
        "novamira",
    );
    let reply = adapter.initialize();
    assert!(reply["error"]["message"]
        .as_str()
        .unwrap()
        .contains(".env.novamira"));
    assert!(!adapter.stop().contains("SECRET-SENTINEL"));
}

#[test]
fn novamira_missing_project_root_file_reports_its_name_without_spawning_bridge() {
    let mut adapter = Adapter::start(
        json!(["sh", "-c", "echo SECRET-SENTINEL >&2; exit 9"]),
        "WP_API_URL=https://wordpress.example.test/wp-json/mcp/novamira\nWP_API_USERNAME=fixture\nWP_API_PASSWORD=fixture-password\n",
        json!({"environment":{"WP_API_URL":"{env:WP_API_URL}","WP_API_USERNAME":"{env:WP_API_USERNAME}","WP_API_PASSWORD":"{env:WP_API_PASSWORD}"}}),
        "novamira",
    );
    fs::remove_file(adapter._project.path().join(".env.novamira")).unwrap();
    let reply = adapter.initialize();
    let message = reply["error"]["message"].as_str().unwrap();
    assert!(message.contains(".env.novamira"));
    assert!(message.contains("utwórz w katalogu głównym projektu"));
    assert!(!adapter.stop().contains("SECRET-SENTINEL"));
}

use std::{
    collections::HashMap,
    fs,
    os::unix::fs::PermissionsExt,
    os::unix::process::ExitStatusExt,
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
    thread,
    time::{Duration, Instant},
};

use serde_json::Value;

use crate::{
    logging::{with_session, Config, Session},
    process::{
        CancellationToken, CommandSpec, ProcessRunner, StdinMode, StreamMode, SystemProcessRunner,
    },
};

fn spec(program: &str, args: &[&str]) -> CommandSpec {
    CommandSpec {
        program: program.into(),
        args: args.iter().map(Into::into).collect(),
        cwd: None,
        env: HashMap::new(),
        clear_env: false,
        stdin: StdinMode::Null,
        stdout: StreamMode::Capture,
        stderr: StreamMode::Capture,
        timeout: Some(Duration::from_secs(5)),
        cancellation: CancellationToken::new(),
    }
}

fn logged<T>(root: &Path, invoke: impl FnOnce() -> T) -> (Session, T) {
    let session = open_session(root);
    let result = with_session(&session, invoke);
    session.flush();
    (session, result)
}

fn payload_bytes(root: &Path, payload_id: &str) -> Vec<u8> {
    let (run_id, sequence) = payload_id.rsplit_once('-').unwrap();
    let run = root.join(format!("run-{run_id}"));
    let sequence = sequence.parse::<u64>().unwrap();
    let file = run.join(format!("payload-{sequence:020}-{:020}.bin", 0));
    fs::read(&file).unwrap_or_else(|error| {
        panic!(
            "payload {payload_id} missing at {}: {error}",
            file.display()
        )
    })
}

fn payloads_for(root: &Path, values: &[Value], stream: &str, scope: &str) -> Vec<u8> {
    let mut chunks = values
        .iter()
        .filter(|event| event_name(event) == "process.data.chunk")
        .filter(|event| event["fields"]["stream"] == stream && event["fields"]["scope"] == scope)
        .collect::<Vec<_>>();
    chunks.sort_by_key(|event| event["fields"]["index"].as_u64().unwrap());
    let mut bytes = Vec::new();
    for chunk in chunks {
        bytes.extend(payload_bytes(
            root,
            chunk["fields"]["payload_id"].as_str().unwrap(),
        ));
    }
    bytes
}

fn assert_chunk_offsets(values: &[Value], stream: &str, scope: &str) {
    let mut chunks = values
        .iter()
        .filter(|event| event_name(event) == "process.data.chunk")
        .filter(|event| event["fields"]["stream"] == stream && event["fields"]["scope"] == scope)
        .collect::<Vec<_>>();
    chunks.sort_by_key(|event| event["fields"]["index"].as_u64().unwrap());
    let mut offset = 0u64;
    for (index, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk["fields"]["index"], index);
        assert_eq!(chunk["fields"]["offset"], offset);
        let observed = chunk["fields"]["observed_length"].as_u64().unwrap();
        assert!(observed <= 64 * 1024);
        offset += observed;
    }
}

#[test]
fn process_context_preserves_argument_boundaries_and_observed_output() {
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    let root = tempfile::tempdir().unwrap();
    let mut command = spec("/bin/sh", &[]);
    command.program = "/usr/bin/python3".into();
    command.args = vec![
        "-c".into(),
        "import os,sys,threading; t=threading.Thread(target=lambda: os.write(1,b'out\\x00\\xff')); t.start(); os.write(2,b'err'); t.join(); sys.exit(17)".into(),
        "".into(),
        "two words".into(),
        "żółć".into(),
        std::ffi::OsString::from_vec(b"arg-\xff".to_vec()),
    ];
    let args = command.args.clone();
    let (session, result) = logged(root.path(), || SystemProcessRunner.run(command));
    let output = result.unwrap();
    assert_eq!(output.status.code(), Some(17));
    assert_eq!(output.stdout, b"out\0\xff");
    assert_eq!(output.stderr, b"err");

    let (_, values) = events(root.path());
    let context = values
        .iter()
        .filter(|event| event_name(event).starts_with("process.context."))
        .collect::<Vec<_>>();
    assert!(context
        .iter()
        .all(|event| event["fields"]["schema_version"] == 1));
    assert_eq!(
        context
            .iter()
            .filter(|event| event_name(event) == "process.context.arg")
            .count(),
        args.len()
    );
    assert!(context
        .iter()
        .any(|event| event_name(event) == "process.context.started"));
    assert!(context
        .iter()
        .any(|event| event_name(event) == "process.context.finished"));
    let arg_events = context
        .iter()
        .filter(|event| event_name(event) == "process.context.arg")
        .collect::<Vec<_>>();
    assert_eq!(arg_events.len(), args.len());
    for (index, (event, arg)) in arg_events.iter().zip(args.iter()).enumerate() {
        assert_eq!(event["fields"]["index"], index);
        let bytes = arg.as_os_str().as_bytes();
        assert_eq!(event["fields"]["bytes"], bytes.len());
        let expected_encoding = if std::str::from_utf8(bytes).is_ok() {
            "utf8"
        } else {
            "hex"
        };
        assert_eq!(event["fields"]["encoding"], expected_encoding);
        let expected_value = if expected_encoding == "utf8" {
            std::str::from_utf8(bytes).unwrap().to_owned()
        } else {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .concat()
        };
        assert_eq!(event["fields"]["value"], expected_value);
    }

    let chunks = values
        .iter()
        .filter(|event| event_name(event) == "process.data.chunk")
        .collect::<Vec<_>>();
    assert_eq!(chunks.len(), 2);
    for (stream, expected) in [
        ("stdout", b"out\0\xff".as_slice()),
        ("stderr", b"err".as_slice()),
    ] {
        let chunk = chunks
            .iter()
            .find(|event| event["fields"]["stream"] == stream)
            .unwrap();
        assert_eq!(chunk["fields"]["observed_length"], expected.len());
        assert_eq!(
            payload_bytes(root.path(), chunk["fields"]["payload_id"].as_str().unwrap()),
            expected
        );
    }
    drop(session);
}

#[test]
fn stdin_and_large_binary_stdout_are_reconstructed_without_loss() {
    let root = tempfile::tempdir().unwrap();
    let input = (0u8..=255).cycle().take(170_123).collect::<Vec<_>>();
    let expected = (0u8..=255)
        .rev()
        .cycle()
        .take(1_200_321)
        .collect::<Vec<_>>();
    let mut command = spec("/usr/bin/python3", &[]);
    command.args = vec!["-c".into(), "import os,sys; d=sys.stdin.buffer.read(); os.write(1, bytes((255-i)%256 for i in range(1200321))); os.write(2, d[:19])".into()];
    command.stdin = StdinMode::Bytes(input.clone());
    let (session, result) = logged(root.path(), || SystemProcessRunner.run(command));
    let output = result.unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, input[..19]);
    assert_eq!(output.stdout, expected);
    let (_, values) = events(root.path());
    assert_eq!(
        payloads_for(root.path(), &values, "stdin", "requested_input"),
        input
    );
    assert_eq!(
        payloads_for(root.path(), &values, "stdin", "written_to_pipe"),
        input
    );
    assert_eq!(
        payloads_for(root.path(), &values, "stdout", "read_from_pipe"),
        expected
    );
    assert_eq!(
        payloads_for(root.path(), &values, "stderr", "read_from_pipe"),
        &input[..19]
    );
    assert_chunk_offsets(&values, "stdin", "requested_input");
    assert_chunk_offsets(&values, "stdin", "written_to_pipe");
    assert_chunk_offsets(&values, "stdout", "read_from_pipe");
    assert_chunk_offsets(&values, "stderr", "read_from_pipe");
    assert!(values
        .iter()
        .filter(|event| event_name(event) == "process.data.chunk")
        .all(|event| event["fields"]["payload_complete"] == true
            && event["fields"]["stored_length"] == event["fields"]["observed_length"]));
    assert!(
        values
            .iter()
            .filter(|event| event_name(event) == "process.stream.snapshot")
            .filter(|event| event["fields"]["boundary"] == "runner_return")
            .count()
            >= 3
    );
    drop(session);
}

#[test]
fn capture_limited_reports_cap_without_claiming_total_length() {
    for stream in ["stdout", "stderr"] {
        for (limit, length, expected_state) in [
            (4, 3, "complete"),
            (4, 5, "limited"),
            (4, 9, "limited"),
            (0, 1, "limited"),
        ] {
            let root = tempfile::tempdir().unwrap();
            let mut command = spec("/usr/bin/python3", &[]);
            let fd = if stream == "stdout" { 1 } else { 2 };
            command.args = vec![
                "-c".into(),
                format!("import os; os.write({fd}, bytes(range({length})))").into(),
            ];
            match stream {
                "stdout" => command.stdout = StreamMode::CaptureLimited(limit),
                "stderr" => command.stderr = StreamMode::CaptureLimited(limit),
                _ => unreachable!(),
            }
            let (session, result) = logged(root.path(), || SystemProcessRunner.run(command));
            let output = result.unwrap();
            let captured = if stream == "stdout" {
                output.stdout.len()
            } else {
                output.stderr.len()
            };
            assert_eq!(captured, length.min(limit.saturating_add(1)));
            let (_, values) = events(root.path());
            let closed = values
                .iter()
                .find(|event| {
                    event_name(event) == "process.stream.closed"
                        && event["fields"]["stream"] == stream
                })
                .unwrap();
            assert_eq!(closed["fields"]["state"], expected_state);
            if expected_state == "limited" {
                assert_ne!(closed["fields"]["reason"], "truncated");
                assert!(closed["fields"]["total_length"].is_null());
            }
            drop(session);
        }
    }
}

#[test]
fn environment_snapshot_matches_clear_env_and_child_values() {
    for clear_env in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let mut command = spec("/usr/bin/python3", &[]);
        command.clear_env = clear_env;
        command.env.insert("PATH".into(), "/synthetic/path".into());
        command
            .env
            .insert("PROCESS_DATA_KEY".into(), "value with spaces".into());
        command.args = vec!["-c".into(), "import os; os.write(1, (os.environ.get('PATH','')+'|'+os.environ.get('PROCESS_DATA_KEY','')+'|'+str('HOME' in os.environ)).encode())".into()];
        let (session, result) = logged(root.path(), || SystemProcessRunner.run(command));
        let output = result.unwrap();
        assert_eq!(output.status.code(), Some(0));
        let text = std::str::from_utf8(&output.stdout).unwrap();
        assert!(text.starts_with("/synthetic/path|value with spaces|"));
        assert_eq!(text.ends_with("True"), !clear_env);
        let (_, values) = events(root.path());
        let started = values
            .iter()
            .find(|event| event_name(event) == "process.context.started")
            .unwrap();
        let logged_env = values
            .iter()
            .filter(|event| event_name(event) == "process.context.env")
            .collect::<Vec<_>>();
        assert_eq!(started["fields"]["environment_count"], logged_env.len());
        assert!(logged_env
            .iter()
            .any(|event| event["fields"]["key"] == "PROCESS_DATA_KEY"));
        assert_eq!(
            logged_env
                .iter()
                .any(|event| event["fields"]["key"] == "HOME"),
            !clear_env
        );
        drop(session);
    }
}

fn open_session(root: &Path) -> Session {
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    Session::open(root, Config::default(), "process-logging-test").unwrap()
}

fn open_session_with_config(root: &Path, config: Config) -> Session {
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).unwrap();
    Session::open(root, config, "process-logging-test").unwrap()
}

fn events(root: &Path) -> (String, Vec<Value>) {
    let run = fs::read_dir(root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir() && path.join("format").exists())
        .max_by_key(|path| path.file_name().unwrap().to_os_string())
        .unwrap();
    let run_id = run.file_name().unwrap().to_string_lossy()[4..].to_owned();
    let records = fs::read_dir(&run)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("events-")
        })
        .collect::<Vec<_>>();
    let mut values = Vec::new();
    for record in records {
        for line in fs::read_to_string(record).unwrap().lines() {
            let value: Value = serde_json::from_str(line).unwrap();
            if value["fields"]["event"].is_string() {
                values.push(value);
            }
        }
    }
    (run_id, values)
}

fn event_name(value: &Value) -> &str {
    value["fields"]["event"].as_str().unwrap()
}

fn process_events<'a>(values: &'a [Value], process_id: &str) -> Vec<&'a Value> {
    values
        .iter()
        .filter(|value| value["fields"]["process_id"] == process_id)
        .filter(|value| {
            matches!(
                event_name(value),
                "process.started" | "process.spawned" | "process.finished" | "process.failed"
            )
        })
        .collect()
}

fn lifecycle(values: &[Value]) -> Vec<&str> {
    values
        .iter()
        .map(event_name)
        .filter(|name| {
            matches!(
                *name,
                "process.started" | "process.spawned" | "process.finished" | "process.failed"
            )
        })
        .collect()
}

fn assert_run(values: &[Value], run_id: &str) {
    assert!(values
        .iter()
        .filter(|value| value["fields"]["event"]
            .as_str()
            .is_some_and(|s| s.starts_with("process.")))
        .all(|value| value["run_id"] == run_id));
}

fn assert_terminal(events: &[&Value], expected: &str) {
    let terminals = events
        .iter()
        .filter(|event| matches!(event_name(event), "process.finished" | "process.failed"))
        .collect::<Vec<_>>();
    assert_eq!(terminals.len(), 1, "expected exactly one terminal event");
    assert_eq!(event_name(terminals[0]), expected);
}

#[test]
fn successful_process_records_pid_duration_and_real_exit_status() {
    let root = tempfile::tempdir().unwrap();
    let (session, result) = logged(root.path(), || {
        SystemProcessRunner.run(spec("/bin/sh", &["-c", "printf '%s' \"$$\""]))
    });
    let output = result.unwrap();
    assert_eq!(output.status.code(), Some(0));
    let (run_id, values) = events(root.path());
    assert_run(&values, &run_id);
    assert_eq!(
        lifecycle(&values),
        ["process.started", "process.spawned", "process.finished"]
    );
    let process_id = values[0]["fields"]["process_id"].as_str().unwrap();
    assert!(!process_id.is_empty());
    let matched = process_events(&values, process_id);
    assert_terminal(&matched, "process.finished");
    let child_pid = matched[1]["fields"]["pid"].as_u64().unwrap();
    assert!(child_pid > 0);
    assert_eq!(
        std::str::from_utf8(&output.stdout).unwrap(),
        child_pid.to_string()
    );
    assert_eq!(matched[2]["fields"]["exit_code"], 0);
    assert!(matched[2]["fields"]["duration_ms"].as_u64().is_some());
    drop(session);
}

#[test]
fn known_nonzero_child_status_is_still_a_successful_runner_result() {
    let root = tempfile::tempdir().unwrap();
    let (_, result) = logged(root.path(), || {
        SystemProcessRunner.run(spec("/bin/sh", &["-c", "exit 37"]))
    });
    assert_eq!(result.unwrap().status.code(), Some(37));
    let (_, values) = events(root.path());
    assert_eq!(
        lifecycle(&values),
        ["process.started", "process.spawned", "process.finished"]
    );
    assert_eq!(values.last().unwrap()["fields"]["exit_code"], 37);
}

#[test]
fn signalled_child_records_the_actual_terminating_signal() {
    let root = tempfile::tempdir().unwrap();
    let (_, result) = logged(root.path(), || {
        SystemProcessRunner.run(spec("/bin/sh", &["-c", "kill -TERM $$"]))
    });
    let output = result.unwrap();
    assert_eq!(output.status.code(), None);
    assert_eq!(output.status.signal(), Some(15));
    let (_, values) = events(root.path());
    assert_eq!(
        lifecycle(&values),
        ["process.started", "process.spawned", "process.finished"]
    );
    assert_eq!(values.last().unwrap()["fields"]["signal"], 15);
    assert!(values.last().unwrap()["fields"]["exit_code"].is_null());
}

#[test]
fn missing_executable_records_spawn_failure_without_spawn_event() {
    let root = tempfile::tempdir().unwrap();
    let mut command = spec("/definitely/not/a/real/executable", &[]);
    command.timeout = Some(Duration::from_secs(1));
    command.stdin = StdinMode::Bytes(b"input never reaches a child".to_vec());
    let (_, result) = logged(root.path(), || SystemProcessRunner.run(command));
    let error = result.unwrap_err();
    assert!(matches!(error, crate::error::ProcessError::Spawn { .. }));
    let (_, values) = events(root.path());
    assert_eq!(lifecycle(&values), ["process.started", "process.failed"]);
    assert_eq!(
        values.last().unwrap()["fields"]["classification"],
        "process.spawn"
    );
    assert_eq!(values.last().unwrap()["fields"]["stage"], "spawn");
    assert!(values.last().unwrap()["fields"]["cause"]
        .as_str()
        .is_some_and(|cause| !cause.is_empty()));
    assert_terminal(
        &process_events(&values, values[0]["fields"]["process_id"].as_str().unwrap()),
        "process.failed",
    );
    assert!(values
        .iter()
        .any(|event| event_name(event) == "process.stream.snapshot"
            && event["fields"]["stream"] == "stdin"
            && event["fields"]["state"] == "not_started"));
}

#[test]
fn cancellation_before_spawn_is_logged_without_a_spawn_event() {
    let root = tempfile::tempdir().unwrap();
    let mut command = spec("/bin/sh", &["-c", "exit 0"]);
    command.stdin = StdinMode::Bytes(b"not prepared".to_vec());
    command.cancellation.cancel();
    let (_, result) = logged(root.path(), || SystemProcessRunner.run(command));
    assert!(matches!(result, Err(crate::error::ProcessError::Cancelled)));
    let (_, values) = events(root.path());
    assert_eq!(lifecycle(&values), ["process.started", "process.failed"]);
    assert_eq!(
        values.last().unwrap()["fields"]["classification"],
        "process.cancelled"
    );
    assert_eq!(values.last().unwrap()["fields"]["stage"], "pre_spawn");
    assert!(values
        .iter()
        .any(|event| event_name(event) == "process.context.finished"
            && event["fields"]["status"] == "not_prepared"));
    assert!(!values.iter().any(|event| matches!(
        event_name(event),
        "process.context.arg"
            | "process.context.env"
            | "process.context.program"
            | "process.context.cwd"
    )));
    assert!(values
        .iter()
        .any(|event| event_name(event) == "process.stream.snapshot"
            && event["fields"]["stream"] == "stdin"
            && event["fields"]["state"] == "not_started"));
}

fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !path.exists() {
        assert!(
            Instant::now() < deadline,
            "child did not reach synchronization point"
        );
        thread::yield_now();
    }
}

#[test]
fn in_flight_cancellation_kills_child_and_records_one_failure() {
    let root = tempfile::tempdir().unwrap();
    let marker = PathBuf::from(root.path()).join("child-ready");
    let cancellation = CancellationToken::new();
    let child_token = cancellation.clone();
    let child_marker = marker.clone();
    let session = open_session(root.path());
    let result = thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let mut command = spec("/bin/sh", &[]);
            command.args = vec![
                "-c".into(),
                "sleep 2 & printf ready > \"$1\"; exec /bin/sleep 30".into(),
                "fixture".into(),
                child_marker.into_os_string(),
            ];
            command.cancellation = child_token;
            tracing::subscriber::with_default(session.subscriber(), || {
                SystemProcessRunner.run(command)
            })
        });
        wait_for_file(&marker);
        cancellation.cancel();
        worker.join().unwrap()
    });
    assert!(matches!(result, Err(crate::error::ProcessError::Cancelled)));
    session.flush();
    let (_, values) = events(root.path());
    assert_eq!(
        lifecycle(&values),
        ["process.started", "process.spawned", "process.failed"]
    );
    assert_eq!(
        values.last().unwrap()["fields"]["classification"],
        "process.cancelled"
    );
    assert_eq!(values.last().unwrap()["fields"]["stage"], "wait");
    assert_terminal(
        &process_events(&values, values[0]["fields"]["process_id"].as_str().unwrap()),
        "process.failed",
    );
}

#[test]
fn timeout_records_failure_and_never_a_normal_finish() {
    let root = tempfile::tempdir().unwrap();
    let (_, result) = logged(root.path(), || {
        let mut command = spec("/bin/sh", &["-c", "exec /bin/sleep 30"]);
        command.timeout = Some(Duration::from_millis(80));
        SystemProcessRunner.run(command)
    });
    assert!(matches!(result, Err(crate::error::ProcessError::Timeout)));
    let (_, values) = events(root.path());
    assert_eq!(
        lifecycle(&values),
        ["process.started", "process.spawned", "process.failed"]
    );
    assert_eq!(
        values.last().unwrap()["fields"]["classification"],
        "process.timeout"
    );
    assert!(values.last().unwrap()["fields"]["duration_ms"]
        .as_u64()
        .is_some());
}

#[test]
fn concurrent_attempts_have_distinct_process_ids_and_correlated_terminals() {
    let root = tempfile::tempdir().unwrap();
    let session = open_session(root.path());
    let barrier = Arc::new(Barrier::new(3));
    let outcomes = thread::scope(|scope| {
        let mut workers = Vec::new();
        for code in [11, 22] {
            let session = session.clone();
            let barrier = Arc::clone(&barrier);
            workers.push(scope.spawn(move || {
                let mut command = spec("/bin/sh", &["-c", "exit 0"]);
                command.args = vec!["-c".into(), format!("exit {code}").into()];
                barrier.wait();
                tracing::subscriber::with_default(session.subscriber(), || {
                    SystemProcessRunner.run(command)
                })
            }));
        }
        barrier.wait();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap().unwrap().status.code().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        outcomes
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        [11, 22].into()
    );
    session.flush();
    let (run_id, values) = events(root.path());
    assert_run(&values, &run_id);
    let ids = values
        .iter()
        .filter(|event| event_name(event) == "process.started")
        .map(|event| event["fields"]["process_id"].as_str().unwrap().to_owned())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(ids.len(), 2);
    for id in ids {
        let matched = process_events(&values, &id);
        assert_eq!(
            matched
                .iter()
                .map(|event| event_name(event))
                .collect::<Vec<_>>(),
            ["process.started", "process.spawned", "process.finished"]
        );
        assert_terminal(&matched, "process.finished");
        assert!(matched[1]["fields"]["pid"]
            .as_u64()
            .is_some_and(|pid| pid > 0));
    }
}

#[test]
fn unavailable_diagnostics_does_not_change_runner_result() {
    let mut command = spec("/bin/sh", &["-c", "printf result; exit 23"]);
    command.timeout = Some(Duration::from_secs(2));
    let result = SystemProcessRunner.run(command).unwrap();
    assert_eq!(result.status.code(), Some(23));
    assert_eq!(result.stdout, b"result");
    assert_eq!(result.stderr, b"");
}

#[test]
fn null_and_inherited_streams_are_reported_without_changing_child_execution() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("child-marker");
    let mut command = spec("/usr/bin/python3", &[]);
    command.stdin = StdinMode::Null;
    command.stdout = StreamMode::Null;
    command.stderr = StreamMode::Null;
    command.args = vec![
        "-c".into(),
        "import sys; n=len(sys.stdin.buffer.read()); open(sys.argv[1],'wb').write(str(n).encode())"
            .into(),
        marker.clone().into_os_string(),
    ];
    let (session, result) = logged(root.path(), || SystemProcessRunner.run(command));
    assert_eq!(result.unwrap().status.code(), Some(0));
    assert_eq!(fs::read(&marker).unwrap(), b"0");
    let (_, values) = events(root.path());
    assert!(values
        .iter()
        .any(|event| event_name(event) == "process.stream.closed"
            && event["fields"]["stream"] == "stdin"
            && event["fields"]["state"] == "not_applicable"
            && event["fields"]["reason"] == "no_input_configured"));
    for stream in ["stdout", "stderr"] {
        assert!(values
            .iter()
            .any(|event| event_name(event) == "process.stream.snapshot"
                && event["fields"]["stream"] == stream
                && event["fields"]["state"] == "unavailable"
                && event["fields"]["reason"] == "redirected_to_null"));
    }
    drop(session);

    let root = tempfile::tempdir().unwrap();
    let mut command = spec("/usr/bin/python3", &[]);
    command.stdin = StdinMode::Inherit;
    command.args = vec!["-c".into(), "import os; os.read(0,1)".into()];
    let (session, result) = logged(root.path(), || SystemProcessRunner.run(command));
    assert_eq!(result.unwrap().status.code(), Some(0));
    let (_, values) = events(root.path());
    assert!(values
        .iter()
        .any(|event| event_name(event) == "process.stream.snapshot"
            && event["fields"]["stream"] == "stdin"
            && event["fields"]["state"] == "unavailable"
            && event["fields"]["reason"] == "inherited_stdin"));
    drop(session);
}

struct ScriptedWriter {
    accepted: Arc<std::sync::Mutex<Vec<u8>>>,
    accepted_before_error: usize,
    calls: usize,
}

impl std::io::Write for ScriptedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.calls += 1;
        if self.calls == 1 {
            return Err(std::io::ErrorKind::Interrupted.into());
        }
        let mut accepted = self.accepted.lock().unwrap();
        if accepted.len() >= self.accepted_before_error {
            return Err(std::io::ErrorKind::BrokenPipe.into());
        }
        let count = bytes
            .len()
            .min(2)
            .min(self.accepted_before_error - accepted.len());
        accepted.extend_from_slice(&bytes[..count]);
        Ok(count)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn write_observer_tracks_partial_writes_interruptions_and_failure_after_prefix() {
    let root = tempfile::tempdir().unwrap();
    let accepted = Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer = ScriptedWriter {
        accepted: Arc::clone(&accepted),
        accepted_before_error: 5,
        calls: 0,
    };
    let session = open_session(root.path());
    let result = with_session(&session, || {
        crate::process::diagnostics::write_for_test(writer, b"abcdefgh")
    });
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::BrokenPipe);
    let accepted = accepted.lock().unwrap().clone();
    assert_eq!(accepted, b"abcde");
    let (_, values) = events(root.path());
    assert_eq!(
        payloads_for(root.path(), &values, "stdin", "written_to_pipe"),
        accepted
    );
    assert!(values
        .iter()
        .filter(|event| event_name(event) == "process.data.chunk"
            && event["fields"]["scope"] == "written_to_pipe")
        .all(|event| event["fields"]["observed_length"].as_u64().unwrap() <= 64 * 1024));
}

#[test]
fn diagnostic_storage_failure_and_absent_session_preserve_process_result() {
    let root = tempfile::tempdir().unwrap();
    let config = Config {
        session_bytes: 1,
        ..Config::default()
    };
    let session = open_session_with_config(root.path(), config);
    let mut command = spec("/usr/bin/python3", &[]);
    command.args = vec![
        "-c".into(),
        "import os; os.write(1,b'operational output')".into(),
    ];
    let result = with_session(&session, || SystemProcessRunner.run(command)).unwrap();
    assert_eq!(result.status.code(), Some(0));
    assert_eq!(result.stdout, b"operational output");
    assert!(
        !session.complete(),
        "the one-byte diagnostic budget must cause an observable storage failure"
    );

    let result = SystemProcessRunner
        .run(spec("/bin/sh", &["-c", "printf intact; exit 29"]))
        .unwrap();
    assert_eq!(result.status.code(), Some(29));
    assert_eq!(result.stdout, b"intact");
}

#[test]
fn concurrent_session_bound_processes_keep_payloads_with_their_process_ids() {
    let root = tempfile::tempdir().unwrap();
    let session = open_session(root.path());
    let barrier = Arc::new(Barrier::new(3));
    let outcomes = thread::scope(|scope| {
        let workers = [b"alpha".to_vec(), b"beta".to_vec()]
            .into_iter()
            .map(|bytes| {
                let session = session.clone();
                let barrier = Arc::clone(&barrier);
                scope.spawn(move || {
                    let script = format!("import os; os.write(1,bytes({bytes:?}))");
                    let mut command = spec("/usr/bin/python3", &[]);
                    command.args = vec!["-c".into(), script.into()];
                    barrier.wait();
                    let output =
                        with_session(&session, || SystemProcessRunner.run(command)).unwrap();
                    output.stdout
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(
        outcomes
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>(),
        [b"alpha".to_vec(), b"beta".to_vec()].into()
    );
    session.flush();
    let (_, values) = events(root.path());
    let chunks = values
        .iter()
        .filter(|event| {
            event_name(event) == "process.data.chunk" && event["fields"]["stream"] == "stdout"
        })
        .collect::<Vec<_>>();
    assert_eq!(chunks.len(), 2);
    let mut recovered = std::collections::HashMap::new();
    for chunk in chunks {
        recovered.insert(
            chunk["fields"]["process_id"].as_str().unwrap().to_owned(),
            payload_bytes(root.path(), chunk["fields"]["payload_id"].as_str().unwrap()),
        );
    }
    assert_eq!(recovered.len(), 2);
    assert!(recovered.values().any(|bytes| bytes == b"alpha"));
    assert!(recovered.values().any(|bytes| bytes == b"beta"));
}

#[test]
fn timeout_with_descendant_holding_pipe_returns_partial_snapshot_without_joining() {
    let root = tempfile::tempdir().unwrap();
    let ready = root.path().join("descendant-ready");
    let session = open_session(root.path());
    let result = with_session(&session, || {
        let mut command = spec("/bin/sh", &[]);
        command.args = vec![
            "-c".into(),
            "sleep 2 & echo ready > \"$1\"; exec sleep 30".into(),
            "fixture".into(),
            ready.clone().into_os_string(),
        ];
        command.timeout = Some(Duration::from_millis(100));
        SystemProcessRunner.run(command)
    });
    assert!(matches!(result, Err(crate::error::ProcessError::Timeout)));
    assert!(ready.exists());
    let (_, values) = events(root.path());
    assert!(values
        .iter()
        .any(|event| event_name(event) == "process.stream.snapshot"
            && event["fields"]["stream"] == "stdout"
            && event["fields"]["state"] == "partial"
            && event["fields"]["boundary"] == "runner_return"));
}

#[test]
fn with_session_restores_previous_session_even_when_nested_scope_unwinds() {
    let outer_root = tempfile::tempdir().unwrap();
    let inner_root = tempfile::tempdir().unwrap();
    let outer = open_session(outer_root.path());
    let inner = open_session(inner_root.path());
    with_session(&outer, || {
        let before = crate::logging::current_session().unwrap().run_id;
        assert_eq!(before, outer.run_id);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            with_session(&inner, || {
                assert_eq!(
                    crate::logging::current_session().unwrap().run_id,
                    inner.run_id
                );
                panic!("exercise RAII restoration");
            });
        }));
        assert_eq!(
            crate::logging::current_session().unwrap().run_id,
            outer.run_id
        );
    });
    assert!(crate::logging::current_session().is_none());
}

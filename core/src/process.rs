use std::{
    collections::HashMap,
    ffi::OsString,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use crate::error::ProcessError;

pub(crate) mod diagnostics;
use diagnostics::{
    DiagnosticContext, ObservationState, ObservedReader, ObservedWriter, ProcessDiagnostics,
    Stream, StreamTracker,
};

static NEXT_PROCESS_ID: AtomicU64 = AtomicU64::new(1);

struct ProcessAttempt {
    process_id: String,
    started: Instant,
}

impl ProcessAttempt {
    fn start() -> Self {
        let attempt = Self {
            process_id: format!(
                "process-{}",
                NEXT_PROCESS_ID.fetch_add(1, Ordering::Relaxed)
            ),
            started: Instant::now(),
        };
        tracing::info!(
            event = "process.started",
            process_id = %attempt.process_id,
        );
        attempt
    }

    fn spawned(&self, pid: u32) {
        tracing::info!(
            event = "process.spawned",
            process_id = %self.process_id,
            pid,
        );
    }

    fn finished(&self, status: &ExitStatus) {
        tracing::info!(
            event = "process.finished",
            process_id = %self.process_id,
            duration_ms = self.started.elapsed().as_millis() as u64,
            exit_code = status.code(),
            signal = exit_signal(status),
        );
    }

    fn failed(&self, stage: &'static str, error: &ProcessError) {
        let classification = match error {
            ProcessError::Spawn { .. } => "process.spawn",
            ProcessError::Timeout => "process.timeout",
            ProcessError::Cancelled => "process.cancelled",
            ProcessError::Io { .. } => "process.io",
            ProcessError::StreamThread => "process.stream_thread",
        };
        tracing::info!(
            event = "process.failed",
            process_id = %self.process_id,
            duration_ms = self.started.elapsed().as_millis() as u64,
            stage,
            classification,
            cause = %error,
        );
    }
}

#[cfg(unix)]
fn exit_signal(status: &ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

#[cfg(not(unix))]
fn exit_signal(_: &ExitStatus) -> Option<i32> {
    None
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StreamMode {
    Capture,
    // Keep at most limit + 1 bytes, allowing the caller to reject overflow.
    CaptureLimited(usize),
    Null,
}

#[derive(Clone, Debug)]
#[allow(dead_code)] // The process boundary supports callers that inherit or pipe stdin.
pub(crate) enum StdinMode {
    Bytes(Vec<u8>),
    Inherit,
    Null,
}

#[derive(Clone, Debug)]
pub(crate) struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub(crate) fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub(crate) fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CommandSpec {
    pub(crate) program: OsString,
    pub(crate) args: Vec<OsString>,
    pub(crate) cwd: Option<PathBuf>,
    pub(crate) env: HashMap<OsString, OsString>,
    pub(crate) clear_env: bool,
    pub(crate) stdin: StdinMode,
    pub(crate) stdout: StreamMode,
    pub(crate) stderr: StreamMode,
    pub(crate) timeout: Option<Duration>,
    pub(crate) cancellation: CancellationToken,
}

#[derive(Debug)]
pub(crate) struct CommandOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

pub(crate) trait ProcessRunner {
    fn run(&self, spec: CommandSpec) -> Result<CommandOutput, ProcessError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SystemProcessRunner;

impl ProcessRunner for SystemProcessRunner {
    fn run(&self, spec: CommandSpec) -> Result<CommandOutput, ProcessError> {
        let attempt = ProcessAttempt::start();
        let diagnostics = ProcessDiagnostics::capture(&attempt.process_id);
        if spec.cancellation.is_cancelled() {
            diagnostics.not_prepared();
            mark_streams_not_started(&diagnostics);
            diagnostics.snapshot(true);
            let error = ProcessError::Cancelled;
            attempt.failed("pre_spawn", &error);
            return Err(error);
        }

        let mut command = Command::new(&spec.program);
        command.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }
        if spec.clear_env {
            command.env_clear();
        }
        command.envs(&spec.env);
        command.stdin(stdin_stdio(&spec.stdin));
        command.stdout(stream_stdio(spec.stdout));
        command.stderr(stream_stdio(spec.stderr));

        diagnostics.start_context(
            &spec.program,
            &spec.args,
            spec.cwd.as_deref(),
            &spec.env,
            spec.clear_env,
        );
        if let StdinMode::Bytes(bytes) = &spec.stdin {
            diagnostics.requested_input(bytes);
        }
        initialize_stream_states(&diagnostics, &spec);

        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(source) => {
                mark_unspawned(&diagnostics);
                diagnostics.snapshot(false);
                let error = ProcessError::Spawn { source };
                attempt.failed("spawn", &error);
                return Err(error);
            }
        };
        attempt.spawned(child.id());
        let stdin_writer = take_stdin(
            &mut child,
            &spec.stdin,
            diagnostics.context.clone(),
            diagnostics.stdin.clone(),
        );
        let stdout_reader = take_reader(
            child.stdout.take(),
            spec.stdout,
            diagnostics.context.clone(),
            Stream::Stdout,
            diagnostics.stdout.clone(),
        );
        let stderr_reader = take_reader(
            child.stderr.take(),
            spec.stderr,
            diagnostics.context.clone(),
            Stream::Stderr,
            diagnostics.stderr.clone(),
        );
        let started = Instant::now();

        let result = wait_for_child(&mut child, started, spec.timeout, &spec.cancellation);
        let status = match result {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                // A descendant may still hold an inherited pipe open. Do not
                // defeat the process deadline by joining its stream readers.
                diagnostics.snapshot(true);
                attempt.failed("wait", &error);
                return Err(error);
            }
        };
        let writer_result = join_writer(stdin_writer);
        let stdout_result = join_reader(stdout_reader, spec.stdout, started, spec.timeout);
        let stderr_result = join_reader(stderr_reader, spec.stderr, started, spec.timeout);
        let (stdout, stderr) = match (writer_result, stdout_result, stderr_result) {
            (Err(error), _, _) => {
                diagnostics.snapshot(false);
                attempt.failed("stdin_write", &error);
                return Err(error);
            }
            (Ok(()), Err(error), _) => {
                diagnostics.snapshot(false);
                attempt.failed("stdout_read", &error);
                return Err(error);
            }
            (Ok(()), Ok(_), Err(error)) => {
                diagnostics.snapshot(false);
                attempt.failed("stderr_read", &error);
                return Err(error);
            }
            (Ok(()), Ok(stdout), Ok(stderr)) => (stdout, stderr),
        };

        diagnostics.snapshot(false);

        let output = CommandOutput {
            status,
            stdout,
            stderr,
        };
        attempt.finished(&output.status);
        Ok(output)
    }
}

fn stdin_stdio(mode: &StdinMode) -> Stdio {
    match mode {
        StdinMode::Bytes(_) => Stdio::piped(),
        StdinMode::Inherit => Stdio::inherit(),
        StdinMode::Null => Stdio::null(),
    }
}

fn stream_stdio(mode: StreamMode) -> Stdio {
    match mode {
        StreamMode::Capture | StreamMode::CaptureLimited(_) => Stdio::piped(),
        StreamMode::Null => Stdio::null(),
    }
}

fn take_stdin(
    child: &mut Child,
    mode: &StdinMode,
    context: Option<DiagnosticContext>,
    tracker: StreamTracker,
) -> Option<thread::JoinHandle<std::io::Result<()>>> {
    let StdinMode::Bytes(bytes) = mode else {
        return None;
    };
    let mut stdin = child.stdin.take()?;
    if context.is_some() {
        tracker.set_state(ObservationState::Partial, None);
    }
    let bytes = bytes.clone();
    Some(thread::spawn(move || {
        if let Some(context) = context.clone() {
            let mut observer = ObservedWriter::new(stdin, context.clone(), tracker.clone());
            let result = observer.write_all(&bytes);
            match &result {
                Ok(()) => {
                    diagnostics::close_writer(&context, &tracker, ObservationState::Complete, None)
                }
                Err(_) => diagnostics::close_writer(
                    &context,
                    &tracker,
                    ObservationState::Partial,
                    Some("write_failed"),
                ),
            }
            result
        } else {
            stdin.write_all(&bytes)
        }
    }))
}

fn take_reader<R: Read + Send + 'static>(
    reader: Option<R>,
    mode: StreamMode,
    context: Option<DiagnosticContext>,
    stream: Stream,
    tracker: StreamTracker,
) -> Option<thread::JoinHandle<std::io::Result<Vec<u8>>>> {
    if reader.is_some() && context.is_some() {
        tracker.set_state(ObservationState::Partial, None);
    }
    reader.map(|mut reader| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            let limit = match mode {
                StreamMode::CaptureLimited(limit) => Some((limit as u64).saturating_add(1)),
                _ => None,
            };
            let read_result = if let Some(limit) = limit {
                if let Some(context) = context.clone() {
                    let observer =
                        ObservedReader::new(reader, context.clone(), stream, tracker.clone());
                    observer.take(limit).read_to_end(&mut bytes)
                } else {
                    reader.take(limit).read_to_end(&mut bytes)
                }
            } else {
                if let Some(context) = context.clone() {
                    let mut observer =
                        ObservedReader::new(reader, context.clone(), stream, tracker.clone());
                    observer.read_to_end(&mut bytes)
                } else {
                    reader.read_to_end(&mut bytes)
                }
            };
            match read_result {
                Ok(_) => {
                    let state = if limit.is_some_and(|cap| bytes.len() as u64 >= cap) {
                        ObservationState::Limited
                    } else {
                        ObservationState::Complete
                    };
                    if let Some(context) = context {
                        diagnostics::close_reader(
                            &context,
                            stream,
                            &tracker,
                            state,
                            (state == ObservationState::Limited).then_some("capture_limit"),
                        );
                    }
                    Ok(bytes)
                }
                Err(error) => {
                    if let Some(context) = context {
                        diagnostics::close_reader(
                            &context,
                            stream,
                            &tracker,
                            ObservationState::Partial,
                            Some("read_failed"),
                        );
                    }
                    Err(error)
                }
            }
        })
    })
}

fn initialize_stream_states(diagnostics: &ProcessDiagnostics, spec: &CommandSpec) {
    match &spec.stdin {
        StdinMode::Bytes(_) => diagnostics
            .stdin
            .set_state(ObservationState::NotStarted, None),
        StdinMode::Inherit => diagnostics
            .stdin
            .set_state(ObservationState::Unavailable, Some("inherited_stdin")),
        StdinMode::Null => diagnostics
            .stdin
            .set_state(ObservationState::NotApplicable, Some("no_input_configured")),
    }
    for (mode, tracker, stream) in [
        (spec.stdout, &diagnostics.stdout, Stream::Stdout),
        (spec.stderr, &diagnostics.stderr, Stream::Stderr),
    ] {
        match mode {
            StreamMode::Capture | StreamMode::CaptureLimited(_) => {
                tracker.set_state(ObservationState::NotStarted, None)
            }
            StreamMode::Null => {
                tracker.set_state(ObservationState::Unavailable, Some("redirected_to_null"));
                if let Some(context) = &diagnostics.context {
                    diagnostics::close_reader(
                        context,
                        stream,
                        tracker,
                        ObservationState::Unavailable,
                        Some("redirected_to_null"),
                    );
                }
            }
        }
    }
    if !matches!(&spec.stdin, StdinMode::Bytes(_)) {
        if let Some(context) = &diagnostics.context {
            diagnostics::close_writer(
                context,
                &diagnostics.stdin,
                diagnostics.stdin.snapshot(false).0,
                diagnostics.stdin.snapshot(false).1.as_deref(),
            );
        }
    }
}

fn mark_streams_not_started(diagnostics: &ProcessDiagnostics) {
    for tracker in [&diagnostics.stdin, &diagnostics.stdout, &diagnostics.stderr] {
        tracker.set_state(ObservationState::NotStarted, None);
    }
}

fn mark_unspawned(diagnostics: &ProcessDiagnostics) {
    for tracker in [&diagnostics.stdin, &diagnostics.stdout, &diagnostics.stderr] {
        let (state, reason, _, _, _) = tracker.snapshot(false);
        if state == ObservationState::NotStarted {
            tracker.set_state(ObservationState::NotStarted, reason.as_deref());
        }
    }
}

fn join_writer(
    writer: Option<thread::JoinHandle<std::io::Result<()>>>,
) -> Result<(), ProcessError> {
    if let Some(writer) = writer {
        writer
            .join()
            .map_err(|_| ProcessError::StreamThread)?
            .map_err(|source| ProcessError::Io { source })?;
    }
    Ok(())
}

fn join_reader(
    reader: Option<thread::JoinHandle<std::io::Result<Vec<u8>>>>,
    mode: StreamMode,
    started: Instant,
    timeout: Option<Duration>,
) -> Result<Vec<u8>, ProcessError> {
    let Some(reader) = reader else {
        return Ok(Vec::new());
    };
    if let (StreamMode::CaptureLimited(_), Some(limit)) = (mode, timeout) {
        while !reader.is_finished() {
            if started.elapsed() >= limit {
                return Err(ProcessError::Timeout);
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
    reader
        .join()
        .map_err(|_| ProcessError::StreamThread)?
        .map_err(|source| ProcessError::Io { source })
}

fn wait_for_child(
    child: &mut Child,
    started: Instant,
    timeout: Option<Duration>,
    cancellation: &CancellationToken,
) -> Result<ExitStatus, ProcessError> {
    loop {
        if cancellation.is_cancelled() {
            return Err(ProcessError::Cancelled);
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|source| ProcessError::Io { source })?
        {
            return Ok(status);
        }
        if timeout.is_some_and(|limit| started.elapsed() >= limit) {
            return Err(ProcessError::Timeout);
        }
        thread::sleep(Duration::from_millis(10));
    }
}

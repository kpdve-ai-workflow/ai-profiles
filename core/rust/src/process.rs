use std::{
    collections::HashMap,
    ffi::OsString,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use crate::error::ProcessError;

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
        if spec.cancellation.is_cancelled() {
            return Err(ProcessError::Cancelled);
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

        let mut child = command
            .spawn()
            .map_err(|source| ProcessError::Spawn { source })?;
        let stdin_writer = take_stdin(&mut child, &spec.stdin);
        let stdout_reader = take_reader(child.stdout.take(), spec.stdout);
        let stderr_reader = take_reader(child.stderr.take(), spec.stderr);
        let started = Instant::now();

        let result = wait_for_child(&mut child, started, spec.timeout, &spec.cancellation);
        let status = match result {
            Ok(status) => status,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                // A descendant may still hold an inherited pipe open. Do not
                // defeat the process deadline by joining its stream readers.
                return Err(error);
            }
        };
        let writer_result = join_writer(stdin_writer);
        let stdout_result = join_reader(stdout_reader, spec.stdout, started, spec.timeout);
        let stderr_result = join_reader(stderr_reader, spec.stderr, started, spec.timeout);
        writer_result?;
        let stdout = stdout_result?;
        let stderr = stderr_result?;

        Ok(CommandOutput {
            status,
            stdout,
            stderr,
        })
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
) -> Option<thread::JoinHandle<std::io::Result<()>>> {
    let StdinMode::Bytes(bytes) = mode else {
        return None;
    };
    let mut stdin = child.stdin.take()?;
    let bytes = bytes.clone();
    Some(thread::spawn(move || stdin.write_all(&bytes)))
}

fn take_reader<R: Read + Send + 'static>(
    reader: Option<R>,
    mode: StreamMode,
) -> Option<thread::JoinHandle<std::io::Result<Vec<u8>>>> {
    reader.map(|mut reader| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            if let StreamMode::CaptureLimited(limit) = mode {
                reader
                    .take((limit as u64).saturating_add(1))
                    .read_to_end(&mut bytes)?;
            } else {
                reader.read_to_end(&mut bytes)?;
            }
            Ok(bytes)
        })
    })
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

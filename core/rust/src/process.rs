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
    Inherit,
    Null,
    RedirectToStderr,
}

#[derive(Clone, Debug)]
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

    fn is_cancelled(&self) -> bool {
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
        command.stdout(stream_stdio(spec.stdout, true));
        command.stderr(stream_stdio(spec.stderr, false));

        let mut child = command
            .spawn()
            .map_err(|source| ProcessError::Spawn { source })?;
        let stdin_writer = take_stdin(&mut child, &spec.stdin);
        let stdout_reader = take_reader(child.stdout.take());
        let stderr_reader = take_reader(child.stderr.take());
        let started = Instant::now();

        let result = wait_for_child(&mut child, started, spec.timeout, &spec.cancellation);
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let writer_result = join_writer(stdin_writer);
        let stdout_result = join_reader(stdout_reader);
        let stderr_result = join_reader(stderr_reader);
        let status = result?;
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

fn stream_stdio(mode: StreamMode, is_stdout: bool) -> Stdio {
    match mode {
        StreamMode::Capture => Stdio::piped(),
        StreamMode::Inherit => Stdio::inherit(),
        StreamMode::Null => Stdio::null(),
        StreamMode::RedirectToStderr if is_stdout => Stdio::from(std::io::stderr()),
        StreamMode::RedirectToStderr => Stdio::inherit(),
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
) -> Option<thread::JoinHandle<std::io::Result<Vec<u8>>>> {
    reader.map(|mut reader| {
        thread::spawn(move || {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes)?;
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
) -> Result<Vec<u8>, ProcessError> {
    let Some(reader) = reader else {
        return Ok(Vec::new());
    };
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

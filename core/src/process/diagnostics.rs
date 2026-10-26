//! Process argument and stream observations. Operational I/O always happens
//! through the wrapped reader/writer; diagnostic storage sees borrowed copies.
use crate::logging::{current_session, PayloadSummary, Session};
use std::{
    ffi::{OsStr, OsString},
    io::{self, Read, Write},
    os::unix::ffi::OsStrExt,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{Arc, Mutex, MutexGuard},
};
use tracing::{dispatcher::Dispatch, Span};

const FRAGMENT_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ObservationState {
    Complete,
    Limited,
    Unavailable,
    Partial,
    NotStarted,
    NotApplicable,
}

impl ObservationState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Limited => "limited",
            Self::Unavailable => "unavailable",
            Self::Partial => "partial",
            Self::NotStarted => "not_started",
            Self::NotApplicable => "not_applicable",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum Stream {
    Stdin,
    Stdout,
    Stderr,
}

impl Stream {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stdin => "stdin",
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
        }
    }
}

#[derive(Clone)]
pub(super) struct DiagnosticContext {
    session: Session,
    dispatch: Dispatch,
    span: Span,
    process_id: String,
}

#[derive(Default)]
struct Counts {
    observed: u64,
    stored: u64,
    chunks: u64,
    closed: bool,
    state: Option<ObservationState>,
    reason: Option<String>,
}

#[derive(Clone)]
pub(super) struct StreamTracker(Arc<Mutex<Counts>>);

impl StreamTracker {
    fn lock(&self) -> MutexGuard<'_, Counts> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(super) fn new(state: ObservationState, reason: Option<&str>) -> Self {
        Self(Arc::new(Mutex::new(Counts {
            state: Some(state),
            reason: reason.map(str::to_owned),
            ..Counts::default()
        })))
    }

    fn observed(&self, length: usize) -> u64 {
        let mut counts = self.lock();
        let offset = counts.observed;
        counts.observed = counts.observed.saturating_add(length as u64);
        offset
    }

    fn stored(&self, length: usize) -> u64 {
        let mut counts = self.lock();
        let index = counts.chunks;
        counts.stored = counts.stored.saturating_add(length as u64);
        counts.chunks = counts.chunks.saturating_add(1);
        index
    }

    pub(super) fn set_state(&self, state: ObservationState, reason: Option<&str>) {
        let mut counts = self.lock();
        counts.state = Some(state);
        counts.reason = reason.map(str::to_owned);
    }

    fn close(&self, state: ObservationState, reason: Option<&str>) -> (u64, u64, u64) {
        let mut counts = self.lock();
        counts.closed = true;
        counts.state = Some(state);
        counts.reason = reason.map(str::to_owned);
        (counts.observed, counts.stored, counts.chunks)
    }

    pub(super) fn snapshot(
        &self,
        timed_out_or_cancelled: bool,
    ) -> (ObservationState, Option<String>, u64, u64, u64) {
        let mut counts = self.lock();
        let state = if !counts.closed
            && timed_out_or_cancelled
            && counts.state == Some(ObservationState::Partial)
        {
            ObservationState::Partial
        } else {
            counts.state.unwrap_or(ObservationState::NotStarted)
        };
        if state == ObservationState::Partial {
            counts.state = Some(state);
        }
        (
            state,
            counts.reason.clone(),
            counts.observed,
            counts.stored,
            counts.chunks,
        )
    }
}

pub(super) struct ProcessDiagnostics {
    pub(super) context: Option<DiagnosticContext>,
    pub(super) stdin_requested: StreamTracker,
    pub(super) stdin: StreamTracker,
    pub(super) stdout: StreamTracker,
    pub(super) stderr: StreamTracker,
}

impl ProcessDiagnostics {
    pub(super) fn capture(process_id: &str) -> Self {
        let context = current_session().map(|session| DiagnosticContext {
            session,
            dispatch: tracing::dispatcher::get_default(Clone::clone),
            span: Span::current(),
            process_id: process_id.to_owned(),
        });
        Self {
            context,
            stdin_requested: StreamTracker::new(ObservationState::NotStarted, None),
            stdin: StreamTracker::new(ObservationState::NotStarted, None),
            stdout: StreamTracker::new(ObservationState::NotStarted, None),
            stderr: StreamTracker::new(ObservationState::NotStarted, None),
        }
    }

    pub(super) fn start_context(
        &self,
        program: &OsStr,
        args: &[OsString],
        cwd: Option<&std::path::Path>,
        env: &std::collections::HashMap<OsString, OsString>,
        clear_env: bool,
    ) {
        let Some(context) = &self.context else { return };
        let mut effective = if clear_env {
            std::collections::HashMap::new()
        } else {
            std::env::vars_os().collect()
        };
        effective.extend(env.iter().map(|(key, value)| (key.clone(), value.clone())));
        let mut pairs = effective.into_iter().collect::<Vec<_>>();
        pairs.sort_by(|a, b| a.0.as_os_str().as_bytes().cmp(b.0.as_os_str().as_bytes()));
        let schema_version = 1u8;
        context.event(|| tracing::info!(event = "process.context.started", schema_version, process_id = %context.process_id, argument_count = args.len(), environment_count = pairs.len(), environment_source = "snapshot_at_spawn_preparation"));
        emit_os(context, "process.context.program", None, program);
        for (index, value) in args.iter().enumerate() {
            emit_os(context, "process.context.arg", Some(index), value);
        }

        for (index, (key, value)) in pairs.iter().enumerate() {
            let (key_encoding, key_value, key_bytes) = encode_os(key);
            let (value_encoding, value_value, value_bytes) = encode_os(value);
            context.event(|| tracing::info!(event = "process.context.env", schema_version, process_id = %context.process_id, index, key_encoding, key = %key_value, key_bytes, value_encoding, value = %value_value, value_bytes, source = "snapshot_at_spawn_preparation"));
        }
        let current = std::env::current_dir();
        match (cwd, current) {
            (Some(configured), Ok(inherited)) => {
                let (configured_encoding, configured_value, configured_bytes) = encode_os(configured.as_os_str());
                let (inherited_encoding, inherited_value, inherited_bytes) = encode_os(inherited.as_os_str());
                context.event(|| tracing::info!(event = "process.context.cwd", schema_version, process_id = %context.process_id, configured = true, configured_encoding, configured_value = %configured_value, configured_bytes, inherited_context = "available", inherited_encoding, inherited_value = %inherited_value, inherited_bytes));
            }
            (Some(configured), Err(error)) => {
                let (encoding, value, bytes) = encode_os(configured.as_os_str());
                context.event(|| tracing::info!(event = "process.context.cwd", schema_version, process_id = %context.process_id, configured = true, encoding, value = %value, bytes, inherited_context = "unavailable", reason = %error));
            }
            (None, Ok(inherited)) => {
                let (encoding, value, bytes) = encode_os(inherited.as_os_str());
                context.event(|| tracing::info!(event = "process.context.cwd", schema_version, process_id = %context.process_id, configured = false, inherited_context = "available", encoding, value = %value, bytes));
            }
            (None, Err(error)) => context.event(|| tracing::info!(event = "process.context.cwd", schema_version, process_id = %context.process_id, configured = false, inherited_context = "unavailable", reason = %error)),
        }
        context.event(|| tracing::info!(event = "process.context.finished", schema_version, process_id = %context.process_id, status = "prepared", argument_count = args.len(), environment_count = pairs.len(), clear_env));
    }

    pub(super) fn not_prepared(&self) {
        let Some(context) = &self.context else { return };
        context.event(|| tracing::info!(event = "process.context.started", schema_version = 1u8, process_id = %context.process_id, status = "not_prepared"));
        context.event(|| tracing::info!(event = "process.context.finished", schema_version = 1u8, process_id = %context.process_id, status = "not_prepared"));
    }

    pub(super) fn requested_input(&self, bytes: &[u8]) {
        if let Some(context) = &self.context {
            record_bytes(
                context,
                Stream::Stdin,
                "requested_input",
                &self.stdin_requested,
                bytes,
            );
            context.close(
                Stream::Stdin,
                "requested_input",
                &self.stdin_requested,
                ObservationState::Complete,
                None,
            );
        }
    }

    pub(super) fn snapshot(&self, timed_out_or_cancelled: bool) {
        let Some(context) = &self.context else { return };
        for (stream, tracker) in [
            (Stream::Stdin, &self.stdin),
            (Stream::Stdout, &self.stdout),
            (Stream::Stderr, &self.stderr),
        ] {
            let (state, reason, observed, stored, chunks) =
                tracker.snapshot(timed_out_or_cancelled);
            context.event(|| tracing::info!(event = "process.stream.snapshot", process_id = %context.process_id, stream = stream.as_str(), boundary = "runner_return", state = state.as_str(), reason = reason.as_deref().unwrap_or(""), observed_bytes = observed, stored_bytes = stored, chunk_count = chunks));
        }
    }
}

impl DiagnosticContext {
    fn event(&self, emit: impl FnOnce()) {
        let dispatch = self.dispatch.clone();
        let span = self.span.clone();
        let _ = catch_unwind(AssertUnwindSafe(|| {
            tracing::dispatcher::with_default(&dispatch, || span.in_scope(emit));
        }));
    }

    fn observe(&self, stream: Stream, scope: &str, tracker: &StreamTracker, bytes: &[u8]) {
        let mut fragments = bytes.chunks(FRAGMENT_BYTES);
        if bytes.is_empty() {
            self.observe_fragment(stream, scope, tracker, &[]);
        }
        for fragment in &mut fragments {
            self.observe_fragment(stream, scope, tracker, fragment);
        }
    }

    fn observe_fragment(
        &self,
        stream: Stream,
        scope: &str,
        tracker: &StreamTracker,
        fragment: &[u8],
    ) {
        let offset = tracker.observed(fragment.len());
        let summary = match catch_unwind(AssertUnwindSafe(|| {
            self.session.payload(BorrowedBytes(fragment))
        })) {
            Ok(summary) => summary,
            Err(_) => PayloadSummary {
                payload_id: String::new(),
                complete: false,
                length: 0,
                sha256: String::new(),
            },
        };
        let index = tracker.stored(summary.length as usize);
        self.event(|| tracing::info!(event = "process.data.chunk", process_id = %self.process_id, stream = stream.as_str(), scope, index, offset, observed_length = fragment.len(), payload_id = %summary.payload_id, stored_length = summary.length, payload_complete = summary.complete));
    }

    fn close(
        &self,
        stream: Stream,
        scope: &str,
        tracker: &StreamTracker,
        state: ObservationState,
        reason: Option<&str>,
    ) {
        let (observed_bytes, stored_bytes, chunk_count) = tracker.close(state, reason);
        self.event(|| tracing::info!(event = "process.stream.closed", process_id = %self.process_id, stream = stream.as_str(), scope, state = state.as_str(), reason = reason.unwrap_or(""), observed_bytes, stored_bytes, chunk_count));
    }
}

struct BorrowedBytes<'a>(&'a [u8]);
impl Read for BorrowedBytes<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let count = output.len().min(self.0.len());
        output[..count].copy_from_slice(&self.0[..count]);
        self.0 = &self.0[count..];
        Ok(count)
    }
}

fn record_bytes(
    context: &DiagnosticContext,
    stream: Stream,
    scope: &str,
    tracker: &StreamTracker,
    bytes: &[u8],
) {
    context.observe(stream, scope, tracker, bytes);
}

fn encode_os(value: &OsStr) -> (&'static str, String, usize) {
    let bytes = value.as_bytes();
    match std::str::from_utf8(bytes) {
        Ok(text) => ("utf8", text.to_owned(), bytes.len()),
        Err(_) => {
            let mut hex = String::with_capacity(bytes.len() * 2);
            for byte in bytes {
                use std::fmt::Write as _;
                let _ = write!(hex, "{byte:02x}");
            }
            ("hex", hex, bytes.len())
        }
    }
}

fn emit_os(context: &DiagnosticContext, event: &'static str, index: Option<usize>, value: &OsStr) {
    let (encoding, value, bytes) = encode_os(value);
    match index {
        Some(index) => context.event(|| tracing::info!(event, schema_version = 1u8, process_id = %context.process_id, index, encoding, value = %value, bytes)),
        None => context.event(|| tracing::info!(event, schema_version = 1u8, process_id = %context.process_id, encoding, value = %value, bytes)),
    }
}

pub(super) struct ObservedReader<R> {
    inner: R,
    context: DiagnosticContext,
    stream: Stream,
    scope: &'static str,
    tracker: StreamTracker,
}

impl<R> ObservedReader<R> {
    pub(super) fn new(
        inner: R,
        context: DiagnosticContext,
        stream: Stream,
        tracker: StreamTracker,
    ) -> Self {
        Self {
            inner,
            context,
            stream,
            scope: "read_from_pipe",
            tracker,
        }
    }
}

impl<R: Read> Read for ObservedReader<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        match self.inner.read(output) {
            Ok(count) if count > 0 => {
                let _ = catch_unwind(AssertUnwindSafe(|| {
                    self.context
                        .observe(self.stream, self.scope, &self.tracker, &output[..count])
                }));
                Ok(count)
            }
            result => result,
        }
    }
}

pub(super) struct ObservedWriter<W> {
    inner: W,
    context: DiagnosticContext,
    tracker: StreamTracker,
}

impl<W> ObservedWriter<W> {
    pub(super) fn new(inner: W, context: DiagnosticContext, tracker: StreamTracker) -> Self {
        Self {
            inner,
            context,
            tracker,
        }
    }
}

impl<W: Write> Write for ObservedWriter<W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(bytes)?;
        if written > 0 {
            let _ = catch_unwind(AssertUnwindSafe(|| {
                self.context.observe(
                    Stream::Stdin,
                    "written_to_pipe",
                    &self.tracker,
                    &bytes[..written],
                )
            }));
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

pub(super) fn close_reader(
    context: &DiagnosticContext,
    stream: Stream,
    tracker: &StreamTracker,
    state: ObservationState,
    reason: Option<&str>,
) {
    context.close(stream, "read_from_pipe", tracker, state, reason);
}

pub(super) fn close_writer(
    context: &DiagnosticContext,
    tracker: &StreamTracker,
    state: ObservationState,
    reason: Option<&str>,
) {
    context.close(Stream::Stdin, "written_to_pipe", tracker, state, reason);
}

#[cfg(test)]
pub(crate) fn write_for_test<W: Write>(writer: W, bytes: &[u8]) -> io::Result<()> {
    let diagnostics = ProcessDiagnostics::capture("observer-test");
    let Some(context) = diagnostics.context else {
        return Err(io::Error::other("test requires an active session"));
    };
    let mut observer = ObservedWriter::new(writer, context, diagnostics.stdin);
    observer.write_all(bytes)
}

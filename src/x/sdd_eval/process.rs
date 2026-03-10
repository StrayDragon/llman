use std::io;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Instant;

#[derive(Debug)]
pub(crate) struct CapturedOutput {
    pub(crate) exit_code: Option<u32>,
    pub(crate) stdout_tail: Vec<u8>,
    pub(crate) stderr_tail: Vec<u8>,
    pub(crate) stdout_total: usize,
    pub(crate) stderr_total: usize,
    pub(crate) duration_ms: u64,
}

#[derive(Debug, Default)]
struct TailRing {
    cap: usize,
    buf: Vec<u8>,
    start: usize,
    len: usize,
    total: usize,
}

impl TailRing {
    fn new(cap: usize) -> Self {
        Self {
            cap,
            buf: Vec::new(),
            start: 0,
            len: 0,
            total: 0,
        }
    }

    fn push(&mut self, bytes: &[u8]) {
        self.total = self.total.saturating_add(bytes.len());

        let cap = self.cap;
        if cap == 0 || bytes.is_empty() {
            return;
        }

        if self.buf.is_empty() {
            self.buf = vec![0; cap];
        }

        if bytes.len() >= cap {
            let tail = &bytes[bytes.len() - cap..];
            self.buf.copy_from_slice(tail);
            self.start = 0;
            self.len = cap;
            return;
        }

        // Drop overflow from the head.
        if self.len + bytes.len() > cap {
            let overflow = self.len + bytes.len() - cap;
            self.start = (self.start + overflow) % cap;
            self.len -= overflow;
        }

        // Append bytes, wrapping as needed.
        let mut write_pos = (self.start + self.len) % cap;
        let mut offset = 0usize;
        let mut remaining = bytes.len();
        while remaining > 0 {
            let to_end = cap - write_pos;
            let to_copy = remaining.min(to_end);
            self.buf[write_pos..write_pos + to_copy]
                .copy_from_slice(&bytes[offset..offset + to_copy]);
            write_pos = (write_pos + to_copy) % cap;
            offset += to_copy;
            remaining -= to_copy;
            self.len += to_copy;
        }
    }

    fn total(&self) -> usize {
        self.total
    }

    fn into_bytes(self) -> Vec<u8> {
        if self.cap == 0 || self.len == 0 {
            return Vec::new();
        }
        if self.start + self.len <= self.cap {
            return self.buf[self.start..self.start + self.len].to_vec();
        }
        let first = &self.buf[self.start..self.cap];
        let second_len = (self.start + self.len) % self.cap;
        let second = &self.buf[..second_len];
        let mut out = Vec::with_capacity(self.len);
        out.extend_from_slice(first);
        out.extend_from_slice(second);
        out
    }
}

fn read_stream_tail<R: Read>(mut reader: R, cap: usize) -> io::Result<TailRing> {
    let mut capture = TailRing::new(cap);
    let mut buf = [0u8; 8192];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => capture.push(&buf[..n]),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
    Ok(capture)
}

pub(crate) fn run_command_capture_tail(
    command: &str,
    args: &[String],
    cwd: &Path,
    env: &[(String, String)],
    tail_cap: usize,
) -> io::Result<CapturedOutput> {
    let start = Instant::now();

    let mut cmd = Command::new(command);
    cmd.args(args);
    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }

    let mut child = cmd.spawn()?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let stdout_thread =
        stdout.map(|handle| thread::spawn(move || read_stream_tail(handle, tail_cap)));
    let stderr_thread =
        stderr.map(|handle| thread::spawn(move || read_stream_tail(handle, tail_cap)));

    let status = child.wait()?;

    let stdout_capture = match stdout_thread {
        Some(t) => match t.join() {
            Ok(Ok(capture)) => capture,
            Ok(Err(err)) => return Err(err),
            Err(_) => return Err(io::Error::other("stdout reader thread panicked")),
        },
        None => TailRing::default(),
    };

    let stderr_capture = match stderr_thread {
        Some(t) => match t.join() {
            Ok(Ok(capture)) => capture,
            Ok(Err(err)) => return Err(err),
            Err(_) => return Err(io::Error::other("stderr reader thread panicked")),
        },
        None => TailRing::default(),
    };

    let stdout_total = stdout_capture.total();
    let stderr_total = stderr_capture.total();

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(CapturedOutput {
        exit_code: status.code().map(|c| c as u32),
        stdout_tail: stdout_capture.into_bytes(),
        stderr_tail: stderr_capture.into_bytes(),
        stdout_total,
        stderr_total,
        duration_ms,
    })
}

pub(crate) fn should_insert_stderr_separator(
    stdout_total: usize,
    stdout_ends_with_newline: bool,
    stderr_total: usize,
) -> bool {
    stderr_total > 0 && (!stdout_ends_with_newline || stdout_total == 0)
}

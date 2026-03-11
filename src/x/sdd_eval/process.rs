use std::io;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;
use tempfile::NamedTempFile;

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

fn read_file_tail(file: &mut std::fs::File, cap: usize) -> io::Result<TailRing> {
    let total = usize::try_from(file.metadata()?.len()).unwrap_or(usize::MAX);
    let mut capture = TailRing::new(cap);
    capture.total = total;

    if total == 0 || cap == 0 {
        return Ok(capture);
    }

    let tail_offset = total.saturating_sub(cap);
    file.seek(SeekFrom::Start(tail_offset as u64))?;

    let mut buf = [0u8; 8192];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => capture.push(&buf[..n]),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }

    capture.total = total;
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
    let stdout_file = NamedTempFile::new()?;
    let stderr_file = NamedTempFile::new()?;

    let mut cmd = Command::new(command);
    cmd.args(args);
    cmd.current_dir(cwd);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::from(stdout_file.reopen()?));
    cmd.stderr(Stdio::from(stderr_file.reopen()?));
    for (k, v) in env {
        cmd.env(k, v);
    }

    let mut child = cmd.spawn()?;

    let status = child.wait()?;

    let mut stdout_reader = stdout_file.reopen()?;
    let stdout_capture = read_file_tail(&mut stdout_reader, tail_cap)?;
    let mut stderr_reader = stderr_file.reopen()?;
    let stderr_capture = read_file_tail(&mut stderr_reader, tail_cap)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[cfg(unix)]
    #[test]
    fn background_process_inheriting_stdio_does_not_block_capture() {
        let temp = tempdir().expect("temp dir");
        let started = Instant::now();

        let output = run_command_capture_tail(
            "sh",
            &[
                "-c".to_string(),
                "printf child-output; (sleep 1) &".to_string(),
            ],
            temp.path(),
            &[],
            1024,
        )
        .expect("capture output");

        assert!(
            started.elapsed().as_millis() < 800,
            "capture unexpectedly blocked on inherited stdio"
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout_tail), "child-output");
    }
}

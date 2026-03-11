use crate::fs_utils::atomic_write_with_mode;
use crate::x::sdd_eval::playbook;
use crate::x::sdd_eval::process::{
    CapturedOutput, run_command_capture_tail, should_insert_stderr_separator,
};
use crate::x::sdd_eval::secrets::SecretSet;
use agent_client_protocol as acp;
use agent_client_protocol::Agent;
use anyhow::{Context, Result, anyhow};
use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::BufRead;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::task::LocalSet;
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariantAcpMetricsV1 {
    pub version: u32,
    pub started_at: String,
    pub finished_at: String,
    pub iterations_attempted: u32,
    pub stop_reasons: Vec<String>,
    pub files_written: Vec<FileWriteRecord>,
    pub files_read: Vec<FileReadRecord>,
    pub terminal_commands: Vec<TerminalCommandRecord>,
    pub permission_requests: Vec<PermissionRecord>,
    pub denied_operations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileWriteRecord {
    pub path: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileReadRecord {
    pub path: String,
    pub bytes: usize,
    pub line: Option<u32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerminalCommandRecord {
    pub terminal_id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    #[serde(default)]
    pub duration_ms: u64,
    pub exit_code: Option<u32>,
    pub truncated: bool,
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionRecord {
    pub option_id: String,
    pub option_kind: String,
    pub option_name: String,
}

const MAX_TEXT_FILE_BYTES: usize = 4 * 1024 * 1024;

impl VariantAcpMetricsV1 {
    pub fn new_now() -> Self {
        Self {
            version: 1,
            started_at: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            finished_at: String::new(),
            iterations_attempted: 0,
            stop_reasons: Vec::new(),
            files_written: Vec::new(),
            files_read: Vec::new(),
            terminal_commands: Vec::new(),
            permission_requests: Vec::new(),
            denied_operations: Vec::new(),
        }
    }

    pub fn finish_now(&mut self) {
        self.finished_at = Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true);
    }
}

pub struct AcpRunResult {
    pub metrics: VariantAcpMetricsV1,
}

pub struct VariantRunParams {
    pub workspace_root: PathBuf,
    pub style: playbook::WorkflowStyle,
    pub agent_command: String,
    pub agent_args: Vec<String>,
    pub agent_env: HashMap<String, String>,
    pub max_iterations: u32,
    pub task_title: String,
    pub task_prompt: String,
    pub session_log_path: PathBuf,
}

pub fn run_variant(params: VariantRunParams) -> Result<AcpRunResult> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("create tokio runtime")?;

    let local = LocalSet::new();

    runtime.block_on(local.run_until(run_variant_async(params)))
}

async fn run_variant_async(params: VariantRunParams) -> Result<AcpRunResult> {
    let VariantRunParams {
        workspace_root,
        style,
        agent_command,
        agent_args,
        agent_env,
        max_iterations,
        task_title,
        task_prompt,
        session_log_path,
    } = params;
    let mut secrets = SecretSet::new();
    for value in agent_env.values() {
        secrets.push(value.clone());
    }

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&session_log_path)
        .with_context(|| format!("open session log {}", session_log_path.display()))?;
    let log_file = Arc::new(Mutex::new(log_file));

    let metrics = Arc::new(Mutex::new(VariantAcpMetricsV1::new_now()));

    let handler = Arc::new(EvalClient::new(
        workspace_root.clone(),
        log_file.clone(),
        metrics.clone(),
        secrets.clone(),
    ));

    let mut child = spawn_agent_process(&agent_command, &agent_args, &agent_env)
        .await
        .with_context(|| format!("spawn ACP agent `{}`", agent_command))?;

    let stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("agent stdin is not available"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("agent stdout is not available"))?;

    if let Some(stderr) = child.stderr.take() {
        let log_file = log_file.clone();
        let secrets = secrets.clone();
        tokio::task::spawn_local(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                let redacted = secrets.redact(&line);
                let _ = writeln_lock(
                    &log_file,
                    format!(
                        "{{\"type\":\"agent_stderr\",\"line\":{}}}",
                        json_escape(&redacted)
                    ),
                );
            }
        });
    }

    let (conn, io_task) =
        acp::ClientSideConnection::new(handler, stdin.compat_write(), stdout.compat(), |fut| {
            tokio::task::spawn_local(fut);
        });

    tokio::task::spawn_local(async move {
        let _ = io_task.await;
    });

    let caps = acp::ClientCapabilities::new()
        .fs(acp::FileSystemCapability::new()
            .read_text_file(true)
            .write_text_file(true))
        .terminal(true);

    let init_req = acp::InitializeRequest::new(acp::ProtocolVersion::LATEST)
        .client_capabilities(caps)
        .client_info(
            acp::Implementation::new("llman", env!("CARGO_PKG_VERSION")).title("llman sdd-eval"),
        );

    let _init_resp = conn.initialize(init_req).await?;

    let session_resp = conn
        .new_session(acp::NewSessionRequest::new(workspace_root.clone()))
        .await?;
    let session_id = session_resp.session_id;

    for iter_idx in 1..=max_iterations {
        let prompt = build_prompt(style, &task_title, &task_prompt, iter_idx, max_iterations);
        metrics.lock().expect("metrics lock").iterations_attempted += 1;
        let resp = conn
            .prompt(acp::PromptRequest::new(
                session_id.clone(),
                vec![acp::ContentBlock::Text(acp::TextContent::new(prompt))],
            ))
            .await?;

        metrics
            .lock()
            .expect("metrics lock")
            .stop_reasons
            .push(format!("{:?}", resp.stop_reason));
    }

    metrics.lock().expect("metrics lock").finish_now();

    // Best-effort cleanup.
    let _ = child.kill().await;
    let _ = child.wait().await;

    let out = metrics.lock().expect("metrics lock").clone();
    Ok(AcpRunResult { metrics: out })
}

fn build_prompt(
    style: playbook::WorkflowStyle,
    task_title: &str,
    task_prompt: &str,
    iter_idx: u32,
    max_iterations: u32,
) -> String {
    let style_str = match style {
        playbook::WorkflowStyle::Sdd => "sdd",
        playbook::WorkflowStyle::SddLegacy => "sdd-legacy",
    };

    if iter_idx == 1 {
        return format!(
            "You are running inside an automated evaluation pipeline.\n\
Workflow style: {style_str}\n\
Max iterations: {max_iterations}\n\
\n\
Task: {task_title}\n\
\n\
{task_prompt}\n\
\n\
Constraints:\n\
- Work only inside this repository/workspace.\n\
- Use the provided file system and terminal tools.\n\
- Prefer small, verifiable steps.\n\
- When you believe the task is complete, say DONE.\n"
        );
    }

    format!(
        "Continue the task (iteration {iter_idx}/{max_iterations}).\n\
If the task is already complete, run a quick verification and say DONE.\n"
    )
}

async fn spawn_agent_process(
    command: &str,
    args: &[String],
    env: &HashMap<String, String>,
) -> Result<tokio::process::Child> {
    let mut cmd = Command::new(command);
    cmd.args(args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    for (k, v) in env {
        cmd.env(k, v);
    }
    let child = cmd.spawn().with_context(|| "spawn agent process")?;
    Ok(child)
}

#[derive(Clone)]
struct EvalClient {
    workspace_root: PathBuf,
    log_file: Arc<Mutex<File>>,
    metrics: Arc<Mutex<VariantAcpMetricsV1>>,
    secrets: SecretSet,
    terminals: Arc<Mutex<TerminalStore>>,
}

impl EvalClient {
    fn new(
        workspace_root: PathBuf,
        log_file: Arc<Mutex<File>>,
        metrics: Arc<Mutex<VariantAcpMetricsV1>>,
        secrets: SecretSet,
    ) -> Self {
        Self {
            workspace_root,
            log_file,
            metrics,
            secrets,
            terminals: Arc::new(Mutex::new(TerminalStore::default())),
        }
    }

    fn deny(&self, msg: impl Into<String>) -> acp::Error {
        let msg = msg.into();
        self.metrics
            .lock()
            .expect("metrics lock")
            .denied_operations
            .push(msg.clone());
        acp::Error::invalid_params().data(serde_json::json!({ "error": msg }))
    }

    fn log_json_line(&self, ty: &str, payload: serde_json::Value) {
        let Ok(raw) = serde_json::to_string(&serde_json::json!({ "type": ty, "payload": payload }))
        else {
            return;
        };
        let redacted = self.secrets.redact(&raw);
        let _ = writeln_lock(&self.log_file, redacted);
    }

    fn validate_path(&self, requested: &Path) -> std::result::Result<PathBuf, acp::Error> {
        if !requested.is_absolute() {
            return Err(self.deny("ACP path must be absolute"));
        }
        if requested
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        {
            return Err(self.deny("Path traversal is not allowed"));
        }
        if !requested.starts_with(&self.workspace_root) {
            return Err(self.deny("Path is outside workspace"));
        }
        if has_symlink_prefix(&self.workspace_root, requested) {
            return Err(self.deny("Symlink traversal is not allowed"));
        }
        Ok(requested.to_path_buf())
    }

    fn validate_cwd(&self, cwd: Option<&PathBuf>) -> std::result::Result<PathBuf, acp::Error> {
        let cwd = cwd.cloned().unwrap_or_else(|| self.workspace_root.clone());
        self.validate_path(&cwd)
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Client for EvalClient {
    async fn request_permission(
        &self,
        args: acp::RequestPermissionRequest,
    ) -> acp::Result<acp::RequestPermissionResponse> {
        let selected = args
            .options
            .iter()
            .find(|o| {
                matches!(
                    o.kind,
                    acp::PermissionOptionKind::AllowOnce | acp::PermissionOptionKind::AllowAlways
                )
            })
            .or_else(|| args.options.first())
            .cloned();

        let Some(selected) = selected else {
            return Ok(acp::RequestPermissionResponse::new(
                acp::RequestPermissionOutcome::Cancelled,
            ));
        };

        self.metrics
            .lock()
            .expect("metrics lock")
            .permission_requests
            .push(PermissionRecord {
                option_id: selected.option_id.to_string(),
                option_kind: format!("{:?}", selected.kind),
                option_name: selected.name.clone(),
            });

        Ok(acp::RequestPermissionResponse::new(
            acp::RequestPermissionOutcome::Selected(acp::SelectedPermissionOutcome::new(
                selected.option_id,
            )),
        ))
    }

    async fn session_notification(&self, args: acp::SessionNotification) -> acp::Result<()> {
        if let Ok(value) = serde_json::to_value(&args) {
            self.log_json_line("session_notification", value);
        }
        Ok(())
    }

    async fn write_text_file(
        &self,
        args: acp::WriteTextFileRequest,
    ) -> acp::Result<acp::WriteTextFileResponse> {
        let path = self.validate_path(&args.path)?;
        validate_write_text_content_size(&args.content)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(acp::Error::into_internal_error)?;
        }
        atomic_write_with_mode(&path, args.content.as_bytes(), None).map_err(|err| {
            acp::Error::into_internal_error(std::io::Error::other(err.to_string()))
        })?;

        self.metrics
            .lock()
            .expect("metrics lock")
            .files_written
            .push(FileWriteRecord {
                path: path.display().to_string(),
                bytes: args.content.len(),
            });

        Ok(acp::WriteTextFileResponse::new())
    }

    async fn read_text_file(
        &self,
        args: acp::ReadTextFileRequest,
    ) -> acp::Result<acp::ReadTextFileResponse> {
        let path = self.validate_path(&args.path)?;
        let wants_sliced = args.line.is_some() || args.limit.is_some();

        let file_len = std::fs::metadata(&path)
            .map(|m| m.len())
            .map_err(acp::Error::into_internal_error)?;
        if !wants_sliced && file_len > (MAX_TEXT_FILE_BYTES as u64) {
            let msg = format!(
                "file is too large to read without line/limit (size={} bytes, max={} bytes)",
                file_len, MAX_TEXT_FILE_BYTES
            );
            return Err(acp::Error::invalid_params().data(serde_json::json!({ "error": msg })));
        }

        #[derive(Debug)]
        enum ReadTextFileError {
            TooLarge(String),
            Io(std::io::Error),
        }

        let path_for_read = path.clone();
        let line = args.line;
        let limit = args.limit;

        let content = tokio::task::spawn_blocking(
            move || -> std::result::Result<String, ReadTextFileError> {
            if !wants_sliced {
                let bytes =
                    std::fs::read(&path_for_read).map_err(ReadTextFileError::Io)?;
                return Ok(String::from_utf8_lossy(&bytes).to_string());
            }

            let start = line.unwrap_or(1).saturating_sub(1) as usize;
            let limit = limit.map(|v| v as usize).unwrap_or(2000);

            let file = std::fs::File::open(&path_for_read).map_err(ReadTextFileError::Io)?;
            let mut reader = std::io::BufReader::new(file);

            let mut idx = 0usize;
            let mut buf = Vec::new();
            while idx < start {
                buf.clear();
                let n = reader
                    .read_until(b'\n', &mut buf)
                    .map_err(ReadTextFileError::Io)?;
                if n == 0 {
                    return Ok(String::new());
                }
                idx += 1;
            }

            let mut out = String::new();
            for _ in 0..limit {
                buf.clear();
                let n = reader
                    .read_until(b'\n', &mut buf)
                    .map_err(ReadTextFileError::Io)?;
                if n == 0 {
                    break;
                }

                if buf.ends_with(b"\n") {
                    buf.pop();
                    if buf.ends_with(b"\r") {
                        buf.pop();
                    }
                } else if buf.ends_with(b"\r") {
                    buf.pop();
                }

                let line = String::from_utf8_lossy(&buf);
                let additional = line.len().saturating_add(if out.is_empty() { 0 } else { 1 });
                if out.len().saturating_add(additional) > MAX_TEXT_FILE_BYTES {
                    return Err(ReadTextFileError::TooLarge(format!(
                        "read_text_file output exceeds max={} bytes; narrow the requested range",
                        MAX_TEXT_FILE_BYTES
                    )));
                }

                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(line.as_ref());
            }

            Ok(out)
        },
        )
        .await
        .map_err(acp::Error::into_internal_error)?
        .map_err(|err| match err {
            ReadTextFileError::TooLarge(msg) => {
                acp::Error::invalid_params().data(serde_json::json!({ "error": msg }))
            }
            ReadTextFileError::Io(err) => acp::Error::into_internal_error(err),
        })?;

        self.metrics
            .lock()
            .expect("metrics lock")
            .files_read
            .push(FileReadRecord {
                path: path.display().to_string(),
                bytes: content.len(),
                line: args.line,
                limit: args.limit,
            });

        Ok(acp::ReadTextFileResponse::new(content))
    }

    async fn create_terminal(
        &self,
        args: acp::CreateTerminalRequest,
    ) -> acp::Result<acp::CreateTerminalResponse> {
        let command = args.command.trim().to_string();
        if command.is_empty() {
            return Err(self.deny("terminal command must not be empty"));
        }
        if !is_allowed_command(&command) {
            return Err(self.deny(format!("terminal command is not allowed: {}", command)));
        }
        let cwd = self.validate_cwd(args.cwd.as_ref())?;

        const DEFAULT_OUTPUT_BYTE_LIMIT: usize = 20_000;
        const MAX_OUTPUT_BYTE_LIMIT: usize = 200_000;
        let output_byte_limit = args
            .output_byte_limit
            .unwrap_or(DEFAULT_OUTPUT_BYTE_LIMIT as u64)
            .min(MAX_OUTPUT_BYTE_LIMIT as u64) as usize;
        let env_pairs: Vec<(String, String)> =
            args.env.into_iter().map(|v| (v.name, v.value)).collect();

        let command_clone = command.clone();
        let args_for_cmd = args.args.clone();
        let args_for_record = args_for_cmd.clone();
        let cwd_clone = cwd.clone();

        let tail_cap = output_byte_limit.saturating_add(self.secrets.max_len());
        let captured: CapturedOutput = tokio::task::spawn_blocking(move || {
            let start = std::time::Instant::now();
            match run_command_capture_tail(
                &command_clone,
                &args_for_cmd,
                &cwd_clone,
                &env_pairs,
                tail_cap,
            ) {
                Ok(captured) => captured,
                Err(err) => {
                    let msg = format!("Failed to execute: {err}");
                    CapturedOutput {
                        exit_code: None,
                        stdout_tail: msg.as_bytes().to_vec(),
                        stderr_tail: Vec::new(),
                        stdout_total: msg.len(),
                        stderr_total: 0,
                        duration_ms: start.elapsed().as_millis() as u64,
                    }
                }
            }
        })
        .await
        .map_err(acp::Error::into_internal_error)?;

        let stdout = String::from_utf8_lossy(&captured.stdout_tail);
        let stderr = String::from_utf8_lossy(&captured.stderr_tail);
        let stdout_ends_with_newline =
            captured.stdout_total > 0 && captured.stdout_tail.last() == Some(&b'\n');
        let insert_sep = should_insert_stderr_separator(
            captured.stdout_total,
            stdout_ends_with_newline,
            captured.stderr_total,
        );
        let combined_total_bytes = captured
            .stdout_total
            .saturating_add(if insert_sep { 1 } else { 0 })
            .saturating_add(captured.stderr_total);

        let mut combined = String::new();
        combined.push_str(stdout.as_ref());
        if insert_sep && !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push_str(stderr.as_ref());

        let exit_code = captured.exit_code;
        let duration_ms = captured.duration_ms;

        let redacted = self.secrets.redact(&combined);
        let (output, _) = truncate_tail_to_byte_limit(&redacted, output_byte_limit);
        let truncated = combined_total_bytes > output_byte_limit;

        let mut terminals = self.terminals.lock().expect("terminals lock");
        let terminal_id = terminals.next_id();
        terminals.records.insert(
            terminal_id.clone(),
            TerminalRecord {
                output: output.clone(),
                truncated,
                exit_code,
            },
        );

        self.metrics
            .lock()
            .expect("metrics lock")
            .terminal_commands
            .push(TerminalCommandRecord {
                terminal_id: terminal_id.to_string(),
                command,
                args: args_for_record,
                cwd: cwd.display().to_string(),
                duration_ms,
                exit_code,
                truncated,
                output,
            });

        Ok(acp::CreateTerminalResponse::new(terminal_id))
    }

    async fn terminal_output(
        &self,
        args: acp::TerminalOutputRequest,
    ) -> acp::Result<acp::TerminalOutputResponse> {
        let terminals = self.terminals.lock().expect("terminals lock");
        let Some(rec) = terminals.records.get(&args.terminal_id) else {
            return Err(acp::Error::resource_not_found(Some(
                args.terminal_id.to_string(),
            )));
        };
        Ok(
            acp::TerminalOutputResponse::new(rec.output.clone(), rec.truncated)
                .exit_status(acp::TerminalExitStatus::new().exit_code(rec.exit_code)),
        )
    }

    async fn wait_for_terminal_exit(
        &self,
        args: acp::WaitForTerminalExitRequest,
    ) -> acp::Result<acp::WaitForTerminalExitResponse> {
        let terminals = self.terminals.lock().expect("terminals lock");
        let Some(rec) = terminals.records.get(&args.terminal_id) else {
            return Err(acp::Error::resource_not_found(Some(
                args.terminal_id.to_string(),
            )));
        };
        Ok(acp::WaitForTerminalExitResponse::new(
            acp::TerminalExitStatus::new().exit_code(rec.exit_code),
        ))
    }

    async fn kill_terminal_command(
        &self,
        args: acp::KillTerminalCommandRequest,
    ) -> acp::Result<acp::KillTerminalCommandResponse> {
        // v1: terminals are run-to-completion synchronously; nothing to kill.
        let terminals = self.terminals.lock().expect("terminals lock");
        if !terminals.records.contains_key(&args.terminal_id) {
            return Err(acp::Error::resource_not_found(Some(
                args.terminal_id.to_string(),
            )));
        }
        Ok(acp::KillTerminalCommandResponse::new())
    }

    async fn release_terminal(
        &self,
        args: acp::ReleaseTerminalRequest,
    ) -> acp::Result<acp::ReleaseTerminalResponse> {
        self.terminals
            .lock()
            .expect("terminals lock")
            .records
            .remove(&args.terminal_id);
        Ok(acp::ReleaseTerminalResponse::new())
    }
}

#[derive(Default)]
struct TerminalStore {
    next: u64,
    records: HashMap<acp::TerminalId, TerminalRecord>,
}

impl TerminalStore {
    fn next_id(&mut self) -> acp::TerminalId {
        self.next += 1;
        acp::TerminalId::new(format!("term-{}", self.next))
    }
}

struct TerminalRecord {
    output: String,
    truncated: bool,
    exit_code: Option<u32>,
}

pub(crate) fn is_allowed_command(command: &str) -> bool {
    // NOTE: This is a lightweight guard (command-name allowlist) for sdd-eval runs.
    // It is NOT an OS-level sandbox. Direct interpreters are intentionally excluded so
    // the agent cannot trivially execute ad-hoc code via `python -c` / `node -e`.
    // Build/test tools remain allowed because sdd-eval needs to exercise the workspace.
    let name = Path::new(command)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(command);
    matches!(
        name,
        "git" | "rg" | "cargo" | "just" | "npm" | "pnpm" | "yarn" | "pytest" | "go" | "make"
    )
}

pub(crate) fn truncate_tail_to_byte_limit(s: &str, limit: usize) -> (String, bool) {
    if s.len() <= limit {
        return (s.to_string(), false);
    }
    let mut bytes = 0usize;
    let mut out = String::new();
    for ch in s.chars().rev() {
        bytes += ch.len_utf8();
        if bytes > limit {
            break;
        }
        out.push(ch);
    }
    let out = out.chars().rev().collect::<String>();
    (out, true)
}

pub(crate) fn has_symlink_prefix(workspace_root: &Path, requested: &Path) -> bool {
    if let Ok(meta) = std::fs::symlink_metadata(workspace_root)
        && meta.file_type().is_symlink()
    {
        return true;
    }
    let mut cur = workspace_root.to_path_buf();
    let Ok(rel) = requested.strip_prefix(workspace_root) else {
        return true;
    };
    for comp in rel.components() {
        let Component::Normal(name) = comp else {
            continue;
        };
        cur.push(name);
        if let Ok(meta) = std::fs::symlink_metadata(&cur)
            && meta.file_type().is_symlink()
        {
            return true;
        }
    }
    false
}

fn writeln_lock(file: &Arc<Mutex<File>>, line: String) -> std::io::Result<()> {
    let mut f = file.lock().expect("log file lock");
    f.write_all(line.as_bytes())?;
    if !line.ends_with('\n') {
        f.write_all(b"\n")?;
    }
    Ok(())
}

fn json_escape(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"<unprintable>\"".to_string())
}

fn validate_write_text_content_size(content: &str) -> acp::Result<()> {
    if content.len() <= MAX_TEXT_FILE_BYTES {
        return Ok(());
    }

    let msg = format!(
        "file content is too large to write (size={} bytes, max={} bytes)",
        content.len(),
        MAX_TEXT_FILE_BYTES
    );
    Err(acp::Error::invalid_params().data(serde_json::json!({ "error": msg })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_interpreters_are_not_allowlisted() {
        assert!(is_allowed_command("git"));
        assert!(is_allowed_command("cargo"));
        assert!(!is_allowed_command("python"));
        assert!(!is_allowed_command("/usr/bin/python3"));
        assert!(!is_allowed_command("node"));
    }

    #[test]
    fn oversized_writes_are_rejected() {
        assert!(validate_write_text_content_size("ok").is_ok());
        assert!(validate_write_text_content_size(&"a".repeat(MAX_TEXT_FILE_BYTES + 1)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_prefix_detects_symlinked_root() {
        use std::os::unix::fs as unix_fs;
        use tempfile::tempdir;

        let dir = tempdir().expect("tempdir");
        let real_root = dir.path().join("real-root");
        std::fs::create_dir_all(&real_root).expect("create real root");

        let link_root = dir.path().join("link-root");
        unix_fs::symlink(&real_root, &link_root).expect("symlink root");

        assert!(has_symlink_prefix(&link_root, &link_root.join("file.txt")));
    }
}

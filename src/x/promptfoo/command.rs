use crate::arg_utils::split_shell_args;
use anyhow::{Result, anyhow, bail};
use clap::{Args, Subcommand};
use std::env;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const ENV_PROMPTFOO_CMD: &str = "LLMAN_PROMPTFOO_CMD";

#[derive(Args)]
#[command(about = "Promptfoo integration (experimental)")]
#[command(subcommand_required = true, arg_required_else_help = true)]
pub struct PromptfooArgs {
    #[command(subcommand)]
    pub command: PromptfooCommand,
}

#[derive(Subcommand)]
pub enum PromptfooCommand {
    /// Check promptfoo availability and print its version
    Check,

    /// Validate config and test suite
    Validate {
        /// Path to configuration file(s). When omitted, uses promptfoo defaults.
        #[arg(long, short = 'c', action = clap::ArgAction::Append)]
        config: Vec<PathBuf>,
        /// Working directory to run promptfoo in
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Extra args passed to promptfoo after `--`
        #[arg(last = true)]
        extra_args: Vec<String>,
    },

    /// Run promptfoo eval
    Eval {
        /// Path to configuration file(s). When omitted, uses promptfoo defaults.
        #[arg(long, short = 'c', action = clap::ArgAction::Append)]
        config: Vec<PathBuf>,
        /// Working directory to run promptfoo in
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Output file paths (repeatable). Formats: csv, json, jsonl, yaml, html, etc.
        #[arg(long, short = 'o', action = clap::ArgAction::Append)]
        output: Vec<PathBuf>,
        /// Maximum number of concurrent API calls
        #[arg(long)]
        max_concurrency: Option<u32>,
        /// Number of times to run each test
        #[arg(long)]
        repeat: Option<u32>,
        /// Delay between each test (ms)
        #[arg(long)]
        delay: Option<u64>,
        /// Do not read or write results to disk cache
        #[arg(long)]
        no_cache: bool,
        /// Print the resolved argv and exit without running promptfoo
        #[arg(long)]
        dry_run: bool,
        /// Extra args passed to promptfoo after `--`
        #[arg(last = true)]
        extra_args: Vec<String>,
    },

    /// Open the promptfoo viewer
    View {
        /// Path to configuration file(s). When omitted, uses promptfoo defaults.
        #[arg(long, short = 'c', action = clap::ArgAction::Append)]
        config: Vec<PathBuf>,
        /// Working directory to run promptfoo in
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Extra args passed to promptfoo after `--`
        #[arg(last = true)]
        extra_args: Vec<String>,
    },
}

pub fn run(args: &PromptfooArgs) -> Result<()> {
    match &args.command {
        PromptfooCommand::Check => cmd_check(),
        PromptfooCommand::Validate {
            config,
            cwd,
            extra_args,
        } => cmd_validate(config, cwd.as_deref(), extra_args),
        PromptfooCommand::Eval {
            config,
            cwd,
            output,
            max_concurrency,
            repeat,
            delay,
            no_cache,
            dry_run,
            extra_args,
        } => cmd_eval(
            config,
            cwd.as_deref(),
            output,
            *max_concurrency,
            *repeat,
            *delay,
            *no_cache,
            *dry_run,
            extra_args,
        ),
        PromptfooCommand::View {
            config,
            cwd,
            extra_args,
        } => cmd_view(config, cwd.as_deref(), extra_args),
    }
}

fn cmd_check() -> Result<()> {
    let (program, base_args) = resolve_promptfoo_cmd()?;

    let mut cmd = Command::new(&program);
    cmd.args(&base_args).arg("--version");

    let out = cmd.output().map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            return anyhow!(not_found_help(&program));
        }
        anyhow!(e).context(format!("run `{}`", display_cmd(&program, &base_args)))
    })?;
    if !out.status.success() {
        bail!(
            "promptfoo check failed (exit={}). stderr:\n{}",
            out.status,
            String::from_utf8_lossy(&out.stderr)
        );
    }

    print!("{}", String::from_utf8_lossy(&out.stdout));
    Ok(())
}

fn cmd_validate(config: &[PathBuf], cwd: Option<&Path>, extra_args: &[String]) -> Result<()> {
    let (program, base_args) = resolve_promptfoo_cmd()?;

    let mut args = Vec::new();
    args.push("validate".to_string());
    args.extend(config_args(config));
    args.extend(extra_args.iter().cloned());

    run_promptfoo(&program, &base_args, cwd, &args, false)
}

#[allow(clippy::too_many_arguments)]
fn cmd_eval(
    config: &[PathBuf],
    cwd: Option<&Path>,
    output: &[PathBuf],
    max_concurrency: Option<u32>,
    repeat: Option<u32>,
    delay: Option<u64>,
    no_cache: bool,
    dry_run: bool,
    extra_args: &[String],
) -> Result<()> {
    let (program, base_args) = resolve_promptfoo_cmd()?;

    let mut args = Vec::new();
    args.push("eval".to_string());
    args.extend(config_args(config));
    for path in output {
        args.push("--output".to_string());
        args.push(path.display().to_string());
    }
    if let Some(value) = max_concurrency {
        args.push("--max-concurrency".to_string());
        args.push(value.to_string());
    }
    if let Some(value) = repeat {
        args.push("--repeat".to_string());
        args.push(value.to_string());
    }
    if let Some(value) = delay {
        args.push("--delay".to_string());
        args.push(value.to_string());
    }
    if no_cache {
        args.push("--no-cache".to_string());
    }
    args.extend(extra_args.iter().cloned());

    run_promptfoo(&program, &base_args, cwd, &args, dry_run)
}

fn cmd_view(config: &[PathBuf], cwd: Option<&Path>, extra_args: &[String]) -> Result<()> {
    let (program, base_args) = resolve_promptfoo_cmd()?;

    let mut args = Vec::new();
    args.push("view".to_string());
    args.extend(config_args(config));
    args.extend(extra_args.iter().cloned());

    run_promptfoo(&program, &base_args, cwd, &args, false)
}

fn run_promptfoo(
    program: &str,
    base_args: &[String],
    cwd: Option<&Path>,
    args: &[String],
    dry_run: bool,
) -> Result<()> {
    if dry_run {
        let mut argv = Vec::new();
        argv.push(program.to_string());
        argv.extend(base_args.iter().cloned());
        argv.extend(args.iter().cloned());

        let cwd_s = cwd.map(|p| p.display().to_string());
        let payload = serde_json::json!({
            "dry_run": true,
            "cwd": cwd_s,
            "argv": argv,
        });
        println!("{}", serde_json::to_string_pretty(&payload).expect("json"));
        return Ok(());
    }

    let mut cmd = Command::new(program);
    cmd.args(base_args).args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    let status = cmd.status().map_err(|e| {
        if e.kind() == io::ErrorKind::NotFound {
            return anyhow!(not_found_help(program));
        }
        anyhow!(e).context(format!("run `{}`", display_cmd(program, base_args)))
    })?;

    if status.success() {
        return Ok(());
    }

    bail!("promptfoo exited with status: {status}");
}

fn resolve_promptfoo_cmd() -> Result<(String, Vec<String>)> {
    let raw = env::var(ENV_PROMPTFOO_CMD).ok().unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(("promptfoo".to_string(), Vec::new()));
    }

    let parts =
        split_shell_args(&raw).map_err(|e| anyhow!("Invalid {ENV_PROMPTFOO_CMD} value: {e}"))?;
    if parts.is_empty() {
        bail!("Invalid {ENV_PROMPTFOO_CMD}: empty command");
    }
    let (program, args) = parts.split_first().expect("non-empty");
    Ok((program.to_string(), args.to_vec()))
}

fn config_args(config: &[PathBuf]) -> Vec<String> {
    let mut out = Vec::new();
    for path in config {
        out.push("--config".to_string());
        out.push(path.display().to_string());
    }
    out
}

fn not_found_help(program: &str) -> String {
    format!(
        "Promptfoo runner not found: `{}`.\n\nInstall promptfoo (e.g. `npm install -g promptfoo` or `brew install promptfoo`) or set `{}` (e.g. `npx promptfoo@latest`).",
        program, ENV_PROMPTFOO_CMD
    )
}

fn display_cmd(program: &str, base_args: &[String]) -> String {
    if base_args.is_empty() {
        return program.to_string();
    }
    format!(
        "{} {}",
        program,
        base_args
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    )
}

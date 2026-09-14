//! External subcommand delegation (cli.feature r56): `llman <name>` with no
//! built-in match resolves `llman-<name>` on `PATH` and runs it as a child
//! process. Language-agnostic forwarding contract: argv verbatim, env
//! inherited (+ one normalization — hub-resolved config dir injected as
//! `LLMAN_CONFIG_DIR` when `-C/--config-dir` was given), stdio/cwd
//! inherited, exit code propagated (`128+signum` on Unix signal death).
//!
//! The `RequiresGlobalConfig` guard deliberately does not apply here:
//! plugins resolve their own configuration.

use crate::config::{ENV_CONFIG_DIR, resolve_config_dir_with};
use anyhow::{Result, anyhow};
use std::process::Command;

pub fn run(name_and_args: &[String], cli_config_dir: Option<&std::path::Path>) -> Result<()> {
    let Some(name) = name_and_args.first() else {
        return Ok(());
    };
    let rest: Vec<&String> = name_and_args[1..].iter().collect();

    let Some(executable) = llman_core::ext_subcommand::resolve(name) else {
        return Err(anyhow!("{}", not_found_message(name)));
    };

    let mut cmd = Command::new(&executable);
    cmd.args(rest);
    if let Some(dir) = cli_config_dir {
        let resolved = resolve_config_dir_with(Some(dir), None)?;
        cmd.env(ENV_CONFIG_DIR, resolved);
    }

    let mut child = cmd.spawn().map_err(|e| {
        anyhow!(
            "{}: {}",
            t!(
                "ext_subcommand.spawn_failed",
                path = executable.display().to_string()
            ),
            e
        )
    })?;
    let status = child.wait()?;

    let code = exit_code_of(&status);
    std::process::exit(code);
}

fn exit_code_of(status: &std::process::ExitStatus) -> i32 {
    status.code().unwrap_or_else(|| {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            status.signal().map(|s| 128 + s).unwrap_or(1)
        }
        #[cfg(not(unix))]
        {
            1
        }
    })
}

fn not_found_message(name: &str) -> String {
    let mut message = t!("ext_subcommand.not_found", name = name).to_string();
    let discovered = llman_core::ext_subcommand::discover();
    if !discovered.is_empty() {
        let list = discovered
            .iter()
            .map(|n| format!("llman-{n}"))
            .collect::<Vec<_>>()
            .join(", ");
        message.push('\n');
        message.push_str(&t!("ext_subcommand.hint", list = list));
    }
    message
}

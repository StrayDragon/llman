use crate::arg_utils::split_shell_args;
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use rust_i18n::t;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::config::{ConfigManager, Metadata};
use super::interactive;

#[derive(Args)]
#[command(
    author,
    version,
    about,
    long_about = "Commands for managing OpenAI Codex configurations"
)]
pub struct CodexArgs {
    #[command(subcommand)]
    pub command: Option<CodexCommands>,

    /// Arguments to pass to codex (when no subcommand)
    #[arg(last = true)]
    pub args: Vec<String>,
}

#[derive(Subcommand)]
pub enum CodexCommands {
    /// Manage Codex configuration groups
    Account {
        #[command(subcommand)]
        command: Option<AccountCommands>,
    },
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// List all configuration groups
    List,
    /// Use a specific group
    Use {
        /// Group name
        name: String,
    },
    /// Create a new group
    Create {
        /// Group name
        name: String,
        /// Template provider (openai, minimax, rightcode, custom)
        #[arg(short, long)]
        template: Option<String>,
    },
    /// Edit a group's configuration
    Edit {
        /// Group name
        name: String,
    },
    /// Import a group from existing codex config
    Import {
        /// Group name
        name: String,
        /// Path to codex config file
        path: PathBuf,
    },
    /// Delete a group
    #[command(alias = "rm")]
    Delete {
        /// Group name
        name: String,
    },
}

pub fn run(args: &CodexArgs) -> Result<()> {
    match &args.command {
        None => {
            // Interactive mode: select group and execute codex
            handle_interactive_execution(&args.args)?;
        }
        Some(CodexCommands::Account { command }) => match command {
            None => {
                // Default: list groups
                handle_account_list()?;
            }
            Some(AccountCommands::List) => {
                handle_account_list()?;
            }
            Some(AccountCommands::Use { name }) => {
                handle_account_use(name)?;
            }
            Some(AccountCommands::Create { name, template }) => {
                handle_account_create(name, template.as_deref())?;
            }
            Some(AccountCommands::Edit { name }) => {
                handle_account_edit(name)?;
            }
            Some(AccountCommands::Import { name, path }) => {
                handle_account_import(name, path)?;
            }
            Some(AccountCommands::Delete { name }) => {
                handle_account_delete(name)?;
            }
        },
    }

    Ok(())
}

fn select_editor_raw() -> String {
    std::env::var("VISUAL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("EDITOR")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| "vi".to_string())
}

fn parse_editor_command(raw: &str) -> Result<(String, Vec<String>)> {
    let parts = split_shell_args(raw).map_err(|e| {
        anyhow::anyhow!(t!(
            "codex.error.invalid_editor_command",
            editor = raw,
            error = e
        ))
    })?;
    match parts.split_first() {
        Some((cmd, args)) if !cmd.trim().is_empty() => Ok((cmd.clone(), args.to_vec())),
        _ => Ok(("vi".to_string(), Vec::new())),
    }
}

/// Handle interactive group selection and execution (llman x codex)
fn handle_interactive_execution(args: &[String]) -> Result<()> {
    let groups = ConfigManager::get_group_names()?;

    if groups.is_empty() {
        bail!(
            "{}\n{}",
            t!("codex.account.no_groups"),
            t!("codex.account.create_hint")
        );
    }

    // Select group
    let group_name = interactive::select_group(&groups)?;

    // Switch to group
    ConfigManager::switch_group(&group_name)?;

    println!("{}", t!("codex.account.switched", name = &group_name));

    // Execute codex
    execute_codex(args)?;

    Ok(())
}

/// Handle account list (llman x codex account list)
fn handle_account_list() -> Result<()> {
    let groups = ConfigManager::get_group_names()?;

    if groups.is_empty() {
        println!("{}", t!("codex.account.no_groups"));
        println!("{}", t!("codex.account.create_hint"));
        return Ok(());
    }

    let metadata = Metadata::load()?;

    println!("{}", t!("codex.account.list_header"));
    println!();

    for name in groups {
        let is_current = metadata.current_group.as_ref() == Some(&name);
        if is_current {
            println!("  {}{}", name, t!("codex.account.current_marker"));
        } else {
            println!("  {}", name);
        }
    }

    println!();

    if let Some(ref current) = metadata.current_group {
        println!("{}", t!("codex.account.current_group", name = current));
    }

    Ok(())
}

/// Handle account use (llman x codex account use \<name\>)
fn handle_account_use(name: &str) -> Result<()> {
    if !ConfigManager::group_exists(name)? {
        bail!("{}", t!("codex.error.group_not_found", name = name));
    }

    ConfigManager::switch_group(name)?;

    println!("{}", t!("codex.account.switched", name = name));

    Ok(())
}

/// Handle account create (llman x codex account create \<name\>)
fn handle_account_create(name: &str, template: Option<&str>) -> Result<()> {
    if ConfigManager::group_exists(name)? {
        bail!("{}", t!("codex.error.group_exists", name = name));
    }

    let template_key = if let Some(t) = template {
        t
    } else {
        // Interactive template selection
        interactive::select_template()?
    };

    let template_content = ConfigManager::get_template(template_key);

    ConfigManager::create_group(name, template_content)?;

    println!("{}", t!("codex.account.created", name = name));
    println!("{}", t!("codex.account.edit_hint", name = name));

    Ok(())
}

/// Handle account edit (llman x codex account edit \<name\>)
fn handle_account_edit(name: &str) -> Result<()> {
    if !ConfigManager::group_exists(name)? {
        bail!("{}", t!("codex.error.group_not_found", name = name));
    }

    let group_path = ConfigManager::group_path(name)?;

    let editor_raw = select_editor_raw();
    let (editor_cmd, editor_args) = parse_editor_command(&editor_raw)?;

    // Open editor
    let status = Command::new(&editor_cmd)
        .args(editor_args)
        .arg(&group_path)
        .status()
        .context(t!("codex.error.open_editor_failed", editor = editor_raw))?;

    if !status.success() {
        bail!("{}", t!("codex.error.editor_exit_status", status = status));
    }

    println!("{}", t!("codex.account.edited", name = name));

    Ok(())
}

/// Handle account import (llman x codex account import \<name\> \<path\>)
fn handle_account_import(name: &str, path: &Path) -> Result<()> {
    if ConfigManager::group_exists(name)? {
        bail!("{}", t!("codex.error.group_exists", name = name));
    }

    if !path.exists() {
        bail!(
            "{}",
            t!("codex.error.file_not_found", path = path.display())
        );
    }

    ConfigManager::import_group(name, path)?;

    println!("{}", t!("codex.account.imported", name = name));

    Ok(())
}

/// Handle account delete (llman x codex account delete \<name\>)
fn handle_account_delete(name: &str) -> Result<()> {
    if !ConfigManager::group_exists(name)? {
        bail!("{}", t!("codex.error.group_not_found", name = name));
    }

    // Confirm deletion
    if !interactive::confirm_delete(name)? {
        println!("{}", t!("codex.account.delete_cancelled"));
        return Ok(());
    }

    ConfigManager::delete_group(name)?;

    // Clear current_group if it was deleted
    let mut metadata = Metadata::load()?;
    if metadata.current_group.as_ref() == Some(&name.to_string()) {
        metadata.current_group = None;
        metadata.save()?;
    }

    println!("{}", t!("codex.account.deleted", name = name));

    Ok(())
}

/// Execute codex with arguments
fn execute_codex(args: &[String]) -> Result<()> {
    // Check if codex is available
    if !is_codex_available() {
        bail!("{}", t!("codex.error.codex_not_found"));
    }

    // Build command
    let mut cmd = Command::new("codex");

    for arg in args {
        cmd.arg(arg);
    }

    // Execute
    let status = cmd.status().context(t!("codex.error.execute_failed"))?;

    if !status.success() {
        bail!("{}", t!("codex.error.exit_status", status = status));
    }

    Ok(())
}

/// Check if codex CLI is available
fn is_codex_available() -> bool {
    Command::new("codex")
        .arg("--version")
        .output()
        .ok()
        .is_some_and(|output| output.status.success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ENV_CONFIG_DIR;
    use crate::test_utils::ENV_MUTEX;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn editor_parsing_supports_args_and_quotes() {
        let (cmd, args) = parse_editor_command("code --wait").expect("parse");
        assert_eq!(cmd, "code");
        assert_eq!(args, vec!["--wait"]);

        let (cmd, args) = parse_editor_command("\"/path with spaces/code\" --wait").expect("parse");
        assert_eq!(cmd, "/path with spaces/code");
        assert_eq!(args, vec!["--wait"]);

        let (cmd, args) = parse_editor_command("   ").expect("parse");
        assert_eq!(cmd, "vi");
        assert!(args.is_empty());
    }

    #[test]
    fn editor_env_prefers_visual_over_editor_and_falls_back_to_vi() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        unsafe {
            env::set_var("VISUAL", "code --wait");
            env::set_var("EDITOR", "vim");
        }
        assert_eq!(select_editor_raw(), "code --wait");

        unsafe {
            env::remove_var("VISUAL");
            env::set_var("EDITOR", "  ");
        }
        assert_eq!(select_editor_raw(), "vi");

        unsafe {
            env::remove_var("EDITOR");
        }
    }

    #[cfg(unix)]
    #[test]
    fn editor_non_zero_exit_status_is_propagated() {
        use std::os::unix::fs::PermissionsExt;

        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let temp = TempDir::new().expect("temp dir");
        unsafe {
            env::set_var(ENV_CONFIG_DIR, temp.path());
        }

        // Create a dummy group so handle_account_edit can proceed.
        let group_path = ConfigManager::group_path("demo").expect("group path");
        if let Some(parent) = group_path.parent() {
            fs::create_dir_all(parent).expect("create parent");
        }
        fs::write(&group_path, "key = \"value\"").expect("write group");

        // Create an "editor" that exits non-zero.
        let editor_path = temp.path().join("fail-editor.sh");
        fs::write(&editor_path, "#!/bin/sh\nexit 42\n").expect("write editor");
        let mut perms = fs::metadata(&editor_path).expect("meta").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&editor_path, perms).expect("chmod");

        unsafe {
            env::remove_var("VISUAL");
            env::set_var("EDITOR", editor_path.to_string_lossy().to_string());
        }

        let err = handle_account_edit("demo").expect_err("should error");
        assert!(err.to_string().contains("Editor exited with status"));

        unsafe {
            env::remove_var(ENV_CONFIG_DIR);
            env::remove_var("EDITOR");
        }
    }
}

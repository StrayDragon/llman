use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
use rust_i18n::t;
use std::path::PathBuf;
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

/// Handle interactive group selection and execution (llman x codex)
fn handle_interactive_execution(args: &[String]) -> Result<()> {
    let groups = ConfigManager::list_groups()?;

    if groups.is_empty() {
        println!("{}", t!("codex.account.no_groups"));
        println!("{}", t!("codex.account.create_hint"));
        return Ok(());
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
    let groups = ConfigManager::list_groups()?;

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
        let marker = if is_current { " *" } else { "" };

        println!("  {}{}", name, marker);
    }

    println!();

    if let Some(ref current) = metadata.current_group {
        println!("{}", t!("codex.account.current_group", name = current));
    }

    Ok(())
}

/// Handle account use (llman x codex account use <name>)
fn handle_account_use(name: &str) -> Result<()> {
    if !ConfigManager::group_exists(name)? {
        bail!("{}", t!("codex.error.group_not_found", name = name));
    }

    ConfigManager::switch_group(name)?;

    println!("{}", t!("codex.account.switched", name = name));

    Ok(())
}

/// Handle account create (llman x codex account create <name>)
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

/// Handle account edit (llman x codex account edit <name>)
fn handle_account_edit(name: &str) -> Result<()> {
    if !ConfigManager::group_exists(name)? {
        bail!("{}", t!("codex.error.group_not_found", name = name));
    }

    let group_path = ConfigManager::group_path(name)?;

    // Get editor from environment
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    // Open editor
    let status = Command::new(&editor)
        .arg(&group_path)
        .status()
        .context("Failed to open editor")?;

    if !status.success() {
        bail!("Editor exited with status: {}", status);
    }

    println!("{}", t!("codex.account.edited", name = name));

    Ok(())
}

/// Handle account import (llman x codex account import <name> <path>)
fn handle_account_import(name: &str, path: &PathBuf) -> Result<()> {
    if ConfigManager::group_exists(name)? {
        bail!("{}", t!("codex.error.group_exists", name = name));
    }

    if !path.exists() {
        bail!("File not found: {}", path.display());
    }

    ConfigManager::import_group(name, path)?;

    println!("{}", t!("codex.account.imported", name = name));

    Ok(())
}

/// Handle account delete (llman x codex account delete <name>)
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
    let status = cmd.status().context("Failed to execute codex")?;

    if !status.success() {
        bail!("Codex exited with status: {}", status);
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

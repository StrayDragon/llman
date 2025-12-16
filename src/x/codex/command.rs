use crate::x::codex::config::{CodexManager, ConfigStatus};
use crate::x::codex::interactive;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use rust_i18n::t;
use std::process::Command;

#[derive(Args)]
#[command(
    author,
    version,
    about,
    long_about = "Commands for managing OpenAI Codex configurations with symlink-based approach"
)]
pub struct CodexArgs {
    #[command(subcommand)]
    pub command: Option<CodexCommands>,
}

#[derive(Subcommand)]
pub enum CodexCommands {
    /// Initialize configuration management environment
    Init,
    /// List all available configurations
    List,
    /// Create a new configuration interactively
    Create {
        /// Configuration name
        name: String,
        /// Use a template (openai, ollama, minimal)
        #[arg(short, long, help = "Template to use for configuration")]
        template: Option<String>,
    },
    /// Edit a configuration
    Edit {
        /// Configuration name (defaults to current)
        #[arg(help = "Configuration name to edit (defaults to current)")]
        name: Option<String>,
    },
    /// Delete a configuration
    Delete {
        /// Configuration name
        #[arg(help = "Configuration name to delete")]
        name: String,
    },
    /// Switch to a specific configuration
    Use {
        /// Configuration name
        #[arg(help = "Configuration name to switch to")]
        name: String,
    },
    /// Show current configuration information
    Show,
    /// Run codex with current configuration
    Run {
        /// Arguments to pass to codex
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Arguments to pass to codex command"
        )]
        args: Vec<String>,
    },
}

pub fn run(args: &CodexArgs) -> Result<()> {
    let manager = CodexManager::new()
        .context("Failed to initialize CodexManager")?;

    match &args.command {
        Some(CodexCommands::Init) => {
            handle_init_command(&manager)?;
        }
        Some(CodexCommands::List) => {
            handle_list_command(&manager)?;
        }
        Some(CodexCommands::Create { name, template }) => {
            handle_create_command(&manager, name, template.as_deref())?;
        }
        Some(CodexCommands::Edit { name }) => {
            handle_edit_command(&manager, name.as_deref())?;
        }
        Some(CodexCommands::Delete { name }) => {
            handle_delete_command(&manager, name)?;
        }
        Some(CodexCommands::Use { name }) => {
            handle_use_command(&manager, name)?;
        }
        Some(CodexCommands::Show) => {
            handle_show_command(&manager)?;
        }
        Some(CodexCommands::Run { args }) => {
            handle_run_command(&manager, args.clone())?;
        }
        None => {
            // Default behavior: show status or interactive mode
            handle_default_command(&manager)?;
        }
    }

    Ok(())
}

fn handle_init_command(manager: &CodexManager) -> Result<()> {
    let status = manager.init_or_detect()
        .context("Failed to initialize configuration management")?;

    match status {
        ConfigStatus::SymlinkActive => {
            println!("✅ {}", t!("codex.init.symlink_active"));
        }
        ConfigStatus::Imported => {
            println!("✅ {}", t!("codex.init.imported"));
        }
        ConfigStatus::Created => {
            println!("✅ {}", t!("codex.init.created"));
        }
        ConfigStatus::Migrated => {
            println!("✅ {}", t!("codex.init.migrated"));
        }
    }

    Ok(())
}

fn handle_list_command(manager: &CodexManager) -> Result<()> {
    // Ensure configuration is initialized
    manager.init_or_detect()
        .context("Failed to initialize configuration management")?;

    let configs = manager.list_configs()
        .context("Failed to list configurations")?;

    if configs.is_empty() {
        println!("📭 {}", t!("codex.list.no_configs"));
        println!("💡 {}", t!("codex.list.suggestion_create"));
    } else {
        println!("📋 {}:", t!("codex.list.available_configs"));

        let current_config = manager.get_current_config()
            .context("Failed to get current configuration")?;

        for config_name in configs {
            let marker = if let Some(current) = &current_config {
                if current == &config_name {
                    "→"
                } else {
                    " "
                }
            } else {
                " "
            };

            println!("  {} {}", marker, config_name);
        }

        if let Some(current) = current_config {
            println!();
            println!("📍 {}: {}", t!("codex.list.current_config"), current);
        }
    }

    Ok(())
}

fn handle_create_command(manager: &CodexManager, name: &str, template: Option<&str>) -> Result<()> {
    // Ensure configuration is initialized
    manager.init_or_detect()
        .context("Failed to initialize configuration management")?;

    // Validate configuration name
    if name.is_empty() {
        eprintln!("❌ {}", t!("codex.create.error.empty_name"));
        return Ok(());
    }

    // Check if configuration already exists
    let configs = manager.list_configs()
        .context("Failed to list configurations")?;
    if configs.contains(&name.to_string()) {
        eprintln!("❌ {}", t!("codex.create.error.exists", name = name));
        return Ok(());
    }

    // Determine template
    let template_name = if let Some(t) = template {
        t.to_string()
    } else {
        // Interactive template selection
        interactive::select_template()?
    };

    // Create configuration
    let config_path = manager.create_config(name, Some(&template_name))
        .with_context(|| format!("Failed to create configuration '{}'", name))?;

    println!("✅ {}", t!("codex.create.success",
        name = name,
        path = config_path.display()
    ));
    println!("💡 {}", t!("codex.create.suggestion_use", name = name));

    Ok(())
}

fn handle_edit_command(manager: &CodexManager, name: Option<&str>) -> Result<()> {
    // Ensure configuration is initialized
    manager.init_or_detect()
        .context("Failed to initialize configuration management")?;

    let config_name = if let Some(n) = name {
        n.to_string()
    } else {
        // Use current configuration or prompt for selection
        if let Some(current) = manager.get_current_config()
            .context("Failed to get current configuration")?
        {
            current
        } else {
            interactive::select_config_to_edit(manager)?
        }
    };

    // Get configuration path
    let configs = manager.list_configs()
        .context("Failed to list configurations")?;

    if !configs.contains(&config_name) {
        eprintln!("❌ {}", t!("codex.edit.error.not_found", name = config_name));
        return Ok(());
    }

    let config_path = manager.configs_dir().join(format!("{}.toml", config_name));

    // Open in default editor
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor)
        .arg(&config_path)
        .status()
        .with_context(|| format!("Failed to open editor '{}' for configuration", editor))?;

    if status.success() {
        println!("✅ {}", t!("codex.edit.success", name = config_name));
    } else {
        eprintln!("❌ {}", t!("codex.edit.error.editor_failed"));
    }

    Ok(())
}

fn handle_delete_command(manager: &CodexManager, name: &str) -> Result<()> {
    // Ensure configuration is initialized
    manager.init_or_detect()
        .context("Failed to initialize configuration management")?;

    // Safety confirmation
    let confirm = interactive::confirm_delete(name)?;
    if !confirm {
        println!("❌ {}", t!("codex.delete.cancelled"));
        return Ok(());
    }

    // Delete configuration
    manager.delete_config(name)
        .with_context(|| format!("Failed to delete configuration '{}'", name))?;

    Ok(())
}

fn handle_use_command(manager: &CodexManager, name: &str) -> Result<()> {
    // Ensure configuration is initialized
    manager.init_or_detect()
        .context("Failed to initialize configuration management")?;

    manager.use_config(name)
        .with_context(|| format!("Failed to switch to configuration '{}'", name))?;

    Ok(())
}

fn handle_show_command(manager: &CodexManager) -> Result<()> {
    // Ensure configuration is initialized
    manager.init_or_detect()
        .context("Failed to initialize configuration management")?;

    let current_config = manager.get_current_config()
        .context("Failed to get current configuration")?;

    if let Some(name) = current_config {
        println!("📍 {}: {}", t!("codex.show.current_config"), name);

        // Show configuration file path
        let config_path = manager.configs_dir().join(format!("{}.toml", name));
        println!("📁 {}: {}", t!("codex.show.config_path"), config_path.display());

        // Show active config path
        println!("🔗 {}: {}", t!("codex.show.active_link"), manager.active_config_path().display());

        // Show configuration content (if it exists and is readable)
        if config_path.exists() {
            println!();
            println!("📄 {}:", t!("codex.show.config_content"));

            let content = std::fs::read_to_string(&config_path)
                .context("Failed to read configuration file")?;

            // Show non-sensitive content only
            for line in content.lines() {
                if !line.to_lowercase().contains("key") &&
                   !line.to_lowercase().contains("token") &&
                   !line.to_lowercase().contains("secret") {
                    println!("  {}", line);
                } else if line.contains("=") {
                    // Mask sensitive values
                    let parts: Vec<&str> = line.splitn(2, '=').collect();
                    if parts.len() == 2 {
                        println!("  {}=***MASKED***", parts[0]);
                    }
                }
            }
        }
    } else {
        println!("❌ {}", t!("codex.show.no_active_config"));
        println!("💡 {}", t!("codex.show.suggestion_use"));
    }

    Ok(())
}

fn handle_run_command(manager: &CodexManager, args: Vec<String>) -> Result<()> {
    // Ensure configuration is initialized
    manager.init_or_detect()
        .context("Failed to initialize configuration management")?;

    let current_config = manager.get_current_config()
        .context("Failed to get current configuration")?;

    if current_config.is_none() {
        eprintln!("❌ {}", t!("codex.run.no_active_config"));
        println!("💡 {}", t!("codex.run.suggestion_use"));
        return Ok(());
    }

    // Check if codex command exists
    let codex_check = Command::new("codex")
        .arg("--version")
        .output();

    match codex_check {
        Ok(output) => {
            if !output.status.success() {
                eprintln!("❌ {}", t!("codex.run.codex_not_found"));
                return Ok(());
            }
        }
        Err(_) => {
            eprintln!("❌ {}", t!("codex.run.codex_not_found"));
            return Ok(());
        }
    }

    // Run codex with the current configuration
    println!("🚀 {}", t!("codex.run.starting",
        config = current_config.unwrap()
    ));

    let mut cmd = Command::new("codex");
    for arg in args {
        cmd.arg(arg);
    }

    let status = cmd.status()
        .context("Failed to execute codex command")?;

    if !status.success() {
        eprintln!("❌ {}", t!("codex.run.failed"));
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn handle_default_command(manager: &CodexManager) -> Result<()> {
    let status = manager.init_or_detect()
        .context("Failed to initialize configuration management")?;

    match status {
        ConfigStatus::SymlinkActive => {
            // Show current status
            if let Some(current) = manager.get_current_config()
                .context("Failed to get current configuration")?
            {
                println!("📋 {}: {}", t!("codex.default.current_config"), current);
                println!();
                println!("💡 {}:", t!("codex.default.available_commands"));
                println!("  llman x codex list        - {}", t!("codex.default.cmd_list"));
                println!("  llman x codex create <name> - {}", t!("codex.default.cmd_create"));
                println!("  llman x codex use <name>   - {}", t!("codex.default.cmd_use"));
                println!("  llman x codex show        - {}", t!("codex.default.cmd_show"));
                println!("  llman x codex run -- [args] - {}", t!("codex.default.cmd_run"));
            } else {
                println!("❌ {}", t!("codex.default.no_active_config"));
                println!("💡 {}", t!("codex.default.suggestion_init"));
            }
        }
        _ => {
            println!("✅ {}", t!("codex.default.initialized"));
            println!("💡 {}", t!("codex.default.suggestion_status"));
        }
    }

    Ok(())
}

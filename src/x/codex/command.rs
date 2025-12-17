use crate::x::codex::config::CodexConfigManager;
use crate::x::codex::interactive;
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::process::Command;

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
}

#[derive(Subcommand)]
pub enum CodexCommands {
    /// Account management commands for handling Codex profiles
    #[command(alias = "a")]
    Account {
        #[arg(short = 'i', long, help = "Interactive account management mode")]
        interactive: bool,
    },
    /// Run codex command (simple wrapper)
    Run {
        /// Arguments to pass to codex (use -- to separate from run options)
        #[arg(
            trailing_var_arg = true,
            allow_hyphen_values = true,
            help = "Arguments to pass to codex (use -- to separate from run options)"
        )]
        args: Vec<String>,
    },
}

// No subcommands - only interactive mode available

pub fn run(args: &CodexArgs) -> Result<()> {
    let manager = CodexConfigManager::new().context("Failed to initialize CodexConfigManager")?;

    match &args.command {
        Some(CodexCommands::Account { interactive }) => {
            if *interactive {
                // Interactive management mode
                handle_account_management(&manager)?;
            } else {
                // Show help
                show_account_help();
            }
        }
        Some(CodexCommands::Run { args: codex_args }) => {
            handle_run_command(codex_args)?;
        }
        None => {
            // Default to profile selection mode
            handle_interactive_account_command(&manager)?;
        }
    }

    Ok(())
}

fn show_account_help() {
    println!("{}", rust_i18n::t!("codex.account.help_title"));
    println!();
    println!("{}", rust_i18n::t!("codex.account.usage"));
    println!("{}", rust_i18n::t!("codex.account.commands"));
    println!();
    println!("{}", rust_i18n::t!("codex.account.examples"));
    println!(
        "  llman x codex account -i        # {}",
        rust_i18n::t!("codex.account.actions.list_profiles")
    );
    println!(
        "  llman x codex                   # {}",
        rust_i18n::t!("codex.account.profile_help")
    );
}

fn handle_account_management(manager: &CodexConfigManager) -> Result<()> {
    // Initialize if needed
    if !manager.config_file_path().exists() {
        manager.initialize()?;
        manager.export()?;
    }

    println!("{}", rust_i18n::t!("codex.account.management_title"));
    println!();

    use inquire::Select;

    let choices = vec![
        rust_i18n::t!("codex.account.actions.upsert_profile"),
        rust_i18n::t!("codex.account.actions.list_profiles"),
        rust_i18n::t!("codex.account.actions.edit_configuration"),
        rust_i18n::t!("codex.account.actions.remove_profile"),
        rust_i18n::t!("codex.account.actions.exit"),
    ];

    let choice = Select::new(&rust_i18n::t!("codex.account.select_action"), choices)
        .with_help_message(&rust_i18n::t!("codex.account.action_help"))
        .prompt();

    let choice = match choice {
        Ok(c) => c,
        Err(_) => {
            println!("{}", rust_i18n::t!("codex.account.failed_select_action"));
            return Ok(());
        }
    };

    if choice == rust_i18n::t!("codex.account.actions.upsert_profile") {
        handle_upsert_profile(manager)?;
    } else if choice == rust_i18n::t!("codex.account.actions.list_profiles") {
        let profiles = manager.list_profiles().context("Failed to list profiles")?;
        if profiles.is_empty() {
            println!("{}", rust_i18n::t!("codex.account.no_profiles_found"));
        } else {
            println!("{}", rust_i18n::t!("codex.account.available_profiles"));
            for profile in profiles {
                println!("  • {}", profile);
            }
        }
    } else if choice == rust_i18n::t!("codex.account.actions.edit_configuration") {
        let config_path = manager.config_file_path();
        println!(
            "{}",
            rust_i18n::t!(
                "codex.account.opening_config_file",
                path = config_path.display()
            )
        );

        let editor_cmd = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

        // Validate editor command to prevent injection
        let editor_parts: Vec<&str> = editor_cmd.split_whitespace().collect();
        if editor_parts.is_empty() {
            eprintln!("❌ Invalid editor command");
            return Ok(());
        }

        let editor_binary = editor_parts[0];
        let mut cmd = Command::new(editor_binary);

        // Add additional arguments if any
        for arg in &editor_parts[1..] {
            cmd.arg(arg);
        }
        cmd.arg(config_path);

        let status = cmd
            .status()
            .with_context(|| format!("Failed to open editor '{}' for configuration", editor_cmd))?;

        if status.success() {
            manager.export()?;
            println!("{}", rust_i18n::t!("codex.account.config_updated"));
        } else {
            eprintln!("{}", rust_i18n::t!("codex.account.editor_failed"));
        }
    } else if choice == rust_i18n::t!("codex.account.actions.remove_profile") {
        let profiles = manager.list_profiles().context("Failed to list profiles")?;
        if profiles.is_empty() {
            println!("{}", rust_i18n::t!("codex.account.no_profiles_to_remove"));
            return Ok(());
        }

        let profile_name = Select::new("Select profile to remove", profiles)
            .prompt()
            .context("Failed to select profile")?;

        if interactive::confirm_delete(&profile_name)? {
            manager.remove_profile(&profile_name)?;
            manager.export()?;
        }
    } else if choice == rust_i18n::t!("codex.account.actions.exit") {
        println!("{}", rust_i18n::t!("codex.account.goodbye"));
    }

    Ok(())
}

fn handle_upsert_profile(manager: &CodexConfigManager) -> Result<()> {
    println!("{}", rust_i18n::t!("codex.account.upsert_title"));
    println!();

    use inquire::Select;

    // Template selection (secure/insecure)
    let templates = [
        (
            "development",
            rust_i18n::t!("codex.account.templates.development"),
        ),
        (
            "production",
            rust_i18n::t!("codex.account.templates.production"),
        ),
    ];

    let template_names = templates.iter().map(|(name, _)| *name).collect();
    let template = Select::new(
        &rust_i18n::t!("codex.account.select_security_template"),
        template_names,
    )
    .with_help_message(&rust_i18n::t!("codex.account.template_help"))
    .prompt()
    .context("Failed to select template")?;

    // Profile name
    let name = interactive::prompt_profile_name().context("Failed to input profile name")?;

    // Create the profile
    manager.create_profile_from_template(&name, template)?;
    manager.export()?;

    println!(
        "{}",
        rust_i18n::t!("codex.account.profile_created", name = name)
    );
    println!("{}", rust_i18n::t!("codex.account.profile_usage_hint"));

    Ok(())
}

fn handle_interactive_account_command(manager: &CodexConfigManager) -> Result<()> {
    // Only initialize if configuration doesn't exist
    if !manager.config_file_path().exists() {
        manager.initialize()?;
        manager.export()?;
    }

    // Default behavior: show interactive profile selection
    let profiles = manager.list_profiles().context("Failed to list profiles")?;

    if profiles.is_empty() {
        println!("{}", rust_i18n::t!("codex.account.no_profiles_configured"));
        println!("{}", rust_i18n::t!("codex.account.use_account_import"));
        return Ok(());
    }

    // Check if codex CLI is available
    if !Command::new("codex")
        .arg("--version")
        .output()
        .ok()
        .is_some_and(|output| output.status.success())
    {
        eprintln!("❌ codex command not found. Please install OpenAI Codex CLI:");
        eprintln!("   npm install -g @openai/codex");
        return Ok(());
    }

    println!("{}", rust_i18n::t!("codex.account.profile_selection_title"));

    use inquire::Select;

    let choice = Select::new(
        &rust_i18n::t!("codex.account.choose_profile"),
        profiles.clone(),
    )
    .with_help_message(&rust_i18n::t!("codex.account.profile_help"))
    .prompt();

    let choice = match choice {
        Ok(c) => c,
        Err(_) => {
            println!("{}", rust_i18n::t!("codex.account.failed_select_profile"));
            return Ok(());
        }
    };

    // Execute codex with selected profile (interactive mode)
    println!(
        "{}",
        rust_i18n::t!("codex.account.running_codex", profile = choice)
    );

    let mut cmd = Command::new("codex");
    cmd.arg("--profile").arg(&choice);

    let status = cmd.status().context("Failed to execute codex command")?;

    if !status.success() {
        eprintln!("❌ {}", rust_i18n::t!("codex.error.failed_codex_command"));
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn handle_run_command(args: &[String]) -> Result<()> {
    if args.is_empty() {
        // Check if codex command exists
        let codex_check = Command::new("codex").arg("--version").output();

        match codex_check {
            Ok(output) => {
                if !output.status.success() {
                    eprintln!("❌ Codex CLI not found. Please install it first:");
                    eprintln!("   npm install -g @openai/codex");
                    return Ok(());
                }
            }
            Err(_) => {
                eprintln!("❌ Codex CLI not found. Please install it first:");
                eprintln!("   npm install -g @openai/codex");
                return Ok(());
            }
        }

        println!("💡 Codex CLI usage examples:");
        println!("   codex \"your request\"");
        println!("   codex --profile dev \"development task\"");
        println!("   codex --model gpt-4 \"use specific model\"");
        println!();
        println!("💡 Available profiles:");
        let manager = CodexConfigManager::new()?;
        let profiles = manager.list_profiles().context("Failed to list profiles")?;
        if profiles.is_empty() {
            println!("   No profiles configured. Use: llman x codex account import");
        } else {
            for profile in profiles {
                println!("   • {}", profile);
            }
        }
        return Ok(());
    }

    // Run codex with provided arguments
    let mut cmd = Command::new("codex");
    for arg in args {
        cmd.arg(arg);
    }

    println!("🚀 Running: codex {}", args.join(" "));

    let status = cmd.status().context("Failed to execute codex command")?;

    if !status.success() {
        eprintln!("❌ Codex command failed");
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

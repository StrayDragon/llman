use crate::x::codex::config::CodexManager;
use anyhow::{Context, Result};
use inquire::{Confirm, Select, Text, validator::Validation};
use rust_i18n::t;

/// Select a template for configuration creation
pub fn select_template() -> Result<String> {
    let templates = vec![
        ("openai", t!("codex.template.openai.description")),
        ("ollama", t!("codex.template.ollama.description")),
        ("minimal", t!("codex.template.minimal.description")),
    ];

    let template_names = templates.iter().map(|(name, _)| *name).collect();

    let selection = Select::new(&t!("codex.interactive.select_template"), template_names)
        .with_help_message(&t!("codex.interactive.template_help"))
        .prompt()
        .context("Failed to select template")?;

    Ok(selection.to_string())
}

/// Select a configuration to edit
pub fn select_config_to_edit(manager: &CodexManager) -> Result<String> {
    let configs = manager.list_configs()
        .context("Failed to list configurations")?;

    if configs.is_empty() {
        eprintln!("❌ {}", t!("codex.edit.error.no_configs"));
        anyhow::bail!("{}", t!("codex.edit.error.no_configs"));
    }

    let selection = Select::new(&t!("codex.interactive.select_config_to_edit"), configs)
        .prompt()
        .context("Failed to select configuration to edit")?;

    Ok(selection)
}

/// Confirm configuration deletion
pub fn confirm_delete(name: &str) -> Result<bool> {
    println!();
    println!("⚠️  {}", t!("codex.delete.warning", name = name));

    Confirm::new(&t!("codex.delete.confirm", name = name))
        .with_default(false)
        .prompt()
        .context("Failed to confirm deletion")
}

/// Interactive configuration name prompt with validation
pub fn prompt_config_name() -> Result<String> {
    Text::new(&t!("codex.interactive.config_name_prompt"))
        .with_validator(|input: &str| {
            let trimmed = input.trim();

            if trimmed.is_empty() {
                Ok(Validation::Invalid(
                    t!("codex.validation.name_required").into(),
                ))
            } else if !trimmed.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                Ok(Validation::Invalid(
                    t!("codex.validation.invalid_chars").into(),
                ))
            } else if trimmed.len() < 2 {
                Ok(Validation::Invalid(
                    t!("codex.validation.name_too_short").into(),
                ))
            } else if trimmed.len() > 50 {
                Ok(Validation::Invalid(
                    t!("codex.validation.name_too_long").into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        })
        .with_help_message(&t!("codex.interactive.config_name_help"))
        .prompt()
        .context("Failed to input configuration name")
}

/// Interactive configuration creation with template selection
pub fn create_configuration_interactive(manager: &CodexManager) -> Result<()> {
    // Prompt for configuration name
    let name = prompt_config_name()?;

    // Check if configuration already exists
    let configs = manager.list_configs()
        .context("Failed to list configurations")?;
    if configs.contains(&name) {
        eprintln!("❌ {}", t!("codex.create.error.exists", name = name));
        return Ok(());
    }

    // Select template
    let template = select_template()?;

    // Create configuration
    let config_path = manager.create_config(&name, Some(&template))
        .with_context(|| format!("Failed to create configuration '{}'", name))?;

    println!("✅ {}", t!("codex.create.success",
        name = name,
        path = config_path.display()
    ));
    println!("💡 {}", t!("codex.create.suggestion_use", name = name));

    // Ask if user wants to edit it now
    if Confirm::new(&t!("codex.interactive.edit_now"))
        .with_default(false)
        .prompt()
        .context("Failed to prompt for edit")?
    {
        open_config_in_editor(&config_path)?;
    }

    Ok(())
}

/// Open configuration file in user's preferred editor
fn open_config_in_editor(config_path: &std::path::Path) -> Result<()> {
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());

    println!("📝 {} {}", t!("codex.interactive.opening_in"), editor);

    let status = std::process::Command::new(&editor)
        .arg(config_path)
        .status()
        .with_context(|| format!("Failed to open editor '{}' for configuration", editor))?;

    if status.success() {
        println!("✅ {}", t!("codex.interactive.edit_completed"));
    } else {
        eprintln!("❌ {}", t!("codex.edit.error.editor_failed"));
    }

    Ok(())
}

/// Interactive configuration switching with preview
pub fn switch_configuration_interactive(manager: &CodexManager) -> Result<()> {
    let configs = manager.list_configs()
        .context("Failed to list configurations")?;

    if configs.is_empty() {
        eprintln!("❌ {}", t!("codex.use.error.no_configs"));
        println!("💡 {}", t!("codex.use.suggestion_create"));
        return Ok(());
    }

    let current_config = manager.get_current_config()
        .context("Failed to get current configuration")?;

    // Create display options with current indicator
    let display_options: Vec<String> = configs.iter()
        .map(|name| {
            let marker = if let Some(current) = &current_config {
                if current == name {
                    format!("{} ", t!("codex.interactive.current_marker"))
                } else {
                    "  ".to_string()
                }
            } else {
                "  ".to_string()
            };
            format!("{}{}", marker, name)
        })
        .collect();

    let selection = Select::new(&t!("codex.interactive.select_config_to_switch"), display_options)
        .with_help_message(&t!("codex.interactive.switch_help"))
        .prompt()
        .context("Failed to select configuration")?;

    // Extract the actual config name (remove marker)
    let config_name = selection.trim().split_whitespace()
        .last()
        .ok_or_else(|| anyhow::anyhow!("Failed to parse selection"))?;

    // Confirm if switching to the same config
    if let Some(current) = &current_config {
        if current == config_name {
            println!("ℹ️ {}", t!("codex.use.already_active", name = config_name));
            return Ok(());
        }
    }

    // Show configuration preview
    show_config_preview(manager, config_name)?;

    // Confirm the switch
    if Confirm::new(&t!("codex.use.confirm", name = config_name))
        .with_default(true)
        .prompt()
        .context("Failed to confirm switch")?
    {
        manager.use_config(config_name)?;
        println!("✅ {}", t!("codex.use.switched", name = config_name));
    } else {
        println!("❌ {}", t!("codex.use.cancelled"));
    }

    Ok(())
}

/// Show a preview of the configuration before switching
fn show_config_preview(manager: &CodexManager, config_name: &str) -> Result<()> {
    let config_path = manager.configs_dir().join(format!("{}.toml", config_name));

    if !config_path.exists() {
        return Ok(());
    }

    println!();
    println!("📄 {}:", t!("codex.interactive.config_preview"));
    println!("  {}", config_path.display());
    println!();

    let content = std::fs::read_to_string(&config_path)
        .context("Failed to read configuration file")?;

    // Show key configuration values (non-sensitive)
    for line in content.lines().take(20) { // Limit preview to 20 lines
        if line.trim().is_empty() || line.trim().starts_with('#') {
            continue;
        }

        if line.contains("model") ||
           line.contains("provider") ||
           line.contains("approval_policy") ||
           line.contains("sandbox_mode") {
            println!("  {}", line);
        } else if line.contains("base_url") && !line.to_lowercase().contains("key") {
            println!("  {}", line);
        }
    }

    if content.lines().count() > 20 {
        println!("  ...");
    }

    println!();
    Ok(())
}
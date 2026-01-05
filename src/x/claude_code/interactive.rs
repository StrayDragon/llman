use crate::x::claude_code::config::{Config, ConfigGroup, get_display_vars, parse_json_config};
use anyhow::{Context, Result};
use inquire::{Confirm, Editor, Select, Text, validator::Validation};
use rust_i18n::t;

pub fn select_config_group(config: &Config) -> Result<Option<String>> {
    if config.is_empty() {
        return Ok(None);
    }

    let group_names = config.group_names();

    let selection = Select::new(
        &t!("claude_code.interactive.select_config_group"),
        group_names,
    )
    .prompt()
    .context("Failed to select configuration group")?;

    Ok(Some(selection))
}

pub fn prompt_import_config() -> Result<Option<(String, ConfigGroup)>> {
    println!("{}", t!("claude_code.interactive.import_title"));

    let name = Text::new(&t!("claude_code.interactive.config_group_name"))
        .with_validator(|input: &str| {
            if input.trim().is_empty() {
                Ok(Validation::Invalid(
                    t!("claude_code.validation.name_required").into(),
                ))
            } else if input.contains(' ') {
                Ok(Validation::Invalid(
                    t!("claude_code.validation.no_spaces").into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()
        .context("Failed to input group name")?;

    println!();
    println!(
        "📝 {} - {}!",
        t!("claude_code.interactive.multi_line_json"),
        t!("claude_code.interactive.you_can_now_paste")
    );
    println!();
    println!("{}:", t!("claude_code.interactive.supported_formats"));
    println!("  Format 1: {{\"env\": {{...}}}}");
    println!("  Format 2: {{\"KEY\": \"value\", ...}}");
    println!();
    println!("{}:", t!("claude_code.interactive.how_to_use_editor"));
    println!(
        "  • {} ({})",
        t!("claude_code.interactive.paste_json"),
        t!("claude_code.interactive.supports_multiple_lines")
    );
    println!(
        "  • {} ({})",
        t!("claude_code.interactive.navigate"),
        t!("claude_code.interactive.use_arrow_keys")
    );
    println!(
        "  • {} ({})",
        t!("claude_code.interactive.finish"),
        t!("claude_code.interactive.ctrl_d_or_enter")
    );
    println!(
        "  • {} ({})",
        t!("claude_code.interactive.cancel"),
        t!("claude_code.interactive.esc")
    );
    println!();
    println!("{}:", t!("claude_code.interactive.example"));
    println!("{{");
    println!("  \"env\": {{");
    println!("    \"ANTHROPIC_BASE_URL\": \"https://api.anthropic.com\",");
    println!("    \"ANTHROPIC_AUTH_TOKEN\": \"your-api-key\",");
    println!("    \"ANTHROPIC_MODEL\": \"claude-3-5-sonnet-20241022\"");
    println!("  }}");
    println!("}}");
    println!();

    let json_input = Editor::new(&t!("claude_code.interactive.json_configuration"))
        .with_help_message(&t!("claude_code.interactive.editor_help"))
        .with_formatter(&|submission| {
            let lines = submission.lines().count();
            let chars = submission.chars().count();
            if lines == 0 {
                String::from("<empty>")
            } else {
                format!("{} lines, {} chars", lines, chars)
            }
        })
        .prompt()
        .context("Failed to input JSON configuration")?;

    match parse_json_config(&json_input) {
        Ok(config_group) => {
            if config_group.is_empty() {
                println!("{}", t!("claude_code.interactive.warning_empty_config"));
                if !Confirm::new(&t!("claude_code.interactive.confirm_empty_import"))
                    .with_default(false)
                    .prompt()
                    .context("Failed to confirm import")?
                {
                    println!("{}", t!("claude_code.interactive.import_cancelled"));
                    return Ok(None);
                }
            } else {
                println!();
                println!("{}:", t!("claude_code.interactive.parsed_env_vars"));
                let display_vars = get_display_vars(&config_group);
                for (key, value) in display_vars {
                    println!("  {} = {}", key, value);
                }
                println!();
            }

            if Confirm::new(&t!("claude_code.interactive.confirm_import", name = name))
                .with_default(true)
                .prompt()
                .context("Failed to confirm import")?
            {
                Ok(Some((name, config_group)))
            } else {
                println!("{}", t!("claude_code.interactive.import_cancelled"));
                Ok(None)
            }
        }
        Err(e) => {
            eprintln!("{}: {}", t!("claude_code.error.parsing_json"), e);
            println!("{}", t!("claude_code.interactive.check_format_retry"));
            Ok(None)
        }
    }
}

pub fn display_config_list(config: &Config) {
    if config.is_empty() {
        println!("{}", t!("claude_code.interactive.no_config_groups"));
        println!("{}", t!("claude_code.interactive.use_import_command"));
        println!(
            "{}: {}",
            t!("claude_code.interactive.config_file_location"),
            crate::x::claude_code::config::Config::config_file_path()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string())
        );
        println!();
        println!("{}:", t!("claude_code.interactive.example_configuration"));
        println!("[groups]");
        println!();
        println!("[groups.my-group]");
        println!("ANTHROPIC_BASE_URL = \"https://api.anthropic.com\"");
        println!("ANTHROPIC_AUTH_TOKEN = \"your-api-key-here\"");
        println!("ANTHROPIC_MODEL = \"claude-3-5-sonnet-20241022\"");
        println!();
        println!(
            "{}: claude-code.toml",
            t!("claude_code.interactive.see_examples")
        );
        return;
    }

    println!("{}:", t!("claude_code.interactive.configuration_groups"));
    println!();

    for name in config.group_names() {
        if let Some(group) = config.get_group(&name) {
            println!("  📝 {}", name);
            let display_vars = get_display_vars(group);
            if display_vars.is_empty() {
                println!("     ({})", t!("claude_code.interactive.no_env_vars"));
            } else {
                for (key, value) in display_vars {
                    println!("     {} = {}", key, value);
                }
            }
            println!();
        }
    }
}

use anyhow::{Context, Result};
use inquire::{Confirm, Select};
use rust_i18n::t;

use super::config::{Metadata, TemplateProvider};

/// Select a group from available groups
pub fn select_group(groups: &[String]) -> Result<String> {
    let metadata = Metadata::load()?;

    // Add indicator for current group
    let options: Vec<String> = groups
        .iter()
        .map(|name| {
            if metadata.current_group.as_ref() == Some(name) {
                format!("{} {}", name, t!("codex.account.current_indicator"))
            } else {
                name.clone()
            }
        })
        .collect();

    let selection = Select::new(&t!("codex.interactive.select_group"), options.clone())
        .prompt()
        .context("Failed to select group")?;

    let index = options
        .iter()
        .position(|option| option == &selection)
        .context("Failed to map selection to group")?;

    Ok(groups[index].clone())
}

/// Select a template provider
pub fn select_template() -> Result<&'static str> {
    let templates = TemplateProvider::all();

    let options: Vec<String> = templates
        .iter()
        .map(|t| t.display_name().to_string())
        .collect();

    let selection = Select::new(&t!("codex.interactive.select_template"), options)
        .prompt()
        .context("Failed to select template")?;

    // Map selection back to template key
    let template = if selection.starts_with("OpenAI") {
        "openai"
    } else if selection.starts_with("MiniMax") {
        "minimax"
    } else if selection.starts_with("RightCode") {
        "rightcode"
    } else {
        "custom"
    };

    Ok(template)
}

/// Confirm group deletion
pub fn confirm_delete(name: &str) -> Result<bool> {
    println!();
    println!(
        "⚠️  {}",
        t!("codex.interactive.delete_warning", name = name)
    );

    Confirm::new(&t!("codex.interactive.confirm_delete", name = name))
        .with_default(false)
        .prompt()
        .context("Failed to confirm deletion")
}

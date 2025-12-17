use anyhow::{Context, Result};
use inquire::{Confirm, Select, Text, validator::Validation};

/// Select a template for profile creation
pub fn select_template() -> Result<String> {
    let templates = [
        (
            "development",
            "Development environment with relaxed settings",
        ),
        ("production", "Production environment with strict security"),
    ];

    let template_names = templates.iter().map(|(name, _)| *name).collect();

    let selection = Select::new("Select a template for your profile", template_names)
        .with_help_message("Choose a template based on your use case")
        .prompt()
        .context("Failed to select template")?;

    Ok(selection.to_string())
}

/// Interactive profile name prompt with validation
pub fn prompt_profile_name() -> Result<String> {
    Text::new("Enter profile name")
        .with_validator(|input: &str| {
            let trimmed = input.trim();

            if trimmed.is_empty() {
                Ok(Validation::Invalid("Profile name is required".into()))
            } else if !trimmed
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            {
                Ok(Validation::Invalid(
                    "Profile name can only contain letters, numbers, hyphens, and underscores"
                        .into(),
                ))
            } else if trimmed.len() < 2 {
                Ok(Validation::Invalid(
                    "Profile name must be at least 2 characters".into(),
                ))
            } else if trimmed.len() > 50 {
                Ok(Validation::Invalid(
                    "Profile name must be less than 50 characters".into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        })
        .with_help_message("Use a descriptive name like 'dev', 'prod', 'project-x'")
        .prompt()
        .context("Failed to input profile name")
}

/// Confirm profile deletion
pub fn confirm_delete(name: &str) -> Result<bool> {
    println!();
    println!("⚠️  You are about to delete profile: {}", name);

    Confirm::new(&format!(
        "Are you sure you want to delete profile '{}'?",
        name
    ))
    .with_default(false)
    .prompt()
    .context("Failed to confirm deletion")
}

/// Confirm directory overwrite
pub fn confirm_overwrite() -> Result<bool> {
    println!();
    Confirm::new("Do you want to continue and potentially overwrite existing files?")
        .with_default(false)
        .prompt()
        .context("Failed to confirm overwrite")
}

/// Confirm creating a new profile
pub fn confirm_create_profile() -> Result<bool> {
    println!();
    Confirm::new("Would you like to create a new profile?")
        .with_default(true)
        .prompt()
        .context("Failed to confirm profile creation")
}

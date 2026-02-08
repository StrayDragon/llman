use crate::x::codex::config::{Config, ProviderConfig};
use anyhow::{Context, Result};
use inquire::{Select, Text};
use rust_i18n::t;

pub fn select_provider(config: &Config) -> Result<Option<String>> {
    if config.is_empty() {
        return Ok(None);
    }

    let names = config.provider_names();

    let selection = Select::new(&t!("codex.interactive.select_group"), names)
        .prompt()
        .context(t!("codex.error.select_group_failed"))?;

    Ok(Some(selection))
}

pub fn prompt_import() -> Result<Option<(String, ProviderConfig)>> {
    let group_name = Text::new(&t!("codex.interactive.import_group_name"))
        .with_help_message(&t!("codex.interactive.import_group_name_help"))
        .prompt()
        .context(t!("codex.error.import_prompt_failed"))?;

    let group_name = group_name.trim().to_string();
    if group_name.is_empty() {
        return Ok(None);
    }

    let base_url = Text::new(&t!("codex.interactive.import_base_url"))
        .with_help_message(&t!("codex.interactive.import_base_url_help"))
        .prompt()
        .context(t!("codex.error.import_prompt_failed"))?;

    let env_key_id = Text::new(&t!("codex.interactive.import_env_key_id"))
        .with_default("CODEX_API_KEY")
        .with_help_message(&t!("codex.interactive.import_env_key_id_help"))
        .prompt()
        .context(t!("codex.error.import_prompt_failed"))?;

    let api_key_value = Text::new(&t!("codex.interactive.import_api_key_value"))
        .with_help_message(&t!("codex.interactive.import_api_key_value_help"))
        .prompt()
        .context(t!("codex.error.import_prompt_failed"))?;

    let mut env = std::collections::HashMap::new();
    env.insert(env_key_id.clone(), api_key_value);

    let provider = ProviderConfig {
        name: group_name.clone(),
        base_url,
        wire_api: "responses".to_string(),
        env_key: env_key_id,
        env,
    };

    Ok(Some((group_name, provider)))
}

use crate::x::claude_code::config as claude_code_config;
use crate::x::codex::config as codex_config;
use crate::x::sdd_eval::playbook;
use anyhow::{Context, Result, bail};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ResolvedPreset {
    pub env: HashMap<String, String>,
}

pub fn resolve_env(kind: playbook::AgentKind, preset: &str) -> Result<ResolvedPreset> {
    match kind {
        playbook::AgentKind::ClaudeCode => resolve_claude_code_env(preset),
        playbook::AgentKind::Codex => resolve_codex_env(preset),
        playbook::AgentKind::Fake => Ok(ResolvedPreset {
            env: HashMap::new(),
        }),
    }
}

fn resolve_claude_code_env(group: &str) -> Result<ResolvedPreset> {
    let config = claude_code_config::Config::load()
        .with_context(|| "load Claude Code config (claude-code.toml)")?;

    let Some(env) = config.get_group(group) else {
        bail!("Claude Code preset group not found: {group}");
    };

    Ok(ResolvedPreset { env: env.clone() })
}

fn resolve_codex_env(group: &str) -> Result<ResolvedPreset> {
    let config = codex_config::Config::load().with_context(|| "load Codex config (codex.toml)")?;
    let Some(provider) = config.get_provider(group) else {
        bail!("Codex preset group not found: {group}");
    };

    Ok(ResolvedPreset {
        env: provider.env.clone(),
    })
}

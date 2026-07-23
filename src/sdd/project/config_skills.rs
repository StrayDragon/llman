//! `llman sdd config` and `llman sdd config skills` — view/edit SDD project config.
//!
//! `config` (no subcommand) prints a read-only overview of config.yaml.
//! `config skills` interactively manages the `extra_skills` list via a MultiSelect,
//! with `--no-interactive` (print state) and `--json` (structured output) fallbacks.

use crate::sdd::project::config::{
    OPTIONAL_SKILL_NAMES, SddConfig, load_or_create_config, write_config,
};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::interactive::is_interactive;
use anyhow::Result;
use inquire::error::InquireError;
use inquire::{Confirm, MultiSelect};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

/// One-line description for each optional skill, keyed by skill name.
/// Keep in sync with `OPTIONAL_SKILL_NAMES` and the template frontmatter `description`.
fn skill_description(name: &str) -> &'static str {
    match name {
        "llman-sdd-new-change" => "Lightweight draft proposal path (`change new --from`)",
        "llman-sdd-continue" => "Fill in missing change artifacts",
        "llman-sdd-ff" => "Alias for `change finalize` (single-commit close-out)",
        "llman-sdd-sync" => "Sync live specs across the change lifecycle",
        "llman-sdd-validate" => "Standalone validation skill",
        "llman-sdd-arch-review" => "Scan shallow modules for deepening candidates",
        "llman-sdd-wayfinder" => "Plan large foggy work as a decision map",
        "llman-sdd-research" => "Delegate external research to a background agent",
        _ => "",
    }
}

/// A selectable skill option shown in the MultiSelect.
#[derive(Clone)]
struct SkillOption {
    name: &'static str,
    desc: &'static str,
}

impl std::fmt::Display for SkillOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} — {}", self.name, self.desc)
    }
}

fn skill_options() -> Vec<SkillOption> {
    OPTIONAL_SKILL_NAMES
        .iter()
        .map(|name| SkillOption {
            name,
            desc: skill_description(name),
        })
        .collect()
}

/// `llman sdd config skills` entry point.
pub fn run(no_interactive: bool, json: bool, root: &Path) -> Result<()> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_or_create_config(&llmanspec_dir)?;

    if json {
        return print_json(&config);
    }

    if !is_interactive(no_interactive) {
        print_state(&config);
        return Ok(());
    }

    interactive_edit(&config, &llmanspec_dir)
}

/// `llman sdd config` (no subcommand) — read-only overview.
pub fn run_overview(root: &Path) -> Result<()> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    let config = load_or_create_config(&llmanspec_dir)?;
    print_overview(&config);
    Ok(())
}

#[derive(Serialize)]
struct SkillsJson<'a> {
    enabled: &'a [String],
    available: Vec<&'a str>,
}

fn print_json(config: &SddConfig) -> Result<()> {
    let enabled: &[String] = config.extra_skills.as_deref().unwrap_or(&[]);
    let enabled_set: HashSet<&str> = enabled.iter().map(|s| s.as_str()).collect();
    let available: Vec<&str> = OPTIONAL_SKILL_NAMES
        .iter()
        .filter(|n| !enabled_set.contains(**n))
        .copied()
        .collect();
    let out = SkillsJson { enabled, available };
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn print_state(config: &SddConfig) {
    let enabled: Vec<&str> = config
        .extra_skills
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();
    let enabled_set: HashSet<&str> = enabled.iter().copied().collect();
    let available: Vec<&str> = OPTIONAL_SKILL_NAMES
        .iter()
        .filter(|n| !enabled_set.contains(**n))
        .copied()
        .collect();

    println!("{}", t!("sdd.config.skills.enabled"));
    if enabled.is_empty() {
        println!("  (none)");
    } else {
        for name in &enabled {
            println!("  [x] {} — {}", name, skill_description(name));
        }
    }
    println!("\n{}", t!("sdd.config.skills.available"));
    if available.is_empty() {
        println!("  (all enabled)");
    } else {
        for name in available {
            println!("  [ ] {} — {}", name, skill_description(name));
        }
    }
    println!("\n{}", t!("sdd.config.skills.non_interactive_hint"));
}

fn print_overview(config: &SddConfig) {
    let enabled_count = config.extra_skills.as_ref().map(|v| v.len()).unwrap_or(0);
    let bdd_status = if config.bdd.is_some() { "on" } else { "off" };
    let archive_status = if config.archive.is_some() {
        "configured"
    } else {
        "default"
    };
    println!("{}: {}", t!("sdd.config.overview.schema"), config.schema);
    println!("{}: {}", t!("sdd.config.overview.locale"), config.locale);
    println!(
        "{}: {} / {}",
        t!("sdd.config.overview.extra_skills"),
        enabled_count,
        OPTIONAL_SKILL_NAMES.len()
    );
    println!("{}: {}", t!("sdd.config.overview.bdd"), bdd_status);
    println!("{}: {}", t!("sdd.config.overview.archive"), archive_status);
}

fn interactive_edit(config: &SddConfig, llmanspec_dir: &Path) -> Result<()> {
    let options = skill_options();
    let enabled: HashSet<&str> = config
        .extra_skills
        .as_ref()
        .map(|v| v.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    // Build default-checked indices from current extra_skills.
    let defaults: Vec<usize> = options
        .iter()
        .enumerate()
        .filter(|(_, o)| enabled.contains(o.name))
        .map(|(i, _)| i)
        .collect();

    let prompt = t!("sdd.config.skills.prompt");
    let selection = match MultiSelect::new(&prompt, options.clone())
        .with_default(&defaults)
        .prompt()
    {
        Ok(sel) => sel,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            println!("{}", t!("sdd.config.skills.cancelled"));
            return Ok(());
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "{}",
                t!("errors.interactive_prompt_error", error = e)
            ));
        }
    };

    let new_skills: Vec<String> = selection.iter().map(|o| o.name.to_string()).collect();
    let current: Vec<String> = config.extra_skills.clone().unwrap_or_default();

    // No change?
    if new_skills == current {
        println!("{}", t!("sdd.config.skills.no_change"));
        return Ok(());
    }

    // Confirm.
    let confirm_prompt = t!("sdd.config.skills.confirm");
    let confirmed = match Confirm::new(&confirm_prompt).with_default(true).prompt() {
        Ok(c) => c,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            println!("{}", t!("sdd.config.skills.cancelled"));
            return Ok(());
        }
        Err(e) => {
            return Err(anyhow::anyhow!(
                "{}",
                t!("errors.interactive_prompt_error", error = e)
            ));
        }
    };
    if !confirmed {
        println!("{}", t!("sdd.config.skills.cancelled"));
        return Ok(());
    }

    // Write back.
    let mut new_config = config.clone();
    new_config.extra_skills = if new_skills.is_empty() {
        None
    } else {
        Some(new_skills)
    };
    write_config(llmanspec_dir, &new_config)?;
    println!("{}", t!("sdd.config.skills.written_hint"));
    Ok(())
}

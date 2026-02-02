use crate::skills::config::load_config;
use crate::skills::interactive::is_interactive;
use crate::skills::registry::Registry;
use crate::skills::scan::discover_skills;
use crate::skills::sync::apply_target_links;
use crate::skills::types::{SkillCandidate, SkillsPaths, TargetConflictStrategy, TargetMode};
use anyhow::Result;
use chrono::Utc;
use clap::{Args, ValueEnum};
use inquire::{MultiSelect, Select};
use std::collections::HashSet;

#[derive(Args)]
#[command(about = "Manage skills", long_about = "Interactive skills manager")]
pub struct SkillsArgs {
    /// Removed: relink sources is no longer supported
    #[arg(long = "relink-sources", hide = true)]
    pub relink_sources_removed: bool,

    /// Conflict policy for link targets (overwrite or skip)
    #[arg(long = "target-conflict", value_enum)]
    pub target_conflict: Option<TargetConflictArg>,

    /// Override skills root directory (env: LLMAN_SKILLS_DIR)
    #[arg(long = "skills-dir")]
    pub skills_dir: Option<std::path::PathBuf>,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum TargetConflictArg {
    Overwrite,
    Skip,
}

pub fn run(args: &SkillsArgs) -> Result<()> {
    if args.relink_sources_removed {
        return Err(anyhow::anyhow!(t!("skills.relink_removed")));
    }
    let interactive = is_interactive();

    let paths = SkillsPaths::resolve_with_override(args.skills_dir.as_deref())?;
    paths.ensure_dirs()?;
    let config = load_config(&paths)?;
    let mut registry = Registry::load(&paths.registry_path)?;
    let target_conflict = args.target_conflict.map(TargetConflictStrategy::from);
    let skills = discover_skills(&paths.root)?;

    for skill in &skills {
        let entry = registry.ensure_skill(&skill.skill_id);
        ensure_target_defaults(entry, &config.targets);
    }

    if skills.is_empty() {
        println!("{}", t!("skills.manager.no_skills"));
        return Ok(());
    }

    if interactive {
        let selected = select_skills(&skills)?;
        if selected.is_empty() {
            registry.save(&paths.registry_path)?;
            return Ok(());
        }
        for skill in &skills {
            if !selected.contains(&skill.skill_id) {
                continue;
            }
            manage_targets_for_skill(skill, &config, &mut registry, target_conflict)?;
        }
    } else {
        for skill in &skills {
            let entry = match registry.skills.get(&skill.skill_id) {
                Some(entry) => entry,
                None => continue,
            };
            apply_target_links(skill, &config, entry, false, target_conflict)?;
            if let Some(entry) = registry.skills.get_mut(&skill.skill_id) {
                entry.updated_at = Some(Utc::now().to_rfc3339());
            }
        }
    }
    registry.save(&paths.registry_path)?;
    Ok(())
}

fn select_skills(skills: &[SkillCandidate]) -> Result<HashSet<String>> {
    let mut skill_ids: Vec<String> = skills.iter().map(|skill| skill.skill_id.clone()).collect();
    skill_ids.sort();
    let selections = MultiSelect::new(&t!("skills.manager.select_skills"), skill_ids)
        .prompt()
        .map_err(|e| anyhow::anyhow!(t!("skills.manager.prompt_failed", error = e)))?;
    Ok(selections.into_iter().collect())
}

fn manage_targets_for_skill(
    skill: &SkillCandidate,
    config: &crate::skills::types::SkillsConfig,
    registry: &mut Registry,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    loop {
        let entry = match registry.skills.get(&skill.skill_id) {
            Some(entry) => entry,
            None => return Ok(()),
        };
        let mut labels = Vec::new();
        let mut target_ids = Vec::new();
        for target in &config.targets {
            if target.mode == TargetMode::Skip {
                labels.push(format!(
                    "{} - {}",
                    t!("skills.manager.state_skip"),
                    target.id
                ));
                target_ids.push(target.id.clone());
                continue;
            }
            let enabled = entry
                .targets
                .get(&target.id)
                .copied()
                .unwrap_or(target.enabled);
            let marker = if enabled {
                t!("skills.manager.state_on")
            } else {
                t!("skills.manager.state_off")
            };
            labels.push(format!("{} - {}", marker, target.id));
            target_ids.push(target.id.clone());
        }
        let back_label = t!("skills.manager.back").to_string();
        labels.push(back_label.clone());

        let selection = Select::new(&t!("skills.manager.select_target"), labels.clone())
            .prompt()
            .map_err(|e| anyhow::anyhow!(t!("skills.manager.prompt_failed", error = e)))?;
        if selection == back_label {
            break;
        }
        let Some(idx) = labels.iter().position(|label| label == &selection) else {
            continue;
        };
        let target_id = &target_ids[idx];
        let Some(entry) = registry.skills.get_mut(&skill.skill_id) else {
            return Ok(());
        };
        if let Some(target) = config.targets.iter().find(|t| t.id == *target_id)
            && target.mode == TargetMode::Skip
        {
            println!("{}", t!("skills.manager.read_only"));
            continue;
        }
        let current = entry.targets.get(target_id).copied().unwrap_or(false);
        entry.targets.insert(target_id.clone(), !current);
    }
    if let Some(entry) = registry.skills.get(&skill.skill_id) {
        apply_target_links(skill, config, entry, true, target_conflict)?;
    }
    if let Some(entry) = registry.skills.get_mut(&skill.skill_id) {
        entry.updated_at = Some(Utc::now().to_rfc3339());
    }
    Ok(())
}

impl From<TargetConflictArg> for TargetConflictStrategy {
    fn from(value: TargetConflictArg) -> Self {
        match value {
            TargetConflictArg::Overwrite => TargetConflictStrategy::Overwrite,
            TargetConflictArg::Skip => TargetConflictStrategy::Skip,
        }
    }
}

fn ensure_target_defaults(
    entry: &mut crate::skills::registry::SkillEntry,
    targets: &[crate::skills::types::ConfigEntry],
) {
    for target in targets {
        entry
            .targets
            .entry(target.id.clone())
            .or_insert(target.enabled);
    }
}

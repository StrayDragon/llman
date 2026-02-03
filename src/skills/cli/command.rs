use crate::skills::catalog::registry::Registry;
use crate::skills::catalog::scan::discover_skills;
use crate::skills::catalog::types::{
    SkillCandidate, SkillsPaths, TargetConflictStrategy, TargetMode,
};
use crate::skills::cli::interactive::is_interactive;
use crate::skills::config::load_config;
use crate::skills::targets::sync::{apply_target_diff, apply_target_links, is_skill_linked};
use anyhow::Result;
use chrono::Utc;
use clap::{Args, ValueEnum};
use inquire::error::InquireError;
use inquire::{Confirm, MultiSelect, Select};
use std::collections::{HashMap, HashSet};
use std::fs;

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
    let skills = dedupe_skills(discover_skills(&paths.root)?);

    if skills.is_empty() {
        println!("{}", t!("skills.manager.no_skills"));
        return Ok(());
    }

    if interactive {
        let Some(target) = select_target(&config)? else {
            return Ok(());
        };
        let Some(selected) = select_skills_for_target(&skills, &target)? else {
            return Ok(());
        };
        if !confirm_apply(&target)? {
            return Ok(());
        }
        apply_target_diff(&skills, &target, &selected, true, target_conflict)?;
        update_registry_for_target(&mut registry, &skills, &target, &selected);
        registry.save(&paths.registry_path)?;
    } else {
        for skill in &skills {
            let entry = registry.ensure_skill(&skill.skill_id);
            ensure_target_defaults(entry, &config.targets);
        }
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
        registry.save(&paths.registry_path)?;
    }
    Ok(())
}

fn select_target(
    config: &crate::skills::catalog::types::SkillsConfig,
) -> Result<Option<crate::skills::catalog::types::ConfigEntry>> {
    loop {
        let mut labels = Vec::new();
        let mut targets = Vec::new();
        for target in &config.targets {
            let label = if target.mode == TargetMode::Skip {
                format!("{} - {}", t!("skills.manager.state_skip"), target.id)
            } else {
                target.id.clone()
            };
            labels.push(label);
            targets.push(target.clone());
        }
        let exit_label = t!("skills.manager.exit").to_string();
        labels.push(exit_label.clone());

        let selection =
            match Select::new(&t!("skills.manager.select_target"), labels.clone()).prompt() {
                Ok(selection) => selection,
                Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                    return Ok(None);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(t!(
                        "skills.manager.prompt_failed",
                        error = e
                    )));
                }
            };
        if selection == exit_label {
            return Ok(None);
        }
        let Some(idx) = labels.iter().position(|label| label == &selection) else {
            continue;
        };
        if let Some(target) = targets.get(idx) {
            if target.mode == TargetMode::Skip {
                println!("{}", t!("skills.manager.read_only"));
                continue;
            }
            return Ok(Some(target.clone()));
        }
    }
}

fn select_skills_for_target(
    skills: &[SkillCandidate],
    target: &crate::skills::catalog::types::ConfigEntry,
) -> Result<Option<HashSet<String>>> {
    let skill_ids: Vec<String> = skills.iter().map(|skill| skill.skill_id.clone()).collect();
    let mut defaults = Vec::new();
    for (idx, skill) in skills.iter().enumerate() {
        if is_skill_linked(skill, target) {
            defaults.push(idx);
        }
    }
    let prompt = t!(
        "skills.manager.select_skills_for_target",
        target = target.id
    );
    let selections = match MultiSelect::new(&prompt, skill_ids)
        .with_default(&defaults)
        .prompt()
    {
        Ok(selections) => selections,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            return Ok(None);
        }
        Err(e) => {
            return Err(anyhow::anyhow!(t!(
                "skills.manager.prompt_failed",
                error = e
            )));
        }
    };
    Ok(Some(selections.into_iter().collect()))
}

fn confirm_apply(target: &crate::skills::catalog::types::ConfigEntry) -> Result<bool> {
    let prompt = t!("skills.manager.confirm_apply", target = target.id);
    let confirmation = match Confirm::new(&prompt).with_default(true).prompt() {
        Ok(confirmation) => confirmation,
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
            return Ok(false);
        }
        Err(e) => {
            return Err(anyhow::anyhow!(t!(
                "skills.manager.prompt_failed",
                error = e
            )));
        }
    };
    Ok(confirmation)
}

fn update_registry_for_target(
    registry: &mut Registry,
    skills: &[SkillCandidate],
    target: &crate::skills::catalog::types::ConfigEntry,
    selected: &HashSet<String>,
) {
    let now = Utc::now().to_rfc3339();
    for skill in skills {
        let entry = registry.ensure_skill(&skill.skill_id);
        entry
            .targets
            .insert(target.id.clone(), selected.contains(&skill.skill_id));
        entry.updated_at = Some(now.clone());
    }
}

fn dedupe_skills(skills: Vec<SkillCandidate>) -> Vec<SkillCandidate> {
    let mut seen: HashMap<String, SkillCandidate> = HashMap::new();
    for skill in skills {
        if let Some(existing) = seen.get(&skill.skill_id) {
            let existing_is_link = is_symlink_dir(&existing.skill_dir);
            let incoming_is_link = is_symlink_dir(&skill.skill_dir);
            let replace = existing_is_link && !incoming_is_link;
            if replace {
                eprintln!(
                    "{}",
                    t!(
                        "skills.manager.duplicate_skill_id",
                        skill = skill.skill_id,
                        path = existing.skill_dir.display(),
                        chosen = skill.skill_dir.display()
                    )
                );
                seen.insert(skill.skill_id.clone(), skill);
            } else {
                eprintln!(
                    "{}",
                    t!(
                        "skills.manager.duplicate_skill_id",
                        skill = skill.skill_id,
                        path = skill.skill_dir.display(),
                        chosen = existing.skill_dir.display()
                    )
                );
            }
            continue;
        }
        seen.insert(skill.skill_id.clone(), skill);
    }
    let mut values = seen.into_values().collect::<Vec<_>>();
    values.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));
    values
}

fn is_symlink_dir(path: &std::path::Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
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
    entry: &mut crate::skills::catalog::registry::SkillEntry,
    targets: &[crate::skills::catalog::types::ConfigEntry],
) {
    for target in targets {
        entry
            .targets
            .entry(target.id.clone())
            .or_insert(target.enabled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::catalog::registry::Registry;
    use std::path::PathBuf;

    #[test]
    fn test_update_registry_for_target_only_updates_target() {
        let mut registry = Registry::default();
        registry
            .ensure_skill("alpha")
            .targets
            .insert("other".to_string(), true);

        let skills = vec![
            SkillCandidate {
                skill_id: "alpha".to_string(),
                skill_dir: PathBuf::from("/tmp/alpha"),
            },
            SkillCandidate {
                skill_id: "beta".to_string(),
                skill_dir: PathBuf::from("/tmp/beta"),
            },
        ];
        let target = crate::skills::catalog::types::ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: PathBuf::from("/tmp/target"),
            enabled: true,
            mode: TargetMode::Link,
        };
        let mut selected = HashSet::new();
        selected.insert("alpha".to_string());

        update_registry_for_target(&mut registry, &skills, &target, &selected);

        let alpha = registry.skills.get("alpha").expect("alpha entry");
        let beta = registry.skills.get("beta").expect("beta entry");
        assert_eq!(alpha.targets.get("claude_user"), Some(&true));
        assert_eq!(beta.targets.get("claude_user"), Some(&false));
        assert_eq!(alpha.targets.get("other"), Some(&true));
        assert!(alpha.updated_at.is_some());
        assert!(beta.updated_at.is_some());
    }
}

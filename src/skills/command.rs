use crate::skills::config::load_config;
use crate::skills::git::find_git_root;
use crate::skills::interactive::{confirm_non_repo, is_interactive};
use crate::skills::registry::Registry;
use crate::skills::sync::{
    InteractiveResolver, apply_target_link, apply_target_links, sync_sources,
};
use crate::skills::types::{SkillsPaths, TargetConflictStrategy, TargetMode};
use anyhow::Result;
use clap::{Args, ValueEnum};
use inquire::Select;
use std::env;

#[derive(Args)]
#[command(about = "Manage skills", long_about = "Interactive skills manager")]
pub struct SkillsArgs {
    /// Removed: relink sources is no longer supported
    #[arg(long = "relink-sources", hide = true)]
    pub relink_sources_removed: bool,

    /// Conflict policy for copy-mode targets (overwrite or skip)
    #[arg(long = "target-conflict", value_enum)]
    pub target_conflict: Option<TargetConflictArg>,
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

    let cwd = env::current_dir()?;
    let repo_root = find_git_root(&cwd);
    if repo_root.is_none() {
        let confirmed = confirm_non_repo(interactive)?;
        if !confirmed {
            println!("{}", t!("skills.non_repo_cancelled"));
            return Ok(());
        }
    }

    let paths = SkillsPaths::resolve()?;
    let config = load_config(&paths, repo_root)?;
    let mut registry = Registry::load(&paths.registry_path)?;
    let mut resolver = InteractiveResolver::new(interactive);

    let _summary = sync_sources(&config, &paths, &mut registry, &mut resolver)?;
    registry.save(&paths.registry_path)?;

    let target_conflict = args.target_conflict.map(TargetConflictStrategy::from);
    apply_target_links(&config, &paths, &mut registry, interactive, target_conflict)?;
    registry.save(&paths.registry_path)?;

    if interactive {
        manage_targets(&config, &paths, &mut registry, target_conflict)?;
        registry.save(&paths.registry_path)?;
    }
    Ok(())
}

fn manage_targets(
    config: &crate::skills::types::SkillsConfig,
    paths: &SkillsPaths,
    registry: &mut Registry,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    if registry.skills.is_empty() {
        println!("{}", t!("skills.manager.no_skills"));
        return Ok(());
    }

    loop {
        let mut skill_ids: Vec<String> = registry.skills.keys().cloned().collect();
        skill_ids.sort();
        let exit_label = t!("skills.manager.exit").to_string();
        skill_ids.push(exit_label.clone());

        let selection = Select::new(&t!("skills.manager.select_skill"), skill_ids)
            .prompt()
            .map_err(|e| anyhow::anyhow!(t!("skills.manager.prompt_failed", error = e)))?;
        if selection == exit_label {
            break;
        }

        manage_targets_for_skill(&selection, config, paths, registry, target_conflict)?;
    }
    Ok(())
}

fn manage_targets_for_skill(
    skill_id: &str,
    config: &crate::skills::types::SkillsConfig,
    paths: &SkillsPaths,
    registry: &mut Registry,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    loop {
        let entry = match registry.skills.get(skill_id) {
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
        let Some(entry) = registry.skills.get_mut(skill_id) else {
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
        if let Some(target) = config.targets.iter().find(|t| t.id == *target_id) {
            apply_target_link(
                skill_id,
                &paths.store_dir,
                target,
                entry,
                true,
                target_conflict,
            )?;
        }
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

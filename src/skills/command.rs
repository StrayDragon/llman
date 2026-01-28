use crate::skills::config::load_config;
use crate::skills::git::find_git_root;
use crate::skills::interactive::{confirm_non_repo, confirm_relink_sources, is_interactive};
use crate::skills::registry::Registry;
use crate::skills::sync::{
    InteractiveResolver, apply_target_link, apply_target_links, sync_sources,
};
use crate::skills::types::SkillsPaths;
use anyhow::Result;
use clap::Args;
use inquire::Select;
use std::env;

#[derive(Args)]
#[command(about = "Manage skills", long_about = "Interactive skills manager")]
pub struct SkillsArgs {
    /// Relink source skill directories to the managed store (required to modify sources)
    #[arg(long = "relink-sources")]
    pub relink_sources: bool,

    /// Skip confirmation prompts (requires --relink-sources)
    #[arg(long, short = 'y')]
    pub yes: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RelinkGate {
    Proceed,
    Skip,
}

pub fn run(args: &SkillsArgs) -> Result<()> {
    let interactive = is_interactive();

    match gate_relink_sources(args, interactive)? {
        RelinkGate::Skip => {
            println!("{}", t!("skills.relink_skipped"));
            return Ok(());
        }
        RelinkGate::Proceed => {}
    }

    let cwd = env::current_dir()?;
    let repo_root = find_git_root(&cwd);
    if repo_root.is_none() {
        let confirmed = confirm_non_repo(interactive)?;
        if !confirmed {
            println!("{}", t!("skills.non_repo_cancelled"));
            return Ok(());
        }
    }

    if interactive && !args.yes {
        let confirmed = confirm_relink_sources(interactive)?;
        if !confirmed {
            println!("{}", t!("skills.relink_cancelled"));
            return Ok(());
        }
    }

    let paths = SkillsPaths::resolve()?;
    let config = load_config(&paths, repo_root)?;
    let mut registry = Registry::load(&paths.registry_path)?;
    let mut resolver = InteractiveResolver::new(interactive);

    let _summary = sync_sources(&config, &paths, &mut registry, &mut resolver)?;
    registry.save(&paths.registry_path)?;

    apply_target_links(&config, &paths, &mut registry)?;
    registry.save(&paths.registry_path)?;

    if interactive {
        manage_targets(&config, &paths, &mut registry)?;
        registry.save(&paths.registry_path)?;
    }
    Ok(())
}

fn gate_relink_sources(args: &SkillsArgs, interactive: bool) -> Result<RelinkGate> {
    if args.yes && !args.relink_sources {
        return Err(anyhow::anyhow!(t!("skills.relink_yes_requires_flag")));
    }
    if !args.relink_sources {
        if interactive {
            return Ok(RelinkGate::Skip);
        }
        return Err(anyhow::anyhow!(t!(
            "skills.relink_required_non_interactive"
        )));
    }
    Ok(RelinkGate::Proceed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_relink_sources_interactive_skip() {
        let args = SkillsArgs {
            relink_sources: false,
            yes: false,
        };
        let result = gate_relink_sources(&args, true).expect("gate relink");
        assert_eq!(result, RelinkGate::Skip);
    }

    #[test]
    fn test_gate_relink_sources_non_interactive_requires_flag() {
        let args = SkillsArgs {
            relink_sources: false,
            yes: false,
        };
        let result = gate_relink_sources(&args, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_gate_relink_sources_requires_flag_for_yes() {
        let args = SkillsArgs {
            relink_sources: false,
            yes: true,
        };
        let result = gate_relink_sources(&args, true);
        assert!(result.is_err());
    }

    #[test]
    fn test_gate_relink_sources_proceed() {
        let args = SkillsArgs {
            relink_sources: true,
            yes: false,
        };
        let result = gate_relink_sources(&args, false).expect("gate relink");
        assert_eq!(result, RelinkGate::Proceed);
    }
}

fn manage_targets(
    config: &crate::skills::types::SkillsConfig,
    paths: &SkillsPaths,
    registry: &mut Registry,
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

        manage_targets_for_skill(&selection, config, paths, registry)?;
    }
    Ok(())
}

fn manage_targets_for_skill(
    skill_id: &str,
    config: &crate::skills::types::SkillsConfig,
    paths: &SkillsPaths,
    registry: &mut Registry,
) -> Result<()> {
    loop {
        let entry = match registry.skills.get(skill_id) {
            Some(entry) => entry,
            None => return Ok(()),
        };
        let mut labels = Vec::new();
        let mut target_ids = Vec::new();
        for target in &config.targets {
            let enabled = entry
                .targets
                .get(&target.id)
                .copied()
                .unwrap_or(target.enabled);
            let marker = if enabled { "on" } else { "off" };
            labels.push(format!("{} - {}", marker, target.id));
            target_ids.push(target.id.clone());
        }
        let back_label = t!("skills.manager.back").to_string();
        labels.push(back_label.clone());

        let selection = Select::new(&t!("skills.manager.select_target"), labels)
            .prompt()
            .map_err(|e| anyhow::anyhow!(t!("skills.manager.prompt_failed", error = e)))?;
        if selection == back_label {
            break;
        }
        let selected_index = selection
            .split(" - ")
            .nth(1)
            .and_then(|id| target_ids.iter().position(|tid| tid == id));
        let Some(idx) = selected_index else {
            continue;
        };
        let target_id = &target_ids[idx];
        let Some(entry) = registry.skills.get_mut(skill_id) else {
            return Ok(());
        };
        let current = entry.targets.get(target_id).copied().unwrap_or(false);
        entry.targets.insert(target_id.clone(), !current);
        if let Some(target) = config.targets.iter().find(|t| t.id == *target_id) {
            apply_target_link(skill_id, &paths.store_dir, target, entry)?;
        }
    }
    Ok(())
}

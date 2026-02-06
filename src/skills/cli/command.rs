use crate::skills::catalog::registry::Registry;
use crate::skills::catalog::scan::discover_skills;
use crate::skills::catalog::types::{
    ConfigEntry, SkillCandidate, SkillsConfig, SkillsPaths, TargetConflictStrategy, TargetMode,
};
use crate::skills::cli::interactive::is_interactive;
use crate::skills::config::load_config;
use crate::skills::targets::sync::SkillSyncCancelled;
use crate::skills::targets::sync::{apply_target_diff, apply_target_links, is_skill_linked};
use anyhow::Result;
use chrono::Utc;
use clap::{Args, ValueEnum};
use inquire::error::InquireError;
use inquire::{Confirm, MultiSelect, Select};
use std::collections::{HashMap, HashSet};
use std::fmt;
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
        let Some(selected) = select_skills_for_target(&skills, &target, &config)? else {
            return Ok(());
        };
        if !confirm_apply(&target)? {
            return Ok(());
        }
        match apply_target_diff(&skills, &target, &selected, true, target_conflict) {
            Ok(()) => {}
            Err(e) if e.is::<SkillSyncCancelled>() => {
                println!("{}", t!("messages.operation_cancelled"));
                return Ok(());
            }
            Err(e) => return Err(e),
        }
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

fn select_target(config: &SkillsConfig) -> Result<Option<ConfigEntry>> {
    #[derive(Clone)]
    enum AgentSelection {
        Agent { choice: AgentChoice },
        Exit { label: String },
    }

    impl fmt::Display for AgentSelection {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Agent { choice } => write!(f, "{}", choice.label),
                Self::Exit { label } => write!(f, "{label}"),
            }
        }
    }

    #[derive(Clone)]
    enum ScopeSelection {
        Scope { choice: ScopeChoice },
        Exit { label: String },
    }

    impl fmt::Display for ScopeSelection {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Scope { choice } => write!(f, "{}", choice.label),
                Self::Exit { label } => write!(f, "{label}"),
            }
        }
    }

    loop {
        let mut agent_choices: Vec<AgentSelection> = selectable_agents(config)
            .into_iter()
            .map(|choice| AgentSelection::Agent { choice })
            .collect();
        let exit_label = format!("[{}]", t!("skills.manager.exit"));
        agent_choices.push(AgentSelection::Exit {
            label: exit_label.clone(),
        });

        let selection =
            match Select::new(&t!("skills.manager.select_agent_tools"), agent_choices).prompt() {
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

        let agent = match selection {
            AgentSelection::Exit { .. } => return Ok(None),
            AgentSelection::Agent { choice } => choice.agent,
        };

        let scope_choices = scopes_for_agent(config, &agent);
        if scope_choices.is_empty() {
            continue;
        }

        if agent == "agent" && scope_choices.len() == 1 {
            let target = scope_choices[0].target.clone();
            if target.mode == TargetMode::Skip {
                println!("{}", t!("skills.manager.read_only"));
                continue;
            }
            return Ok(Some(target));
        }

        let mut choices: Vec<ScopeSelection> = scope_choices
            .into_iter()
            .map(|choice| ScopeSelection::Scope { choice })
            .collect();
        choices.push(ScopeSelection::Exit { label: exit_label });

        let scope_selection =
            match Select::new(&t!("skills.manager.select_scope"), choices).prompt() {
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

        match scope_selection {
            ScopeSelection::Exit { .. } => return Ok(None),
            ScopeSelection::Scope { choice } => {
                if choice.target.mode == TargetMode::Skip {
                    println!("{}", t!("skills.manager.read_only"));
                    continue;
                }
                return Ok(Some(choice.target));
            }
        }
    }
}

#[derive(Clone)]
struct AgentChoice {
    agent: String,
    label: String,
}

#[derive(Clone)]
struct ScopeChoice {
    target: ConfigEntry,
    label: String,
}

fn selectable_agents(config: &SkillsConfig) -> Vec<AgentChoice> {
    let mut unique = HashSet::new();
    for target in &config.targets {
        unique.insert(target.agent.clone());
    }
    let mut agents = unique.into_iter().collect::<Vec<_>>();
    agents.sort_by(|a, b| agent_order(a).cmp(&agent_order(b)).then_with(|| a.cmp(b)));
    agents
        .into_iter()
        .map(|agent| AgentChoice {
            label: display_agent_label(&agent),
            agent,
        })
        .collect()
}

fn scopes_for_agent(config: &SkillsConfig, agent: &str) -> Vec<ScopeChoice> {
    let mut scopes: Vec<ScopeChoice> = config
        .targets
        .iter()
        .filter(|target| target.agent == agent)
        .map(|target| {
            let mut label = display_scope_label(agent, &target.scope);
            if target.mode == TargetMode::Skip {
                label = format!("{} - {}", t!("skills.manager.state_skip"), label);
            }
            ScopeChoice {
                target: target.clone(),
                label,
            }
        })
        .collect();

    scopes.sort_by(|a, b| {
        scope_order(&a.target.agent, &a.target.scope)
            .cmp(&scope_order(&b.target.agent, &b.target.scope))
            .then_with(|| a.label.cmp(&b.label))
            .then_with(|| a.target.id.cmp(&b.target.id))
    });

    let mut label_counts: HashMap<String, usize> = HashMap::new();
    for scope in &scopes {
        *label_counts.entry(scope.label.clone()).or_default() += 1;
    }
    for scope in &mut scopes {
        if label_counts.get(&scope.label).copied().unwrap_or(0) > 1 {
            scope.label = format!("{} ({})", scope.label, scope.target.id);
        }
    }

    scopes
}

fn display_agent_label(agent: &str) -> String {
    match agent {
        "agent" => "_agentskills_".to_string(),
        other => other.to_string(),
    }
}

fn display_scope_label(agent: &str, scope: &str) -> String {
    match (agent, scope) {
        ("claude", "user") => "Personal (All your projects)".to_string(),
        ("claude", "project") => "Project (This project only)".to_string(),
        ("codex", "user") => "User (All your projects)".to_string(),
        ("codex", "repo") => "Repo (This project only)".to_string(),
        ("agent", "global") => "Global".to_string(),
        _ => scope.to_string(),
    }
}

fn agent_order(agent: &str) -> u8 {
    match agent {
        "claude" => 0,
        "codex" => 1,
        "agent" => 2,
        _ => 3,
    }
}

fn scope_order(agent: &str, scope: &str) -> u8 {
    match (agent, scope) {
        ("claude", "user") => 0,
        ("claude", "project") => 1,
        ("codex", "user") => 0,
        ("codex", "repo") => 1,
        ("agent", "global") => 0,
        _ => 10,
    }
}

fn select_skills_for_target(
    skills: &[SkillCandidate],
    target: &crate::skills::catalog::types::ConfigEntry,
    config: &SkillsConfig,
) -> Result<Option<HashSet<String>>> {
    let visible_skills = visible_skills_for_target(skills, target, config);
    let hidden_count = skills.len().saturating_sub(visible_skills.len());
    if matches!(target.scope.as_str(), "project" | "repo") && hidden_count > 0 {
        println!(
            "{}",
            t!(
                "skills.manager.hidden_user_scope_skills_hint",
                count = hidden_count
            )
        );
    }
    let skill_ids: Vec<String> = visible_skills
        .iter()
        .map(|skill| skill.skill_id.clone())
        .collect();
    let defaults = default_skill_indexes(&visible_skills, target, config);
    let prompt = t!("skills.manager.select_skills");
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

fn visible_skills_for_target(
    skills: &[SkillCandidate],
    target: &ConfigEntry,
    config: &SkillsConfig,
) -> Vec<SkillCandidate> {
    if !matches!(target.scope.as_str(), "project" | "repo") {
        return skills.to_vec();
    }

    let Some(user_target) = config
        .targets
        .iter()
        .find(|entry| entry.agent == target.agent && entry.scope == "user")
    else {
        return skills.to_vec();
    };

    skills
        .iter()
        .filter(|skill| {
            let linked_in_target = is_skill_linked(skill, target);
            if linked_in_target {
                return true;
            }
            !is_skill_linked(skill, user_target)
        })
        .cloned()
        .collect()
}

fn default_skill_indexes(
    skills: &[SkillCandidate],
    target: &ConfigEntry,
    config: &SkillsConfig,
) -> Vec<usize> {
    let mut defaults = linked_skill_indexes(skills, target);
    if !defaults.is_empty() {
        return defaults;
    }

    if !matches!(target.scope.as_str(), "project" | "repo") {
        return defaults;
    }

    let Some(user_target) = config
        .targets
        .iter()
        .find(|entry| entry.agent == target.agent && entry.scope == "user")
    else {
        return defaults;
    };

    defaults = linked_skill_indexes(skills, user_target);
    defaults
}

fn linked_skill_indexes(skills: &[SkillCandidate], target: &ConfigEntry) -> Vec<usize> {
    let mut indexes = Vec::new();
    for (idx, skill) in skills.iter().enumerate() {
        if is_skill_linked(skill, target) {
            indexes.push(idx);
        }
    }
    indexes
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
    use crate::skills::catalog::types::{ConfigEntry, SkillsConfig};
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

    #[test]
    fn test_selectable_agents_uses_expected_order() {
        let config = SkillsConfig {
            targets: vec![
                ConfigEntry {
                    id: "claude_user".to_string(),
                    agent: "claude".to_string(),
                    scope: "user".to_string(),
                    path: PathBuf::from("/tmp/claude-user"),
                    enabled: true,
                    mode: TargetMode::Link,
                },
                ConfigEntry {
                    id: "codex_user".to_string(),
                    agent: "codex".to_string(),
                    scope: "user".to_string(),
                    path: PathBuf::from("/tmp/codex-user"),
                    enabled: true,
                    mode: TargetMode::Link,
                },
                ConfigEntry {
                    id: "agent_global".to_string(),
                    agent: "agent".to_string(),
                    scope: "global".to_string(),
                    path: PathBuf::from("/tmp/agent-global"),
                    enabled: true,
                    mode: TargetMode::Link,
                },
            ],
        };

        let agents = selectable_agents(&config);
        let labels: Vec<String> = agents.iter().map(|choice| choice.label.clone()).collect();
        assert_eq!(labels, vec!["claude", "codex", "_agentskills_"]);
    }

    #[test]
    fn test_scope_label_for_known_agent_and_scope() {
        assert_eq!(
            display_scope_label("claude", "user"),
            "Personal (All your projects)"
        );
        assert_eq!(
            display_scope_label("claude", "project"),
            "Project (This project only)"
        );
        assert_eq!(
            display_scope_label("codex", "user"),
            "User (All your projects)"
        );
        assert_eq!(
            display_scope_label("codex", "repo"),
            "Repo (This project only)"
        );
        assert_eq!(display_scope_label("agent", "global"), "Global");
    }

    #[test]
    fn test_scope_label_fallback_for_unknown_scope() {
        assert_eq!(display_scope_label("claude", "team"), "team");
        assert_eq!(display_scope_label("unknown", "custom"), "custom");
    }

    #[test]
    fn test_agent_scopes_filters_and_orders_expected_scopes() {
        let config = SkillsConfig {
            targets: vec![
                ConfigEntry {
                    id: "claude_project".to_string(),
                    agent: "claude".to_string(),
                    scope: "project".to_string(),
                    path: PathBuf::from("/tmp/claude-project"),
                    enabled: true,
                    mode: TargetMode::Link,
                },
                ConfigEntry {
                    id: "claude_user".to_string(),
                    agent: "claude".to_string(),
                    scope: "user".to_string(),
                    path: PathBuf::from("/tmp/claude-user"),
                    enabled: true,
                    mode: TargetMode::Link,
                },
                ConfigEntry {
                    id: "claude_team".to_string(),
                    agent: "claude".to_string(),
                    scope: "team".to_string(),
                    path: PathBuf::from("/tmp/claude-team"),
                    enabled: true,
                    mode: TargetMode::Link,
                },
            ],
        };

        let scopes = scopes_for_agent(&config, "claude");
        let labels: Vec<String> = scopes.iter().map(|choice| choice.label.clone()).collect();
        assert_eq!(
            labels,
            vec![
                "Personal (All your projects)".to_string(),
                "Project (This project only)".to_string(),
                "team".to_string()
            ]
        );
        assert_eq!(scopes[0].target.id, "claude_user");
        assert_eq!(scopes[1].target.id, "claude_project");
        assert_eq!(scopes[2].target.id, "claude_team");
    }

    #[test]
    fn test_default_skill_indexes_prefers_target_links() {
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

        let config = SkillsConfig {
            targets: vec![
                ConfigEntry {
                    id: "claude_user".to_string(),
                    agent: "claude".to_string(),
                    scope: "user".to_string(),
                    path: PathBuf::from("/tmp/claude-user"),
                    enabled: true,
                    mode: TargetMode::Link,
                },
                ConfigEntry {
                    id: "claude_project".to_string(),
                    agent: "claude".to_string(),
                    scope: "project".to_string(),
                    path: PathBuf::from("/tmp/claude-project"),
                    enabled: true,
                    mode: TargetMode::Link,
                },
            ],
        };

        let target = config
            .targets
            .iter()
            .find(|entry| entry.id == "claude_project")
            .expect("target");

        let defaults = default_skill_indexes(&skills, target, &config);
        assert!(defaults.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn test_default_skill_indexes_falls_back_to_user_links_for_project_scope() {
        use std::fs;
        use std::os::unix::fs as unix_fs;
        use tempfile::TempDir;

        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let alpha_dir = root.join("alpha");
        let beta_dir = root.join("beta");
        fs::create_dir_all(&alpha_dir).expect("alpha dir");
        fs::create_dir_all(&beta_dir).expect("beta dir");

        let claude_user = root.join("claude-user");
        let claude_project = root.join("claude-project");
        fs::create_dir_all(&claude_user).expect("claude user dir");
        fs::create_dir_all(&claude_project).expect("claude project dir");
        unix_fs::symlink(&alpha_dir, claude_user.join("alpha")).expect("link alpha");

        let skills = vec![
            SkillCandidate {
                skill_id: "alpha".to_string(),
                skill_dir: alpha_dir,
            },
            SkillCandidate {
                skill_id: "beta".to_string(),
                skill_dir: beta_dir,
            },
        ];

        let config = SkillsConfig {
            targets: vec![
                ConfigEntry {
                    id: "claude_user".to_string(),
                    agent: "claude".to_string(),
                    scope: "user".to_string(),
                    path: claude_user,
                    enabled: true,
                    mode: TargetMode::Link,
                },
                ConfigEntry {
                    id: "claude_project".to_string(),
                    agent: "claude".to_string(),
                    scope: "project".to_string(),
                    path: claude_project,
                    enabled: true,
                    mode: TargetMode::Link,
                },
            ],
        };

        let target = config
            .targets
            .iter()
            .find(|entry| entry.id == "claude_project")
            .expect("target");

        let defaults = default_skill_indexes(&skills, target, &config);
        assert_eq!(defaults, vec![0]);
    }

    #[cfg(unix)]
    #[test]
    fn test_visible_skills_excludes_user_linked_for_project_scope() {
        use std::fs;
        use std::os::unix::fs as unix_fs;
        use tempfile::TempDir;

        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let alpha_dir = root.join("alpha");
        let beta_dir = root.join("beta");
        fs::create_dir_all(&alpha_dir).expect("alpha dir");
        fs::create_dir_all(&beta_dir).expect("beta dir");

        let claude_user = root.join("claude-user");
        let claude_project = root.join("claude-project");
        fs::create_dir_all(&claude_user).expect("claude user dir");
        fs::create_dir_all(&claude_project).expect("claude project dir");
        unix_fs::symlink(&alpha_dir, claude_user.join("alpha")).expect("link alpha user");

        let skills = vec![
            SkillCandidate {
                skill_id: "alpha".to_string(),
                skill_dir: alpha_dir,
            },
            SkillCandidate {
                skill_id: "beta".to_string(),
                skill_dir: beta_dir,
            },
        ];

        let config = SkillsConfig {
            targets: vec![
                ConfigEntry {
                    id: "claude_user".to_string(),
                    agent: "claude".to_string(),
                    scope: "user".to_string(),
                    path: claude_user,
                    enabled: true,
                    mode: TargetMode::Link,
                },
                ConfigEntry {
                    id: "claude_project".to_string(),
                    agent: "claude".to_string(),
                    scope: "project".to_string(),
                    path: claude_project,
                    enabled: true,
                    mode: TargetMode::Link,
                },
            ],
        };

        let target = config
            .targets
            .iter()
            .find(|entry| entry.id == "claude_project")
            .expect("target");

        let visible = visible_skills_for_target(&skills, target, &config);
        let visible_ids: Vec<String> = visible.iter().map(|skill| skill.skill_id.clone()).collect();
        assert_eq!(visible_ids, vec!["beta".to_string()]);
    }

    #[cfg(unix)]
    #[test]
    fn test_visible_skills_keeps_project_linked_even_if_user_linked() {
        use std::fs;
        use std::os::unix::fs as unix_fs;
        use tempfile::TempDir;

        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let alpha_dir = root.join("alpha");
        let beta_dir = root.join("beta");
        fs::create_dir_all(&alpha_dir).expect("alpha dir");
        fs::create_dir_all(&beta_dir).expect("beta dir");

        let claude_user = root.join("claude-user");
        let claude_project = root.join("claude-project");
        fs::create_dir_all(&claude_user).expect("claude user dir");
        fs::create_dir_all(&claude_project).expect("claude project dir");
        unix_fs::symlink(&alpha_dir, claude_user.join("alpha")).expect("link alpha user");
        unix_fs::symlink(&alpha_dir, claude_project.join("alpha")).expect("link alpha project");

        let skills = vec![
            SkillCandidate {
                skill_id: "alpha".to_string(),
                skill_dir: alpha_dir,
            },
            SkillCandidate {
                skill_id: "beta".to_string(),
                skill_dir: beta_dir,
            },
        ];

        let config = SkillsConfig {
            targets: vec![
                ConfigEntry {
                    id: "claude_user".to_string(),
                    agent: "claude".to_string(),
                    scope: "user".to_string(),
                    path: claude_user,
                    enabled: true,
                    mode: TargetMode::Link,
                },
                ConfigEntry {
                    id: "claude_project".to_string(),
                    agent: "claude".to_string(),
                    scope: "project".to_string(),
                    path: claude_project,
                    enabled: true,
                    mode: TargetMode::Link,
                },
            ],
        };

        let target = config
            .targets
            .iter()
            .find(|entry| entry.id == "claude_project")
            .expect("target");

        let visible = visible_skills_for_target(&skills, target, &config);
        let visible_ids: Vec<String> = visible.iter().map(|skill| skill.skill_id.clone()).collect();
        assert_eq!(visible_ids, vec!["alpha".to_string(), "beta".to_string()]);
    }
}

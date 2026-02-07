use crate::skills::catalog::registry::{PresetEntry, Registry};
use crate::skills::catalog::scan::discover_skills;
use crate::skills::catalog::types::{
    ConfigEntry, SkillCandidate, SkillsConfig, SkillsPaths, TargetConflictStrategy, TargetMode,
};
use crate::skills::cli::interactive::is_interactive;
use crate::skills::cli::tui_picker;
use crate::skills::cli::tui_picker::{TuiEntry, TuiEntryKind};
use crate::skills::config::load_config;
use crate::skills::targets::sync::SkillSyncCancelled;
use crate::skills::targets::sync::{apply_target_diff, apply_target_links, is_skill_linked};
use anyhow::Result;
use chrono::Utc;
use clap::{Args, ValueEnum};
use inquire::error::InquireError;
use inquire::{Confirm, Select};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

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

#[derive(Clone, Debug)]
struct RuntimePreset {
    description: Option<String>,
    extends: Option<String>,
    skill_dirs: Vec<String>,
}

#[derive(Clone)]
struct PresetOption {
    name: String,
    skill_ids: Vec<String>,
    label: String,
}

impl fmt::Display for PresetOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label)
    }
}

#[derive(Clone)]
enum SkillOption {
    Preset(PresetOption),
    Skill { skill_id: String, label: String },
}

struct InteractiveSelection {
    target: ConfigEntry,
    selected: HashSet<String>,
}

#[derive(Clone, Debug, Default)]
struct SkillDirCatalog {
    dir_to_skill_id: HashMap<String, String>,
    skill_id_to_dir_name: HashMap<String, String>,
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
    let skill_dir_catalog = build_skill_dir_catalog(&paths.root, &skills)?;
    let runtime_presets = build_runtime_presets(&registry, &skill_dir_catalog);
    validate_runtime_presets(&runtime_presets, &skill_dir_catalog)?;

    if skills.is_empty() {
        println!("{}", t!("skills.manager.no_skills"));
        return Ok(());
    }

    if interactive {
        let Some(interactive_selection) =
            run_interactive_selection(&skills, &config, &runtime_presets, &skill_dir_catalog)?
        else {
            return Ok(());
        };
        if !confirm_apply(&interactive_selection.target)? {
            return Ok(());
        }
        match apply_target_diff(
            &skills,
            &interactive_selection.target,
            &interactive_selection.selected,
            true,
            target_conflict,
        ) {
            Ok(()) => {}
            Err(e) if e.is::<SkillSyncCancelled>() => {
                println!("{}", t!("messages.operation_cancelled"));
                return Ok(());
            }
            Err(e) => return Err(e),
        }
        update_registry_for_target(
            &mut registry,
            &skills,
            &interactive_selection.target,
            &interactive_selection.selected,
        );
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

fn run_interactive_selection(
    skills: &[SkillCandidate],
    config: &SkillsConfig,
    runtime_presets: &HashMap<String, RuntimePreset>,
    skill_dir_catalog: &SkillDirCatalog,
) -> Result<Option<InteractiveSelection>> {
    let Some(target) = select_target(config)? else {
        return Ok(None);
    };
    let Some(selected) =
        select_skills_for_target(skills, &target, config, skill_dir_catalog, runtime_presets)?
    else {
        return Ok(None);
    };
    Ok(Some(InteractiveSelection { target, selected }))
}

fn preset_display_label(name: &str, count: usize, description: Option<&str>) -> String {
    match description {
        Some(description) if !description.trim().is_empty() => {
            format!("{} ({} skills) - {}", name, count, description)
        }
        _ => format!("{} ({} skills)", name, count),
    }
}

fn build_runtime_presets(
    registry: &Registry,
    skill_dir_catalog: &SkillDirCatalog,
) -> HashMap<String, RuntimePreset> {
    if !registry.presets.is_empty() {
        return registry_presets_to_runtime(&registry.presets);
    }
    infer_runtime_presets_from_catalog(skill_dir_catalog)
}

fn registry_presets_to_runtime(
    presets: &HashMap<String, PresetEntry>,
) -> HashMap<String, RuntimePreset> {
    let mut out = HashMap::new();
    for (name, preset) in presets {
        out.insert(
            name.clone(),
            RuntimePreset {
                description: preset.description.clone(),
                extends: preset.extends.clone(),
                skill_dirs: preset.skill_dirs.clone(),
            },
        );
    }
    out
}

fn infer_runtime_presets_from_catalog(
    skill_dir_catalog: &SkillDirCatalog,
) -> HashMap<String, RuntimePreset> {
    let mut out = HashMap::<String, RuntimePreset>::new();
    for dir_name in skill_dir_catalog.dir_to_skill_id.keys() {
        let Some((preset_name, _)) = dir_name.split_once('.') else {
            continue;
        };
        let entry = out
            .entry(preset_name.to_string())
            .or_insert_with(|| RuntimePreset {
                description: None,
                extends: None,
                skill_dirs: Vec::new(),
            });
        if !entry.skill_dirs.iter().any(|existing| existing == dir_name) {
            entry.skill_dirs.push(dir_name.to_string());
        }
    }
    out
}

fn validate_runtime_presets(
    presets: &HashMap<String, RuntimePreset>,
    skill_dir_catalog: &SkillDirCatalog,
) -> Result<()> {
    let known_dirs: HashSet<String> = skill_dir_catalog.dir_to_skill_id.keys().cloned().collect();

    for (name, preset) in presets {
        if let Some(parent) = &preset.extends
            && !presets.contains_key(parent)
        {
            return Err(anyhow::anyhow!(t!(
                "skills.manager.preset_missing_parent",
                preset = name,
                parent = parent
            )));
        }
        for dir in &preset.skill_dirs {
            if !known_dirs.contains(dir) {
                return Err(anyhow::anyhow!(t!(
                    "skills.manager.preset_unknown_skill_dir",
                    preset = name,
                    dir = dir
                )));
            }
        }
        let resolved = resolve_preset_skill_dirs(presets, name, &mut Vec::new())?;
        if resolved.is_empty() {
            return Err(anyhow::anyhow!(t!(
                "skills.manager.preset_empty",
                preset = name
            )));
        }
    }
    Ok(())
}

fn resolve_preset_skill_ids(
    presets: &HashMap<String, RuntimePreset>,
    preset_name: &str,
    skill_dir_catalog: &SkillDirCatalog,
) -> Result<Vec<String>> {
    let dirs = resolve_preset_skill_dirs(presets, preset_name, &mut Vec::new())?;
    let mut out = Vec::new();
    for dir in dirs {
        if let Some(skill_id) = skill_dir_catalog.dir_to_skill_id.get(dir.as_str())
            && !out.iter().any(|existing| existing == skill_id)
        {
            out.push(skill_id.clone());
        }
    }
    Ok(out)
}

fn build_skill_dir_catalog(root: &Path, skills: &[SkillCandidate]) -> Result<SkillDirCatalog> {
    let root_skill_dirs = discover_root_skill_dirs(root)?;
    if root_skill_dirs.is_empty() {
        return Ok(SkillDirCatalog::default());
    }

    let mut canonical_to_skill_id: HashMap<PathBuf, String> = HashMap::new();
    for skill in skills {
        let Ok(canonical) = fs::canonicalize(&skill.skill_dir) else {
            continue;
        };
        canonical_to_skill_id
            .entry(canonical)
            .or_insert_with(|| skill.skill_id.clone());
    }

    let mut dir_to_skill_id = HashMap::new();
    for (dir_name, canonical_path) in root_skill_dirs {
        if let Some(skill_id) = canonical_to_skill_id.get(&canonical_path) {
            dir_to_skill_id.insert(dir_name, skill_id.clone());
        }
    }

    let mut skill_id_to_dir_name = HashMap::new();
    for (dir_name, skill_id) in &dir_to_skill_id {
        match skill_id_to_dir_name.get(skill_id) {
            None => {
                skill_id_to_dir_name.insert(skill_id.clone(), dir_name.clone());
            }
            Some(existing) if is_better_dir_name(dir_name, existing) => {
                skill_id_to_dir_name.insert(skill_id.clone(), dir_name.clone());
            }
            _ => {}
        }
    }

    Ok(SkillDirCatalog {
        dir_to_skill_id,
        skill_id_to_dir_name,
    })
}

fn is_better_dir_name(candidate: &str, existing: &str) -> bool {
    match (candidate.contains('.'), existing.contains('.')) {
        (true, false) => true,
        (false, true) => false,
        _ => candidate < existing,
    }
}

fn discover_root_skill_dirs(root: &Path) -> Result<HashMap<String, PathBuf>> {
    let mut out = HashMap::new();
    if !root.exists() {
        return Ok(out);
    }

    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if dir_name == "store" {
            continue;
        }

        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        let is_dir = if meta.file_type().is_symlink() {
            fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false)
        } else {
            meta.is_dir()
        };
        if !is_dir {
            continue;
        }

        let skill_file = path.join("SKILL.md");
        if !fs::metadata(&skill_file)
            .map(|m| m.is_file())
            .unwrap_or(false)
        {
            continue;
        }

        let Ok(canonical_path) = fs::canonicalize(&path) else {
            continue;
        };
        out.insert(dir_name, canonical_path);
    }

    Ok(out)
}

fn resolve_preset_skill_dirs(
    presets: &HashMap<String, RuntimePreset>,
    preset_name: &str,
    visiting: &mut Vec<String>,
) -> Result<Vec<String>> {
    if visiting.iter().any(|name| name == preset_name) {
        let mut chain = visiting.clone();
        chain.push(preset_name.to_string());
        return Err(anyhow::anyhow!(t!(
            "skills.manager.preset_cycle",
            chain = chain.join(" -> ")
        )));
    }

    let preset = presets.get(preset_name).ok_or_else(|| {
        anyhow::anyhow!(t!("skills.manager.preset_not_found", preset = preset_name))
    })?;

    visiting.push(preset_name.to_string());
    let mut out = Vec::new();

    if let Some(parent) = &preset.extends {
        out.extend(resolve_preset_skill_dirs(presets, parent, visiting)?);
    }

    for dir in &preset.skill_dirs {
        if !out.iter().any(|existing| existing == dir) {
            out.push(dir.clone());
        }
    }

    visiting.pop();
    Ok(out)
}

fn skill_dir_name(skill: &SkillCandidate) -> Option<&str> {
    skill.skill_dir.file_name()?.to_str()
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
    skill_dir_catalog: &SkillDirCatalog,
    runtime_presets: &HashMap<String, RuntimePreset>,
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
    let options = grouped_skill_options(&visible_skills, skill_dir_catalog, runtime_presets)?;
    let default_skill_indexes = default_skill_indexes(&visible_skills, &options, target, config);
    let defaults = default_indexes_with_preset_state(&options, &default_skill_indexes);
    let default_selected_skills = selected_skill_ids_from_indexes(&options, &defaults);

    let tui_entries = options_to_tui_entries(&options);
    tui_picker::pick(
        &t!("skills.manager.select_skills"),
        &tui_entries,
        &default_selected_skills,
    )
}

fn options_to_tui_entries(options: &[SkillOption]) -> Vec<TuiEntry> {
    options
        .iter()
        .map(|option| match option {
            SkillOption::Preset(PresetOption {
                skill_ids, label, ..
            }) => TuiEntry {
                label: label.clone(),
                kind: TuiEntryKind::Preset {
                    skill_ids: skill_ids.clone(),
                },
            },
            SkillOption::Skill { skill_id, label } => TuiEntry {
                label: label.clone(),
                kind: TuiEntryKind::Skill {
                    skill_id: skill_id.clone(),
                },
            },
        })
        .collect()
}

fn grouped_skill_options(
    skills: &[SkillCandidate],
    skill_dir_catalog: &SkillDirCatalog,
    runtime_presets: &HashMap<String, RuntimePreset>,
) -> Result<Vec<SkillOption>> {
    let mut out = preset_options_from_skills(skills, skill_dir_catalog, runtime_presets)?;

    let mut entries: Vec<(String, String)> = skills
        .iter()
        .map(|skill| {
            let dir_name = display_dir_name_for_skill(skill, skill_dir_catalog);
            let label = format_skill_option_label(&skill.skill_id, &dir_name);
            (skill.skill_id.clone(), label)
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    for (skill_id, label) in entries {
        out.push(SkillOption::Skill { skill_id, label });
    }

    Ok(out)
}

fn default_indexes_with_preset_state(
    options: &[SkillOption],
    default_skill_indexes: &[usize],
) -> Vec<usize> {
    let selected_skill_ids = selected_skill_ids_from_indexes(options, default_skill_indexes);

    let mut out = default_skill_indexes.to_vec();
    for (idx, option) in options.iter().enumerate() {
        if let SkillOption::Preset(PresetOption { skill_ids, .. }) = option
            && !skill_ids.is_empty()
            && skill_ids
                .iter()
                .all(|skill_id| selected_skill_ids.contains(skill_id))
        {
            out.push(idx);
        }
    }

    out.sort_unstable();
    out.dedup();
    out
}

fn selected_skill_ids_from_indexes(options: &[SkillOption], indexes: &[usize]) -> HashSet<String> {
    indexes
        .iter()
        .filter_map(|index| match options.get(*index) {
            Some(SkillOption::Skill { skill_id, .. }) => Some(skill_id.clone()),
            _ => None,
        })
        .collect()
}

fn preset_options_from_skills(
    skills: &[SkillCandidate],
    skill_dir_catalog: &SkillDirCatalog,
    runtime_presets: &HashMap<String, RuntimePreset>,
) -> Result<Vec<SkillOption>> {
    let mut preset_names = runtime_presets.keys().cloned().collect::<Vec<_>>();
    preset_names.sort();

    let visible_skill_ids: HashSet<&str> =
        skills.iter().map(|skill| skill.skill_id.as_str()).collect();
    let mut out = Vec::new();
    for preset_name in preset_names {
        let Some(preset) = runtime_presets.get(&preset_name) else {
            continue;
        };
        let resolved_ids =
            resolve_preset_skill_ids(runtime_presets, &preset_name, skill_dir_catalog)?;
        let skill_ids = resolved_ids
            .into_iter()
            .filter(|skill_id| visible_skill_ids.contains(skill_id.as_str()))
            .collect::<Vec<_>>();
        if skill_ids.is_empty() {
            continue;
        }
        out.push(SkillOption::Preset(PresetOption {
            name: preset_name.clone(),
            skill_ids,
            label: preset_display_label(
                &preset_name,
                preset.skill_dirs.len(),
                preset.description.as_deref(),
            ),
        }));
    }

    let grouped = infer_group_presets(skills, skill_dir_catalog);
    for option in grouped {
        if out
            .iter()
            .any(|existing| matches!(existing, SkillOption::Preset(existing) if existing.name == option.name))
        {
            continue;
        }
        out.push(SkillOption::Preset(option));
    }

    out.sort_by(|a, b| match (a, b) {
        (SkillOption::Preset(a), SkillOption::Preset(b)) => a.name.cmp(&b.name),
        (SkillOption::Preset(_), SkillOption::Skill { .. }) => std::cmp::Ordering::Less,
        (SkillOption::Skill { .. }, SkillOption::Preset(_)) => std::cmp::Ordering::Greater,
        (SkillOption::Skill { label: a, .. }, SkillOption::Skill { label: b, .. }) => a.cmp(b),
    });

    Ok(out)
}

fn infer_group_presets(
    skills: &[SkillCandidate],
    skill_dir_catalog: &SkillDirCatalog,
) -> Vec<PresetOption> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for skill in skills {
        let dir_name = display_dir_name_for_skill(skill, skill_dir_catalog);
        let group =
            infer_skill_group_from_dir(&dir_name).unwrap_or_else(|| "ungrouped".to_string());
        groups
            .entry(group)
            .or_default()
            .push(skill.skill_id.clone());
    }

    let mut group_names = groups.keys().cloned().collect::<Vec<_>>();
    group_names.sort_by(|a, b| {
        if a == "ungrouped" {
            return std::cmp::Ordering::Greater;
        }
        if b == "ungrouped" {
            return std::cmp::Ordering::Less;
        }
        a.cmp(b)
    });

    let mut out = Vec::new();
    for group_name in group_names {
        let Some(mut skill_ids) = groups.remove(&group_name) else {
            continue;
        };
        skill_ids.sort();
        skill_ids.dedup();
        out.push(PresetOption {
            name: group_name.clone(),
            label: preset_display_label(&group_name, skill_ids.len(), None),
            skill_ids,
        });
    }

    out
}

fn infer_skill_group_from_dir(dir_name: &str) -> Option<String> {
    dir_name
        .split_once('.')
        .map(|(group, _)| group.to_string())
        .filter(|group| !group.is_empty())
}

fn display_dir_name_for_skill(
    skill: &SkillCandidate,
    skill_dir_catalog: &SkillDirCatalog,
) -> String {
    skill_dir_catalog
        .skill_id_to_dir_name
        .get(&skill.skill_id)
        .cloned()
        .or_else(|| skill_dir_name(skill).map(ToOwned::to_owned))
        .unwrap_or_else(|| skill.skill_id.clone())
}

fn format_skill_option_label(skill_id: &str, dir_name: &str) -> String {
    format!("{} ({})", skill_id, dir_name)
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
    options: &[SkillOption],
    target: &ConfigEntry,
    config: &SkillsConfig,
) -> Vec<usize> {
    let mut defaults = linked_skill_indexes(skills, target);
    if !defaults.is_empty() {
        return map_skill_defaults_to_option_indexes(skills, options, &defaults);
    }

    if !matches!(target.scope.as_str(), "project" | "repo") {
        return map_skill_defaults_to_option_indexes(skills, options, &defaults);
    }

    let Some(user_target) = config
        .targets
        .iter()
        .find(|entry| entry.agent == target.agent && entry.scope == "user")
    else {
        return map_skill_defaults_to_option_indexes(skills, options, &defaults);
    };

    defaults = linked_skill_indexes(skills, user_target);
    map_skill_defaults_to_option_indexes(skills, options, &defaults)
}

fn map_skill_defaults_to_option_indexes(
    skills: &[SkillCandidate],
    options: &[SkillOption],
    skill_indexes: &[usize],
) -> Vec<usize> {
    let selected_skill_ids: HashSet<&str> = skill_indexes
        .iter()
        .filter_map(|idx| skills.get(*idx))
        .map(|skill| skill.skill_id.as_str())
        .collect();
    options
        .iter()
        .enumerate()
        .filter_map(|(idx, option)| match option {
            SkillOption::Skill { skill_id, .. }
                if selected_skill_ids.contains(skill_id.as_str()) =>
            {
                Some(idx)
            }
            _ => None,
        })
        .collect()
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

        let catalog = SkillDirCatalog::default();
        let options = grouped_skill_options(&skills, &catalog, &HashMap::new()).expect("options");

        let defaults = default_skill_indexes(&skills, &options, target, &config);
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

        let catalog = SkillDirCatalog::default();
        let options = grouped_skill_options(&skills, &catalog, &HashMap::new()).expect("options");

        let defaults = default_skill_indexes(&skills, &options, target, &config);
        assert_eq!(defaults, vec![1]);
    }

    #[test]
    fn test_infer_runtime_presets_from_skills() {
        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([
                (
                    "superpowers.brainstorming".to_string(),
                    "brainstorming".to_string(),
                ),
                (
                    "superpowers.writing-plans".to_string(),
                    "writing-plans".to_string(),
                ),
                ("mermaid-expert".to_string(), "mermaid-expert".to_string()),
            ]),
            skill_id_to_dir_name: HashMap::from([
                (
                    "brainstorming".to_string(),
                    "superpowers.brainstorming".to_string(),
                ),
                (
                    "writing-plans".to_string(),
                    "superpowers.writing-plans".to_string(),
                ),
                ("mermaid-expert".to_string(), "mermaid-expert".to_string()),
            ]),
        };

        let inferred = infer_runtime_presets_from_catalog(&catalog);
        let superpowers = inferred.get("superpowers").expect("superpowers preset");
        assert_eq!(superpowers.skill_dirs.len(), 2);
        assert!(
            superpowers
                .skill_dirs
                .contains(&"superpowers.brainstorming".to_string())
        );
        assert!(
            superpowers
                .skill_dirs
                .contains(&"superpowers.writing-plans".to_string())
        );
        assert!(!inferred.contains_key("mermaid-expert"));
    }

    #[test]
    fn test_resolve_preset_skill_dirs_extends_and_dedupes() {
        let mut presets = HashMap::new();
        presets.insert(
            "daily".to_string(),
            RuntimePreset {
                description: None,
                extends: None,
                skill_dirs: vec!["superpowers.brainstorming".to_string()],
            },
        );
        presets.insert(
            "full-stack".to_string(),
            RuntimePreset {
                description: None,
                extends: Some("daily".to_string()),
                skill_dirs: vec![
                    "superpowers.brainstorming".to_string(),
                    "vercel-labs.react-best-practices".to_string(),
                ],
            },
        );

        let resolved = resolve_preset_skill_dirs(&presets, "full-stack", &mut Vec::new())
            .expect("resolve preset");
        assert_eq!(
            resolved,
            vec![
                "superpowers.brainstorming".to_string(),
                "vercel-labs.react-best-practices".to_string()
            ]
        );
    }

    #[test]
    fn test_validate_runtime_presets_detects_cycle() {
        let mut presets = HashMap::new();
        presets.insert(
            "a".to_string(),
            RuntimePreset {
                description: None,
                extends: Some("b".to_string()),
                skill_dirs: vec!["a.alpha".to_string()],
            },
        );
        presets.insert(
            "b".to_string(),
            RuntimePreset {
                description: None,
                extends: Some("a".to_string()),
                skill_dirs: vec!["b.beta".to_string()],
            },
        );
        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([
                ("a.alpha".to_string(), "alpha".to_string()),
                ("b.beta".to_string(), "beta".to_string()),
            ]),
            skill_id_to_dir_name: HashMap::from([
                ("alpha".to_string(), "a.alpha".to_string()),
                ("beta".to_string(), "b.beta".to_string()),
            ]),
        };

        let err = validate_runtime_presets(&presets, &catalog).expect_err("cycle error");
        assert!(
            err.to_string()
                .contains("Circular preset dependency detected")
        );
    }

    #[test]
    fn test_validate_runtime_presets_detects_unknown_skill_dir() {
        let mut presets = HashMap::new();
        presets.insert(
            "daily".to_string(),
            RuntimePreset {
                description: None,
                extends: None,
                skill_dirs: vec!["missing.skill".to_string()],
            },
        );
        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([("alpha".to_string(), "alpha".to_string())]),
            skill_id_to_dir_name: HashMap::from([("alpha".to_string(), "alpha".to_string())]),
        };

        let err = validate_runtime_presets(&presets, &catalog).expect_err("unknown dir error");
        assert!(err.to_string().contains("references unknown skill dir"));
    }

    #[test]
    fn test_validate_runtime_presets_detects_empty_preset() {
        let mut presets = HashMap::new();
        presets.insert(
            "daily".to_string(),
            RuntimePreset {
                description: None,
                extends: None,
                skill_dirs: vec![],
            },
        );
        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([("alpha".to_string(), "alpha".to_string())]),
            skill_id_to_dir_name: HashMap::from([("alpha".to_string(), "alpha".to_string())]),
        };

        let err = validate_runtime_presets(&presets, &catalog).expect_err("empty preset error");
        assert!(err.to_string().contains("resolves to an empty skill set"));
    }

    #[test]
    fn test_resolve_preset_selections_uses_dir_catalog_mapping() {
        let presets = HashMap::from([(
            "superpowers".to_string(),
            RuntimePreset {
                description: None,
                extends: None,
                skill_dirs: vec!["superpowers.brainstorming".to_string()],
            },
        )]);

        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([(
                "superpowers.brainstorming".to_string(),
                "brainstorming".to_string(),
            )]),
            skill_id_to_dir_name: HashMap::from([(
                "brainstorming".to_string(),
                "superpowers.brainstorming".to_string(),
            )]),
        };

        let selected =
            resolve_preset_skill_ids(&presets, "superpowers", &catalog).expect("resolve selected");
        assert!(selected.contains(&"brainstorming".to_string()));
    }

    #[test]
    fn test_resolve_preset_selections_merges_and_dedupes() {
        let presets = HashMap::from([
            (
                "superpowers".to_string(),
                RuntimePreset {
                    description: None,
                    extends: None,
                    skill_dirs: vec![
                        "superpowers.brainstorming".to_string(),
                        "superpowers.writing-plans".to_string(),
                    ],
                },
            ),
            (
                "daily".to_string(),
                RuntimePreset {
                    description: None,
                    extends: None,
                    skill_dirs: vec![
                        "superpowers.brainstorming".to_string(),
                        "mermaid-expert".to_string(),
                    ],
                },
            ),
        ]);

        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([
                (
                    "superpowers.brainstorming".to_string(),
                    "brainstorming".to_string(),
                ),
                (
                    "superpowers.writing-plans".to_string(),
                    "writing-plans".to_string(),
                ),
                ("mermaid-expert".to_string(), "mermaid-expert".to_string()),
            ]),
            skill_id_to_dir_name: HashMap::from([
                (
                    "brainstorming".to_string(),
                    "superpowers.brainstorming".to_string(),
                ),
                (
                    "writing-plans".to_string(),
                    "superpowers.writing-plans".to_string(),
                ),
                ("mermaid-expert".to_string(), "mermaid-expert".to_string()),
            ]),
        };

        let mut selected = HashSet::new();
        for preset_name in ["superpowers", "daily"] {
            let resolved =
                resolve_preset_skill_ids(&presets, preset_name, &catalog).expect("resolve preset");
            for skill_id in resolved {
                selected.insert(skill_id);
            }
        }

        assert_eq!(selected.len(), 3);
        assert!(selected.contains("brainstorming"));
        assert!(selected.contains("writing-plans"));
        assert!(selected.contains("mermaid-expert"));
    }

    #[test]
    fn test_build_runtime_presets_prefers_dir_name_over_skill_id() {
        let registry = Registry::default();
        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([(
                "superpowers.brainstorming".to_string(),
                "brainstorming".to_string(),
            )]),
            skill_id_to_dir_name: HashMap::from([(
                "brainstorming".to_string(),
                "superpowers.brainstorming".to_string(),
            )]),
        };

        let presets = build_runtime_presets(&registry, &catalog);
        let superpowers = presets.get("superpowers").expect("superpowers preset");
        assert_eq!(
            superpowers.skill_dirs,
            vec!["superpowers.brainstorming".to_string()]
        );
    }

    #[test]
    fn test_grouped_skill_options_includes_group_headers() {
        let skills = vec![
            SkillCandidate {
                skill_id: "superpowers.brainstorming".to_string(),
                skill_dir: PathBuf::from("/tmp/superpowers.brainstorming"),
            },
            SkillCandidate {
                skill_id: "mermaid-expert".to_string(),
                skill_dir: PathBuf::from("/tmp/mermaid-expert"),
            },
        ];

        let catalog = SkillDirCatalog::default();
        let options = grouped_skill_options(&skills, &catalog, &HashMap::new()).expect("options");
        assert!(matches!(options.first(), Some(SkillOption::Preset(_))));
        let skill_count = options
            .iter()
            .filter(|option| matches!(option, SkillOption::Skill { .. }))
            .count();
        assert_eq!(skill_count, 2);

        let preset_labels: Vec<String> = options
            .iter()
            .filter_map(|option| match option {
                SkillOption::Preset(PresetOption { label, .. }) => Some(label.clone()),
                SkillOption::Skill { .. } => None,
            })
            .collect();
        assert!(
            preset_labels
                .iter()
                .any(|label| label == "superpowers (1 skills)")
        );

        let labels: Vec<String> = options
            .iter()
            .filter_map(|option| match option {
                SkillOption::Skill { label, .. } => Some(label.clone()),
                SkillOption::Preset(_) => None,
            })
            .collect();
        assert!(
            labels
                .iter()
                .any(|label| { label == "superpowers.brainstorming (superpowers.brainstorming)" })
        );
        assert!(
            labels
                .iter()
                .any(|label| label == "mermaid-expert (mermaid-expert)")
        );
    }

    #[test]
    fn test_grouped_skill_options_displays_mapped_dir_name() {
        let skills = vec![SkillCandidate {
            skill_id: "brainstorming".to_string(),
            skill_dir: PathBuf::from("/tmp/__submodules__/op7418.Humanizer-zh"),
        }];
        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([(
                "superpowers.brainstorming".to_string(),
                "brainstorming".to_string(),
            )]),
            skill_id_to_dir_name: HashMap::from([(
                "brainstorming".to_string(),
                "superpowers.brainstorming".to_string(),
            )]),
        };

        let options = grouped_skill_options(&skills, &catalog, &HashMap::new()).expect("options");
        let labels: Vec<String> = options
            .iter()
            .filter_map(|option| match option {
                SkillOption::Skill { label, .. } => Some(label.clone()),
                SkillOption::Preset(_) => None,
            })
            .collect();

        assert_eq!(
            labels,
            vec!["brainstorming (superpowers.brainstorming)".to_string()]
        );
    }

    #[test]
    fn test_default_indexes_with_preset_state_only_marks_fully_selected_presets() {
        let options = vec![
            SkillOption::Preset(PresetOption {
                name: "dakesan".to_string(),
                skill_ids: vec!["marimo-editor".to_string(), "marimo-inspect".to_string()],
                label: "dakesan (2 skills)".to_string(),
            }),
            SkillOption::Skill {
                skill_id: "marimo-editor".to_string(),
                label: "marimo-editor (dakesan.marimo-editor)".to_string(),
            },
            SkillOption::Skill {
                skill_id: "marimo-inspect".to_string(),
                label: "marimo-inspect (dakesan.marimo-inspect)".to_string(),
            },
            SkillOption::Preset(PresetOption {
                name: "astral-sh".to_string(),
                skill_ids: vec!["ruff".to_string()],
                label: "astral-sh (1 skills)".to_string(),
            }),
            SkillOption::Skill {
                skill_id: "ruff".to_string(),
                label: "ruff (astral-sh.ruff)".to_string(),
            },
        ];

        let indexes = default_indexes_with_preset_state(&options, &[1, 4]);
        assert_eq!(indexes, vec![1, 3, 4]);
    }

    #[test]
    fn test_grouped_skill_options_includes_runtime_and_group_presets() {
        let skills = vec![
            SkillCandidate {
                skill_id: "brainstorming".to_string(),
                skill_dir: PathBuf::from("/tmp/superpowers.brainstorming"),
            },
            SkillCandidate {
                skill_id: "mermaid-expert".to_string(),
                skill_dir: PathBuf::from("/tmp/mermaid-expert"),
            },
        ];
        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([
                (
                    "superpowers.brainstorming".to_string(),
                    "brainstorming".to_string(),
                ),
                ("mermaid-expert".to_string(), "mermaid-expert".to_string()),
            ]),
            skill_id_to_dir_name: HashMap::from([
                (
                    "brainstorming".to_string(),
                    "superpowers.brainstorming".to_string(),
                ),
                ("mermaid-expert".to_string(), "mermaid-expert".to_string()),
            ]),
        };
        let runtime_presets = HashMap::from([(
            "daily".to_string(),
            RuntimePreset {
                description: None,
                extends: None,
                skill_dirs: vec!["superpowers.brainstorming".to_string()],
            },
        )]);

        let options =
            grouped_skill_options(&skills, &catalog, &runtime_presets).expect("grouped options");
        let preset_labels: Vec<String> = options
            .iter()
            .filter_map(|option| match option {
                SkillOption::Preset(PresetOption { label, .. }) => Some(label.clone()),
                SkillOption::Skill { .. } => None,
            })
            .collect();

        assert!(
            preset_labels
                .iter()
                .any(|label| label == "daily (1 skills)")
        );
        assert!(
            preset_labels
                .iter()
                .any(|label| label == "superpowers (1 skills)")
        );
    }

    #[test]
    fn test_grouped_skill_options_runtime_preset_label_count_uses_direct_skill_dirs() {
        let skills = vec![
            SkillCandidate {
                skill_id: "brainstorming".to_string(),
                skill_dir: PathBuf::from("/tmp/superpowers.brainstorming"),
            },
            SkillCandidate {
                skill_id: "mermaid-expert".to_string(),
                skill_dir: PathBuf::from("/tmp/mermaid-expert"),
            },
        ];
        let catalog = SkillDirCatalog {
            dir_to_skill_id: HashMap::from([
                (
                    "superpowers.brainstorming".to_string(),
                    "brainstorming".to_string(),
                ),
                ("mermaid-expert".to_string(), "mermaid-expert".to_string()),
            ]),
            skill_id_to_dir_name: HashMap::from([
                (
                    "brainstorming".to_string(),
                    "superpowers.brainstorming".to_string(),
                ),
                ("mermaid-expert".to_string(), "mermaid-expert".to_string()),
            ]),
        };
        let runtime_presets = HashMap::from([
            (
                "base".to_string(),
                RuntimePreset {
                    description: None,
                    extends: None,
                    skill_dirs: vec!["superpowers.brainstorming".to_string()],
                },
            ),
            (
                "daily".to_string(),
                RuntimePreset {
                    description: None,
                    extends: Some("base".to_string()),
                    skill_dirs: vec!["mermaid-expert".to_string()],
                },
            ),
        ]);

        let options =
            grouped_skill_options(&skills, &catalog, &runtime_presets).expect("grouped options");
        let daily_label = options
            .iter()
            .find_map(|option| match option {
                SkillOption::Preset(PresetOption { name, label, .. }) if name == "daily" => {
                    Some(label.clone())
                }
                _ => None,
            })
            .expect("daily preset label");

        assert_eq!(daily_label, "daily (1 skills)");
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

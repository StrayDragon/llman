use crate::skills::hash::hash_skill_dir_filtered;
use crate::skills::registry::Registry;
use crate::skills::scan::discover_skills;
use crate::skills::types::{
    ConfigEntry, ConflictOption, SkillCandidate, SkillsConfig, SkillsPaths, SyncSummary,
    TargetConflictStrategy, TargetMode,
};
use anyhow::{Result, anyhow};
use chrono::Utc;
use inquire::Select;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub trait ConflictResolver {
    fn resolve(
        &mut self,
        skill_id: &str,
        options: &[ConflictOption],
        default_index: usize,
    ) -> Result<usize>;
}

pub struct InteractiveResolver {
    interactive: bool,
}

impl InteractiveResolver {
    pub fn new(interactive: bool) -> Self {
        Self { interactive }
    }
}

impl ConflictResolver for InteractiveResolver {
    fn resolve(
        &mut self,
        skill_id: &str,
        options: &[ConflictOption],
        default_index: usize,
    ) -> Result<usize> {
        if options.is_empty() {
            return Err(anyhow!(t!("skills.conflict.no_options", skill = skill_id)));
        }
        if !self.interactive {
            println!("{}", t!("skills.conflict.auto_selected", skill = skill_id));
            return Ok(default_index);
        }
        let mut labels = Vec::new();
        for option in options {
            labels.push(format!("{} ({})", option.hash, option.source_id));
        }
        let prompt = t!("skills.conflict.prompt", skill = skill_id);
        let selected = Select::new(&prompt, labels.clone())
            .with_starting_cursor(default_index)
            .prompt()
            .map_err(|e| anyhow!(t!("skills.conflict.prompt_failed", error = e)))?;
        let idx = labels
            .iter()
            .position(|label| label == &selected)
            .unwrap_or(default_index);
        Ok(idx)
    }
}

pub fn sync_sources(
    config: &SkillsConfig,
    paths: &SkillsPaths,
    registry: &mut Registry,
    resolver: &mut dyn ConflictResolver,
) -> Result<SyncSummary> {
    paths.ensure_dirs()?;
    let mut summary = SyncSummary::default();
    let mut by_skill: HashMap<String, Vec<SkillCandidate>> = HashMap::new();

    for source in &config.sources {
        if !source.enabled {
            continue;
        }
        if !source.path.exists() {
            eprintln!(
                "{}",
                t!("skills.source_missing", path = source.path.display())
            );
            continue;
        }
        summary.scanned_sources += 1;
        let discovered = discover_skills(source)?;
        summary.discovered_skills += discovered.len();
        for candidate in discovered {
            by_skill
                .entry(candidate.skill_id.clone())
                .or_default()
                .push(candidate);
        }
    }

    for (skill_id, candidates) in by_skill {
        if candidates.is_empty() {
            continue;
        }
        let options = unique_options(&candidates);
        let default_index = default_conflict_index(
            &options,
            &config.sources,
            registry
                .skills
                .get(&skill_id)
                .and_then(|entry| entry.current_hash.as_deref()),
        );
        let selected_index = if options.len() > 1 {
            summary.conflicts += 1;
            resolver.resolve(&skill_id, &options, default_index)?
        } else {
            default_index
        };
        let selected = options
            .get(selected_index)
            .unwrap_or_else(|| &options[default_index])
            .hash
            .clone();

        for candidate in &candidates {
            ensure_snapshot(
                &candidate.skill_dir,
                &paths.store_dir,
                &candidate.skill_id,
                &candidate.hash,
                registry,
                &mut summary,
            )?;
        }

        let entry = registry.ensure_skill(&skill_id);
        entry.current_hash = Some(selected.clone());
        ensure_target_defaults(entry, &config.targets);
        ensure_current_link(&paths.store_dir, &skill_id, &selected)?;
    }

    for (skill_id, entry) in registry.skills.iter_mut() {
        ensure_target_defaults(entry, &config.targets);
        if entry.current_hash.is_none()
            && let Some(hash) = entry.versions.keys().next()
        {
            entry.current_hash = Some(hash.to_string());
            ensure_current_link(&paths.store_dir, skill_id, hash)?;
        }
    }

    Ok(summary)
}

pub fn apply_target_links(
    config: &SkillsConfig,
    paths: &SkillsPaths,
    registry: &mut Registry,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    for (skill_id, entry) in registry.skills.iter() {
        let Some(hash) = entry.current_hash.as_ref() else {
            continue;
        };
        ensure_current_link(&paths.store_dir, skill_id, hash)?;
        for target in &config.targets {
            apply_target_link(
                skill_id,
                &paths.store_dir,
                target,
                entry,
                interactive,
                target_conflict,
            )?;
        }
    }
    Ok(())
}

pub fn apply_target_link(
    skill_id: &str,
    store_root: &Path,
    target: &ConfigEntry,
    entry: &crate::skills::registry::SkillEntry,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    let enabled = entry
        .targets
        .get(&target.id)
        .copied()
        .unwrap_or(target.enabled);

    match target.mode {
        TargetMode::Skip => Ok(()),
        TargetMode::Link => {
            if enabled {
                ensure_target_link(skill_id, store_root, target)?;
            } else {
                remove_target_link(skill_id, target)?;
            }
            Ok(())
        }
        TargetMode::Copy => {
            if enabled {
                ensure_target_copy(
                    skill_id,
                    store_root,
                    target,
                    entry,
                    interactive,
                    target_conflict,
                )?;
            } else {
                remove_target_copy(skill_id, target)?;
            }
            Ok(())
        }
    }
}

fn unique_options(candidates: &[SkillCandidate]) -> Vec<ConflictOption> {
    let mut seen = HashSet::new();
    let mut options = Vec::new();
    for candidate in candidates {
        if seen.insert(candidate.hash.clone()) {
            options.push(ConflictOption {
                hash: candidate.hash.clone(),
                source_id: candidate.source_id.clone(),
                source_path: candidate.source_path.clone(),
                skill_dir: candidate.skill_dir.clone(),
            });
        }
    }
    options
}

fn default_conflict_index(
    options: &[ConflictOption],
    sources: &[ConfigEntry],
    current_hash: Option<&str>,
) -> usize {
    if let Some(hash) = current_hash
        && let Some(pos) = options.iter().position(|opt| opt.hash == hash)
    {
        return pos;
    }
    let source_map: HashMap<&str, &ConfigEntry> =
        sources.iter().map(|s| (s.id.as_str(), s)).collect();
    let mut best_idx = 0;
    let mut best_score = i32::MIN;
    for (idx, option) in options.iter().enumerate() {
        let score = source_map
            .get(option.source_id.as_str())
            .map(|entry| source_priority(entry))
            .unwrap_or(0);
        if score > best_score {
            best_score = score;
            best_idx = idx;
        }
    }
    best_idx
}

fn source_priority(entry: &ConfigEntry) -> i32 {
    let agent_score = match entry.agent.as_str() {
        "claude" => 30,
        "codex" => 20,
        "agent" => 10,
        _ => 0,
    };
    let scope_bonus = match entry.scope.as_str() {
        "repo" => 2,
        _ => 0,
    };
    agent_score + scope_bonus
}

fn ensure_snapshot(
    skill_dir: &Path,
    store_root: &Path,
    skill_id: &str,
    hash: &str,
    registry: &mut Registry,
    summary: &mut SyncSummary,
) -> Result<()> {
    let version_dir = store_root.join(skill_id).join("versions").join(hash);
    if version_dir.exists() {
        registry.ensure_version(skill_id, hash);
        return Ok(());
    }
    fs::create_dir_all(&version_dir)?;
    copy_dir_filtered(skill_dir, &version_dir)?;
    registry.ensure_version(skill_id, hash);
    summary.imported_versions += 1;
    Ok(())
}

fn copy_dir_filtered(source: &Path, dest: &Path) -> Result<()> {
    let walker = ignore::WalkBuilder::new(source)
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .build();
    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        if entry
            .file_type()
            .is_some_and(|file_type| file_type.is_symlink())
        {
            continue;
        }
        if path.is_file() {
            let rel = path.strip_prefix(source).map_err(|_| {
                anyhow!(
                    "{} {}",
                    t!("skills.copy.strip_prefix_failed"),
                    path.display()
                )
            })?;
            let target = dest.join(rel);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(path, &target)?;
        }
    }
    Ok(())
}

fn ensure_current_link(store_root: &Path, skill_id: &str, hash: &str) -> Result<()> {
    let skill_root = store_root.join(skill_id);
    fs::create_dir_all(&skill_root)?;
    let current = skill_root.join("current");
    let target = skill_root.join("versions").join(hash);
    if current.exists() {
        remove_path(&current)?;
    }
    create_symlink(&target, &current)?;
    Ok(())
}

fn ensure_target_defaults(
    entry: &mut crate::skills::registry::SkillEntry,
    targets: &[ConfigEntry],
) {
    for target in targets {
        entry
            .targets
            .entry(target.id.clone())
            .or_insert(target.enabled);
    }
}

fn ensure_target_link(skill_id: &str, store_root: &Path, target: &ConfigEntry) -> Result<()> {
    if !target.path.exists() {
        fs::create_dir_all(&target.path)?;
    } else if !target.path.is_dir() {
        eprintln!(
            "{}",
            t!("skills.target.not_directory", path = target.path.display())
        );
        return Ok(());
    }
    let link_path = target.path.join(skill_id);
    let store_current = store_root.join(skill_id).join("current");
    if link_path.exists() {
        if fs::symlink_metadata(&link_path)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
        {
            let existing = fs::read_link(&link_path).ok();
            if existing.as_ref() == Some(&store_current) {
                return Ok(());
            }
            remove_path(&link_path)?;
        } else {
            eprintln!(
                "{}",
                t!("skills.target.not_symlink", path = link_path.display())
            );
            return Ok(());
        }
    }
    create_symlink(&store_current, &link_path)?;
    Ok(())
}

fn remove_target_link(skill_id: &str, target: &ConfigEntry) -> Result<()> {
    let link_path = target.path.join(skill_id);
    if !link_path.exists() {
        return Ok(());
    }
    if fs::symlink_metadata(&link_path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
    {
        remove_path(&link_path)?;
    } else {
        eprintln!(
            "{}",
            t!("skills.target.not_symlink", path = link_path.display())
        );
    }
    Ok(())
}

const TARGET_METADATA_FILE: &str = ".llman-skill.json";

#[derive(Debug, Serialize, Deserialize)]
struct ManagedSkillMeta {
    skill_id: String,
    hash: String,
    updated_at: String,
    last_written_hash: String,
    managed_by: String,
}

fn ensure_target_copy(
    skill_id: &str,
    store_root: &Path,
    target: &ConfigEntry,
    entry: &crate::skills::registry::SkillEntry,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    let Some(hash) = entry.current_hash.as_ref() else {
        return Ok(());
    };
    if !target.path.exists() {
        fs::create_dir_all(&target.path)?;
    } else if !target.path.is_dir() {
        eprintln!(
            "{}",
            t!("skills.target.not_directory", path = target.path.display())
        );
        return Ok(());
    }

    let dest_dir = target.path.join(skill_id);
    let source_dir = store_root.join(skill_id).join("versions").join(hash);

    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir)?;
        copy_dir_filtered(&source_dir, &dest_dir)?;
        write_target_metadata(skill_id, hash, &dest_dir)?;
        return Ok(());
    }

    let meta = read_target_metadata(&dest_dir)?;
    if let Some(meta) = meta {
        let current_hash = hash_skill_dir_filtered(&dest_dir, &[TARGET_METADATA_FILE])?;
        if current_hash != meta.last_written_hash {
            let decision = resolve_target_conflict(skill_id, target, interactive, target_conflict)?;
            if decision == TargetConflictStrategy::Skip {
                return Ok(());
            }
        } else if meta.hash == *hash {
            return Ok(());
        }
    } else {
        let decision = resolve_target_conflict(skill_id, target, interactive, target_conflict)?;
        if decision == TargetConflictStrategy::Skip {
            return Ok(());
        }
    }

    remove_path(&dest_dir)?;
    fs::create_dir_all(&dest_dir)?;
    copy_dir_filtered(&source_dir, &dest_dir)?;
    write_target_metadata(skill_id, hash, &dest_dir)?;

    Ok(())
}

fn remove_target_copy(skill_id: &str, target: &ConfigEntry) -> Result<()> {
    let dest_dir = target.path.join(skill_id);
    if !dest_dir.exists() {
        return Ok(());
    }
    if read_target_metadata(&dest_dir)?.is_some() {
        remove_path(&dest_dir)?;
    } else {
        eprintln!(
            "{}",
            t!("skills.target.not_managed", path = dest_dir.display())
        );
    }
    Ok(())
}

fn resolve_target_conflict(
    skill_id: &str,
    target: &ConfigEntry,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<TargetConflictStrategy> {
    if let Some(conflict) = target_conflict {
        return Ok(conflict);
    }
    if !interactive {
        return Err(anyhow!(t!(
            "skills.target_conflict.requires_flag",
            skill = skill_id,
            target = target.id
        )));
    }
    let prompt = t!(
        "skills.target_conflict.prompt",
        skill = skill_id,
        target = target.id
    );
    let overwrite_label = t!("skills.target_conflict.option_overwrite").to_string();
    let skip_label = t!("skills.target_conflict.option_skip").to_string();
    let selection = Select::new(&prompt, vec![overwrite_label.clone(), skip_label.clone()])
        .prompt()
        .map_err(|e| anyhow!(t!("skills.target_conflict.prompt_failed", error = e)))?;
    if selection == overwrite_label {
        Ok(TargetConflictStrategy::Overwrite)
    } else {
        Ok(TargetConflictStrategy::Skip)
    }
}

fn read_target_metadata(target_dir: &Path) -> Result<Option<ManagedSkillMeta>> {
    let path = target_dir.join(TARGET_METADATA_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    match serde_json::from_str::<ManagedSkillMeta>(&content) {
        Ok(meta) => Ok(Some(meta)),
        Err(err) => {
            eprintln!(
                "{}",
                t!(
                    "skills.target.metadata_invalid",
                    path = path.display(),
                    error = err
                )
            );
            Ok(None)
        }
    }
}

fn write_target_metadata(skill_id: &str, hash: &str, target_dir: &Path) -> Result<()> {
    let meta = ManagedSkillMeta {
        skill_id: skill_id.to_string(),
        hash: hash.to_string(),
        updated_at: Utc::now().to_rfc3339(),
        last_written_hash: hash.to_string(),
        managed_by: "llman".to_string(),
    };
    let path = target_dir.join(TARGET_METADATA_FILE);
    let content = serde_json::to_string_pretty(&meta)?;
    fs::write(path, content)?;
    Ok(())
}

fn remove_path(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || meta.is_file() {
        fs::remove_file(path)?;
    } else if meta.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

#[cfg(unix)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::unix::fs::symlink(target, link)?;
    Ok(())
}

#[cfg(windows)]
fn create_symlink(target: &Path, link: &Path) -> Result<()> {
    std::os::windows::fs::symlink_dir(target, link)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::config::load_config;
    use crate::skills::types::SkillsPaths;
    use crate::test_utils::ENV_MUTEX;
    use tempfile::TempDir;

    struct DefaultResolver;

    impl ConflictResolver for DefaultResolver {
        fn resolve(
            &mut self,
            _skill_id: &str,
            _options: &[ConflictOption],
            default_index: usize,
        ) -> Result<usize> {
            Ok(default_index)
        }
    }

    #[test]
    fn test_conflict_selection_keeps_versions() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let config_root = root.join("config");
        fs::create_dir_all(&config_root).expect("config root");
        let home_root = root.join("home");
        fs::create_dir_all(&home_root).expect("home root");
        unsafe {
            std::env::set_var("LLMAN_CONFIG_DIR", &config_root);
            std::env::set_var("HOME", &home_root);
            std::env::set_var("CLAUDE_HOME", home_root.join("claude"));
            std::env::set_var("CODEX_HOME", home_root.join("codex"));
        }
        let paths = SkillsPaths::resolve().expect("paths");
        let source_a = root.join("source-a");
        let source_b = root.join("source-b");
        fs::create_dir_all(&source_a).expect("source a");
        fs::create_dir_all(&source_b).expect("source b");

        let skill_a = source_a.join("skill");
        fs::create_dir_all(&skill_a).expect("skill a dir");
        fs::write(skill_a.join("SKILL.md"), "A").expect("skill a file");
        let skill_b = source_b.join("skill");
        fs::create_dir_all(&skill_b).expect("skill b dir");
        fs::write(skill_b.join("SKILL.md"), "B").expect("skill b file");

        let mut config = load_config(&paths, None).expect("config");
        config.sources = vec![
            ConfigEntry {
                id: "a".to_string(),
                agent: "claude".to_string(),
                scope: "user".to_string(),
                path: source_a,
                enabled: true,
                mode: TargetMode::Link,
            },
            ConfigEntry {
                id: "b".to_string(),
                agent: "codex".to_string(),
                scope: "user".to_string(),
                path: source_b,
                enabled: true,
                mode: TargetMode::Link,
            },
        ];

        let mut registry = Registry::default();
        let mut resolver = DefaultResolver;
        sync_sources(&config, &paths, &mut registry, &mut resolver).expect("sync");

        unsafe {
            std::env::remove_var("LLMAN_CONFIG_DIR");
            std::env::remove_var("HOME");
            std::env::remove_var("CLAUDE_HOME");
            std::env::remove_var("CODEX_HOME");
        }

        let entry = registry.skills.get("skill").expect("skill entry");
        assert_eq!(entry.versions.len(), 2);
        assert!(entry.current_hash.is_some());
    }
}

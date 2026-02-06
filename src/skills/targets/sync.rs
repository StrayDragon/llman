use crate::skills::catalog::types::{
    ConfigEntry, SkillCandidate, SkillsConfig, TargetConflictStrategy, TargetMode,
};
use anyhow::{Result, anyhow};
use inquire::Select;
use inquire::error::InquireError;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
#[error("cancelled")]
pub(crate) struct SkillSyncCancelled;

pub fn apply_target_links(
    skill: &SkillCandidate,
    config: &SkillsConfig,
    entry: &crate::skills::catalog::registry::SkillEntry,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    for target in &config.targets {
        let enabled = entry
            .targets
            .get(&target.id)
            .copied()
            .unwrap_or(target.enabled);
        apply_target_link(skill, target, enabled, interactive, target_conflict)?;
    }
    Ok(())
}

pub fn apply_target_diff(
    skills: &[SkillCandidate],
    target: &ConfigEntry,
    desired: &HashSet<String>,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    apply_target_diff_with_conflict_resolver(
        skills,
        target,
        desired,
        interactive,
        target_conflict,
        |skill_id, target| resolve_target_conflict(skill_id, target, true, None),
    )
}

fn apply_target_diff_with_conflict_resolver<F>(
    skills: &[SkillCandidate],
    target: &ConfigEntry,
    desired: &HashSet<String>,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
    mut resolve_conflict: F,
) -> Result<()>
where
    F: FnMut(&str, &ConfigEntry) -> Result<TargetConflictStrategy>,
{
    if target.mode == TargetMode::Skip {
        return Ok(());
    }

    #[derive(Debug, Clone, Copy)]
    struct PlannedOp {
        skill_index: usize,
        enabled: bool,
        conflict_strategy: Option<TargetConflictStrategy>,
    }

    let mut planned = Vec::new();

    // Phase 1: plan and resolve any interactive conflicts before applying changes,
    // so that a user cancel results in a full no-op.
    for (index, skill) in skills.iter().enumerate() {
        let current = is_skill_linked(skill, target);
        let wanted = desired.contains(&skill.skill_id);
        if current == wanted {
            continue;
        }

        let mut conflict_strategy = None;
        if wanted {
            let link_path = target.path.join(&skill.skill_id);
            let entry_exists = fs::symlink_metadata(&link_path).is_ok();
            if entry_exists {
                conflict_strategy = match target_conflict {
                    Some(strategy) => Some(strategy),
                    None => {
                        if !interactive {
                            return Err(anyhow!(t!(
                                "skills.target_conflict.requires_flag",
                                skill = skill.skill_id,
                                target = target.id
                            )));
                        }
                        Some(resolve_conflict(&skill.skill_id, target)?)
                    }
                };
            }
        }

        planned.push(PlannedOp {
            skill_index: index,
            enabled: wanted,
            conflict_strategy,
        });
    }

    // Phase 2: apply planned changes with any conflict strategy pre-resolved.
    for op in planned {
        let skill = skills
            .get(op.skill_index)
            .ok_or_else(|| anyhow!("invalid skill index"))?;
        apply_target_link(skill, target, op.enabled, interactive, op.conflict_strategy)?;
    }
    Ok(())
}

pub fn is_skill_linked(skill: &SkillCandidate, target: &ConfigEntry) -> bool {
    let link_path = target.path.join(&skill.skill_id);
    let meta = match fs::symlink_metadata(&link_path) {
        Ok(meta) => meta,
        Err(_) => return false,
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    let existing = match fs::read_link(&link_path) {
        Ok(path) => path,
        Err(_) => return false,
    };
    if existing == skill.skill_dir {
        return true;
    }
    let existing_abs = if existing.is_absolute() {
        existing
    } else {
        link_path.parent().unwrap_or(&target.path).join(existing)
    };
    let Ok(existing_canon) = fs::canonicalize(existing_abs) else {
        return false;
    };
    let Ok(desired_canon) = fs::canonicalize(&skill.skill_dir) else {
        return false;
    };
    existing_canon == desired_canon
}

pub fn apply_target_link(
    skill: &SkillCandidate,
    target: &ConfigEntry,
    enabled: bool,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    match target.mode {
        TargetMode::Skip => Ok(()),
        TargetMode::Link => {
            if enabled {
                ensure_target_link(skill, target, interactive, target_conflict)?;
            } else {
                remove_target_link(skill, target)?;
            }
            Ok(())
        }
    }
}

fn ensure_target_link(
    skill: &SkillCandidate,
    target: &ConfigEntry,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    if !target.path.exists() {
        fs::create_dir_all(&target.path)?;
    } else if !target.path.is_dir() {
        eprintln!(
            "{}",
            t!("skills.target.not_directory", path = target.path.display())
        );
        return Ok(());
    }

    let link_path = target.path.join(&skill.skill_id);
    let desired = &skill.skill_dir;

    if fs::symlink_metadata(&link_path).is_ok() {
        let is_symlink = fs::symlink_metadata(&link_path)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false);
        if is_symlink {
            let existing = fs::read_link(&link_path).ok();
            if existing.as_ref() == Some(desired) {
                return Ok(());
            }
        }

        let decision =
            resolve_target_conflict(&skill.skill_id, target, interactive, target_conflict)?;
        if decision == TargetConflictStrategy::Skip {
            return Ok(());
        }
        remove_path(&link_path)?;
    }

    create_symlink(desired, &link_path)?;
    Ok(())
}

fn remove_target_link(skill: &SkillCandidate, target: &ConfigEntry) -> Result<()> {
    let link_path = target.path.join(&skill.skill_id);
    let meta = match fs::symlink_metadata(&link_path) {
        Ok(meta) => meta,
        Err(_) => return Ok(()),
    };
    if meta.file_type().is_symlink() {
        remove_path(&link_path)?;
        return Ok(());
    }
    eprintln!(
        "{}",
        t!("skills.target.not_symlink", path = link_path.display())
    );
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
    let selection =
        match Select::new(&prompt, vec![overwrite_label.clone(), skip_label.clone()]).prompt() {
            Ok(selection) => selection,
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                return Err(SkillSyncCancelled.into());
            }
            Err(e) => {
                return Err(anyhow!(t!(
                    "skills.target_conflict.prompt_failed",
                    error = e
                )));
            }
        };
    if selection == overwrite_label {
        Ok(TargetConflictStrategy::Overwrite)
    } else {
        Ok(TargetConflictStrategy::Skip)
    }
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
    use crate::skills::catalog::registry::SkillEntry;
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_non_interactive_conflict_requires_flag() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = temp.path().join("targets");
        fs::create_dir_all(&target_root).expect("create target root");
        let conflict_path = target_root.join("skill");
        fs::create_dir_all(&conflict_path).expect("create conflict");

        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: target_root,
            enabled: true,
            mode: TargetMode::Link,
        };
        let config = SkillsConfig {
            targets: vec![target],
        };
        let entry = SkillEntry {
            targets: HashMap::new(),
            updated_at: None,
        };

        let err = apply_target_links(&skill, &config, &entry, false, None)
            .expect_err("should require target-conflict");
        assert!(err.to_string().contains("--target-conflict"));
    }

    #[cfg(unix)]
    #[test]
    fn test_overwrite_conflict_replaces_entry() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = temp.path().join("targets");
        fs::create_dir_all(&target_root).expect("create target root");
        let conflict_path = target_root.join("skill");
        fs::create_dir_all(&conflict_path).expect("create conflict");

        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Link,
        };
        let config = SkillsConfig {
            targets: vec![target],
        };
        let entry = SkillEntry {
            targets: HashMap::new(),
            updated_at: None,
        };

        apply_target_links(
            &skill,
            &config,
            &entry,
            false,
            Some(TargetConflictStrategy::Overwrite),
        )
        .expect("overwrite conflict");

        let link_path = target_root.join("skill");
        let meta = fs::symlink_metadata(&link_path).expect("symlink metadata");
        assert!(meta.file_type().is_symlink());
        let target = fs::read_link(&link_path).expect("read link");
        assert_eq!(target, skill_dir);
    }

    #[cfg(unix)]
    #[test]
    fn test_is_skill_linked_with_relative_symlink() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let skills_root = root.join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = root.join("targets");
        fs::create_dir_all(&target_root).expect("create target root");
        let link_path = target_root.join("skill");
        unix_fs::symlink("../skills/skill", &link_path).expect("create relative symlink");

        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: target_root,
            enabled: true,
            mode: TargetMode::Link,
        };

        assert!(is_skill_linked(&skill, &target));
    }

    #[cfg(unix)]
    #[test]
    fn test_apply_target_diff_respects_current_state() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let skills_root = root.join("skills");
        let skill_dir = skills_root.join("skill");
        let other_dir = skills_root.join("other");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::create_dir_all(&other_dir).expect("create other dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = root.join("targets");
        fs::create_dir_all(&target_root).expect("create target root");
        let link_path = target_root.join("skill");
        unix_fs::symlink(&other_dir, &link_path).expect("create conflicting symlink");

        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Link,
        };
        let mut desired = HashSet::new();

        apply_target_diff(&[skill.clone()], &target, &desired, true, None).expect("apply diff");
        assert!(link_path.exists());

        desired.insert("skill".to_string());
        apply_target_diff(
            &[skill],
            &target,
            &desired,
            true,
            Some(TargetConflictStrategy::Overwrite),
        )
        .expect("apply diff");
        let target_path = fs::read_link(&link_path).expect("read link");
        assert_eq!(target_path, skill_dir);
    }

    #[cfg(unix)]
    #[test]
    fn test_dangling_symlink_is_treated_as_existing_entry() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = temp.path().join("targets");
        fs::create_dir_all(&target_root).expect("create target root");

        let link_path = target_root.join("skill");
        unix_fs::symlink(temp.path().join("missing"), &link_path).expect("dangling symlink");

        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Link,
        };

        apply_target_link(
            &skill,
            &target,
            true,
            false,
            Some(TargetConflictStrategy::Overwrite),
        )
        .expect("overwrite dangling");

        let meta = fs::symlink_metadata(&link_path).expect("metadata");
        assert!(meta.file_type().is_symlink());
        let dest = fs::read_link(&link_path).expect("read link");
        assert_eq!(dest, skill_dir);

        apply_target_link(&skill, &target, false, false, None).expect("remove");
        assert!(fs::symlink_metadata(&link_path).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_conflict_cancel_aborts_without_side_effects() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = temp.path().join("targets");
        fs::create_dir_all(&target_root).expect("create target root");

        let link_path = target_root.join("skill");
        fs::create_dir_all(&link_path).expect("create conflict dir");

        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "claude_user".to_string(),
            agent: "claude".to_string(),
            scope: "user".to_string(),
            path: target_root,
            enabled: true,
            mode: TargetMode::Link,
        };

        let mut desired = HashSet::new();
        desired.insert("skill".to_string());

        let err = apply_target_diff_with_conflict_resolver(
            &[skill],
            &target,
            &desired,
            true,
            None,
            |_skill_id, _target| Err(SkillSyncCancelled.into()),
        )
        .expect_err("cancel");
        assert!(err.is::<SkillSyncCancelled>());

        // The conflict entry is unchanged: still a directory and not replaced by a symlink.
        let meta = fs::symlink_metadata(&link_path).expect("meta");
        assert!(meta.is_dir());
        assert!(!meta.file_type().is_symlink());
    }
}

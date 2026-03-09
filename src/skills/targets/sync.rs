use crate::path_utils::relative_path_from_dir;
use crate::skills::catalog::types::{
    ConfigEntry, SkillCandidate, SkillsConfig, TargetConflictStrategy, TargetMode,
};
use anyhow::{Context, Result, anyhow};
use inquire::Select;
use inquire::error::InquireError;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
#[error("cancelled")]
pub(crate) struct SkillSyncCancelled;

const VENDORED_METADATA_FILE: &str = ".llman-vendored.json";

pub fn apply_target_links(
    skill: &SkillCandidate,
    config: &SkillsConfig,
    desired_by_target: &HashMap<String, bool>,
    interactive: bool,
    target_conflict: Option<TargetConflictStrategy>,
) -> Result<()> {
    for target in &config.targets {
        let enabled = desired_by_target
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
    resolve_conflict: F,
) -> Result<()>
where
    F: FnMut(&str, &ConfigEntry) -> Result<TargetConflictStrategy>,
{
    match target.mode {
        TargetMode::Skip => Ok(()),
        TargetMode::Link => apply_target_diff_link_with_conflict_resolver(
            skills,
            target,
            desired,
            interactive,
            target_conflict,
            resolve_conflict,
        ),
        TargetMode::Copy => apply_target_diff_copy_with_conflict_resolver(
            skills,
            target,
            desired,
            interactive,
            target_conflict,
            resolve_conflict,
        ),
    }
}

fn apply_target_diff_link_with_conflict_resolver<F>(
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
        let needs_compaction = wanted
            && current
            && should_compact_target_links(target)
            && link_target_is_absolute(target, &skill.skill_id);

        if current == wanted && !needs_compaction {
            continue;
        }

        let mut conflict_strategy = None;
        if wanted && !current {
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

fn apply_target_diff_copy_with_conflict_resolver<F>(
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
        let wanted = desired.contains(&skill.skill_id);
        let state = copy_state(skill, target)?;

        if wanted {
            if state.up_to_date {
                continue;
            }

            let mut conflict_strategy = None;
            if state.enable_conflict {
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

            planned.push(PlannedOp {
                skill_index: index,
                enabled: true,
                conflict_strategy,
            });
        } else if state.present {
            planned.push(PlannedOp {
                skill_index: index,
                enabled: false,
                conflict_strategy: None,
            });
        }
    }

    // Legacy cleanup: stop recording vendored state via `.llman-vendored.json`.
    // Do this after resolving any interactive prompts so cancel remains a no-op.
    cleanup_vendored_metadata_files(target, skills)?;

    // Phase 2: apply planned changes with any conflict strategy pre-resolved.
    for op in planned {
        let skill = skills
            .get(op.skill_index)
            .ok_or_else(|| anyhow!("invalid skill index"))?;
        apply_target_link(skill, target, op.enabled, interactive, op.conflict_strategy)?;
    }
    Ok(())
}

fn should_compact_target_links(target: &ConfigEntry) -> bool {
    matches!(target.scope.as_str(), "project" | "repo")
}

fn link_target_is_absolute(target: &ConfigEntry, skill_id: &str) -> bool {
    let link_path = target.path.join(skill_id);
    let Ok(meta) = fs::symlink_metadata(&link_path) else {
        return false;
    };
    if !meta.file_type().is_symlink() {
        return false;
    }
    let Ok(dest) = fs::read_link(&link_path) else {
        return false;
    };
    dest.is_absolute()
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

pub fn is_skill_present(skill: &SkillCandidate, target: &ConfigEntry) -> bool {
    match target.mode {
        TargetMode::Skip => false,
        TargetMode::Link => is_skill_linked(skill, target),
        TargetMode::Copy => is_skill_vendored(skill, target).unwrap_or(false),
    }
}

#[derive(Debug, Clone, Copy)]
struct CopyState {
    present: bool,
    up_to_date: bool,
    enable_conflict: bool,
}

fn copy_state(skill: &SkillCandidate, target: &ConfigEntry) -> Result<CopyState> {
    let entry_path = target.path.join(&skill.skill_id);
    let meta = match fs::symlink_metadata(&entry_path) {
        Ok(meta) => meta,
        Err(_) => {
            return Ok(CopyState {
                present: false,
                up_to_date: false,
                enable_conflict: false,
            });
        }
    };

    if meta.file_type().is_symlink() {
        let linked = is_skill_linked(skill, target);
        return Ok(CopyState {
            present: linked,
            up_to_date: false,
            enable_conflict: !linked,
        });
    }

    if meta.is_file() {
        return Ok(CopyState {
            present: false,
            up_to_date: false,
            enable_conflict: true,
        });
    }

    if !meta.is_dir() {
        return Ok(CopyState {
            present: false,
            up_to_date: false,
            enable_conflict: true,
        });
    }

    if is_dir_empty(&entry_path).unwrap_or(false) {
        return Ok(CopyState {
            present: false,
            up_to_date: false,
            enable_conflict: false,
        });
    }

    let desired_digest = compute_dir_digest(&skill.skill_dir)?;
    let existing_digest = compute_dir_digest(&entry_path)?;
    let up_to_date = existing_digest == desired_digest;
    Ok(CopyState {
        // Without vendored metadata we only consider a copied directory "managed" when it matches
        // the current source. This keeps removal conservative.
        present: up_to_date,
        up_to_date,
        enable_conflict: !up_to_date,
    })
}

fn is_skill_vendored(skill: &SkillCandidate, target: &ConfigEntry) -> Result<bool> {
    let entry_path = target.path.join(&skill.skill_id);
    let meta = match fs::symlink_metadata(&entry_path) {
        Ok(meta) => meta,
        Err(_) => return Ok(false),
    };
    if meta.file_type().is_symlink() {
        return Ok(is_skill_linked(skill, target));
    }
    Ok(meta.is_dir())
}

fn is_dir_empty(path: &Path) -> Result<bool> {
    let mut entries = fs::read_dir(path)?;
    Ok(entries.next().is_none())
}

fn remove_file_if_exists(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_file() || meta.file_type().is_symlink() => {
            fs::remove_file(path)?;
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

fn cleanup_vendored_metadata_files(target: &ConfigEntry, skills: &[SkillCandidate]) -> Result<()> {
    if !target.path.is_dir() {
        return Ok(());
    }

    // Target-level marker from recent releases.
    remove_file_if_exists(&target.path.join(VENDORED_METADATA_FILE))?;

    // Per-skill marker from older releases.
    for skill in skills {
        remove_file_if_exists(
            &target
                .path
                .join(&skill.skill_id)
                .join(VENDORED_METADATA_FILE),
        )?;
    }
    Ok(())
}

fn compute_dir_digest(root: &Path) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut stack = Vec::new();
    hash_dir_recursive(root, root, &mut hasher, &mut stack)?;
    let digest = hasher.finalize();
    Ok(hex_lower(&digest))
}

fn hash_dir_recursive(
    root: &Path,
    dir: &Path,
    hasher: &mut Sha256,
    stack: &mut Vec<PathBuf>,
) -> Result<()> {
    let canon = fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if stack.contains(&canon) {
        // Directory symlink cycle: treat as empty directory.
        return Ok(());
    }
    stack.push(canon);

    let result = (|| {
        let mut entries = fs::read_dir(dir)?.collect::<std::result::Result<Vec<_>, _>>()?;
        entries.sort_by(|a, b| {
            a.file_name()
                .to_string_lossy()
                .cmp(&b.file_name().to_string_lossy())
        });

        for entry in entries {
            let name = entry.file_name();
            if name.to_string_lossy() == VENDORED_METADATA_FILE {
                continue;
            }

            let path = entry.path();
            let rel = match path.strip_prefix(root) {
                Ok(rel) => rel,
                Err(_) => {
                    return Err(anyhow!(
                        "{} {}",
                        t!("skills.hash.strip_prefix_failed"),
                        path.display()
                    ));
                }
            };

            let link_meta = match fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };

            let follow_meta = if link_meta.file_type().is_symlink() {
                match fs::metadata(&path) {
                    Ok(meta) => meta,
                    // Dangling/invalid symlink: ignore it for digest purposes.
                    Err(_) => continue,
                }
            } else {
                link_meta
            };

            hasher.update(rel.to_string_lossy().as_bytes());
            hasher.update([0u8]);

            if follow_meta.is_dir() {
                hasher.update(b"dir");
                hasher.update([0u8]);
                hash_dir_recursive(root, &path, hasher, stack)?;
                continue;
            }

            if follow_meta.is_file() {
                hasher.update(b"file");
                hasher.update([0u8]);
                let mut file = fs::File::open(&path)?;
                let mut buf = [0u8; 8192];
                loop {
                    let read = file.read(&mut buf)?;
                    if read == 0 {
                        break;
                    }
                    hasher.update(&buf[..read]);
                }
            }
        }
        Ok(())
    })();

    stack.pop();
    result
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
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
        TargetMode::Copy => {
            if enabled {
                ensure_target_copy(skill, target, interactive, target_conflict)?;
            } else {
                remove_target_copy(skill, target)?;
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
    let compact = should_compact_target_links(target);
    let preferred_target = if compact {
        relative_path_from_dir(&target.path, desired).unwrap_or_else(|| desired.to_path_buf())
    } else {
        desired.to_path_buf()
    };

    if fs::symlink_metadata(&link_path).is_ok() {
        if fs::symlink_metadata(&link_path)
            .map(|meta| meta.file_type().is_symlink())
            .unwrap_or(false)
        {
            // Already linked (absolute or relative); optionally "compact" the symlink target for
            // repo-scoped targets by rewriting absolute link targets to relative ones.
            if is_skill_linked(skill, target) {
                if compact
                    && fs::read_link(&link_path)
                        .ok()
                        .is_some_and(|dest| dest.is_absolute())
                    && !preferred_target.is_absolute()
                {
                    remove_path(&link_path)?;
                    create_symlink(&preferred_target, &link_path)?;
                }
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

    create_symlink(&preferred_target, &link_path)?;
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

fn ensure_target_copy(
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

    let entry_path = target.path.join(&skill.skill_id);
    if let Ok(meta) = fs::symlink_metadata(&entry_path) {
        if meta.file_type().is_symlink() {
            // Migration path: if an older config created a symlink, replace it with a vendored copy.
            if is_skill_linked(skill, target) {
                remove_existing_entry_for_copy(&entry_path)?;
            } else {
                let decision =
                    resolve_target_conflict(&skill.skill_id, target, interactive, target_conflict)?;
                if decision == TargetConflictStrategy::Skip {
                    return Ok(());
                }
                remove_existing_entry_for_copy(&entry_path)?;
            }
        } else if meta.is_dir() {
            if is_dir_empty(&entry_path).unwrap_or(false) {
                // Empty directory: safe to replace without prompting.
                fs::remove_dir(&entry_path)?;
            } else {
                let desired_digest = compute_dir_digest(&skill.skill_dir)?;
                let existing_digest = compute_dir_digest(&entry_path)?;
                if existing_digest == desired_digest {
                    // Legacy cleanup: older releases stored per-skill metadata markers.
                    remove_file_if_exists(&entry_path.join(VENDORED_METADATA_FILE))?;
                    return Ok(());
                }

                let decision =
                    resolve_target_conflict(&skill.skill_id, target, interactive, target_conflict)?;
                if decision == TargetConflictStrategy::Skip {
                    return Ok(());
                }
                fs::remove_dir_all(&entry_path)?;
            }
        } else {
            let decision =
                resolve_target_conflict(&skill.skill_id, target, interactive, target_conflict)?;
            if decision == TargetConflictStrategy::Skip {
                return Ok(());
            }
            remove_existing_entry_for_copy(&entry_path)?;
        }
    }

    copy_dir_all_follow_links(&skill.skill_dir, &entry_path)?;
    // Ensure we never create or keep vendored marker files.
    remove_file_if_exists(&entry_path.join(VENDORED_METADATA_FILE))?;
    Ok(())
}

fn remove_target_copy(skill: &SkillCandidate, target: &ConfigEntry) -> Result<()> {
    let entry_path = target.path.join(&skill.skill_id);
    let meta = match fs::symlink_metadata(&entry_path) {
        Ok(meta) => meta,
        Err(_) => return Ok(()),
    };

    if meta.file_type().is_symlink() {
        if is_skill_linked(skill, target) {
            remove_path(&entry_path)?;
        } else {
            eprintln!(
                "{}",
                t!("skills.target.not_managed", path = entry_path.display())
            );
        }
        return Ok(());
    }

    if meta.is_dir() {
        if is_dir_empty(&entry_path).unwrap_or(false) {
            fs::remove_dir(&entry_path)?;
            return Ok(());
        }

        let desired_digest = compute_dir_digest(&skill.skill_dir)?;
        let existing_digest = compute_dir_digest(&entry_path)?;
        if existing_digest != desired_digest {
            eprintln!(
                "{}",
                t!("skills.target.not_managed", path = entry_path.display())
            );
            return Ok(());
        }

        fs::remove_dir_all(&entry_path)?;
        return Ok(());
    }

    eprintln!(
        "{}",
        t!("skills.target.not_managed", path = entry_path.display())
    );
    Ok(())
}

fn remove_existing_entry_for_copy(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() || meta.is_file() {
        fs::remove_file(path)?;
        return Ok(());
    }
    if meta.is_dir() {
        fs::remove_dir_all(path)?;
    }
    Ok(())
}

fn copy_dir_all_follow_links(from: &Path, to: &Path) -> Result<()> {
    let mut stack = Vec::new();
    copy_dir_all_follow_links_inner(from, to, &mut stack)
}

fn copy_dir_all_follow_links_inner(from: &Path, to: &Path, stack: &mut Vec<PathBuf>) -> Result<()> {
    fs::create_dir_all(to).with_context(|| format!("create directory {}", to.display()))?;

    let canon = fs::canonicalize(from).unwrap_or_else(|_| from.to_path_buf());
    if stack.contains(&canon) {
        // Directory symlink cycle: keep an empty directory and stop.
        return Ok(());
    }
    stack.push(canon);

    let result = (|| {
        let mut entries = fs::read_dir(from)
            .with_context(|| format!("read directory {}", from.display()))?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        entries.sort_by(|a, b| {
            a.file_name()
                .to_string_lossy()
                .cmp(&b.file_name().to_string_lossy())
        });

        for entry in entries {
            let name = entry.file_name();
            if name.to_string_lossy() == VENDORED_METADATA_FILE {
                continue;
            }
            let src = entry.path();
            let dst = to.join(&name);

            let link_meta = match fs::symlink_metadata(&src) {
                Ok(meta) => meta,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };

            if link_meta.file_type().is_symlink() {
                let follow_meta = match fs::metadata(&src) {
                    Ok(meta) => meta,
                    // Dangling/invalid symlink: ignore it for copy purposes.
                    Err(_) => continue,
                };

                if follow_meta.is_dir() {
                    copy_dir_all_follow_links_inner(&src, &dst, stack)?;
                } else if follow_meta.is_file() {
                    fs::copy(&src, &dst).with_context(|| {
                        format!("copy file {} -> {}", src.display(), dst.display())
                    })?;
                }
                continue;
            }

            if link_meta.is_dir() {
                copy_dir_all_follow_links_inner(&src, &dst, stack)?;
            } else if link_meta.is_file() {
                fs::copy(&src, &dst)
                    .with_context(|| format!("copy file {} -> {}", src.display(), dst.display()))?;
            }
        }
        Ok(())
    })();

    stack.pop();
    result
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
        return Ok(());
    }
    if meta.is_dir() {
        let mut entries = fs::read_dir(path)?;
        if entries.next().is_some() {
            return Err(anyhow!(
                "Refusing to delete non-empty directory: {}",
                path.display()
            ));
        }
        fs::remove_dir(path)?;
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
        let desired_by_target = HashMap::new();

        let err = apply_target_links(&skill, &config, &desired_by_target, false, None)
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
        let desired_by_target = HashMap::new();

        apply_target_links(
            &skill,
            &config,
            &desired_by_target,
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
    fn test_overwrite_conflict_refuses_non_empty_directory() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = temp.path().join("targets");
        fs::create_dir_all(&target_root).expect("create target root");
        let conflict_path = target_root.join("skill");
        fs::create_dir_all(&conflict_path).expect("create conflict");
        fs::write(conflict_path.join("keep.txt"), "keep").expect("write keep");

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
        let desired_by_target = HashMap::new();

        let err = apply_target_links(
            &skill,
            &config,
            &desired_by_target,
            false,
            Some(TargetConflictStrategy::Overwrite),
        )
        .expect_err("should refuse deleting non-empty directory");
        assert!(
            err.to_string()
                .contains("Refusing to delete non-empty directory")
        );
        assert!(conflict_path.join("keep.txt").exists());
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

        apply_target_diff(std::slice::from_ref(&skill), &target, &desired, true, None)
            .expect("apply diff");
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
    fn test_project_scope_compacts_existing_absolute_link_to_relative() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();
        let skills_root = root.join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = root.join("repo").join(".claude").join("skills");
        fs::create_dir_all(&target_root).expect("create target root");
        let link_path = target_root.join("skill");
        unix_fs::symlink(&skill_dir, &link_path).expect("create absolute symlink");

        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "claude_project".to_string(),
            agent: "claude".to_string(),
            scope: "project".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Link,
        };

        let mut desired = HashSet::new();
        desired.insert("skill".to_string());
        apply_target_diff(std::slice::from_ref(&skill), &target, &desired, false, None)
            .expect("apply diff");

        let dest = fs::read_link(&link_path).expect("read link");
        assert!(!dest.is_absolute(), "expected compact relative symlink");
        let expected = relative_path_from_dir(&target_root, &skill_dir).expect("relative path");
        assert_eq!(dest, expected);
        assert!(is_skill_linked(&skill, &target));
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
        assert!(
            skill_dir.join("SKILL.md").exists(),
            "removing the symlink must not delete the target skill directory"
        );
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

    #[test]
    fn test_copy_mode_creates_vendored_directory() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = temp.path().join("targets");
        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "codex_repo".to_string(),
            agent: "codex".to_string(),
            scope: "repo".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Copy,
        };

        let mut desired = HashSet::new();
        desired.insert("skill".to_string());
        apply_target_diff(std::slice::from_ref(&skill), &target, &desired, false, None)
            .expect("apply diff");

        let entry_path = target_root.join("skill");
        let meta = fs::symlink_metadata(&entry_path).expect("metadata");
        assert!(meta.is_dir());
        assert!(!meta.file_type().is_symlink());
        assert!(entry_path.join("SKILL.md").exists());
        assert!(!entry_path.join(VENDORED_METADATA_FILE).exists());
        assert!(
            !target_root.join(VENDORED_METADATA_FILE).exists(),
            "should not create vendored metadata file"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_copy_mode_ignores_dangling_symlink_in_source() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        // A dangling symlink should not break copy mode. This commonly appears in repos where
        // optional files are linked but not always present.
        unix_fs::symlink("missing.txt", skill_dir.join("dangling.txt")).expect("create symlink");

        let target_root = temp.path().join("targets");
        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "codex_repo".to_string(),
            agent: "codex".to_string(),
            scope: "repo".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Copy,
        };

        let mut desired = HashSet::new();
        desired.insert("skill".to_string());

        apply_target_diff(std::slice::from_ref(&skill), &target, &desired, false, None)
            .expect("apply diff");
        apply_target_diff(std::slice::from_ref(&skill), &target, &desired, false, None)
            .expect("apply diff again (digest)");

        let entry_path = target_root.join("skill");
        assert!(entry_path.join("SKILL.md").exists());
        assert!(
            !entry_path.join("dangling.txt").exists(),
            "dangling symlink should be ignored in vendored copy"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_copy_mode_handles_symlink_directory_cycles() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let sub = skill_dir.join("sub");
        fs::create_dir_all(&sub).expect("create sub dir");
        fs::write(sub.join("a.txt"), "a").expect("write file");
        unix_fs::symlink("..", sub.join("back")).expect("create back symlink");

        let target_root = temp.path().join("targets");
        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "codex_repo".to_string(),
            agent: "codex".to_string(),
            scope: "repo".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Copy,
        };

        let mut desired = HashSet::new();
        desired.insert("skill".to_string());

        apply_target_diff(std::slice::from_ref(&skill), &target, &desired, false, None)
            .expect("apply diff");
        apply_target_diff(std::slice::from_ref(&skill), &target, &desired, false, None)
            .expect("apply diff again (digest)");

        let entry_path = target_root.join("skill");
        assert!(entry_path.join("sub").join("a.txt").exists());

        let back_path = entry_path.join("sub").join("back");
        let meta = fs::symlink_metadata(&back_path).expect("meta");
        assert!(meta.is_dir());
        assert!(!meta.file_type().is_symlink());
    }

    #[test]
    fn test_copy_mode_recopies_when_source_changes() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "v1").expect("write skill v1");

        let target_root = temp.path().join("targets");
        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "codex_repo".to_string(),
            agent: "codex".to_string(),
            scope: "repo".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Copy,
        };

        let mut desired = HashSet::new();
        desired.insert("skill".to_string());
        apply_target_diff(std::slice::from_ref(&skill), &target, &desired, false, None)
            .expect("apply diff v1");

        let entry_path = target_root.join("skill");
        assert_eq!(
            fs::read_to_string(entry_path.join("SKILL.md")).expect("read vendored skill"),
            "v1"
        );

        fs::write(skill_dir.join("SKILL.md"), "v2").expect("write skill v2");
        apply_target_diff(
            std::slice::from_ref(&skill),
            &target,
            &desired,
            false,
            Some(TargetConflictStrategy::Overwrite),
        )
        .expect("apply diff v2");
        assert_eq!(
            fs::read_to_string(entry_path.join("SKILL.md")).expect("read vendored skill"),
            "v2"
        );
        assert!(
            !target_root.join(VENDORED_METADATA_FILE).exists(),
            "should not create vendored metadata file"
        );
    }

    #[test]
    fn test_copy_mode_non_interactive_conflict_requires_flag() {
        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = temp.path().join("targets");
        fs::create_dir_all(target_root.join("skill")).expect("create conflicting dir");
        fs::write(target_root.join("skill").join("keep.txt"), "keep").expect("write keep");

        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "codex_repo".to_string(),
            agent: "codex".to_string(),
            scope: "repo".to_string(),
            path: target_root,
            enabled: true,
            mode: TargetMode::Copy,
        };
        let config = SkillsConfig {
            targets: vec![target],
        };
        let desired_by_target = HashMap::new();

        let err = apply_target_links(&skill, &config, &desired_by_target, false, None)
            .expect_err("should require target-conflict");
        assert!(err.to_string().contains("--target-conflict"));
    }

    #[cfg(unix)]
    #[test]
    fn test_copy_mode_migrates_symlink_to_directory() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().expect("temp dir");
        let skills_root = temp.path().join("skills");
        let skill_dir = skills_root.join("skill");
        fs::create_dir_all(&skill_dir).expect("create skill dir");
        fs::write(skill_dir.join("SKILL.md"), "# skill").expect("write skill");

        let target_root = temp.path().join("targets");
        fs::create_dir_all(&target_root).expect("create target root");
        let entry_path = target_root.join("skill");
        unix_fs::symlink(&skill_dir, &entry_path).expect("create symlink");

        let skill = SkillCandidate {
            skill_id: "skill".to_string(),
            skill_dir: skill_dir.clone(),
        };
        let target = ConfigEntry {
            id: "codex_repo".to_string(),
            agent: "codex".to_string(),
            scope: "repo".to_string(),
            path: target_root.clone(),
            enabled: true,
            mode: TargetMode::Copy,
        };

        apply_target_link(&skill, &target, true, false, None).expect("migrate");

        let meta = fs::symlink_metadata(&entry_path).expect("metadata");
        assert!(meta.is_dir());
        assert!(!meta.file_type().is_symlink());
        assert!(entry_path.join("SKILL.md").exists());
        assert!(!entry_path.join(VENDORED_METADATA_FILE).exists());
        assert!(
            !target_root.join(VENDORED_METADATA_FILE).exists(),
            "should not create vendored metadata file"
        );
    }
}

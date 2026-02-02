use crate::skills::types::SkillCandidate;
use anyhow::Result;
use ignore::WalkBuilder;
use serde_yaml::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn discover_skills(root: &Path) -> Result<Vec<SkillCandidate>> {
    let mut candidates = Vec::new();
    if !root.exists() {
        return Ok(candidates);
    }

    let mut seen_dirs: HashSet<PathBuf> = HashSet::new();
    let store_dir = root.join("store");
    let walker = WalkBuilder::new(root)
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false)
        .filter_entry(move |entry| entry.path() != store_dir)
        .build();

    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == "SKILL.md") {
            if !skill_file_exists(path) {
                continue;
            }
            let Some(skill_dir) = path.parent() else {
                continue;
            };
            record_skill_dir(skill_dir, path, &mut seen_dirs, &mut candidates);
            continue;
        }
        if entry
            .file_type()
            .is_some_and(|file_type| file_type.is_symlink())
            && is_symlink_dir(path)
            && let Some(skill_file) = resolve_symlink_skill_file(path)
        {
            record_skill_dir(path, &skill_file, &mut seen_dirs, &mut candidates);
        }
    }

    Ok(candidates)
}

fn record_skill_dir(
    skill_dir: &Path,
    skill_file: &Path,
    seen_dirs: &mut HashSet<PathBuf>,
    candidates: &mut Vec<SkillCandidate>,
) {
    let canonical = match fs::canonicalize(skill_dir) {
        Ok(path) => path,
        Err(_) => return,
    };
    if !seen_dirs.insert(canonical) {
        return;
    }
    let skill_id = resolve_skill_id(skill_dir, skill_file);
    candidates.push(SkillCandidate {
        skill_id,
        skill_dir: skill_dir.to_path_buf(),
    });
}

fn resolve_skill_id(skill_dir: &Path, skill_file: &Path) -> String {
    let fallback = skill_dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("skill")
        .to_string();
    let Some(frontmatter_name) = read_frontmatter_name(skill_file) else {
        return fallback;
    };
    let slug = slugify(&frontmatter_name);
    if slug.is_empty() { fallback } else { slug }
}

fn read_frontmatter_name(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let mut lines = content.lines();
    if lines.next()? != "---" {
        return None;
    }
    let mut yaml = String::new();
    for line in lines {
        if line.trim() == "---" {
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if yaml.trim().is_empty() {
        return None;
    }
    let parsed: Value = serde_yaml::from_str(&yaml).ok()?;
    parsed
        .get("name")
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
}

fn is_symlink_dir(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|meta| meta.file_type().is_symlink())
        .unwrap_or(false)
}

fn resolve_symlink_skill_file(path: &Path) -> Option<PathBuf> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_dir() {
        return None;
    }
    let skill_file = path.join("SKILL.md");
    if skill_file_exists(&skill_file) {
        Some(skill_file)
    } else {
        None
    }
}

fn skill_file_exists(path: &Path) -> bool {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return fs::metadata(path).map(|m| m.is_file()).unwrap_or(false);
        }
        return meta.is_file();
    }
    false
}

pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    let trimmed = out.trim_matches('-');
    let mut truncated = trimmed.chars().take(64).collect::<String>();
    if truncated.ends_with('-') {
        truncated = truncated.trim_end_matches('-').to_string();
    }
    truncated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::ENV_MUTEX;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Slint GUI Expert"), "slint-gui-expert");
        assert_eq!(slugify("***"), "");
    }

    #[test]
    fn test_skill_id_fallback() {
        let temp = TempDir::new().expect("temp dir");
        let skill_dir = temp.path().join("MySkill");
        fs::create_dir_all(&skill_dir).expect("create dir");
        let skill_file = skill_dir.join("SKILL.md");
        fs::write(&skill_file, "# no frontmatter").expect("write file");
        let id = resolve_skill_id(&skill_dir, &skill_file);
        assert_eq!(id, "MySkill");
    }

    #[cfg(unix)]
    #[test]
    fn test_discover_respects_ignore_and_symlink_skill() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("source");
        fs::create_dir_all(&root).expect("create source");
        fs::write(root.join(".gitignore"), "ignored-skill/\n").expect("write gitignore");

        let ignored = root.join("ignored-skill");
        fs::create_dir_all(&ignored).expect("create ignored skill");
        fs::write(ignored.join("SKILL.md"), "# ignored").expect("write skill");

        let kept = root.join("kept-skill");
        fs::create_dir_all(&kept).expect("create kept skill");
        fs::write(kept.join("SKILL.md"), "---\nname: Keep Me\n---\n").expect("write skill");

        let linked = root.join("linked-skill");
        fs::create_dir_all(&linked).expect("create linked skill");
        fs::write(linked.join("SKILL.md"), "---\nname: Linked Skill\n---\n").expect("write skill");

        let symlinked = root.join("symlink-skill");
        unix_fs::symlink(&linked, &symlinked).expect("create symlink dir");

        let template = root.join("template-skill.md");
        fs::write(&template, "---\nname: File Linked\n---\n").expect("write template");
        let symlink_file_dir = root.join("symlink-file-skill");
        fs::create_dir_all(&symlink_file_dir).expect("create symlink file dir");
        unix_fs::symlink(&template, symlink_file_dir.join("SKILL.md"))
            .expect("create symlink file");

        let mut discovered = discover_skills(&root).expect("discover skills");
        discovered.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));
        assert_eq!(discovered.len(), 3);
        assert_eq!(discovered[0].skill_id, "file-linked");
        assert_eq!(discovered[1].skill_id, "keep-me");
        assert_eq!(discovered[2].skill_id, "linked-skill");
    }

    #[test]
    fn test_discover_respects_global_ignore() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        let temp = TempDir::new().expect("temp dir");
        let home_root = temp.path().join("home");
        let xdg_config = temp.path().join("xdg");
        let global_ignore = xdg_config.join("git").join("ignore");
        fs::create_dir_all(global_ignore.parent().unwrap()).expect("create git ignore dir");
        fs::write(&global_ignore, "global-skill/\n").expect("write global ignore");
        unsafe {
            env::set_var("HOME", &home_root);
            env::set_var("XDG_CONFIG_HOME", &xdg_config);
            env::set_var("GIT_CONFIG_NOSYSTEM", "1");
        }

        let root = temp.path().join("source");
        fs::create_dir_all(&root).expect("create source");
        let ignored = root.join("global-skill");
        fs::create_dir_all(&ignored).expect("create ignored skill");
        fs::write(ignored.join("SKILL.md"), "# ignored").expect("write skill");

        let kept = root.join("kept-skill");
        fs::create_dir_all(&kept).expect("create kept skill");
        fs::write(kept.join("SKILL.md"), "# kept").expect("write skill");

        let discovered = discover_skills(&root).expect("discover skills");
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].skill_id, "kept-skill");

        unsafe {
            env::remove_var("HOME");
            env::remove_var("XDG_CONFIG_HOME");
            env::remove_var("GIT_CONFIG_NOSYSTEM");
        }
    }
}

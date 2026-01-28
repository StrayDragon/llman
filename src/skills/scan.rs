use crate::skills::hash::hash_skill_dir;
use crate::skills::types::{ConfigEntry, SkillCandidate};
use anyhow::Result;
use ignore::WalkBuilder;
use serde_yaml::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn discover_skills(source: &ConfigEntry) -> Result<Vec<SkillCandidate>> {
    let mut candidates = Vec::new();
    if !source.path.exists() {
        return Ok(candidates);
    }

    let mut seen_dirs: HashSet<PathBuf> = HashSet::new();
    let walker = WalkBuilder::new(&source.path)
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
        if !path.is_file() {
            continue;
        }
        if path.file_name().is_some_and(|name| name == "SKILL.md") {
            if entry
                .file_type()
                .is_some_and(|file_type| file_type.is_symlink())
            {
                continue;
            }
            let Some(skill_dir) = path.parent() else {
                continue;
            };
            if is_symlink_dir(skill_dir) {
                continue;
            }
            if !seen_dirs.insert(skill_dir.to_path_buf()) {
                continue;
            }
            let skill_id = resolve_skill_id(skill_dir, path);
            let hash = hash_skill_dir(skill_dir)?;
            candidates.push(SkillCandidate {
                skill_id,
                hash,
                source_id: source.id.clone(),
                source_path: source.path.clone(),
                skill_dir: skill_dir.to_path_buf(),
            });
        }
    }

    Ok(candidates)
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
    use crate::skills::types::TargetMode;
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
    fn test_discover_skips_ignored_and_symlink() {
        use std::os::unix::fs as unix_fs;

        let temp = TempDir::new().expect("temp dir");
        let source_root = temp.path().join("source");
        fs::create_dir_all(&source_root).expect("create source");
        fs::write(source_root.join(".gitignore"), "ignored-skill/\n").expect("write gitignore");

        let ignored = source_root.join("ignored-skill");
        fs::create_dir_all(&ignored).expect("create ignored skill");
        fs::write(ignored.join("SKILL.md"), "# ignored").expect("write skill");

        let kept = source_root.join("kept-skill");
        fs::create_dir_all(&kept).expect("create kept skill");
        fs::write(kept.join("SKILL.md"), "---\nname: Keep Me\n---\n").expect("write skill");

        let symlinked = source_root.join("symlink-skill");
        fs::create_dir_all(&symlinked).expect("create symlink skill");
        unix_fs::symlink(kept.join("SKILL.md"), symlinked.join("SKILL.md"))
            .expect("create symlink");

        let source = ConfigEntry {
            id: "test".to_string(),
            agent: "agent".to_string(),
            scope: "user".to_string(),
            path: source_root,
            enabled: true,
            mode: TargetMode::Link,
        };
        let discovered = discover_skills(&source).expect("discover skills");
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].skill_id, "keep-me");
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

        let source_root = temp.path().join("source");
        fs::create_dir_all(&source_root).expect("create source");
        let ignored = source_root.join("global-skill");
        fs::create_dir_all(&ignored).expect("create ignored skill");
        fs::write(ignored.join("SKILL.md"), "# ignored").expect("write skill");

        let kept = source_root.join("kept-skill");
        fs::create_dir_all(&kept).expect("create kept skill");
        fs::write(kept.join("SKILL.md"), "# kept").expect("write skill");

        let source = ConfigEntry {
            id: "test".to_string(),
            agent: "agent".to_string(),
            scope: "user".to_string(),
            path: source_root,
            enabled: true,
            mode: TargetMode::Link,
        };
        let discovered = discover_skills(&source).expect("discover skills");
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].skill_id, "kept-skill");

        unsafe {
            env::remove_var("HOME");
            env::remove_var("XDG_CONFIG_HOME");
            env::remove_var("GIT_CONFIG_NOSYSTEM");
        }
    }
}

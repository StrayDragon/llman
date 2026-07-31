use crate::skills::catalog::types::{RepoSource, SkillCandidate};
use anyhow::Result;
use ignore::WalkBuilder;
use serde_yaml::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn discover_skills(root: &Path) -> Result<Vec<SkillCandidate>> {
    discover_skills_with_global_ignore(root, None)
}

/// Discover skills across multiple repo sources, tagging each candidate with
/// its originating repo's `repo_id`/`repo_name`. Within a single repo the
/// canonical-directory dedup is preserved; cross-repo `skill_id` collisions are
/// resolved later by the caller (see `dedupe_skills`), which keeps the first
/// occurrence in repo list order (D2).
pub fn discover_skills_from_repos(repos: &[RepoSource]) -> Result<Vec<SkillCandidate>> {
    let mut candidates = Vec::new();
    let mut seen_dirs: HashSet<PathBuf> = HashSet::new();
    for repo in repos {
        let mut repo_candidates = Vec::new();
        discover_skills_with_global_ignore_into(
            &repo.path,
            None,
            &mut seen_dirs,
            &mut repo_candidates,
        )?;
        for mut candidate in repo_candidates {
            candidate.repo_id = Some(repo.id.clone());
            candidate.repo_name = repo.name.clone();
            candidates.push(candidate);
        }
    }
    Ok(candidates)
}

fn discover_skills_with_global_ignore(
    root: &Path,
    global_ignore: Option<&Path>,
) -> Result<Vec<SkillCandidate>> {
    let mut candidates = Vec::new();
    let mut seen_dirs: HashSet<PathBuf> = HashSet::new();
    discover_skills_with_global_ignore_into(root, global_ignore, &mut seen_dirs, &mut candidates)?;
    Ok(candidates)
}

fn discover_skills_with_global_ignore_into(
    root: &Path,
    global_ignore: Option<&Path>,
    seen_dirs: &mut HashSet<PathBuf>,
    candidates: &mut Vec<SkillCandidate>,
) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }

    // One-level discovery only: `<root>/<skill-dir>/SKILL.md` (depth 2). Nested
    // trees such as `__submodules__/.../skills/X` are intentionally ignored so
    // each `skills.repo[].path` behaves as a flat skill shelf.
    let store_dir = root.join("store");
    let mut builder = WalkBuilder::new(root);
    builder
        .hidden(false)
        .follow_links(false)
        .git_ignore(true)
        .git_exclude(true)
        .require_git(false)
        .max_depth(Some(2))
        .filter_entry(move |entry| entry.path() != store_dir);
    if let Some(ignore_path) = global_ignore {
        builder.git_global(false).current_dir(root);
        if let Some(err) = builder.add_ignore(ignore_path) {
            return Err(err.into());
        }
    } else {
        builder.git_global(true);
    }
    let walker = builder.build();

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
            // Only accept SKILL.md that lives directly under a first-level child
            // of `root` (not deeper nesting that max_depth might still surface
            // via unusual layouts).
            if skill_dir.parent() != Some(root) {
                continue;
            }
            record_skill_dir(skill_dir, path, seen_dirs, candidates);
            continue;
        }
        if entry
            .file_type()
            .is_some_and(|file_type| file_type.is_symlink())
            && is_symlink_dir(path)
            && path.parent() == Some(root)
            && let Some(skill_file) = resolve_symlink_skill_file(path)
        {
            record_skill_dir(path, &skill_file, seen_dirs, candidates);
        }
    }

    Ok(())
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
        repo_id: None,
        repo_name: None,
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

/// Read the `metadata.version` field from a SKILL.md frontmatter.
/// Returns None if the field is missing or cannot be parsed.
pub fn read_frontmatter_version(path: &Path) -> Option<String> {
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
        .get("metadata")
        .and_then(|m| m.get("version"))
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
}

/// Extract major.minor version from a semver string.
/// For example, "0.0.50" -> "0.0"
fn extract_major_minor(version: &str) -> Option<String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 2 {
        Some(format!("{}.{}", parts[0], parts[1]))
    } else {
        None
    }
}

/// Check if a skill version is compatible with the current CLI version.
/// Returns a warning message if versions are incompatible, None otherwise.
pub fn check_skill_version_compat(skill_path: &Path) -> Option<String> {
    let skill_version = read_frontmatter_version(skill_path)?;
    let cli_version = env!("CARGO_PKG_VERSION");

    let skill_major_minor = extract_major_minor(&skill_version);
    let cli_major_minor = extract_major_minor(cli_version);

    match (skill_major_minor, cli_major_minor) {
        (Some(skill_mm), Some(cli_mm)) => {
            if skill_mm != cli_mm {
                Some(format!(
                    "Warning: This skill was generated for llman {}, but you are running {}. Content may be outdated.",
                    skill_version, cli_version
                ))
            } else {
                None
            }
        }
        _ => None,
    }
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
    fn test_discover_skips_nested_skill_dirs() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path().join("source");
        fs::create_dir_all(&root).expect("create source");

        let top = root.join("top-skill");
        fs::create_dir_all(&top).expect("create top skill");
        fs::write(top.join("SKILL.md"), "---\nname: Top Skill\n---\n").expect("write top");

        let nested = root
            .join("__submodules__")
            .join("vendor")
            .join(".claude")
            .join("skills")
            .join("nested-skill");
        fs::create_dir_all(&nested).expect("create nested skill");
        fs::write(nested.join("SKILL.md"), "---\nname: Nested Skill\n---\n").expect("write nested");

        let discovered = discover_skills(&root).expect("discover skills");
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].skill_id, "top-skill");
    }

    #[test]
    fn test_discover_respects_global_ignore() {
        let temp = TempDir::new().expect("temp dir");
        let global_ignore = temp.path().join("global-ignore");
        fs::write(&global_ignore, "global-skill/\n").expect("write global ignore");

        let root = temp.path().join("source");
        fs::create_dir_all(&root).expect("create source");
        let ignored = root.join("global-skill");
        fs::create_dir_all(&ignored).expect("create ignored skill");
        fs::write(ignored.join("SKILL.md"), "# ignored").expect("write skill");

        let kept = root.join("kept-skill");
        fs::create_dir_all(&kept).expect("create kept skill");
        fs::write(kept.join("SKILL.md"), "# kept").expect("write skill");

        let discovered = discover_skills_with_global_ignore(&root, Some(&global_ignore))
            .expect("discover skills");
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].skill_id, "kept-skill");
    }

    #[test]
    fn test_read_frontmatter_version_with_metadata() {
        let temp = TempDir::new().expect("temp dir");
        let skill_file = temp.path().join("SKILL.md");
        fs::write(
            &skill_file,
            "---\nname: test-skill\nmetadata:\n  version: \"0.0.50\"\n---\n# Test",
        )
        .expect("write file");

        let version = read_frontmatter_version(&skill_file);
        assert_eq!(version, Some("0.0.50".to_string()));
    }

    #[test]
    fn test_read_frontmatter_version_without_metadata() {
        let temp = TempDir::new().expect("temp dir");
        let skill_file = temp.path().join("SKILL.md");
        fs::write(&skill_file, "---\nname: test-skill\n---\n# Test").expect("write file");

        let version = read_frontmatter_version(&skill_file);
        assert_eq!(version, None);
    }

    #[test]
    fn test_read_frontmatter_version_no_frontmatter() {
        let temp = TempDir::new().expect("temp dir");
        let skill_file = temp.path().join("SKILL.md");
        fs::write(&skill_file, "# No frontmatter").expect("write file");

        let version = read_frontmatter_version(&skill_file);
        assert_eq!(version, None);
    }

    #[test]
    fn test_extract_major_minor() {
        assert_eq!(extract_major_minor("0.0.50"), Some("0.0".to_string()));
        assert_eq!(extract_major_minor("1.2.3"), Some("1.2".to_string()));
        assert_eq!(extract_major_minor("0.1"), Some("0.1".to_string()));
        assert_eq!(extract_major_minor("1"), None);
        assert_eq!(extract_major_minor(""), None);
    }

    #[test]
    fn test_check_skill_version_compat_matching() {
        let temp = TempDir::new().expect("temp dir");
        let skill_file = temp.path().join("SKILL.md");
        let cli_version = env!("CARGO_PKG_VERSION");
        fs::write(
            &skill_file,
            format!(
                "---\nname: test-skill\nmetadata:\n  version: \"{}\"\n---\n# Test",
                cli_version
            ),
        )
        .expect("write file");

        let warning = check_skill_version_compat(&skill_file);
        assert_eq!(warning, None);
    }

    #[test]
    fn test_check_skill_version_compat_mismatch() {
        let temp = TempDir::new().expect("temp dir");
        let skill_file = temp.path().join("SKILL.md");
        // Use a different major.minor version (1.0 vs current 0.0)
        fs::write(
            &skill_file,
            "---\nname: test-skill\nmetadata:\n  version: \"1.0.0\"\n---\n# Test",
        )
        .expect("write file");

        let warning = check_skill_version_compat(&skill_file);
        assert!(warning.is_some());
        let msg = warning.unwrap();
        assert!(msg.contains("1.0.0"));
        assert!(msg.contains("may be outdated"));
    }

    #[test]
    fn test_check_skill_version_compat_no_version() {
        let temp = TempDir::new().expect("temp dir");
        let skill_file = temp.path().join("SKILL.md");
        fs::write(&skill_file, "---\nname: test-skill\n---\n# Test").expect("write file");

        let warning = check_skill_version_compat(&skill_file);
        assert_eq!(warning, None);
    }

    fn make_repo(index: usize, name: Option<&str>, path: PathBuf) -> RepoSource {
        RepoSource::from_index(index, name.map(str::to_string), path)
    }

    fn plant_skill(repo_root: &Path, dir_name: &str, skill_name: Option<&str>) {
        let dir = repo_root.join(dir_name);
        fs::create_dir_all(&dir).expect("create skill dir");
        let body = match skill_name {
            Some(name) => format!("---\nname: {name}\n---\n# body"),
            None => "# no frontmatter".to_string(),
        };
        fs::write(dir.join("SKILL.md"), body).expect("write skill");
    }

    #[test]
    fn test_discover_from_repos_tags_repo_metadata() {
        let temp = TempDir::new().expect("temp dir");
        let repo_a = temp.path().join("repo-a");
        let repo_b = temp.path().join("repo-b");
        fs::create_dir_all(&repo_a).expect("mkdir a");
        fs::create_dir_all(&repo_b).expect("mkdir b");
        plant_skill(&repo_a, "alpha", Some("Alpha"));
        plant_skill(&repo_b, "beta", Some("Beta"));

        let repos = vec![
            make_repo(0, Some("Team"), repo_a.clone()),
            make_repo(1, None, repo_b.clone()),
        ];
        let mut found = discover_skills_from_repos(&repos).expect("discover");
        found.sort_by(|a, b| a.skill_id.cmp(&b.skill_id));

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].skill_id, "alpha");
        assert_eq!(found[0].repo_id.as_deref(), Some("Team"));
        assert_eq!(found[0].repo_name.as_deref(), Some("Team"));
        assert_eq!(found[1].skill_id, "beta");
        // Second repo has no name → id is the positional index string.
        assert_eq!(found[1].repo_id.as_deref(), Some("1"));
        assert!(found[1].repo_name.is_none());
    }

    #[test]
    fn test_discover_from_repos_keeps_first_on_cross_repo_collision() {
        let temp = TempDir::new().expect("temp dir");
        let repo_a = temp.path().join("repo-a");
        let repo_b = temp.path().join("repo-b");
        fs::create_dir_all(&repo_a).expect("mkdir a");
        fs::create_dir_all(&repo_b).expect("mkdir b");
        // Same skill_id "shared" in both repos.
        plant_skill(&repo_a, "first", Some("Shared"));
        plant_skill(&repo_b, "second", Some("Shared"));

        let repos = vec![
            make_repo(0, Some("A"), repo_a.clone()),
            make_repo(1, Some("B"), repo_b.clone()),
        ];
        // Discovery itself surfaces both (canonical-dedup is per-filesystem path);
        // the cross-repo "first in list order wins" contract (D2) is enforced by
        // `dedupe_skills` in the caller. Here we assert both carry distinct repo
        // provenance so the caller can dedupe deterministically.
        let found = discover_skills_from_repos(&repos).expect("discover");
        assert_eq!(found.len(), 2);
        let ids: HashSet<&str> = found
            .iter()
            .map(|c| c.repo_id.as_deref().unwrap())
            .collect();
        assert!(ids.contains("A"));
        assert!(ids.contains("B"));
        // Both resolve to the same skill_id.
        assert!(found.iter().all(|c| c.skill_id == "shared"));
    }
}

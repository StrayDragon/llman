//! Hygiene checks for installed managed SDD skills (`llman-sdd-*`).
//!
//! Rejects leftover unrendered MiniJinja statement tags (`{% ... %}`) in skill
//! bodies. Managed-identity and cleanup boundaries are the `llman-sdd-` name
//! prefix (r90) — frontmatter carries no llman metadata gate (r95 retired).

use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};

const MANAGED_SKILL_PREFIX: &str = "llman-sdd-";

/// Scan `.agents/skills/llman-sdd-*` and ERROR if any skill body still
/// contains unrendered MiniJinja statement tags (`{% ... %}`). Non-prefixed
/// custom skills are ignored.
pub(crate) fn check_installed_skills_hygiene(root: &Path) -> Result<()> {
    let skills_dir = root.join(".agents").join("skills");
    if !skills_dir.exists() {
        return Ok(());
    }

    let mut jinja_violations: Vec<(PathBuf, String)> = Vec::new();

    for entry in fs::read_dir(&skills_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let dir_name = entry.file_name().to_string_lossy().to_string();
        if !dir_name.starts_with(MANAGED_SKILL_PREFIX) {
            continue;
        }
        let skill_md = entry.path().join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }

        let content = match fs::read_to_string(&skill_md) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Some(snippet) = first_unrendered_jinja_snippet(&content) {
            jinja_violations.push((
                skill_md,
                format!("unrendered MiniJinja tag near: {snippet}"),
            ));
        }
    }

    if !jinja_violations.is_empty() {
        let mut detail = String::new();
        for (path, reason) in &jinja_violations {
            detail.push_str(&format!("\n  - {}: {reason}", path.display()));
        }
        return Err(anyhow!(t!(
            "sdd.skill_consistency.unrendered_template_syntax",
            details = detail.as_str(),
            fix = "llman sdd init --update"
        )));
    }

    Ok(())
}

/// True when installed skill body still contains MiniJinja statement openers.
fn first_unrendered_jinja_snippet(content: &str) -> Option<String> {
    let idx = content.find("{%")?;
    let end = (idx + 48).min(content.len());
    let mut snippet = content[idx..end].replace('\n', " ");
    if end < content.len() {
        snippet.push('…');
    }
    Some(snippet)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_skill(root: &Path, name: &str, body: &str) {
        let dir = root.join(".agents/skills").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), body).unwrap();
    }

    #[test]
    fn ok_when_no_skills_dir() {
        let tmp = TempDir::new().unwrap();
        check_installed_skills_hygiene(tmp.path()).unwrap();
    }

    #[test]
    fn ignores_custom_skill_without_prefix() {
        let tmp = TempDir::new().unwrap();
        write_skill(tmp.path(), "my-custom-skill", "body with {% if x %}\n");
        check_installed_skills_hygiene(tmp.path()).unwrap();
    }

    /// Managed skill frontmatter carries no llman metadata gate anymore
    /// (r95 retired) — name/description/version-only frontmatter is clean.
    #[test]
    fn ok_for_clean_rendered_skill() {
        let tmp = TempDir::new().unwrap();
        write_skill(
            tmp.path(),
            "llman-sdd-explore",
            "---\nname: llman-sdd-explore\nmetadata:\n  version: \"0.0.71\"\n---\nbody\n",
        );
        check_installed_skills_hygiene(tmp.path()).unwrap();
    }

    #[test]
    fn errors_on_unrendered_jinja_in_body() {
        let tmp = TempDir::new().unwrap();
        write_skill(
            tmp.path(),
            "llman-sdd-explore",
            "---\nname: llman-sdd-explore\nmetadata:\n  version: \"0.0.71\"\n---\n{% if bdd_enabled %}\n- attach\n{% endif %}\n",
        );
        let err = check_installed_skills_hygiene(tmp.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("unrendered") || msg.contains("MiniJinja"),
            "{msg}"
        );
        assert!(msg.contains("init --update"), "{msg}");
        assert!(msg.contains("{%"), "{msg}");
    }
}

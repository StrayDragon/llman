//! Consistency checks for installed managed SDD skills (`llman-sdd-*`).
//!
//! Validates `metadata.llman_sdd.skill_set` (r95; `bdd_mode` is retired and
//! MUST NOT be required or checked), and rejects leftover unrendered MiniJinja
//! tags (e.g. `{% if ... %}`) in skill bodies.

use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const MANAGED_SKILL_PREFIX: &str = "llman-sdd-";
const SKILL_SET_VALUES: &[&str] = &["default", "optional"];

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    #[serde(default)]
    metadata: Option<SkillMetadata>,
}

#[derive(Debug, Deserialize)]
struct SkillMetadata {
    #[serde(default)]
    llman_sdd: Option<LlmanSddMeta>,
}

#[derive(Debug, Deserialize)]
struct LlmanSddMeta {
    #[serde(default)]
    skill_set: Option<String>,
}

/// Scan `.agents/skills/llman-sdd-*` and ERROR if `metadata.llman_sdd` is
/// missing, `skill_set` is invalid, or the skill body still contains
/// unrendered MiniJinja statement tags (`{% ... %}`). A leftover `bdd_mode`
/// key MUST NOT fail (retired, r95). Non-prefixed custom skills are ignored.
pub(crate) fn check_installed_skills_metadata(root: &Path) -> Result<()> {
    let skills_dir = root.join(".agents").join("skills");
    if !skills_dir.exists() {
        return Ok(());
    }

    let mut meta_violations: Vec<(PathBuf, String)> = Vec::new();
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
            meta_violations.push((skill_md, "missing SKILL.md".to_string()));
            continue;
        }

        let content = match fs::read_to_string(&skill_md) {
            Ok(c) => c,
            Err(e) => {
                meta_violations.push((skill_md, format!("read failed: {e}")));
                continue;
            }
        };

        if let Err(msg) = validate_llman_sdd_meta(&content) {
            meta_violations.push((skill_md.clone(), msg));
        }

        if let Some(snippet) = first_unrendered_jinja_snippet(&content) {
            jinja_violations.push((
                skill_md.clone(),
                format!("unrendered MiniJinja tag near: {snippet}"),
            ));
        }
    }

    if !meta_violations.is_empty() {
        let mut detail = String::new();
        for (path, reason) in &meta_violations {
            detail.push_str(&format!("\n  - {}: {reason}", path.display()));
        }
        return Err(anyhow!(t!(
            "sdd.skill_consistency.skill_set_invalid",
            expected = SKILL_SET_VALUES.join("|"),
            details = detail.as_str(),
            fix = "llman sdd init --update"
        )));
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

fn validate_llman_sdd_meta(content: &str) -> Result<(), String> {
    let Some(yaml) = extract_frontmatter_yaml(content) else {
        return Err("missing frontmatter (no llman_sdd metadata)".to_string());
    };
    let fm: SkillFrontmatter =
        serde_saphyr::from_str(yaml).map_err(|e| format!("frontmatter parse error: {e}"))?;
    let Some(skill_set) = fm
        .metadata
        .and_then(|m| m.llman_sdd)
        .and_then(|l| l.skill_set)
    else {
        return Err("missing metadata.llman_sdd.skill_set".to_string());
    };
    let skill_set = skill_set.trim().to_ascii_lowercase();
    if SKILL_SET_VALUES.contains(&skill_set.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "invalid skill_set={skill_set} (want {})",
            SKILL_SET_VALUES.join("|")
        ))
    }
}

fn extract_frontmatter_yaml(content: &str) -> Option<&str> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after = &trimmed[3..];
    let after = after.strip_prefix('\n').unwrap_or(after);
    let end = after.find("\n---")?;
    Some(&after[..end])
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
        check_installed_skills_metadata(tmp.path()).unwrap();
    }

    #[test]
    fn ignores_custom_skill_without_prefix() {
        let tmp = TempDir::new().unwrap();
        write_skill(tmp.path(), "my-custom-skill", "no frontmatter\n");
        check_installed_skills_metadata(tmp.path()).unwrap();
    }

    #[test]
    fn errors_when_metadata_missing() {
        let tmp = TempDir::new().unwrap();
        write_skill(tmp.path(), "llman-sdd-explore", "planted\n");
        let err = check_installed_skills_metadata(tmp.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("init --update"), "{msg}");
        assert!(
            msg.contains("llman-sdd-explore") || msg.contains("missing"),
            "{msg}"
        );
    }

    #[test]
    fn errors_on_invalid_skill_set() {
        let tmp = TempDir::new().unwrap();
        write_skill(
            tmp.path(),
            "llman-sdd-explore",
            "---\nname: llman-sdd-explore\nmetadata:\n  version: \"1.0.0\"\n  llman_sdd:\n    skill_set: sometimes\n---\nbody\n",
        );
        let err = check_installed_skills_metadata(tmp.path()).unwrap_err();
        assert!(format!("{err:#}").contains("invalid skill_set=sometimes"));
    }

    /// r95 (amended): both enum values pass, and a leftover `bdd_mode` key in
    /// old installed artifacts MUST NOT fail — the retired key is ignored.
    #[test]
    fn ok_for_both_skill_sets_and_leftover_bdd_mode() {
        let tmp = TempDir::new().unwrap();
        for skill_set in ["default", "optional"] {
            write_skill(
                tmp.path(),
                "llman-sdd-explore",
                &format!(
                    "---\nname: llman-sdd-explore\nmetadata:\n  version: \"1.0.0\"\n  llman_sdd:\n    skill_set: {skill_set}\n---\nbody\n"
                ),
            );
            check_installed_skills_metadata(tmp.path()).unwrap();
        }
        write_skill(
            tmp.path(),
            "llman-sdd-explore",
            "---\nname: llman-sdd-explore\nmetadata:\n  version: \"1.0.0\"\n  llman_sdd:\n    bdd_mode: on\n    skill_set: default\n---\nbody\n",
        );
        check_installed_skills_metadata(tmp.path()).unwrap();
    }

    #[test]
    fn errors_on_unrendered_jinja_in_body() {
        let tmp = TempDir::new().unwrap();
        write_skill(
            tmp.path(),
            "llman-sdd-explore",
            "---\nname: llman-sdd-explore\nmetadata:\n  version: \"1.0.0\"\n  llman_sdd:\n    skill_set: default\n---\n{% if bdd_enabled %}\n- attach\n{% endif %}\n",
        );
        let err = check_installed_skills_metadata(tmp.path()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("unrendered") || msg.contains("MiniJinja"),
            "{msg}"
        );
        assert!(msg.contains("init --update"), "{msg}");
        assert!(msg.contains("{%"), "{msg}");
    }
}

//! Consistency checks for installed managed SDD skills (`llman-sdd-*`).
//!
//! Validates `metadata.llman_sdd.bdd_mode` against `config.yaml` (`bdd:` present → on),
//! and rejects leftover unrendered MiniJinja tags (e.g. `{% if ... %}`) in skill bodies.

use super::config::SddConfig;
use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

const MANAGED_SKILL_PREFIX: &str = "llman-sdd-";

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
    bdd_mode: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    skill_set: Option<String>,
}

/// Expected `bdd_mode` for the project: `on` if `bdd:` is configured, else `off`.
pub fn expected_bdd_mode(config: &SddConfig) -> &'static str {
    if config.bdd.is_some() { "on" } else { "off" }
}

/// Scan `.agents/skills/llman-sdd-*` and ERROR if `llman_sdd.bdd_mode` is missing,
/// invalid, or mismatches `config`, or if the skill body still contains unrendered
/// MiniJinja statement tags (`{% ... %}`). Non-prefixed custom skills are ignored.
pub fn check_installed_skills_bdd_mode(root: &Path, config: &SddConfig) -> Result<()> {
    let skills_dir = root.join(".agents").join("skills");
    if !skills_dir.exists() {
        return Ok(());
    }

    let expected = expected_bdd_mode(config);
    let mut bdd_violations: Vec<(PathBuf, String)> = Vec::new();
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
            bdd_violations.push((skill_md, "missing SKILL.md".to_string()));
            continue;
        }

        let content = match fs::read_to_string(&skill_md) {
            Ok(c) => c,
            Err(e) => {
                bdd_violations.push((skill_md, format!("read failed: {e}")));
                continue;
            }
        };

        match read_bdd_mode_from_content(&content) {
            Ok(Some(mode)) if mode == expected => {}
            Ok(Some(mode)) => {
                bdd_violations.push((
                    skill_md.clone(),
                    format!("bdd_mode={mode}, expected {expected}"),
                ));
            }
            Ok(None) => {
                bdd_violations.push((
                    skill_md.clone(),
                    format!("missing metadata.llman_sdd.bdd_mode (expected {expected})"),
                ));
            }
            Err(msg) => {
                bdd_violations.push((skill_md.clone(), msg));
            }
        }

        if let Some(snippet) = first_unrendered_jinja_snippet(&content) {
            jinja_violations.push((
                skill_md,
                format!("unrendered MiniJinja tag near: {snippet}"),
            ));
        }
    }

    if !bdd_violations.is_empty() {
        let mut detail = String::new();
        for (path, reason) in &bdd_violations {
            detail.push_str(&format!("\n  - {}: {reason}", path.display()));
        }
        return Err(anyhow!(t!(
            "sdd.skill_consistency.bdd_mode_mismatch",
            expected = expected,
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

fn read_bdd_mode_from_content(content: &str) -> Result<Option<String>, String> {
    let Some(yaml) = extract_frontmatter_yaml(content) else {
        return Ok(None);
    };
    let fm: SkillFrontmatter =
        serde_yaml::from_str(yaml).map_err(|e| format!("frontmatter parse error: {e}"))?;
    let mode = fm
        .metadata
        .and_then(|m| m.llman_sdd)
        .and_then(|l| l.bdd_mode);
    match mode {
        Some(m) => {
            let m = m.trim().to_ascii_lowercase();
            if m == "on" || m == "off" {
                Ok(Some(m))
            } else {
                Err(format!("invalid bdd_mode={m} (want on|off)"))
            }
        }
        None => Ok(None),
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
    use crate::sdd::project::config::{BddConfig, SddConfig};
    use std::fs;
    use tempfile::TempDir;

    fn write_skill(root: &Path, name: &str, body: &str) {
        let dir = root.join(".agents/skills").join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), body).unwrap();
    }

    fn cfg_bdd_on() -> SddConfig {
        SddConfig {
            schema: "spec-driven".into(),
            locale: "en".into(),
            bdd: Some(BddConfig {
                framework: "rstest-bdd".into(),
                feature_dir: None,
                default_language: None,
                run_command: Some("cargo test --features bdd".into()),
                verify_prompt: None,
            }),
            extra_skills: None,
            archive: None,
        }
    }

    fn cfg_bdd_off() -> SddConfig {
        SddConfig {
            schema: "spec-driven".into(),
            locale: "en".into(),
            bdd: None,
            extra_skills: None,
            archive: None,
        }
    }

    #[test]
    fn ok_when_no_skills_dir() {
        let tmp = TempDir::new().unwrap();
        check_installed_skills_bdd_mode(tmp.path(), &cfg_bdd_on()).unwrap();
    }

    #[test]
    fn ignores_custom_skill_without_prefix() {
        let tmp = TempDir::new().unwrap();
        write_skill(tmp.path(), "my-custom-skill", "no frontmatter\n");
        check_installed_skills_bdd_mode(tmp.path(), &cfg_bdd_on()).unwrap();
    }

    #[test]
    fn errors_when_metadata_missing() {
        let tmp = TempDir::new().unwrap();
        write_skill(tmp.path(), "llman-sdd-explore", "planted\n");
        let err = check_installed_skills_bdd_mode(tmp.path(), &cfg_bdd_on()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("init --update"), "{msg}");
        assert!(
            msg.contains("llman-sdd-explore") || msg.contains("missing"),
            "{msg}"
        );
    }

    #[test]
    fn errors_on_mismatch() {
        let tmp = TempDir::new().unwrap();
        write_skill(
            tmp.path(),
            "llman-sdd-explore",
            "---\nname: llman-sdd-explore\nmetadata:\n  version: \"1.0.0\"\n  llman_sdd:\n    bdd_mode: off\n    skill_set: default\n---\nbody\n",
        );
        let err = check_installed_skills_bdd_mode(tmp.path(), &cfg_bdd_on()).unwrap_err();
        assert!(format!("{err:#}").contains("expected on"));
    }

    #[test]
    fn ok_when_matching() {
        let tmp = TempDir::new().unwrap();
        write_skill(
            tmp.path(),
            "llman-sdd-explore",
            "---\nname: llman-sdd-explore\nmetadata:\n  version: \"1.0.0\"\n  llman_sdd:\n    bdd_mode: on\n    skill_set: default\n---\nbody\n",
        );
        check_installed_skills_bdd_mode(tmp.path(), &cfg_bdd_on()).unwrap();
        write_skill(
            tmp.path(),
            "llman-sdd-explore",
            "---\nname: llman-sdd-explore\nmetadata:\n  version: \"1.0.0\"\n  llman_sdd:\n    bdd_mode: off\n    skill_set: default\n---\nbody\n",
        );
        check_installed_skills_bdd_mode(tmp.path(), &cfg_bdd_off()).unwrap();
    }

    #[test]
    fn errors_on_unrendered_jinja_in_body() {
        let tmp = TempDir::new().unwrap();
        write_skill(
            tmp.path(),
            "llman-sdd-explore",
            "---\nname: llman-sdd-explore\nmetadata:\n  version: \"1.0.0\"\n  llman_sdd:\n    bdd_mode: on\n    skill_set: default\n---\n{% if bdd_enabled %}\n- attach\n{% endif %}\n",
        );
        let err = check_installed_skills_bdd_mode(tmp.path(), &cfg_bdd_on()).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("unrendered") || msg.contains("MiniJinja"),
            "{msg}"
        );
        assert!(msg.contains("init --update"), "{msg}");
        assert!(msg.contains("{%"), "{msg}");
    }
}

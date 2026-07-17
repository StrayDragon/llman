use super::config::load_or_create_config;
use super::templates::skill_templates;
use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

const REQUIRED_ETHICS_KEYS: &[&str] = &[
    "ethics.risk_level",
    "ethics.prohibited_actions",
    "ethics.required_evidence",
    "ethics.refusal_contract",
    "ethics.escalation_policy",
];

/// Prefix for managed SDD skills. Only directories under this prefix are
/// candidates for remove-then-refresh during `update-skills` / `init --update`.
const MANAGED_SKILL_PREFIX: &str = "llman-sdd-";

pub fn run() -> Result<()> {
    run_with_root(Path::new("."))
}

pub(crate) fn run_with_root(root: &Path) -> Result<()> {
    let llmanspec_path = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        let cmd = "llman sdd init";
        return Err(anyhow!(t!("sdd.update_skills.no_llmanspec", cmd = cmd)));
    }

    let config = load_or_create_config(&llmanspec_path)?;

    let templates = skill_templates(&config, root)?;
    enforce_ethics_governance(&templates)?;
    let skills_base = root.join(".agents").join("skills");

    // Remove stale managed skills before writing current candidates
    // (defaults + config.extra_skills).
    cleanup_stale_skills(&skills_base, &templates)?;

    write_tool_skills(&skills_base, &templates)?;

    Ok(())
}

fn enforce_ethics_governance(templates: &[super::templates::SkillTemplate]) -> Result<()> {
    for template in templates {
        for key in REQUIRED_ETHICS_KEYS {
            if !template.content.contains(key) {
                return Err(anyhow!(
                    "missing required ethics governance key '{}' in template '{}'",
                    key,
                    template.name
                ));
            }
        }
    }
    Ok(())
}

/// Remove directories under `.agents/skills/` that look managed (`llman-sdd-*`)
/// but are not in the current candidate set (defaults + `extra_skills`).
/// Non-prefixed custom skills are left untouched.
fn cleanup_stale_skills(base: &Path, templates: &[super::templates::SkillTemplate]) -> Result<()> {
    let candidates: HashSet<String> = templates
        .iter()
        .map(|t| t.name.trim_end_matches(".md").to_string())
        .collect();

    if !base.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(base)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();

        // Only manage the llman-sdd-* namespace; never delete unrelated skills.
        if !dir_name.starts_with(MANAGED_SKILL_PREFIX) {
            continue;
        }

        if !candidates.contains(&dir_name) {
            let skill_path = entry.path();
            fs::remove_dir_all(&skill_path)?;
            eprintln!("Cleaned up stale skill: {}", dir_name);
        }
    }

    Ok(())
}

fn write_tool_skills(base: &Path, templates: &[super::templates::SkillTemplate]) -> Result<()> {
    fs::create_dir_all(base)?;
    for template in templates {
        let dir_name = template.name.trim_end_matches(".md");
        let skill_dir = base.join(dir_name);
        fs::create_dir_all(&skill_dir)?;
        let skill_path = skill_dir.join("SKILL.md");
        // Ensure trailing newline so repeated `init --update` does not thrash
        // against end-of-file-fixer / editor norms.
        let mut bytes = template.content.as_bytes().to_vec();
        if !bytes.ends_with(b"\n") {
            bytes.push(b'\n');
        }
        atomic_write_with_mode(&skill_path, &bytes, None)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const EXPECTED_WORKFLOW_SKILLS: &[&str] = &[
        "llman-sdd-explore",
        "llman-sdd-propose",
        "llman-sdd-apply",
        "llman-sdd-verify",
        "llman-sdd-quick",
        "llman-sdd-specs-compact",
        "llman-sdd-archive",
        "llman-sdd-graph",
        "llman-sdd-apply-cycle",
    ];

    #[test]
    fn update_skills_writes_skills_to_agents_skills() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        super::run_with_root(root).expect("update-skills");

        for skill in EXPECTED_WORKFLOW_SKILLS {
            assert!(
                root.join(".agents/skills")
                    .join(skill)
                    .join("SKILL.md")
                    .exists(),
                "missing skill {skill}"
            );
        }
    }

    #[test]
    fn update_skills_new_style_requires_ethics_governance_keys() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let override_skill = root.join("templates/sdd/en/skills/llman-sdd-explore.md");
        fs::create_dir_all(override_skill.parent().expect("parent")).expect("mkdir");
        fs::write(
            &override_skill,
            r#"---
name: "llman-sdd-explore"
description: "override for test"
---

## Context
- test
## Goal
- test
## Constraints
- test
## Workflow
- test
## Decision Policy
- test
## Output Contract
- test
## Ethics Governance
- `ethics.risk_level`: test
- `ethics.prohibited_actions`: test
- `ethics.required_evidence`: test
- `ethics.refusal_contract`: test
"#,
        )
        .expect("write override");

        let result = super::run_with_root(root);
        assert!(result.is_err(), "expected missing ethics key failure");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("ethics.escalation_policy"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn update_skills_errors_without_llmanspec() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let result = super::run_with_root(root);
        assert!(result.is_err(), "expected error without llmanspec dir");
        let err = result.unwrap_err().to_string().to_lowercase();
        assert!(
            err.contains("no llmanspec"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn update_skills_does_not_write_optional_skills_by_default() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        super::run_with_root(root).expect("update-skills");

        let optional_skills = [
            "llman-sdd-new-change",
            "llman-sdd-continue",
            "llman-sdd-ff",
            "llman-sdd-sync",
            "llman-sdd-validate",
        ];
        for skill in &optional_skills {
            assert!(
                !root
                    .join(".agents/skills")
                    .join(skill)
                    .join("SKILL.md")
                    .exists(),
                "optional skill {skill} should not be written by default"
            );
        }

        // verify is a default skill, should exist
        assert!(
            root.join(".agents/skills/llman-sdd-verify/SKILL.md")
                .exists()
        );
        // quick is a default skill, should exist
        assert!(
            root.join(".agents/skills/llman-sdd-quick/SKILL.md")
                .exists()
        );
    }

    #[test]
    fn update_skills_writes_extra_skills_when_configured() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
        fs::create_dir_all(&llmanspec_dir).expect("create llmanspec");

        let config_path = llmanspec_dir.join("config.yaml");
        fs::write(
            &config_path,
            "schema: spec-driven\nlocale: en\nextra_skills:\n  - llman-sdd-sync\n  - llman-sdd-new-change\n",
        )
        .expect("write config");

        super::run_with_root(root).expect("update-skills");

        // Default skills present
        assert!(
            root.join(".agents/skills/llman-sdd-explore/SKILL.md")
                .exists()
        );
        assert!(
            root.join(".agents/skills/llman-sdd-apply/SKILL.md")
                .exists()
        );
        // verify is now a default skill
        assert!(
            root.join(".agents/skills/llman-sdd-verify/SKILL.md")
                .exists()
        );

        // Enabled extra skills present
        assert!(root.join(".agents/skills/llman-sdd-sync/SKILL.md").exists());
        assert!(
            root.join(".agents/skills/llman-sdd-new-change/SKILL.md")
                .exists()
        );

        // Non-enabled optional skills absent
        assert!(
            !root
                .join(".agents/skills/llman-sdd-continue/SKILL.md")
                .exists()
        );
        assert!(!root.join(".agents/skills/llman-sdd-ff/SKILL.md").exists());
    }

    #[test]
    fn cleanup_stale_skills_removes_stale_optional_skills() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
        fs::create_dir_all(&llmanspec_dir).expect("create llmanspec");

        // Create config with extra_skills
        let config_path = llmanspec_dir.join("config.yaml");
        fs::write(
            &config_path,
            "schema: spec-driven\nlocale: en\nextra_skills:\n  - llman-sdd-sync\n  - llman-sdd-new-change\n",
        )
        .expect("write config");

        // First run to create skills
        super::run_with_root(root).expect("update-skills");

        // Verify skills exist
        assert!(root.join(".agents/skills/llman-sdd-sync/SKILL.md").exists());
        assert!(
            root.join(".agents/skills/llman-sdd-new-change/SKILL.md")
                .exists()
        );

        // Manually create a stale skill directory
        let stale_skill_dir = root.join(".agents/skills/llman-sdd-validate");
        fs::create_dir_all(&stale_skill_dir).expect("create stale skill dir");
        fs::write(stale_skill_dir.join("SKILL.md"), "stale content").expect("write stale skill");
        assert!(stale_skill_dir.exists());

        // Update config to remove llman-sdd-new-change
        fs::write(
            &config_path,
            "schema: spec-driven\nlocale: en\nextra_skills:\n  - llman-sdd-sync\n",
        )
        .expect("write config");

        // Second run should cleanup stale skills
        super::run_with_root(root).expect("update-skills");

        // Verify stale skills are removed
        assert!(
            !stale_skill_dir.exists(),
            "stale skill llman-sdd-validate should be removed"
        );
        assert!(
            !root
                .join(".agents/skills/llman-sdd-new-change/SKILL.md")
                .exists(),
            "removed skill llman-sdd-new-change should be cleaned up"
        );

        // Verify kept skills still exist
        assert!(root.join(".agents/skills/llman-sdd-sync/SKILL.md").exists());
        assert!(
            root.join(".agents/skills/llman-sdd-explore/SKILL.md")
                .exists()
        );
    }

    #[test]
    fn cleanup_stale_skills_preserves_core_skills() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
        fs::create_dir_all(&llmanspec_dir).expect("create llmanspec");

        // Create config with no extra_skills
        let config_path = llmanspec_dir.join("config.yaml");
        fs::write(&config_path, "schema: spec-driven\nlocale: en\n").expect("write config");

        // First run to create core skills
        super::run_with_root(root).expect("update-skills");

        // Verify core skills exist
        assert!(
            root.join(".agents/skills/llman-sdd-explore/SKILL.md")
                .exists()
        );
        assert!(
            root.join(".agents/skills/llman-sdd-apply/SKILL.md")
                .exists()
        );
        // verify is now a default skill
        assert!(
            root.join(".agents/skills/llman-sdd-verify/SKILL.md")
                .exists()
        );

        // Second run should not remove core skills
        super::run_with_root(root).expect("update-skills");

        // Verify core skills still exist
        assert!(
            root.join(".agents/skills/llman-sdd-explore/SKILL.md")
                .exists()
        );
        assert!(
            root.join(".agents/skills/llman-sdd-apply/SKILL.md")
                .exists()
        );
        assert!(
            root.join(".agents/skills/llman-sdd-verify/SKILL.md")
                .exists()
        );
    }

    #[test]
    fn cleanup_stale_skills_preserves_custom_skills() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
        fs::create_dir_all(&llmanspec_dir).expect("create llmanspec");

        // Create config with no extra_skills
        let config_path = llmanspec_dir.join("config.yaml");
        fs::write(&config_path, "schema: spec-driven\nlocale: en\n").expect("write config");

        // Create a custom skill directory (not llman-sdd-* managed namespace)
        let custom_skill_dir = root.join(".agents/skills/my-custom-skill");
        fs::create_dir_all(&custom_skill_dir).expect("create custom skill dir");
        fs::write(custom_skill_dir.join("SKILL.md"), "custom content").expect("write custom skill");

        // Run update
        super::run_with_root(root).expect("update-skills");

        // Verify custom skill is preserved
        assert!(
            custom_skill_dir.join("SKILL.md").exists(),
            "custom skill should not be removed"
        );
    }

    #[test]
    fn cleanup_stale_skills_removes_deprecated_managed_core_skill() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
        fs::create_dir_all(&llmanspec_dir).expect("create llmanspec");

        let config_path = llmanspec_dir.join("config.yaml");
        fs::write(&config_path, "schema: spec-driven\nlocale: en\n").expect("write config");

        super::run_with_root(root).expect("update-skills");

        // Simulate a previously shipped core skill that is no longer a candidate
        // (e.g. llman-sdd-solidify after Git-native BDD-on).
        let deprecated = root.join(".agents/skills/llman-sdd-solidify");
        fs::create_dir_all(&deprecated).expect("create deprecated skill");
        fs::write(deprecated.join("SKILL.md"), "deprecated").expect("write deprecated");

        super::run_with_root(root).expect("update-skills second pass");

        assert!(
            !deprecated.exists(),
            "deprecated llman-sdd-* core skill must be removed on update"
        );
        assert!(
            root.join(".agents/skills/llman-sdd-explore/SKILL.md")
                .exists(),
            "current default candidates must still be written"
        );
    }

    #[test]
    fn cleanup_stale_skills_keeps_extra_skills_extend_candidates() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
        fs::create_dir_all(&llmanspec_dir).expect("create llmanspec");

        let config_path = llmanspec_dir.join("config.yaml");
        fs::write(
            &config_path,
            "schema: spec-driven\nlocale: en\nextra_skills:\n  - llman-sdd-sync\n",
        )
        .expect("write config");

        super::run_with_root(root).expect("update-skills");

        assert!(root.join(".agents/skills/llman-sdd-sync/SKILL.md").exists());

        // Second pass with same extend list must keep the extra skill
        super::run_with_root(root).expect("update-skills again");
        assert!(
            root.join(".agents/skills/llman-sdd-sync/SKILL.md").exists(),
            "extra_skills extend candidates must remain"
        );
    }

    #[test]
    fn cleanup_stale_skills_removes_all_optional_when_none_configured() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
        fs::create_dir_all(&llmanspec_dir).expect("create llmanspec");

        // Create config with extra_skills
        let config_path = llmanspec_dir.join("config.yaml");
        fs::write(
            &config_path,
            "schema: spec-driven\nlocale: en\nextra_skills:\n  - llman-sdd-sync\n  - llman-sdd-new-change\n",
        )
        .expect("write config");

        // First run to create optional skills
        super::run_with_root(root).expect("update-skills");

        // Verify optional skills exist
        assert!(root.join(".agents/skills/llman-sdd-sync/SKILL.md").exists());
        assert!(
            root.join(".agents/skills/llman-sdd-new-change/SKILL.md")
                .exists()
        );

        // Update config to remove all extra_skills
        fs::write(&config_path, "schema: spec-driven\nlocale: en\n").expect("write config");

        // Second run should cleanup all optional skills
        super::run_with_root(root).expect("update-skills");

        // Verify optional skills are removed
        assert!(
            !root.join(".agents/skills/llman-sdd-sync/SKILL.md").exists(),
            "optional skill llman-sdd-sync should be removed"
        );
        assert!(
            !root
                .join(".agents/skills/llman-sdd-new-change/SKILL.md")
                .exists(),
            "optional skill llman-sdd-new-change should be removed"
        );

        // Verify core skills still exist (verify is now a default skill)
        assert!(
            root.join(".agents/skills/llman-sdd-verify/SKILL.md")
                .exists()
        );
        assert!(
            root.join(".agents/skills/llman-sdd-explore/SKILL.md")
                .exists()
        );
    }
}

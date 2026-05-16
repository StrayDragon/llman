use super::config::load_or_create_config;
use super::templates::skill_templates;
use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

const REQUIRED_ETHICS_KEYS: &[&str] = &[
    "ethics.risk_level",
    "ethics.prohibited_actions",
    "ethics.required_evidence",
    "ethics.refusal_contract",
    "ethics.escalation_policy",
];

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

fn write_tool_skills(base: &Path, templates: &[super::templates::SkillTemplate]) -> Result<()> {
    fs::create_dir_all(base)?;
    for template in templates {
        let dir_name = template.name.trim_end_matches(".md");
        let skill_dir = base.join(dir_name);
        fs::create_dir_all(&skill_dir)?;
        let skill_path = skill_dir.join("SKILL.md");
        atomic_write_with_mode(&skill_path, template.content.as_bytes(), None)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const EXPECTED_WORKFLOW_SKILLS: &[&str] = &[
        "llman-sdd-onboard",
        "llman-sdd-explore",
        "llman-sdd-propose",
        "llman-sdd-apply",
        "llman-sdd-specs-compact",
        "llman-sdd-archive",
        "llman-sdd-graph",
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

        let override_skill = root.join("templates/sdd/en/skills/llman-sdd-onboard.md");
        fs::create_dir_all(override_skill.parent().expect("parent")).expect("mkdir");
        fs::write(
            &override_skill,
            r#"---
name: "llman-sdd-onboard"
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
            "llman-sdd-show",
            "llman-sdd-sync",
            "llman-sdd-validate",
            "llman-sdd-verify",
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
            "schema: spec-driven\nlocale: en\nextra_skills:\n  - llman-sdd-verify\n  - llman-sdd-show\n",
        )
        .expect("write config");

        super::run_with_root(root).expect("update-skills");

        // Default skills present
        assert!(
            root.join(".agents/skills/llman-sdd-onboard/SKILL.md")
                .exists()
        );
        assert!(
            root.join(".agents/skills/llman-sdd-apply/SKILL.md")
                .exists()
        );

        // Enabled extra skills present
        assert!(
            root.join(".agents/skills/llman-sdd-verify/SKILL.md")
                .exists()
        );
        assert!(root.join(".agents/skills/llman-sdd-show/SKILL.md").exists());

        // Non-enabled optional skills absent
        assert!(
            !root
                .join(".agents/skills/llman-sdd-new-change/SKILL.md")
                .exists()
        );
        assert!(
            !root
                .join(".agents/skills/llman-sdd-continue/SKILL.md")
                .exists()
        );
    }
}

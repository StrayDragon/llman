use super::config::load_or_create_config;
use super::templates::{skill_templates, workflow_command_templates};
use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct UpdateSkillsArgs {
    pub no_interactive: bool,
    pub commands_only: bool,
    pub skills_only: bool,
}

const LLMAN_SDD_COMMAND_IDS: &[&str] = &[
    "explore", "onboard", "propose", "new", "continue", "ff", "apply", "verify", "sync", "archive",
];

const REQUIRED_ETHICS_KEYS: &[&str] = &[
    "ethics.risk_level",
    "ethics.prohibited_actions",
    "ethics.required_evidence",
    "ethics.refusal_contract",
    "ethics.escalation_policy",
];

pub fn run(args: UpdateSkillsArgs) -> Result<()> {
    run_with_root(Path::new("."), args)
}

pub(crate) fn run_with_root(root: &Path, args: UpdateSkillsArgs) -> Result<()> {
    let llmanspec_path = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        let cmd = "llman sdd init";
        return Err(anyhow!(t!("sdd.update_skills.no_llmanspec", cmd = cmd)));
    }

    let generate_skills = !args.commands_only;
    let generate_commands = !args.skills_only;

    let config = load_or_create_config(&llmanspec_path)?;

    if generate_skills {
        let templates = skill_templates(&config, root)?;
        enforce_ethics_governance(&templates)?;
        let skills_base = root.join(".agents").join("skills");
        write_tool_skills(&skills_base, &templates)?;
    }

    if generate_commands {
        let commands = workflow_command_templates(&config, root)?;
        write_llman_sdd_claude_commands(root, &commands)?;
    }

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

fn write_llman_sdd_claude_commands(
    root: &Path,
    commands: &[super::templates::WorkflowCommandTemplate],
) -> Result<()> {
    let base = root.join(".claude/commands/llman-sdd");
    fs::create_dir_all(&base)?;
    for id in LLMAN_SDD_COMMAND_IDS {
        let body = find_command_body(commands, id)?;
        let cmd_name = format!("LLMAN SDD: {}", capitalize(id));
        let cmd_description = format!("Run the llman-sdd-{} skill", id);
        let cmd_tags = vec!["workflow", "sdd", "llman-sdd", id];
        let content = format!(
            "---\nname: {}\ndescription: {}\ncategory: {}\ntags: {}\n---\n\n{}\n",
            yaml_string(&cmd_name),
            yaml_string(&cmd_description),
            yaml_string("Workflow"),
            yaml_tags(&cmd_tags),
            body.trim_end()
        );
        let path = base.join(format!("{}.md", id));
        atomic_write_with_mode(&path, content.as_bytes(), None)?;
    }
    Ok(())
}

fn find_command_body<'a>(
    commands: &'a [super::templates::WorkflowCommandTemplate],
    id: &str,
) -> Result<&'a str> {
    commands
        .iter()
        .find(|t| t.id == id)
        .map(|t| t.content.as_str())
        .ok_or_else(|| {
            anyhow!(t!(
                "sdd.update_skills.missing_workflow_command_template",
                id = id
            ))
        })
}

fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_uppercase().to_string() + chars.as_str(),
    }
}

fn yaml_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{}\"", escaped)
}

fn yaml_tags(tags: &[&str]) -> String {
    let items: Vec<String> = tags.iter().map(|tag| yaml_string(tag)).collect();
    format!("[{}]", items.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const EXPECTED_WORKFLOW_COMMANDS: &[&str] = &[
        "explore", "onboard", "propose", "new", "continue", "ff", "apply", "verify", "sync",
        "archive",
    ];

    const EXPECTED_WORKFLOW_SKILLS: &[&str] = &[
        "llman-sdd-onboard",
        "llman-sdd-propose",
        "llman-sdd-new-change",
        "llman-sdd-archive",
        "llman-sdd-explore",
        "llman-sdd-continue",
        "llman-sdd-ff",
        "llman-sdd-apply",
        "llman-sdd-verify",
        "llman-sdd-sync",
    ];

    #[test]
    fn update_skills_writes_skills_to_agents_skills() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let args = UpdateSkillsArgs {
            no_interactive: true,
            commands_only: false,
            skills_only: false,
        };

        super::run_with_root(root, args).expect("update-skills");

        for skill in EXPECTED_WORKFLOW_SKILLS {
            assert!(
                root.join(".agents/skills")
                    .join(skill)
                    .join("SKILL.md")
                    .exists(),
                "missing skill {skill}"
            );
        }

        for cmd in EXPECTED_WORKFLOW_COMMANDS {
            let path = root
                .join(".claude/commands/llman-sdd")
                .join(format!("{cmd}.md"));
            assert!(path.exists(), "missing command {cmd}");
            let content = fs::read_to_string(&path).expect("read command");
            assert!(
                content.contains("name:"),
                "command missing frontmatter name: {cmd}"
            );
            assert!(
                content.contains("category:"),
                "command missing frontmatter category: {cmd}"
            );
            assert!(
                content.contains("name: \"LLMAN SDD:"),
                "command name format mismatch: {cmd}"
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

        let args = UpdateSkillsArgs {
            no_interactive: true,
            commands_only: false,
            skills_only: false,
        };

        let result = super::run_with_root(root, args);
        assert!(result.is_err(), "expected missing ethics key failure");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("ethics.escalation_policy"),
            "unexpected error message: {err}"
        );
    }

    #[test]
    fn update_skills_commands_only_skips_skills_output() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let args = UpdateSkillsArgs {
            no_interactive: true,
            commands_only: true,
            skills_only: false,
        };

        super::run_with_root(root, args).expect("update-skills");
        assert!(root.join(".claude/commands/llman-sdd/new.md").exists());
        assert!(!root.join(".agents/skills").exists());
    }

    #[test]
    fn update_skills_skills_only_skips_workflow_commands() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let args = UpdateSkillsArgs {
            no_interactive: true,
            commands_only: false,
            skills_only: true,
        };

        super::run_with_root(root, args).expect("update-skills");
        assert!(
            root.join(".agents/skills/llman-sdd-onboard/SKILL.md")
                .exists()
        );
        assert!(!root.join(".claude/commands/llman-sdd").exists());
    }

    #[test]
    fn update_skills_errors_without_llmanspec() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();

        let args = UpdateSkillsArgs {
            no_interactive: true,
            commands_only: false,
            skills_only: false,
        };

        let result = super::run_with_root(root, args);
        assert!(result.is_err(), "expected error without llmanspec dir");
        let err = result.unwrap_err().to_string().to_lowercase();
        assert!(
            err.contains("no llmanspec"),
            "unexpected error message: {err}"
        );
    }
}

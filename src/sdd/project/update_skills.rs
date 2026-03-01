use super::config::{SddConfig, load_or_create_config, resolve_skill_path};
use super::templates::{TemplateStyle, skill_templates, workflow_command_templates};
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::interactive::is_interactive;
use anyhow::{Result, anyhow};
use inquire::{Confirm, MultiSelect, Text};
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct UpdateSkillsArgs {
    pub all: bool,
    pub tool: Vec<String>,
    pub path: Option<PathBuf>,
    pub no_interactive: bool,
    pub commands_only: bool,
    pub skills_only: bool,
    pub style: TemplateStyle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum SkillTool {
    Claude,
    Codex,
}

impl SkillTool {
    fn from_str(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "claude" | "claude-code" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex",
        }
    }
}

impl fmt::Display for SkillTool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

struct WorkflowCommandSpec {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    tags: &'static [&'static str],
}

const LLMAN_SDD_COMMAND_SPECS: &[WorkflowCommandSpec] = &[
    WorkflowCommandSpec {
        id: "explore",
        name: "LLMAN SDD: Explore",
        description: "Enter explore mode - think through ideas and clarify requirements",
        tags: &["workflow", "sdd", "llman-sdd", "explore"],
    },
    WorkflowCommandSpec {
        id: "onboard",
        name: "LLMAN SDD: Onboard",
        description: "Guided onboarding through a complete llman SDD workflow cycle",
        tags: &["workflow", "sdd", "llman-sdd", "onboard"],
    },
    WorkflowCommandSpec {
        id: "new",
        name: "LLMAN SDD: New",
        description: "Start a new llman SDD change",
        tags: &["workflow", "sdd", "llman-sdd", "new"],
    },
    WorkflowCommandSpec {
        id: "continue",
        name: "LLMAN SDD: Continue",
        description: "Continue working on a change - create the next artifact",
        tags: &["workflow", "sdd", "llman-sdd", "continue"],
    },
    WorkflowCommandSpec {
        id: "ff",
        name: "LLMAN SDD: Fast-Forward",
        description: "Create all change artifacts quickly",
        tags: &["workflow", "sdd", "llman-sdd", "ff"],
    },
    WorkflowCommandSpec {
        id: "apply",
        name: "LLMAN SDD: Apply",
        description: "Implement tasks from a change",
        tags: &["workflow", "sdd", "llman-sdd", "apply"],
    },
    WorkflowCommandSpec {
        id: "verify",
        name: "LLMAN SDD: Verify",
        description: "Verify implementation matches the change artifacts",
        tags: &["workflow", "sdd", "llman-sdd", "verify"],
    },
    WorkflowCommandSpec {
        id: "sync",
        name: "LLMAN SDD: Sync",
        description: "Manually sync delta specs into main specs without archiving",
        tags: &["workflow", "sdd", "llman-sdd", "sync"],
    },
    WorkflowCommandSpec {
        id: "archive",
        name: "LLMAN SDD: Archive",
        description: "Archive one or multiple completed changes",
        tags: &["workflow", "sdd", "llman-sdd", "archive"],
    },
];

#[derive(Default)]
struct LegacyBindings {
    claude_dirs: Vec<PathBuf>,
    codex_prompts: Vec<PathBuf>,
}

impl LegacyBindings {
    fn is_empty(&self) -> bool {
        self.claude_dirs.is_empty() && self.codex_prompts.is_empty()
    }

    fn display_paths(&self, root: &Path) -> Vec<String> {
        let mut paths = Vec::new();
        for dir in &self.claude_dirs {
            paths.push(display_relative(root, dir));
        }
        for path in &self.codex_prompts {
            paths.push(display_relative(root, path));
        }
        paths.sort();
        paths
    }
}

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

fn run_with_root(root: &Path, args: UpdateSkillsArgs) -> Result<()> {
    let llmanspec_path = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        let cmd = match args.style {
            TemplateStyle::New => "llman sdd init",
            TemplateStyle::Legacy => "llman sdd-legacy init",
        };
        return Err(anyhow!(t!("sdd.update_skills.no_llmanspec", cmd = cmd)));
    }

    let interactive = is_interactive(args.no_interactive);
    let generate_skills = !args.commands_only;
    let generate_commands = !args.skills_only;

    let tools = resolve_tools(&args, interactive)?;
    if args.commands_only && !tools.contains(&SkillTool::Claude) {
        return Err(anyhow!(t!(
            "sdd.update_skills.commands_only_requires_claude"
        )));
    }

    if generate_skills && args.path.is_some() && tools.len() > 1 {
        return Err(anyhow!(t!("sdd.update_skills.multi_tool_path_conflict")));
    }

    let legacy = detect_legacy_bindings(root, &tools)?;
    if !legacy.is_empty() {
        if !interactive {
            return Err(anyhow!(t!("sdd.update_skills.legacy_requires_interactive")));
        }
        migrate_legacy_bindings(root, &legacy)?;
    }

    let config = load_or_create_config(&llmanspec_path)?;
    if generate_skills {
        let outputs = resolve_outputs(root, &config, &tools, args.path.as_deref(), interactive)?;
        let templates = skill_templates(&config, root, args.style)?;
        if args.style == TemplateStyle::New {
            enforce_ethics_governance(&templates)?;
        }
        for path in outputs {
            write_tool_skills(&path, &templates)?;
        }
    }

    if generate_commands {
        let commands = workflow_command_templates(&config, root, args.style)?;
        write_llman_sdd_commands(root, &tools, &commands)?;
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

fn resolve_tools(args: &UpdateSkillsArgs, interactive: bool) -> Result<Vec<SkillTool>> {
    if args.all {
        return Ok(vec![SkillTool::Claude, SkillTool::Codex]);
    }

    if !args.tool.is_empty() {
        return parse_tool_args(&args.tool);
    }

    if !interactive {
        return Err(anyhow!(t!("sdd.update_skills.tools_required")));
    }

    let options = vec![SkillTool::Claude, SkillTool::Codex];
    let picked = MultiSelect::new(&t!("sdd.update_skills.select_tools"), options).prompt()?;
    if picked.is_empty() {
        return Err(anyhow!(t!("sdd.update_skills.no_tools_selected")));
    }
    Ok(picked)
}

fn parse_tool_args(values: &[String]) -> Result<Vec<SkillTool>> {
    let mut selected = BTreeSet::new();
    for value in values {
        for entry in value.split(',') {
            let tool = SkillTool::from_str(entry)
                .ok_or_else(|| anyhow!(t!("sdd.update_skills.invalid_tool", tool = entry)))?;
            selected.insert(tool);
        }
    }
    Ok(selected.into_iter().collect())
}

fn resolve_outputs(
    root: &Path,
    config: &SddConfig,
    tools: &[SkillTool],
    override_path: Option<&Path>,
    interactive: bool,
) -> Result<Vec<PathBuf>> {
    let mut outputs = Vec::new();
    for tool in tools {
        let default_path = match tool {
            SkillTool::Claude => &config.skills.claude_path,
            SkillTool::Codex => &config.skills.codex_path,
        };
        let resolved = if let Some(path) = override_path {
            resolve_override_path(root, path)
        } else if interactive {
            let prompt = t!("sdd.update_skills.prompt_path", tool = tool.label());
            let input = Text::new(&prompt).with_default(default_path).prompt()?;
            resolve_skill_path(root, &input)
        } else {
            resolve_skill_path(root, default_path)
        };

        outputs.push(resolved);
    }
    Ok(outputs)
}

fn resolve_override_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn write_tool_skills(base: &Path, templates: &[super::templates::SkillTemplate]) -> Result<()> {
    fs::create_dir_all(base)?;
    for template in templates {
        let dir_name = template.name.trim_end_matches(".md");
        let skill_dir = base.join(dir_name);
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), &template.content)?;
    }
    Ok(())
}

fn detect_legacy_bindings(root: &Path, tools: &[SkillTool]) -> Result<LegacyBindings> {
    let mut legacy = LegacyBindings::default();

    if tools.contains(&SkillTool::Claude) {
        for dir in [
            root.join(".claude/commands/openspec"),
            root.join(".claude/commands/opsx"),
        ] {
            if dir.exists() {
                legacy.claude_dirs.push(dir);
            }
        }
    }

    if tools.contains(&SkillTool::Codex) {
        let prompts_dir = root.join(".codex/prompts");
        if prompts_dir.is_dir() {
            for entry in fs::read_dir(&prompts_dir)? {
                let entry = entry?;
                if !entry.file_type()?.is_file() {
                    continue;
                }
                let name = entry.file_name();
                let Some(name) = name.to_str() else {
                    continue;
                };
                if (name.starts_with("openspec-") || name.starts_with("opsx-"))
                    && name.ends_with(".md")
                {
                    legacy.codex_prompts.push(entry.path());
                }
            }
        }
    }

    Ok(legacy)
}

fn migrate_legacy_bindings(root: &Path, legacy: &LegacyBindings) -> Result<()> {
    let paths = legacy.display_paths(root);
    eprintln!(
        "{}",
        t!(
            "sdd.update_skills.legacy_detected",
            paths = paths.join("\n- ")
        )
    );

    let proceed = Confirm::new(&t!("sdd.update_skills.legacy_confirm"))
        .with_default(false)
        .prompt()?;
    if !proceed {
        return Err(anyhow!(t!("sdd.update_skills.legacy_aborted")));
    }

    let phrase = t!("sdd.update_skills.legacy_confirm_phrase");
    let typed = Text::new(&t!(
        "sdd.update_skills.legacy_confirm_prompt",
        phrase = phrase
    ))
    .prompt()?;
    if typed.trim() != phrase {
        return Err(anyhow!(t!("sdd.update_skills.legacy_phrase_mismatch")));
    }

    for dir in &legacy.claude_dirs {
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
    }
    for path in &legacy.codex_prompts {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }

    Ok(())
}

fn write_llman_sdd_commands(
    root: &Path,
    tools: &[SkillTool],
    commands: &[super::templates::WorkflowCommandTemplate],
) -> Result<()> {
    if tools.contains(&SkillTool::Claude) {
        write_llman_sdd_claude_commands(root, commands)?;
    }
    Ok(())
}

fn write_llman_sdd_claude_commands(
    root: &Path,
    commands: &[super::templates::WorkflowCommandTemplate],
) -> Result<()> {
    let base = root.join(".claude/commands/llman-sdd");
    fs::create_dir_all(&base)?;
    for spec in LLMAN_SDD_COMMAND_SPECS {
        let body = find_command_body(commands, spec.id)?;
        let content = format!(
            "---\nname: {}\ndescription: {}\ncategory: {}\ntags: {}\n---\n\n{}\n",
            yaml_string(spec.name),
            yaml_string(spec.description),
            yaml_string("Workflow"),
            yaml_tags(spec.tags),
            body.trim_end()
        );
        fs::write(base.join(format!("{}.md", spec.id)), content)?;
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

fn display_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    const EXPECTED_WORKFLOW_COMMANDS: &[&str] = &[
        "explore", "onboard", "new", "continue", "ff", "apply", "verify", "sync", "archive",
    ];

    const EXPECTED_WORKFLOW_SKILLS: &[&str] = &[
        "llman-sdd-onboard",
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
    fn resolve_override_path_respects_relative() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        let path = Path::new(".claude/skills");
        let resolved = resolve_override_path(root, path);
        assert_eq!(resolved, root.join(".claude/skills"));
    }

    #[test]
    fn parse_tool_args_supports_csv() {
        let values = vec!["claude,codex".to_string()];
        let tools = parse_tool_args(&values).expect("tools");
        assert_eq!(tools, vec![SkillTool::Claude, SkillTool::Codex]);
    }

    #[test]
    fn rejects_multi_tool_override_path() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let args = UpdateSkillsArgs {
            all: true,
            tool: Vec::new(),
            path: Some(PathBuf::from("./skills-out")),
            no_interactive: true,
            commands_only: false,
            skills_only: false,
            style: TemplateStyle::New,
        };

        let result = super::run_with_root(root, args);
        assert!(result.is_err());
        assert!(!root.join("skills-out").exists());
        assert!(!root.join(LLMANSPEC_DIR_NAME).join("config.yaml").exists());
    }

    #[test]
    fn update_skills_writes_workflow_skills_and_commands_for_claude() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let args = UpdateSkillsArgs {
            all: false,
            tool: vec!["claude".to_string()],
            path: None,
            no_interactive: true,
            commands_only: false,
            skills_only: false,
            style: TemplateStyle::New,
        };

        super::run_with_root(root, args).expect("update-skills");

        for skill in EXPECTED_WORKFLOW_SKILLS {
            assert!(
                root.join(".claude/skills")
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
                content.contains("<!-- llman-template-version: 1 -->"),
                "command does not include command body template: {cmd}"
            );
        }

        assert!(
            !root.join(".claude/commands/openspec").exists(),
            "must not create legacy command dir"
        );
    }

    #[test]
    fn update_skills_codex_generates_skills_without_command_prompts() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let args = UpdateSkillsArgs {
            all: false,
            tool: vec!["codex".to_string()],
            path: None,
            no_interactive: true,
            commands_only: false,
            skills_only: false,
            style: TemplateStyle::New,
        };

        super::run_with_root(root, args).expect("update-skills");

        for skill in EXPECTED_WORKFLOW_SKILLS {
            assert!(
                root.join(".codex/skills")
                    .join(skill)
                    .join("SKILL.md")
                    .exists(),
                "missing codex skill {skill}"
            );
        }

        for cmd in EXPECTED_WORKFLOW_COMMANDS {
            let path = root
                .join(".codex/prompts")
                .join(format!("llman-sdd-{cmd}.md"));
            assert!(!path.exists(), "unexpected codex prompt {cmd}");
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
metadata:
  llman-template-version: 1
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
            all: false,
            tool: vec!["codex".to_string()],
            path: None,
            no_interactive: true,
            commands_only: false,
            skills_only: false,
            style: TemplateStyle::New,
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
    fn update_skills_commands_only_requires_claude() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let args = UpdateSkillsArgs {
            all: false,
            tool: vec!["codex".to_string()],
            path: None,
            no_interactive: true,
            commands_only: true,
            skills_only: false,
            style: TemplateStyle::New,
        };

        let result = super::run_with_root(root, args);
        assert!(result.is_err(), "expected commands-only codex to fail");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("No selected tool supports llman sdd workflow commands"),
            "unexpected error message: {err}"
        );
        assert!(!root.join(".claude/commands/llman-sdd").exists());
        assert!(!root.join(".codex/prompts").exists());
        assert!(!root.join(".codex/skills").exists());
    }

    #[test]
    fn update_skills_refuses_legacy_migration_in_no_interactive_mode() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        fs::create_dir_all(root.join(".claude/commands/openspec")).expect("legacy dir");
        fs::create_dir_all(root.join(".codex/prompts")).expect("codex prompts dir");
        fs::write(root.join(".codex/prompts/openspec-proposal.md"), "legacy")
            .expect("legacy prompt");

        let args = UpdateSkillsArgs {
            all: true,
            tool: Vec::new(),
            path: None,
            no_interactive: true,
            commands_only: false,
            skills_only: false,
            style: TemplateStyle::New,
        };

        let result = super::run_with_root(root, args);
        assert!(result.is_err(), "expected refusal in no-interactive mode");

        assert!(
            root.join(".claude/commands/openspec").exists(),
            "must not delete legacy dir in no-interactive mode"
        );
        assert!(
            root.join(".codex/prompts/openspec-proposal.md").exists(),
            "must not delete legacy prompt in no-interactive mode"
        );
    }

    #[test]
    fn update_skills_commands_only_skips_skills_output() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let args = UpdateSkillsArgs {
            all: false,
            tool: vec!["claude".to_string()],
            path: Some(PathBuf::from("./skills-out")),
            no_interactive: true,
            commands_only: true,
            skills_only: false,
            style: TemplateStyle::New,
        };

        super::run_with_root(root, args).expect("update-skills");
        assert!(root.join(".claude/commands/llman-sdd/new.md").exists());
        assert!(!root.join("skills-out").exists());
        assert!(!root.join(".claude/skills").exists());
    }

    #[test]
    fn update_skills_skills_only_skips_workflow_commands() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path();
        fs::create_dir_all(root.join(LLMANSPEC_DIR_NAME)).expect("create llmanspec");

        let args = UpdateSkillsArgs {
            all: false,
            tool: vec!["claude".to_string()],
            path: None,
            no_interactive: true,
            commands_only: false,
            skills_only: true,
            style: TemplateStyle::New,
        };

        super::run_with_root(root, args).expect("update-skills");
        assert!(
            root.join(".claude/skills/llman-sdd-onboard/SKILL.md")
                .exists()
        );
        assert!(!root.join(".claude/commands/llman-sdd").exists());
    }
}

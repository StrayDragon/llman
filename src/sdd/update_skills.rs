use crate::sdd::config::{SddConfig, load_or_create_config, resolve_skill_path};
use crate::sdd::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::interactive::is_interactive;
use crate::sdd::templates::skill_templates;
use anyhow::{Result, anyhow};
use inquire::{MultiSelect, Text};
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

pub fn run(args: UpdateSkillsArgs) -> Result<()> {
    run_with_root(Path::new("."), args)
}

fn run_with_root(root: &Path, args: UpdateSkillsArgs) -> Result<()> {
    let llmanspec_path = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_path.exists() {
        return Err(anyhow!(t!(
            "sdd.update_skills.no_llmanspec",
            cmd = "llman sdd init"
        )));
    }

    let config = load_or_create_config(&llmanspec_path)?;
    let interactive = is_interactive(args.no_interactive);
    let tools = resolve_tools(&args, interactive)?;
    let outputs = resolve_outputs(root, &config, &tools, args.path.as_deref(), interactive)?;
    let templates = skill_templates(&config, root)?;

    for path in outputs {
        write_tool_skills(&path, &templates)?;
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

fn write_tool_skills(
    base: &Path,
    templates: &[crate::sdd::templates::SkillTemplate],
) -> Result<()> {
    fs::create_dir_all(base)?;
    for template in templates {
        let dir_name = template.name.trim_end_matches(".md");
        let skill_dir = base.join(dir_name);
        fs::create_dir_all(&skill_dir)?;
        fs::write(skill_dir.join("SKILL.md"), &template.content)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn resolve_override_path_respects_relative() {
        let root = env::temp_dir().join("llman-sdd-skills");
        let path = Path::new(".claude/skills");
        let resolved = resolve_override_path(&root, path);
        assert_eq!(resolved, root.join(".claude/skills"));
    }

    #[test]
    fn parse_tool_args_supports_csv() {
        let values = vec!["claude,codex".to_string()];
        let tools = parse_tool_args(&values).expect("tools");
        assert_eq!(tools, vec![SkillTool::Claude, SkillTool::Codex]);
    }
}

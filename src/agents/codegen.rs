use crate::agents::command::FrameworkArg;
use crate::agents::manifest::load_agent_manifest_v1;
use crate::config::resolve_config_dir;
use crate::skills::catalog::types::SkillsPaths;
use crate::skills::cli::interactive::is_interactive;
use anyhow::{Context, Result, anyhow};
use inquire::Confirm;
use inquire::error::InquireError;
use minijinja::{Environment, context};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run_gen_code(
    id: &str,
    framework: FrameworkArg,
    out: &PathBuf,
    force: bool,
    skills_dir_override: Option<&Path>,
) -> Result<()> {
    if id.trim().is_empty() {
        return Err(anyhow!("agent id is required"));
    }

    let interactive = is_interactive();
    let config_dir = resolve_config_dir(None)?;
    let paths = SkillsPaths::resolve_with_override(skills_dir_override)?;

    let agent_skill_file = paths.root.join(id).join("SKILL.md");
    if !agent_skill_file.exists() {
        return Err(anyhow!(
            "Missing agent-skill file: {} (run `llman agents new {}` first)",
            agent_skill_file.display(),
            id
        ));
    }

    let manifest_file = config_dir.join("agents").join(id).join("agent.toml");
    if !manifest_file.exists() {
        return Err(anyhow!(
            "Missing agent manifest: {} (run `llman agents new {}` first)",
            manifest_file.display(),
            id
        ));
    }

    fs::create_dir_all(out)?;
    let out_file = out.join("agent.py");
    if out_file.exists() && !force {
        if !interactive {
            return Err(anyhow!(
                "Output file already exists: {} (use --force to overwrite)",
                out_file.display()
            ));
        }
        let prompt = t!("messages.file_exists_overwrite", path = out_file.display());
        let confirm = match Confirm::new(&prompt).with_default(false).prompt() {
            Ok(confirm) => confirm,
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                println!("{}", t!("messages.operation_cancelled"));
                return Ok(());
            }
            Err(e) => return Err(anyhow!("overwrite confirmation prompt failed: {}", e)),
        };
        if !confirm {
            println!("{}", t!("messages.operation_cancelled"));
            return Ok(());
        }
    }

    let manifest = load_agent_manifest_v1(&manifest_file)?;
    let skill_raw = fs::read_to_string(&agent_skill_file)
        .with_context(|| format!("read {}", agent_skill_file.display()))?;
    let system_prompt = extract_skill_body_markdown(&skill_raw).trim().to_string();
    let system_prompt_json = serde_json::to_string(&system_prompt).context("json encode prompt")?;

    let template = match framework {
        FrameworkArg::PydanticAi => TEMPLATE_PYDANTIC_AI,
        FrameworkArg::CrewAi => TEMPLATE_CREWAI,
    };

    let mut env = Environment::new();
    env.add_template("agent.py", template)
        .context("load agent.py template")?;
    let tmpl = env
        .get_template("agent.py")
        .context("get agent.py template")?;
    let rendered = tmpl
        .render(context! {
            agent_id => id,
            system_prompt_json => system_prompt_json,
            includes => manifest.includes,
            skills_meta => manifest.skills,
        })
        .context("render agent.py template")?;

    fs::write(&out_file, rendered).with_context(|| format!("write {}", out_file.display()))?;
    Ok(())
}

const TEMPLATE_PYDANTIC_AI: &str = include_str!("../../templates/agents/pydantic-ai/agent.py.j2");
const TEMPLATE_CREWAI: &str = include_str!("../../templates/agents/crewai/agent.py.j2");

fn extract_skill_body_markdown(raw: &str) -> &str {
    let mut chunks = raw.split_inclusive('\n');
    let Some(first) = chunks.next() else {
        return raw;
    };
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return raw;
    }

    let mut offset = first.len();
    for chunk in chunks {
        offset += chunk.len();
        if chunk.trim() == "---" {
            break;
        }
    }

    raw.get(offset..).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_body_ignores_frontmatter() {
        let raw = "---\nname: test\n---\n\nhello\nworld\n";
        assert_eq!(extract_skill_body_markdown(raw), "\nhello\nworld\n");
    }
}

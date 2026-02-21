use crate::agents::builder::AgentPresetBuildOutput;
use crate::agents::manifest::AgentManifestV1;
use crate::config::resolve_config_dir;
use crate::skills::catalog::scan::discover_skills;
use crate::skills::catalog::types::SkillsPaths;
use crate::skills::cli::interactive::is_interactive;
use crate::skills::cli::tui_picker::{TuiEntry, TuiEntryKind};
use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(about = "Manage agent presets")]
#[command(subcommand_required = true)]
pub struct AgentsArgs {
    /// Override skills root directory (env: LLMAN_SKILLS_DIR)
    #[arg(long = "skills-dir", global = true)]
    pub skills_dir: Option<PathBuf>,

    #[command(subcommand)]
    pub command: AgentsCommand,
}

#[derive(Subcommand)]
pub enum AgentsCommand {
    /// Create a new agent preset (agent-skill + agent manifest)
    New {
        id: String,
        #[arg(long)]
        force: bool,
        /// Generate preset content with a local LLM client (requires feature: agents-ai)
        #[arg(long)]
        ai: bool,
    },
    /// Generate a minimal runnable code module from an agent preset
    GenCode {
        id: String,
        #[arg(long, value_enum)]
        framework: FrameworkArg,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        force: bool,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum FrameworkArg {
    #[value(name = "pydantic-ai")]
    PydanticAi,
    #[value(name = "crewai")]
    CrewAi,
}

pub fn run(args: &AgentsArgs) -> Result<()> {
    match &args.command {
        AgentsCommand::New { id, force, ai } => {
            run_new(id, *force, *ai, args.skills_dir.as_deref())
        }
        AgentsCommand::GenCode {
            id,
            framework,
            out,
            force,
        } => crate::agents::codegen::run_gen_code(
            id,
            *framework,
            out,
            *force,
            args.skills_dir.as_deref(),
        ),
    }
}

fn run_new(id: &str, force: bool, ai: bool, skills_dir_override: Option<&Path>) -> Result<()> {
    if id.trim().is_empty() {
        return Err(anyhow!("agent id is required"));
    }

    let interactive = is_interactive();
    let config_dir = resolve_config_dir(None)?;
    let paths = SkillsPaths::resolve_with_override(skills_dir_override)?;
    paths.ensure_dirs()?;

    let agent_skill_dir = paths.root.join(id);
    let agent_skill_file = agent_skill_dir.join("SKILL.md");
    let agent_manifest_dir = config_dir.join("agents").join(id);
    let agent_manifest_file = agent_manifest_dir.join("agent.toml");

    if !force && (agent_skill_dir.exists() || agent_manifest_dir.exists()) {
        return Err(anyhow!(
            "Agent preset already exists: {} or {} (use --force to overwrite)",
            agent_skill_dir.display(),
            agent_manifest_dir.display()
        ));
    }

    let discovered = discover_skills(&paths.root)?;

    let mut ai_output: Option<AgentPresetBuildOutput> = None;
    let mut includes = Vec::<String>::new();
    if ai {
        let output = run_ai_builder(id, &discovered)?;
        includes = output.includes.clone();
        ai_output = Some(output);
    } else if interactive {
        let picked = pick_includes_tui(id, &paths.root, &discovered)?;
        let Some(picked) = picked else {
            println!("{}", t!("messages.operation_cancelled"));
            return Ok(());
        };
        includes = picked.into_iter().collect::<Vec<_>>();
    }

    includes.retain(|skill_id| skill_id != id);
    includes.sort();
    includes.dedup();

    if interactive && !ai {
        // TUI already handled cancellation; allow empty selection
    }

    let mut manifest = AgentManifestV1::new(id.to_string(), includes);
    if let Some(output) = &ai_output {
        manifest.description = Some(output.description.clone());
    }
    manifest.normalize();

    let skill_md = match &ai_output {
        Some(output) => format!(
            "---\nname: {id}\n---\n\n{}\n",
            output.system_prompt_md.trim_end()
        ),
        None => default_agent_skill_markdown(id),
    };

    // Commit writes only after selection is complete (cancel-safe).
    if force {
        remove_path_if_exists(&agent_skill_dir)?;
        remove_path_if_exists(&agent_manifest_dir)?;
    }

    fs::create_dir_all(&agent_skill_dir)?;
    fs::write(&agent_skill_file, skill_md)?;

    fs::create_dir_all(&agent_manifest_dir)?;
    manifest.write_to_path(&agent_manifest_file)?;

    Ok(())
}

fn default_agent_skill_markdown(id: &str) -> String {
    format!(
        "---\nname: {id}\n---\n\nWrite your system prompt for `{id}` here.\n\n## Requirements\n\n- \n"
    )
}

fn pick_includes_tui(
    agent_id: &str,
    skills_root: &Path,
    discovered: &[crate::skills::catalog::types::SkillCandidate],
) -> Result<Option<HashSet<String>>> {
    let mut entries = Vec::<TuiEntry>::new();
    for skill in discovered {
        let dir_name = skill
            .skill_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&skill.skill_id);
        let label = format!("{} ({})", skill.skill_id, dir_name);
        entries.push(TuiEntry {
            label,
            kind: TuiEntryKind::Skill {
                skill_id: skill.skill_id.clone(),
            },
        });
    }

    let defaults = HashSet::new();
    crate::skills::cli::tui_picker::pick(
        &format!(
            "Select includes for {agent_id} (from {})",
            skills_root.display()
        ),
        &entries,
        &defaults,
    )
}

fn remove_path_if_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let meta = fs::symlink_metadata(path)?;
    if meta.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn run_ai_builder(
    id: &str,
    discovered: &[crate::skills::catalog::types::SkillCandidate],
) -> Result<AgentPresetBuildOutput> {
    #[cfg(not(feature = "agents-ai"))]
    {
        let _ = (id, discovered);
        Err(anyhow!(
            "`--ai` requires building llman with feature `agents-ai` (rebuild with `cargo +nightly build --features agents-ai`)."
        ))
    }
    #[cfg(feature = "agents-ai")]
    {
        let available_skill_ids = discovered
            .iter()
            .map(|skill| skill.skill_id.clone())
            .filter(|skill_id| skill_id != id)
            .collect::<Vec<_>>();

        let request = crate::agents::builder::AgentPresetBuildRequest {
            agent_id: id.to_string(),
            available_skill_ids,
        };
        crate::agents::builder::build_with_openai(&request)
    }
}

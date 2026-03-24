use crate::config::{CLAUDE_CODE_APP, Config};
use crate::prompts::managed_file::write_llman_managed_block;
use crate::prompts::paths::{claude_home_dir, cwd, project_root};
use crate::prompts::store as prompt_store;
use crate::skills::cli::interactive::is_interactive;
use anyhow::{Result, anyhow, bail};
use clap::{Args, Subcommand, ValueEnum};
use inquire::{Confirm, MultiSelect};
use std::fs;
use std::path::{Path, PathBuf};

const CLAUDE_MEMORY_FILE: &str = "CLAUDE.md";

#[derive(Args, Debug, Clone)]
#[command(about = "Manage Claude Code prompt templates and inject CLAUDE.md")]
#[command(subcommand_required = false)]
pub struct ClaudeCodePromptsArgs {
    #[command(subcommand)]
    pub command: Option<ClaudeCodePromptsCommand>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ClaudeCodePromptsCommand {
    /// Generate Claude Code memory injection from a template
    Gen {
        #[arg(long, required = true)]
        template: String,

        /// Target scope(s) for injection. Repeat or use a comma list (e.g. --scope global --scope project)
        #[arg(long, value_enum, value_delimiter = ',', action = clap::ArgAction::Append, default_value = "project")]
        scope: Vec<PromptScopeArg>,

        /// Force generation, skip repository detection / overwrite checks
        #[arg(long)]
        force: bool,
    },
    /// List available templates
    List,
    /// Create or update a template
    Upsert {
        #[arg(long)]
        name: String,
        #[command(flatten)]
        content: TemplateContentSource,
    },
    /// Remove a template
    Rm {
        #[arg(long)]
        name: String,
        /// Skip confirmation prompts (required for non-interactive deletes)
        #[arg(long)]
        yes: bool,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptScopeArg {
    Global,
    Project,
}

#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct TemplateContentSource {
    #[arg(long)]
    pub content: Option<String>,
    #[arg(long)]
    pub file: Option<PathBuf>,
}

pub fn run(args: &ClaudeCodePromptsArgs) -> Result<()> {
    let interactive = is_interactive();
    match &args.command {
        None => {
            if interactive {
                return run_wizard();
            }
            bail!("In non-interactive mode, a subcommand is required (gen/list/upsert/rm).");
        }
        Some(ClaudeCodePromptsCommand::Gen {
            template,
            scope,
            force,
        }) => run_gen(template, scope, *force, interactive),
        Some(ClaudeCodePromptsCommand::List) => run_list(),
        Some(ClaudeCodePromptsCommand::Upsert { name, content }) => {
            run_upsert(name, content.content.as_deref(), content.file.as_deref())
        }
        Some(ClaudeCodePromptsCommand::Rm { name, yes }) => run_rm(name, *yes, interactive),
    }
}

fn run_list() -> Result<()> {
    let config = Config::new()?;
    let rules = prompt_store::list_templates(&config, CLAUDE_CODE_APP)?;
    if rules.is_empty() {
        println!("  {}", t!("errors.no_rules_found"));
        return Ok(());
    }
    for rule in rules {
        println!("  {}", t!("prompt.list.rule_item", name = rule));
    }
    Ok(())
}

fn run_upsert(name: &str, content: Option<&str>, file: Option<&Path>) -> Result<()> {
    let config = Config::new()?;
    let rule_content = if let Some(content) = content {
        content.to_string()
    } else if let Some(file_path) = file {
        fs::read_to_string(file_path)?
    } else {
        return Err(anyhow!(t!("messages.content_or_file_required")));
    };

    let path = prompt_store::upsert_template(&config, CLAUDE_CODE_APP, name, &rule_content)?;
    println!("{}", t!("messages.rule_saved", path = path.display()));
    Ok(())
}

fn run_rm(name: &str, yes: bool, interactive: bool) -> Result<()> {
    if yes {
        let config = Config::new()?;
        prompt_store::remove_template(&config, CLAUDE_CODE_APP, name)?;
        println!("{}", t!("messages.rule_deleted", name = name));
        return Ok(());
    }

    if !interactive {
        return Err(anyhow!(t!(
            "errors.non_interactive_delete_requires_yes",
            name = name
        )));
    }

    let confirm = Confirm::new(&t!("messages.confirm_delete", name = name))
        .with_default(false)
        .prompt()?;

    if confirm {
        let config = Config::new()?;
        prompt_store::remove_template(&config, CLAUDE_CODE_APP, name)?;
        println!("{}", t!("messages.rule_deleted", name = name));
    } else {
        println!("{}", t!("messages.operation_cancelled"));
    }

    Ok(())
}

fn run_gen(
    template: &str,
    scopes: &[PromptScopeArg],
    force: bool,
    interactive: bool,
) -> Result<()> {
    let config = Config::new()?;
    let content = prompt_store::read_template(&config, CLAUDE_CODE_APP, template)?;
    let cwd = cwd()?;

    let mut first_error: Option<anyhow::Error> = None;

    for scope in [PromptScopeArg::Global, PromptScopeArg::Project]
        .into_iter()
        .filter(|s| scopes.contains(s))
    {
        let path = match claude_memory_path(&cwd, scope, force, interactive) {
            Ok(Some(path)) => path,
            Ok(None) => continue,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
                continue;
            }
        };

        match write_llman_managed_block(&path, &content, force, interactive) {
            Ok(true) => println!("{}", t!("messages.rule_generated", path = path.display())),
            Ok(false) => {}
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
            }
        }
    }

    if let Some(err) = first_error {
        return Err(err);
    }
    Ok(())
}

fn claude_memory_path(
    cwd: &Path,
    scope: PromptScopeArg,
    force: bool,
    interactive: bool,
) -> Result<Option<PathBuf>> {
    match scope {
        PromptScopeArg::Global => Ok(Some(claude_home_dir()?.join(CLAUDE_MEMORY_FILE))),
        PromptScopeArg::Project => project_root(cwd, force, interactive)
            .map(|root| root.map(|root| root.join(CLAUDE_MEMORY_FILE))),
    }
}

fn run_wizard() -> Result<()> {
    println!("{}", t!("interactive.title"));

    let config = Config::new()?;
    let templates = prompt_store::list_templates(&config, CLAUDE_CODE_APP)?;
    if templates.is_empty() {
        println!("{}", t!("interactive.no_templates"));
        println!("{}", t!("interactive.no_templates_hint"));
        return Ok(());
    }

    let scopes = {
        let options = vec!["project", "global"];
        let picked = MultiSelect::new(&t!("prompt.scope.select"), options).prompt()?;
        let scopes = picked
            .into_iter()
            .filter_map(|p| match p {
                "global" => Some(PromptScopeArg::Global),
                "project" => Some(PromptScopeArg::Project),
                _ => None,
            })
            .collect::<Vec<_>>();
        if scopes.is_empty() {
            println!("{}", t!("messages.operation_cancelled"));
            return Ok(());
        }
        scopes
    };

    let picked_templates =
        MultiSelect::new(&t!("interactive.select_template"), templates).prompt()?;
    if picked_templates.is_empty() {
        println!("{}", t!("messages.operation_cancelled"));
        return Ok(());
    }

    let body = prompt_store::build_llman_prompts_body(&config, CLAUDE_CODE_APP, &picked_templates)?;

    let cwd = cwd()?;
    let interactive = true;
    for scope in scopes {
        let path = match claude_memory_path(&cwd, scope, false, interactive)? {
            Some(path) => path,
            None => continue,
        };
        if write_llman_managed_block(&path, &body, false, interactive)? {
            println!("{}", t!("messages.rule_generated", path = path.display()));
        }
    }

    println!("{}", t!("messages.rule_generation_success"));
    Ok(())
}

use crate::config::{CURSOR_APP, Config, TARGET_CURSOR_RULES_DIR};
use crate::fs_utils::atomic_write_with_mode;
use crate::path_utils::{safe_parent_for_creation, validate_path_segment};
use crate::prompts::confirm::confirm_overwrite;
use crate::prompts::paths::{cwd, project_root};
use crate::prompts::store as prompt_store;
use crate::skills::cli::interactive::is_interactive;
use anyhow::{Result, anyhow, bail};
use clap::{Args, Subcommand, ValueEnum};
use inquire::{Confirm, MultiSelect};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Args, Debug, Clone)]
#[command(about = "Manage Cursor prompt templates and generate Cursor rules")]
#[command(subcommand_required = false)]
pub struct CursorPromptsArgs {
    #[command(subcommand)]
    pub command: Option<CursorPromptsCommand>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CursorPromptsCommand {
    /// Generate Cursor rules from a template
    Gen {
        #[arg(long, required = true)]
        template: String,

        /// Target scope(s) for injection. Cursor only supports `project`.
        #[arg(long, value_enum, value_delimiter = ',', action = clap::ArgAction::Append, default_value = "project")]
        scope: Vec<PromptScopeArg>,

        /// Output file name (defaults to template name)
        #[arg(long)]
        name: Option<String>,

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

pub fn run(args: &CursorPromptsArgs) -> Result<()> {
    let interactive = is_interactive();
    match &args.command {
        None => {
            if interactive {
                return run_wizard();
            }
            bail!("In non-interactive mode, a subcommand is required (gen/list/upsert/rm).");
        }
        Some(CursorPromptsCommand::Gen {
            template,
            scope,
            name,
            force,
        }) => run_gen(template, scope, name.as_deref(), *force, interactive),
        Some(CursorPromptsCommand::List) => run_list(),
        Some(CursorPromptsCommand::Upsert { name, content }) => {
            run_upsert(name, content.content.as_deref(), content.file.as_deref())
        }
        Some(CursorPromptsCommand::Rm { name, yes }) => run_rm(name, *yes, interactive),
    }
}

fn run_list() -> Result<()> {
    let config = Config::new()?;
    let rules = prompt_store::list_templates(&config, CURSOR_APP)?;
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

    let path = prompt_store::upsert_template(&config, CURSOR_APP, name, &rule_content)?;
    println!("{}", t!("messages.rule_saved", path = path.display()));
    Ok(())
}

fn run_rm(name: &str, yes: bool, interactive: bool) -> Result<()> {
    if yes {
        let config = Config::new()?;
        prompt_store::remove_template(&config, CURSOR_APP, name)?;
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
        prompt_store::remove_template(&config, CURSOR_APP, name)?;
        println!("{}", t!("messages.rule_deleted", name = name));
    } else {
        println!("{}", t!("messages.operation_cancelled"));
    }

    Ok(())
}

fn run_gen(
    template: &str,
    scopes: &[PromptScopeArg],
    name: Option<&str>,
    force: bool,
    interactive: bool,
) -> Result<()> {
    if scopes.iter().any(|s| *s != PromptScopeArg::Project) {
        return Err(anyhow!(t!(
            "errors.invalid_scope_for_app",
            app = CURSOR_APP,
            scope = "global"
        )));
    }

    let cwd = cwd()?;
    let Some(root) = project_root(&cwd, force, interactive)? else {
        return Ok(());
    };

    let config = Config::new()?;
    let content = prompt_store::read_template(&config, CURSOR_APP, template)?;

    let output_name = name.unwrap_or(template);
    let output_name = validate_path_segment(output_name, "prompt name")
        .map_err(|e| anyhow!("invalid prompt name: {e}"))?;

    let target_path = root
        .join(TARGET_CURSOR_RULES_DIR)
        .join(format!("{output_name}.mdc"));

    if target_path.exists() && !force {
        let overwrite = confirm_overwrite(&target_path, interactive)?;
        if !overwrite {
            println!("{}", t!("messages.operation_cancelled"));
            return Ok(());
        }
    }

    if let Some(parent) = safe_parent_for_creation(&target_path) {
        fs::create_dir_all(parent)?;
    }
    atomic_write_with_mode(&target_path, content.as_bytes(), None)?;
    println!(
        "{}",
        t!("messages.rule_generated", path = target_path.display())
    );
    Ok(())
}

fn run_wizard() -> Result<()> {
    let cwd = cwd()?;
    let Some(root) = project_root(&cwd, false, true)? else {
        return Ok(());
    };

    let config = Config::new()?;
    let templates = prompt_store::list_templates(&config, CURSOR_APP)?;
    if templates.is_empty() {
        println!("{}", t!("interactive.no_templates"));
        println!("{}", t!("interactive.no_templates_hint"));
        return Ok(());
    }

    let picked = MultiSelect::new(&t!("interactive.select_template"), templates).prompt()?;
    if picked.is_empty() {
        println!("{}", t!("messages.operation_cancelled"));
        return Ok(());
    }

    for template in picked {
        let content = prompt_store::read_template(&config, CURSOR_APP, &template)?;
        let name = validate_path_segment(&template, "prompt name")
            .map_err(|e| anyhow!("invalid prompt name: {e}"))?;
        let target_path = root
            .join(TARGET_CURSOR_RULES_DIR)
            .join(format!("{name}.mdc"));
        if target_path.exists() {
            let overwrite = confirm_overwrite(&target_path, true)?;
            if !overwrite {
                continue;
            }
        }
        if let Some(parent) = safe_parent_for_creation(&target_path) {
            fs::create_dir_all(parent)?;
        }
        atomic_write_with_mode(&target_path, content.as_bytes(), None)?;
        println!(
            "{}",
            t!("messages.rule_generated", path = target_path.display())
        );
    }

    Ok(())
}

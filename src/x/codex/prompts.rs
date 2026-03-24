use crate::config::{CODEX_APP, Config};
use crate::fs_utils::atomic_write_with_mode;
use crate::path_utils::{safe_parent_for_creation, validate_path_segment};
use crate::prompts::confirm::confirm_overwrite;
use crate::prompts::managed_file::write_llman_managed_block;
use crate::prompts::paths::{codex_home_dir, cwd, project_root};
use crate::prompts::store as prompt_store;
use crate::skills::cli::interactive::is_interactive;
use anyhow::{Result, anyhow, bail};
use clap::{Args, Subcommand, ValueEnum};
use inquire::MultiSelect;
use std::fs;
use std::path::{Path, PathBuf};

const CODEX_AGENTS_FILE: &str = "AGENTS.md";
const CODEX_AGENTS_OVERRIDE_FILE: &str = "AGENTS.override.md";

#[derive(Args, Debug, Clone)]
#[command(about = "Manage Codex prompt templates and inject Codex configuration files")]
#[command(subcommand_required = false)]
pub struct CodexPromptsArgs {
    #[command(subcommand)]
    pub command: Option<CodexPromptsCommand>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CodexPromptsCommand {
    /// Generate Codex prompts / project doc injection from a template
    Gen {
        #[arg(long, required = true)]
        template: String,

        /// Target scope(s) for injection. Repeat or use a comma list (e.g. --scope global --scope project)
        #[arg(long, value_enum, value_delimiter = ',', action = clap::ArgAction::Append, default_value = "project")]
        scope: Vec<PromptScopeArg>,

        /// Injection target(s): `prompts` (custom prompts) and/or `project-doc` (AGENTS*.md)
        #[arg(long, value_enum, value_delimiter = ',', action = clap::ArgAction::Append)]
        target: Vec<CodexTargetArg>,

        /// For codex + target=project-doc: write to AGENTS.override.md instead of AGENTS.md
        #[arg(long = "override")]
        override_file: bool,

        /// Output file name (defaults to template name). Only applies to `--target prompts`.
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

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum CodexTargetArg {
    /// Codex custom prompts under `prompts/*.md`
    Prompts,
    /// Codex project doc injection into `AGENTS*.md`
    #[value(name = "project-doc")]
    ProjectDoc,
}

#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct TemplateContentSource {
    #[arg(long)]
    pub content: Option<String>,
    #[arg(long)]
    pub file: Option<PathBuf>,
}

pub fn run(args: &CodexPromptsArgs) -> Result<()> {
    let interactive = is_interactive();
    match &args.command {
        None => {
            if interactive {
                return run_wizard();
            }
            bail!("In non-interactive mode, a subcommand is required (gen/list/upsert/rm).");
        }
        Some(CodexPromptsCommand::Gen {
            template,
            scope,
            target,
            override_file,
            name,
            force,
        }) => run_gen(
            template,
            scope,
            target,
            *override_file,
            name.as_deref(),
            *force,
            interactive,
        ),
        Some(CodexPromptsCommand::List) => run_list(),
        Some(CodexPromptsCommand::Upsert { name, content }) => {
            run_upsert(name, content.content.as_deref(), content.file.as_deref())
        }
        Some(CodexPromptsCommand::Rm { name, yes }) => run_rm(name, *yes, interactive),
    }
}

fn run_list() -> Result<()> {
    let config = Config::new()?;
    let rules = prompt_store::list_templates(&config, CODEX_APP)?;
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

    let path = prompt_store::upsert_template(&config, CODEX_APP, name, &rule_content)?;
    println!("{}", t!("messages.rule_saved", path = path.display()));
    Ok(())
}

fn run_rm(name: &str, yes: bool, interactive: bool) -> Result<()> {
    if yes {
        let config = Config::new()?;
        prompt_store::remove_template(&config, CODEX_APP, name)?;
        println!("{}", t!("messages.rule_deleted", name = name));
        return Ok(());
    }

    if !interactive {
        return Err(anyhow!(t!(
            "errors.non_interactive_delete_requires_yes",
            name = name
        )));
    }

    let confirm = inquire::Confirm::new(&t!("messages.confirm_delete", name = name))
        .with_default(false)
        .prompt()?;

    if confirm {
        let config = Config::new()?;
        prompt_store::remove_template(&config, CODEX_APP, name)?;
        println!("{}", t!("messages.rule_deleted", name = name));
    } else {
        println!("{}", t!("messages.operation_cancelled"));
    }

    Ok(())
}

fn normalized_targets(targets: &[CodexTargetArg]) -> Vec<CodexTargetArg> {
    if targets.is_empty() {
        return vec![CodexTargetArg::Prompts];
    }
    let mut out = targets.to_vec();
    out.sort();
    out.dedup();
    out
}

fn run_gen(
    template: &str,
    scopes: &[PromptScopeArg],
    targets: &[CodexTargetArg],
    override_file: bool,
    name: Option<&str>,
    force: bool,
    interactive: bool,
) -> Result<()> {
    let targets = normalized_targets(targets);
    if override_file && !targets.contains(&CodexTargetArg::ProjectDoc) {
        return Err(anyhow!(t!("errors.override_requires_agents_target")));
    }

    let config = Config::new()?;
    let content = prompt_store::read_template(&config, CODEX_APP, template)?;

    let output_name = name.unwrap_or(template);
    let output_name = validate_path_segment(output_name, "prompt name")
        .map_err(|e| anyhow!("invalid prompt name: {e}"))?;

    let cwd = cwd()?;

    let mut first_error: Option<anyhow::Error> = None;

    if targets.contains(&CodexTargetArg::Prompts)
        && let Err(e) =
            write_codex_prompt_files(&cwd, &output_name, scopes, force, interactive, &content)
    {
        first_error = Some(e);
    }

    if targets.contains(&CodexTargetArg::ProjectDoc) {
        let body = format!("## llman prompts: {template}\n\n{}", content.trim_end());
        if let Err(e) =
            write_codex_project_doc_files(&cwd, scopes, override_file, force, interactive, &body)
            && first_error.is_none()
        {
            first_error = Some(e);
        }
    }

    if let Some(err) = first_error {
        return Err(err);
    }
    Ok(())
}

fn write_codex_prompt_files(
    cwd: &Path,
    name: &str,
    scopes: &[PromptScopeArg],
    force: bool,
    interactive: bool,
    content: &str,
) -> Result<()> {
    let mut first_error: Option<anyhow::Error> = None;

    for scope in [PromptScopeArg::Global, PromptScopeArg::Project]
        .into_iter()
        .filter(|s| scopes.contains(s))
    {
        let target_path = match codex_prompt_path(cwd, scope, name, force, interactive) {
            Ok(Some(path)) => path,
            Ok(None) => continue,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
                continue;
            }
        };

        if target_path.exists() && !force {
            let overwrite = match confirm_overwrite(&target_path, interactive) {
                Ok(v) => v,
                Err(e) => {
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                    continue;
                }
            };
            if !overwrite {
                println!("{}", t!("messages.operation_cancelled"));
                continue;
            }
        }

        if let Some(parent) = safe_parent_for_creation(&target_path)
            && let Err(e) = fs::create_dir_all(parent)
        {
            if first_error.is_none() {
                first_error = Some(e.into());
            }
            continue;
        }

        if let Err(e) = atomic_write_with_mode(&target_path, content.as_bytes(), None) {
            if first_error.is_none() {
                first_error = Some(e);
            }
            continue;
        }

        println!(
            "{}",
            t!("messages.rule_generated", path = target_path.display())
        );
    }

    if let Some(err) = first_error {
        return Err(err);
    }
    Ok(())
}

fn write_codex_project_doc_files(
    cwd: &Path,
    scopes: &[PromptScopeArg],
    override_file: bool,
    force: bool,
    interactive: bool,
    body: &str,
) -> Result<()> {
    let file_name = if override_file {
        CODEX_AGENTS_OVERRIDE_FILE
    } else {
        CODEX_AGENTS_FILE
    };

    let mut first_error: Option<anyhow::Error> = None;

    for scope in [PromptScopeArg::Global, PromptScopeArg::Project]
        .into_iter()
        .filter(|s| scopes.contains(s))
    {
        let path = match codex_project_doc_path(cwd, scope, file_name, force, interactive) {
            Ok(Some(path)) => path,
            Ok(None) => continue,
            Err(e) => {
                if first_error.is_none() {
                    first_error = Some(e);
                }
                continue;
            }
        };

        match write_llman_managed_block(&path, body, force, interactive) {
            Ok(true) => {
                println!("{}", t!("messages.rule_generated", path = path.display()));
            }
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

fn codex_prompt_path(
    cwd: &Path,
    scope: PromptScopeArg,
    name: &str,
    force: bool,
    interactive: bool,
) -> Result<Option<PathBuf>> {
    match scope {
        PromptScopeArg::Global => Ok(Some(
            codex_home_dir()?.join("prompts").join(format!("{name}.md")),
        )),
        PromptScopeArg::Project => project_root(cwd, force, interactive).map(|root| {
            root.map(|root| {
                root.join(".codex")
                    .join("prompts")
                    .join(format!("{name}.md"))
            })
        }),
    }
}

fn codex_project_doc_path(
    cwd: &Path,
    scope: PromptScopeArg,
    file_name: &str,
    force: bool,
    interactive: bool,
) -> Result<Option<PathBuf>> {
    match scope {
        PromptScopeArg::Global => Ok(Some(codex_home_dir()?.join(file_name))),
        PromptScopeArg::Project => {
            project_root(cwd, force, interactive).map(|root| root.map(|root| root.join(file_name)))
        }
    }
}

fn run_wizard() -> Result<()> {
    println!("{}", t!("interactive.title"));

    let config = Config::new()?;
    let templates = prompt_store::list_templates(&config, CODEX_APP)?;
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

    let targets = {
        let options = vec!["prompts", CODEX_AGENTS_FILE, CODEX_AGENTS_OVERRIDE_FILE];
        let picked = MultiSelect::new(&t!("prompt.codex.target.select"), options).prompt()?;
        if picked.is_empty() {
            println!("{}", t!("messages.operation_cancelled"));
            return Ok(());
        }
        picked
    };

    let picked_templates =
        MultiSelect::new(&t!("interactive.select_template"), templates).prompt()?;
    if picked_templates.is_empty() {
        println!("{}", t!("messages.operation_cancelled"));
        return Ok(());
    }

    let cwd = cwd()?;
    let interactive = true;

    if targets.contains(&"prompts") {
        for template in &picked_templates {
            let content = prompt_store::read_template(&config, CODEX_APP, template)?;
            let name = validate_path_segment(template, "prompt name")
                .map_err(|e| anyhow!("invalid prompt name: {e}"))?;
            write_codex_prompt_files(&cwd, &name, &scopes, false, interactive, &content)?;
        }
    }

    if targets.contains(&CODEX_AGENTS_FILE) {
        let body = prompt_store::build_llman_prompts_body(&config, CODEX_APP, &picked_templates)?;
        write_codex_project_doc_files(&cwd, &scopes, false, false, interactive, &body)?;
    }

    if targets.contains(&CODEX_AGENTS_OVERRIDE_FILE) {
        let body = prompt_store::build_llman_prompts_body(&config, CODEX_APP, &picked_templates)?;
        write_codex_project_doc_files(&cwd, &scopes, true, false, interactive, &body)?;
    }

    println!("{}", t!("messages.rule_generation_success"));
    Ok(())
}

use crate::config::resolve_config_dir;
use crate::fs_utils::atomic_write_with_mode;
use crate::managed_block::{
    LLMAN_PROMPTS_MARKER_END, LLMAN_PROMPTS_MARKER_START, has_llman_prompt_markers,
    update_text_with_markers,
};
use crate::path_utils::validate_path_segment;
use crate::prompts::store as prompt_store;
use crate::skills::cli::interactive::is_interactive;
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand, ValueEnum};
use inquire::{Confirm, MultiSelect, Select};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

#[derive(Args, Debug, Clone)]
#[command(about = "Manage Codex custom agent configurations")]
#[command(subcommand_required = false)]
pub struct CodexAgentsArgs {
    #[command(subcommand)]
    pub command: Option<CodexAgentsCommand>,

    /// Managed agents directory (default: $LLMAN_CONFIG_DIR/codex/agents)
    #[arg(long = "managed-dir", global = true)]
    pub managed_dir: Option<PathBuf>,

    /// Override Codex home directory (uses <codex-home>/agents unless --agents-dir is set)
    #[arg(long = "codex-home", global = true)]
    pub codex_home: Option<PathBuf>,

    /// Override Codex agents directory directly
    #[arg(long = "agents-dir", global = true)]
    pub agents_dir: Option<PathBuf>,

    /// Only apply to selected agent names (repeatable; matches <name>.toml)
    #[arg(long, value_delimiter = ',', action = clap::ArgAction::Append, global = true)]
    pub only: Vec<String>,

    /// Show plan but do not write any files
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Assume "yes" for confirmations (required for non-interactive write operations)
    #[arg(long, global = true)]
    pub yes: bool,

    /// Force write operations without interactive confirmation (alias of --yes)
    #[arg(long, global = true)]
    pub force: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum CodexAgentsCommand {
    /// Show managed/target status (read-only)
    Status,
    /// Import target agents into managed directory
    Import,
    /// Sync managed agents to Codex agents directory
    Sync {
        #[arg(long, value_enum, default_value_t = SyncMode::Link)]
        mode: SyncMode,
    },
    /// Inject prompt templates into developer_instructions in managed agent TOMLs
    Inject {
        /// Prompt template name(s) under $LLMAN_CONFIG_DIR/prompt/codex/*.md (repeatable)
        #[arg(long, value_delimiter = ',', action = clap::ArgAction::Append)]
        template: Vec<String>,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    #[value(name = "link")]
    Link,
    #[value(name = "copy")]
    Copy,
}

pub fn run(args: &CodexAgentsArgs) -> Result<()> {
    let interactive = is_interactive();
    match &args.command {
        None => {
            if interactive {
                return run_wizard(args);
            }
            bail!("In non-interactive mode, a subcommand is required (status/import/sync/inject).");
        }
        Some(CodexAgentsCommand::Status) => run_status(args),
        Some(CodexAgentsCommand::Import) => run_import(args, interactive),
        Some(CodexAgentsCommand::Sync { mode }) => run_sync(args, *mode, interactive),
        Some(CodexAgentsCommand::Inject { template }) => run_inject(args, template, interactive),
    }
}

fn run_status(args: &CodexAgentsArgs) -> Result<()> {
    let managed_dir = resolve_managed_dir(args)?;
    let target_dir = resolve_target_agents_dir(args)?;

    println!("Managed agents dir: {}", managed_dir.display());
    println!("Target agents dir:  {}", target_dir.display());

    let managed = list_toml_stems(&managed_dir)?;
    if managed.is_empty() {
        println!("No managed agent TOMLs found.");
        return Ok(());
    }

    let selected = select_stems(&managed, &args.only, "agent name")?;
    println!();

    for stem in selected {
        let managed_file = managed_dir.join(format!("{stem}.toml"));
        let target_file = target_dir.join(format!("{stem}.toml"));

        let schema_state = describe_agent_schema_state(&managed_file)?;
        let target_state = describe_target_state(&managed_file, &target_file)?;
        let inject_state = describe_inject_state(&managed_file)?;

        println!(
            "- {stem}: schema={schema_state}; target={target_state}; inject={inject_state}",
            stem = stem,
            schema_state = schema_state,
            target_state = target_state,
            inject_state = inject_state
        );
    }

    Ok(())
}

fn run_import(args: &CodexAgentsArgs, interactive: bool) -> Result<()> {
    let managed_dir = resolve_managed_dir(args)?;
    let target_dir = resolve_target_agents_dir(args)?;
    let available = list_toml_stems(&target_dir)?;
    let selected = select_stems(&available, &args.only, "agent name")?;

    let plan = plan_import(&managed_dir, &target_dir, &selected)?;
    apply_plan(plan, args, interactive)
}

fn run_sync(args: &CodexAgentsArgs, mode: SyncMode, interactive: bool) -> Result<()> {
    let managed_dir = resolve_managed_dir(args)?;
    let target_dir = resolve_target_agents_dir(args)?;
    let available = list_toml_stems(&managed_dir)?;
    let selected = select_stems(&available, &args.only, "agent name")?;

    let plan = plan_sync(&managed_dir, &target_dir, &selected, mode)?;
    apply_plan(plan, args, interactive)
}

fn run_inject(args: &CodexAgentsArgs, templates: &[String], interactive: bool) -> Result<()> {
    if templates.is_empty() {
        bail!("--template is required");
    }

    let managed_dir = resolve_managed_dir(args)?;
    let available = list_toml_stems(&managed_dir)?;
    let selected = select_stems(&available, &args.only, "agent name")?;

    let body = build_injection_body(templates)?;
    let plan = plan_inject(&managed_dir, &selected, &body)?;
    apply_plan(plan, args, interactive)
}

fn run_wizard(args: &CodexAgentsArgs) -> Result<()> {
    let managed_dir = resolve_managed_dir(args)?;
    let target_dir = resolve_target_agents_dir(args)?;
    let confirm_all = args.yes || args.force;

    println!("Managed agents dir: {}", managed_dir.display());
    println!("Target agents dir:  {}", target_dir.display());
    println!();

    let action = Select::new(
        "Select an action:",
        vec!["status", "import", "inject", "sync"],
    )
    .prompt()
    .context("select action")?;

    match action {
        "status" => run_status(args),
        "import" => {
            let available = list_toml_stems(&target_dir)?;
            if available.is_empty() {
                bail!(
                    "No agent TOMLs found under target dir: {}",
                    target_dir.display()
                );
            }
            let selected = MultiSelect::new("Select agent files to import:", available)
                .with_all_selected_by_default()
                .prompt()
                .context("select agents to import")?;
            let plan = plan_import(&managed_dir, &target_dir, &selected)?;
            apply_plan_with_override(plan, args, true, confirm_all)
        }
        "inject" => {
            let available = list_toml_stems(&managed_dir)?;
            if available.is_empty() {
                bail!(
                    "No managed agent TOMLs found under: {}",
                    managed_dir.display()
                );
            }
            let selected = MultiSelect::new("Select managed agents to inject:", available)
                .with_all_selected_by_default()
                .prompt()
                .context("select agents to inject")?;

            let templates = list_codex_prompt_templates()?;
            if templates.is_empty() {
                bail!("No codex prompt templates found under $LLMAN_CONFIG_DIR/prompt/codex");
            }
            let picked = MultiSelect::new("Select codex prompt templates to inject:", templates)
                .prompt()
                .context("select templates")?;
            if picked.is_empty() {
                bail!("At least one template is required for inject");
            }

            let body = build_injection_body(&picked)?;
            let plan = plan_inject(&managed_dir, &selected, &body)?;
            apply_plan_with_override(plan, args, true, confirm_all)
        }
        "sync" => {
            let available = list_toml_stems(&managed_dir)?;
            if available.is_empty() {
                bail!(
                    "No managed agent TOMLs found under: {}",
                    managed_dir.display()
                );
            }
            let selected = MultiSelect::new("Select managed agents to sync:", available)
                .with_all_selected_by_default()
                .prompt()
                .context("select agents to sync")?;

            let mode = Select::new("Select sync mode:", vec!["link", "copy"])
                .prompt()
                .context("select sync mode")?;
            let mode = match mode {
                "link" => SyncMode::Link,
                "copy" => SyncMode::Copy,
                _ => unreachable!("validated selection"),
            };

            let plan = plan_sync(&managed_dir, &target_dir, &selected, mode)?;
            apply_plan_with_override(plan, args, true, confirm_all)
        }
        _ => unreachable!("validated selection"),
    }
}

#[derive(Debug, Clone)]
enum PlanOp {
    EnsureDir { path: PathBuf },
    Backup { from: PathBuf, to: PathBuf },
    Copy { from: PathBuf, to: PathBuf },
    Link { from: PathBuf, to: PathBuf },
    WriteFile { path: PathBuf, content: String },
    Note { message: String },
}

#[derive(Debug, Clone, Default)]
struct Plan {
    ops: Vec<PlanOp>,
}

impl Plan {
    fn has_writes(&self) -> bool {
        self.ops.iter().any(|op| {
            matches!(
                op,
                PlanOp::EnsureDir { .. }
                    | PlanOp::Backup { .. }
                    | PlanOp::Copy { .. }
                    | PlanOp::Link { .. }
                    | PlanOp::WriteFile { .. }
            )
        })
    }

    fn print(&self) {
        for op in &self.ops {
            match op {
                PlanOp::EnsureDir { path } => println!("PLAN mkdir -p {}", path.display()),
                PlanOp::Backup { from, to } => {
                    println!("PLAN backup {} -> {}", from.display(), to.display())
                }
                PlanOp::Copy { from, to } => {
                    println!("PLAN copy {} -> {}", from.display(), to.display())
                }
                PlanOp::Link { from, to } => {
                    println!("PLAN link {} -> {}", to.display(), from.display())
                }
                PlanOp::WriteFile { path, .. } => println!("PLAN write {}", path.display()),
                PlanOp::Note { message } => println!("PLAN note: {}", message),
            }
        }
    }
}

fn apply_plan(plan: Plan, args: &CodexAgentsArgs, interactive: bool) -> Result<()> {
    apply_plan_with_override(plan, args, interactive, args.yes || args.force)
}

fn apply_plan_with_override(
    plan: Plan,
    args: &CodexAgentsArgs,
    interactive: bool,
    confirm_all: bool,
) -> Result<()> {
    if plan.ops.is_empty() {
        println!("No changes.");
        return Ok(());
    }

    plan.print();

    if args.dry_run {
        return Ok(());
    }

    if plan.has_writes() && !confirm_all {
        if !interactive {
            bail!(
                "This operation would write files. Re-run with --dry-run to preview or --yes/--force to proceed."
            );
        }

        let proceed = Confirm::new("Proceed with these changes?")
            .with_default(false)
            .prompt()
            .context("confirm apply")?;
        if !proceed {
            println!("Cancelled.");
            return Ok(());
        }
    }

    execute_plan(&plan)
}

fn execute_plan(plan: &Plan) -> Result<()> {
    for op in &plan.ops {
        match op {
            PlanOp::EnsureDir { path } => {
                fs::create_dir_all(path)
                    .with_context(|| format!("create dir: {}", path.display()))?;
            }
            PlanOp::Backup { from, to } => {
                fs::rename(from, to)
                    .with_context(|| format!("backup {} -> {}", from.display(), to.display()))?;
            }
            PlanOp::Copy { from, to } => {
                let content =
                    fs::read(from).with_context(|| format!("read source: {}", from.display()))?;
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("create dir: {}", parent.display()))?;
                }
                atomic_write_with_mode(to, &content, None)
                    .with_context(|| format!("write: {}", to.display()))?;
            }
            PlanOp::Link { from, to } => {
                #[cfg(unix)]
                {
                    use std::os::unix::fs as unix_fs;
                    if let Some(parent) = to.parent() {
                        fs::create_dir_all(parent)
                            .with_context(|| format!("create dir: {}", parent.display()))?;
                    }
                    unix_fs::symlink(from, to).with_context(|| {
                        format!("symlink {} -> {}", to.display(), from.display())
                    })?;
                }
                #[cfg(not(unix))]
                {
                    let _ = (from, to);
                    bail!("symlink mode is not supported on this platform; use --mode copy");
                }
            }
            PlanOp::WriteFile { path, content } => {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("create dir: {}", parent.display()))?;
                }
                atomic_write_with_mode(path, content.as_bytes(), None)
                    .with_context(|| format!("write: {}", path.display()))?;
            }
            PlanOp::Note { .. } => {}
        }
    }
    Ok(())
}

fn plan_import(managed_dir: &Path, target_dir: &Path, stems: &[String]) -> Result<Plan> {
    let mut plan = Plan::default();
    if !managed_dir.exists() {
        plan.ops.push(PlanOp::EnsureDir {
            path: managed_dir.to_path_buf(),
        });
    }

    for stem in stems {
        let from = target_dir.join(format!("{stem}.toml"));
        let to = managed_dir.join(format!("{stem}.toml"));

        if !from.exists() {
            plan.ops.push(PlanOp::Note {
                message: format!("skip missing source: {}", from.display()),
            });
            continue;
        }

        let source_bytes = fs::read(&from).with_context(|| format!("read: {}", from.display()))?;
        let needs_overwrite = if fs::symlink_metadata(&to).is_ok_and(|m| m.file_type().is_symlink())
        {
            true
        } else if to.exists() {
            fs::read(&to)
                .map(|existing| existing != source_bytes)
                .unwrap_or(true)
        } else {
            true
        };

        if !needs_overwrite {
            continue;
        }

        if fs::symlink_metadata(&to).is_ok() {
            let backup = backup_path_for(&to)?;
            plan.ops.push(PlanOp::Backup {
                from: to.clone(),
                to: backup,
            });
        }

        plan.ops.push(PlanOp::Copy { from, to });
    }

    Ok(plan)
}

fn plan_sync(
    managed_dir: &Path,
    target_dir: &Path,
    stems: &[String],
    mode: SyncMode,
) -> Result<Plan> {
    let mut plan = Plan::default();
    if !target_dir.exists() {
        plan.ops.push(PlanOp::EnsureDir {
            path: target_dir.to_path_buf(),
        });
    }

    for stem in stems {
        let from = managed_dir.join(format!("{stem}.toml"));
        let to = target_dir.join(format!("{stem}.toml"));

        if !from.exists() {
            plan.ops.push(PlanOp::Note {
                message: format!("skip missing managed file: {}", from.display()),
            });
            continue;
        }

        if mode == SyncMode::Link {
            if is_correct_symlink(&from, &to).unwrap_or(false) {
                continue;
            }
        } else if mode == SyncMode::Copy && to.exists() {
            let meta = fs::symlink_metadata(&to)?;
            if !meta.file_type().is_symlink() {
                let a = fs::read(&from).unwrap_or_default();
                let b = fs::read(&to).unwrap_or_default();
                if a == b {
                    continue;
                }
            }
        }

        if fs::symlink_metadata(&to).is_ok() {
            let backup = backup_path_for(&to)?;
            plan.ops.push(PlanOp::Backup {
                from: to.clone(),
                to: backup,
            });
        }

        match mode {
            SyncMode::Link => plan.ops.push(PlanOp::Link { from, to }),
            SyncMode::Copy => plan.ops.push(PlanOp::Copy { from, to }),
        }
    }

    Ok(plan)
}

fn plan_inject(managed_dir: &Path, stems: &[String], body: &str) -> Result<Plan> {
    let mut plan = Plan::default();

    for stem in stems {
        let path = managed_dir.join(format!("{stem}.toml"));
        if !path.exists() {
            plan.ops.push(PlanOp::Note {
                message: format!("skip missing managed file: {}", path.display()),
            });
            continue;
        }

        let content =
            fs::read_to_string(&path).with_context(|| format!("read: {}", path.display()))?;
        let Some(updated) = inject_into_toml_developer_instructions(&content, body)? else {
            plan.ops.push(PlanOp::Note {
                message: format!("skip (no developer_instructions): {}", path.display()),
            });
            continue;
        };

        if updated == content {
            continue;
        }

        plan.ops.push(PlanOp::WriteFile {
            path,
            content: updated,
        });
    }

    Ok(plan)
}

fn resolve_managed_dir(args: &CodexAgentsArgs) -> Result<PathBuf> {
    if let Some(path) = args.managed_dir.as_ref() {
        return Ok(path.clone());
    }
    Ok(resolve_config_dir(None)?.join("codex").join("agents"))
}

fn resolve_target_agents_dir(args: &CodexAgentsArgs) -> Result<PathBuf> {
    if let Some(path) = args.agents_dir.as_ref() {
        return Ok(path.clone());
    }

    let home = if let Some(home) = args.codex_home.as_ref() {
        home.clone()
    } else if let Ok(env_home) = env::var("CODEX_HOME") {
        let trimmed = env_home.trim();
        if trimmed.is_empty() {
            crate::config::home_dir()?.join(".codex")
        } else {
            PathBuf::from(trimmed)
        }
    } else {
        crate::config::home_dir()?.join(".codex")
    };

    Ok(home.join("agents"))
}

fn list_toml_stems(dir: &Path) -> Result<Vec<String>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut stems = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("read dir: {}", dir.display()))? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };
        if !name.ends_with(".toml") {
            continue;
        }
        let stem = name.trim_end_matches(".toml").to_string();
        if stem.is_empty() {
            continue;
        }
        stems.push(stem);
    }

    stems.sort();
    stems.dedup();
    Ok(stems)
}

fn select_stems(available: &[String], only: &[String], what: &str) -> Result<Vec<String>> {
    if only.is_empty() {
        return Ok(available.to_vec());
    }

    let mut wanted = Vec::new();
    let available_set: HashSet<&str> = available.iter().map(|s| s.as_str()).collect();
    for raw in only {
        let stem = validate_path_segment(raw, what)?;
        if !available_set.contains(stem.as_str()) {
            bail!("{what} not found: {stem}");
        }
        wanted.push(stem);
    }
    wanted.sort();
    wanted.dedup();
    Ok(wanted)
}

fn backup_path_for(path: &Path) -> Result<PathBuf> {
    let ts = chrono::Utc::now().format("%Y%m%d%H%M%S").to_string();
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("backup");
    Ok(path.with_file_name(format!("{file_name}.llman.bak.{ts}")))
}

fn is_correct_symlink(from: &Path, link: &Path) -> Result<bool> {
    let meta =
        fs::symlink_metadata(link).with_context(|| format!("metadata: {}", link.display()))?;
    if !meta.file_type().is_symlink() {
        return Ok(false);
    }

    let target = fs::read_link(link).with_context(|| format!("readlink: {}", link.display()))?;
    let resolved = if target.is_absolute() {
        target
    } else {
        link.parent().unwrap_or_else(|| Path::new(".")).join(target)
    };

    Ok(resolved == from)
}

fn describe_target_state(managed: &Path, target: &Path) -> Result<String> {
    if fs::symlink_metadata(target).is_err() {
        return Ok("missing".to_string());
    }

    let meta =
        fs::symlink_metadata(target).with_context(|| format!("metadata: {}", target.display()))?;
    if meta.file_type().is_symlink() {
        let ok = is_correct_symlink(managed, target).unwrap_or(false);
        return Ok(if ok { "linked" } else { "wrong-link" }.to_string());
    }

    let managed_bytes =
        fs::read(managed).with_context(|| format!("read: {}", managed.display()))?;
    let target_bytes = fs::read(target).with_context(|| format!("read: {}", target.display()))?;
    Ok(if managed_bytes == target_bytes {
        "copied"
    } else {
        "diff"
    }
    .to_string())
}

fn describe_inject_state(managed: &Path) -> Result<String> {
    let content =
        fs::read_to_string(managed).with_context(|| format!("read: {}", managed.display()))?;
    let Some((open_end, close)) = dev_instructions_inner_range(&content) else {
        if content.contains("developer_instructions") {
            return Ok("unsupported-format".to_string());
        }
        return Ok("no-developer_instructions".to_string());
    };
    let inner = &content[open_end..close];
    let has_markers = has_llman_prompt_markers(inner);
    Ok(if has_markers { "managed" } else { "injectable" }.to_string())
}

fn describe_agent_schema_state(managed: &Path) -> Result<String> {
    let content =
        fs::read_to_string(managed).with_context(|| format!("read: {}", managed.display()))?;
    let doc: Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Ok("invalid-toml".to_string()),
    };
    let Some(table) = doc.as_table() else {
        return Ok("invalid-toml".to_string());
    };

    let looks_like_agent = table.contains_key("name")
        || table.contains_key("description")
        || table.contains_key("developer_instructions");
    if !looks_like_agent {
        return Ok("overlay".to_string());
    }

    let mut missing = Vec::new();
    if table
        .get("name")
        .and_then(|v| v.as_str())
        .is_none_or(|s| s.trim().is_empty())
    {
        missing.push("name");
    }
    if table
        .get("description")
        .and_then(|v| v.as_str())
        .is_none_or(|s| s.trim().is_empty())
    {
        missing.push("description");
    }
    if table
        .get("developer_instructions")
        .and_then(|v| v.as_str())
        .is_none_or(|s| s.trim().is_empty())
    {
        missing.push("developer_instructions");
    }

    if missing.is_empty() {
        return Ok("agent-ok".to_string());
    }

    Ok(format!("agent-missing({})", missing.join(",")))
}

fn list_codex_prompt_templates() -> Result<Vec<String>> {
    let config = crate::config::Config::new()?;
    prompt_store::list_templates(&config, crate::config::CODEX_APP)
}

fn build_injection_body(templates: &[String]) -> Result<String> {
    let config = crate::config::Config::new()?;
    prompt_store::build_llman_prompts_body(&config, crate::config::CODEX_APP, templates)
}

fn inject_into_toml_developer_instructions(content: &str, body: &str) -> Result<Option<String>> {
    // Locate: developer_instructions = """ ... """
    // We intentionally support only the common triple-quote form to avoid rewriting TOML.
    let (open_start, open_end) = match find_dev_instructions_open(content) {
        Some(v) => v,
        None => return Ok(None),
    };
    let close = find_triple_quote_close(content, open_end).ok_or_else(|| {
        anyhow::anyhow!("Unterminated developer_instructions triple-quote string")
    })?;

    let existing = &content[open_end..close];
    let updated_inner = update_text_with_markers(
        existing,
        body,
        true,
        LLMAN_PROMPTS_MARKER_START,
        LLMAN_PROMPTS_MARKER_END,
    );
    if updated_inner == existing {
        return Ok(Some(content.to_string()));
    }

    let mut out = String::with_capacity(content.len() + body.len() + 64);
    out.push_str(&content[..open_start]);
    out.push_str(&content[open_start..open_end]);
    out.push_str(&updated_inner);
    out.push_str(&content[close..]);
    Ok(Some(out))
}

fn find_dev_instructions_open(content: &str) -> Option<(usize, usize)> {
    // Line-wise scan to avoid matching inside strings/comments.
    let mut offset = 0usize;
    for line in content.split_inclusive('\n') {
        let raw = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = raw.trim_start();

        if trimmed.starts_with('#') {
            offset += line.len();
            continue;
        }
        if !trimmed.starts_with("developer_instructions") {
            offset += line.len();
            continue;
        }

        // Remove trailing comment portion for the assignment line.
        let without_comment = trimmed.split('#').next().unwrap_or(trimmed);
        let Some(eq_idx) = without_comment.find('=') else {
            offset += line.len();
            continue;
        };

        let rhs_in_without = &without_comment[eq_idx + 1..];
        let rhs = rhs_in_without.trim_start();
        let Some(pos) = rhs.find("\"\"\"") else {
            offset += line.len();
            continue;
        };

        // Compute absolute indices: start delimiter position in full content, and end of opening delimiter.
        let before_trim = raw.len() - trimmed.len();
        let before_rhs = rhs_in_without.len() - rhs.len();
        let rhs_abs = offset + before_trim + (eq_idx + 1) + before_rhs;
        let open_start = rhs_abs + pos;
        return Some((open_start, open_start + 3));
    }

    None
}

fn find_triple_quote_close(content: &str, from: usize) -> Option<usize> {
    content[from..].find("\"\"\"").map(|i| from + i)
}

fn dev_instructions_inner_range(content: &str) -> Option<(usize, usize)> {
    let (_open_start, open_end) = find_dev_instructions_open(content)?;
    let close = find_triple_quote_close(content, open_end)?;
    Some((open_end, close))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::override_runtime_config_dir;
    use tempfile::TempDir;

    #[test]
    fn inject_updates_existing_marker_block() {
        let input = r#"
name = "reviewer"

developer_instructions = """
hello
<!-- LLMAN-PROMPTS:START -->
old
<!-- LLMAN-PROMPTS:END -->
bye
"""
"#;
        let out = inject_into_toml_developer_instructions(input, "new-body")
            .expect("inject")
            .expect("has dev instructions");
        assert!(out.contains("new-body"));
        assert!(!out.contains("\nold\n"));
    }

    #[test]
    fn inject_appends_marker_when_missing() {
        let input = r#"
developer_instructions = """
hello
"""
"#;
        let out = inject_into_toml_developer_instructions(input, "body")
            .expect("inject")
            .expect("has dev instructions");
        assert!(out.contains(LLMAN_PROMPTS_MARKER_START));
        assert!(out.contains("body"));
        assert!(out.contains(LLMAN_PROMPTS_MARKER_END));
    }

    #[test]
    fn inject_returns_none_when_no_developer_instructions() {
        let input = r#"model = "gpt-5.4-mini""#;
        let out = inject_into_toml_developer_instructions(input, "body").expect("inject");
        assert!(out.is_none());
    }

    #[test]
    fn dry_run_plan_does_not_write_target_file() {
        let temp = TempDir::new().expect("temp dir");
        let _guard = override_runtime_config_dir(temp.path().join("llman-config"));

        let managed_dir = resolve_config_dir(None)
            .unwrap()
            .join("codex")
            .join("agents");
        let codex_home = temp.path().join("codex-home");
        let target_dir = codex_home.join("agents");

        fs::create_dir_all(&managed_dir).unwrap();
        fs::write(managed_dir.join("a.toml"), "x").unwrap();

        let args = CodexAgentsArgs {
            command: Some(CodexAgentsCommand::Sync {
                mode: SyncMode::Copy,
            }),
            managed_dir: None,
            codex_home: Some(codex_home),
            agents_dir: None,
            only: vec![],
            dry_run: true,
            yes: true,
            force: false,
        };

        run(&args).expect("run");
        assert!(!target_dir.join("a.toml").exists());
    }

    #[test]
    fn non_interactive_write_requires_yes_or_force() {
        let temp = TempDir::new().expect("temp dir");
        let _guard = override_runtime_config_dir(temp.path().join("llman-config"));

        let managed_dir = resolve_config_dir(None)
            .unwrap()
            .join("codex")
            .join("agents");
        let codex_home = temp.path().join("codex-home");
        let target_dir = codex_home.join("agents");

        fs::create_dir_all(&managed_dir).unwrap();
        fs::write(managed_dir.join("a.toml"), "x").unwrap();

        let args = CodexAgentsArgs {
            command: Some(CodexAgentsCommand::Sync {
                mode: SyncMode::Copy,
            }),
            managed_dir: None,
            codex_home: Some(codex_home),
            agents_dir: None,
            only: vec![],
            dry_run: false,
            yes: false,
            force: false,
        };

        let err = run(&args).expect_err("should require --yes/--force in non-interactive mode");
        assert!(err.to_string().contains("--dry-run") || err.to_string().contains("--yes"));
        assert!(!target_dir.join("a.toml").exists());
    }
}

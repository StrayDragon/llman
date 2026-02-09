use crate::sdd::change::archive;
use crate::sdd::project::{init, update, update_skills};
use crate::sdd::shared::{list, show, validate};
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct SddArgs {
    #[command(subcommand)]
    pub command: SddCommands,
}

#[derive(Subcommand)]
pub enum SddCommands {
    /// Initialize llmanspec in your project
    Init {
        /// Target path (default: current directory)
        path: Option<PathBuf>,
        /// Locale for templates (default: en)
        #[arg(long)]
        lang: Option<String>,
    },
    /// Update llmanspec instruction files
    Update {
        /// Target path (default: current directory)
        path: Option<PathBuf>,
    },
    /// Generate or update llman sdd skills
    UpdateSkills {
        /// Generate skills for all tools (OPSX commands only for Claude)
        #[arg(long)]
        all: bool,
        /// Tool to generate skills for: claude,codex (repeatable; OPSX commands only for claude)
        #[arg(long, value_delimiter = ',')]
        tool: Vec<String>,
        /// Override output path for generated skills
        #[arg(long)]
        path: Option<PathBuf>,
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
        /// Generate only OPSX slash commands for Claude (no skills)
        #[arg(long, conflicts_with = "skills_only")]
        commands_only: bool,
        /// Generate only skills (no OPSX slash commands)
        #[arg(long, conflicts_with = "commands_only")]
        skills_only: bool,
    },
    /// List changes or specs
    List {
        /// List specs instead of changes
        #[arg(long)]
        specs: bool,
        /// List changes explicitly (default)
        #[arg(long)]
        changes: bool,
        /// Sort order: "recent" (default) or "name"
        #[arg(long, default_value = "recent")]
        sort: String,
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show a change or spec
    Show {
        /// Item name (change id or spec id)
        item: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Specify item type when ambiguous: change|spec
        #[arg(long = "type")]
        item_type: Option<String>,
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
        /// Change-only: show only deltas (JSON only)
        #[arg(long)]
        deltas_only: bool,
        /// Change-only: alias for --deltas-only (deprecated)
        #[arg(long)]
        requirements_only: bool,
        /// Spec-only: show only requirements (JSON only)
        #[arg(long)]
        requirements: bool,
        /// Spec-only: exclude scenarios (JSON only)
        #[arg(long)]
        no_scenarios: bool,
        /// Spec-only: show specific requirement by ID (1-based)
        #[arg(short = 'r', long)]
        requirement: Option<usize>,
    },
    /// Validate changes and specs
    Validate {
        /// Item name (change id or spec id)
        item: Option<String>,
        /// Validate all changes and specs
        #[arg(long)]
        all: bool,
        /// Validate all changes
        #[arg(long)]
        changes: bool,
        /// Validate all specs
        #[arg(long)]
        specs: bool,
        /// Specify item type when ambiguous: change|spec
        #[arg(long = "type")]
        item_type: Option<String>,
        /// Enable strict validation mode
        #[arg(long)]
        strict: bool,
        /// Output validation results as JSON
        #[arg(long)]
        json: bool,
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
    },
    /// Archive a change and update main specs
    Archive {
        /// Change id
        change: Option<String>,
        /// Skip updating specs
        #[arg(long)]
        skip_specs: bool,
        /// Dry run mode
        #[arg(long)]
        dry_run: bool,
        /// Force archive even if validation fails
        #[arg(long, hide = true)]
        force: bool,
    },
}

pub fn run(args: &SddArgs) -> Result<()> {
    match &args.command {
        SddCommands::Init { path, lang } => init::run(
            path.as_deref().unwrap_or_else(|| std::path::Path::new(".")),
            lang.as_deref(),
        ),
        SddCommands::Update { path } => {
            update::run(path.as_deref().unwrap_or_else(|| std::path::Path::new(".")))
        }
        SddCommands::UpdateSkills {
            all,
            tool,
            path,
            no_interactive,
            commands_only,
            skills_only,
        } => update_skills::run(update_skills::UpdateSkillsArgs {
            all: *all,
            tool: tool.clone(),
            path: path.clone(),
            no_interactive: *no_interactive,
            commands_only: *commands_only,
            skills_only: *skills_only,
        }),
        SddCommands::List {
            specs,
            changes,
            sort,
            json,
        } => list::run(list::ListArgs {
            specs: *specs,
            changes: *changes,
            sort: sort.clone(),
            json: *json,
        }),
        SddCommands::Show {
            item,
            json,
            item_type,
            no_interactive,
            deltas_only,
            requirements_only,
            requirements,
            no_scenarios,
            requirement,
        } => show::run(show::ShowArgs {
            item: item.clone(),
            json: *json,
            item_type: item_type.clone(),
            no_interactive: *no_interactive,
            deltas_only: *deltas_only,
            requirements_only: *requirements_only,
            requirements: *requirements,
            no_scenarios: *no_scenarios,
            requirement: *requirement,
        }),
        SddCommands::Validate {
            item,
            all,
            changes,
            specs,
            item_type,
            strict,
            json,
            no_interactive,
        } => validate::run(validate::ValidateArgs {
            item: item.clone(),
            all: *all,
            changes: *changes,
            specs: *specs,
            item_type: item_type.clone(),
            strict: *strict,
            json: *json,
            no_interactive: *no_interactive,
        }),
        SddCommands::Archive {
            change,
            skip_specs,
            dry_run,
            force,
        } => archive::run(archive::ArchiveArgs {
            change: change.clone(),
            skip_specs: *skip_specs,
            dry_run: *dry_run,
            force: *force,
        }),
    }
}

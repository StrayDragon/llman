use crate::sdd::authoring;
use crate::sdd::change::archive;
use crate::sdd::change::freeze;
use crate::sdd::project::{init, interop};
use crate::sdd::shared::{graph, list, show, validate};
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct SddArgs {
    #[command(subcommand)]
    pub command: SddCommands,
}

#[derive(Args)]
pub struct SddSpecArgs {
    #[command(subcommand)]
    pub command: SddSpecCommands,
}

#[derive(Subcommand)]
pub enum SddSpecCommands {
    /// Generate a main spec skeleton for a capability
    Skeleton {
        /// Capability / spec id
        capability: String,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// Add a requirement row to a spec
    AddRequirement {
        /// Capability / spec id
        capability: String,
        /// Requirement id (req_id)
        req_id: String,
        /// Requirement title (human-facing)
        #[arg(long)]
        title: String,
        /// Requirement statement (MUST/SHALL)
        #[arg(long)]
        statement: String,
    },
    /// Add a scenario row to a spec
    AddScenario {
        /// Capability / spec id
        capability: String,
        /// Requirement id (req_id)
        req_id: String,
        /// Scenario id
        scenario_id: String,
        /// GIVEN (optional)
        #[arg(long, default_value = "")]
        given: String,
        /// WHEN (required)
        #[arg(long = "when")]
        when_: String,
        /// THEN (required)
        #[arg(long = "then")]
        then_: String,
    },
}

#[derive(Args)]
pub struct SddDeltaArgs {
    #[command(subcommand)]
    pub command: SddDeltaCommands,
}

#[derive(Subcommand)]
pub enum SddDeltaCommands {
    /// Generate a delta spec skeleton for a change + capability (no YAML frontmatter)
    Skeleton {
        /// Change id
        change_id: String,
        /// Capability / spec id
        capability: String,
        /// Overwrite existing files
        #[arg(long)]
        force: bool,
    },
    /// Add a delta op row
    AddOp {
        /// Change id
        change_id: String,
        /// Capability / spec id
        capability: String,
        /// Op: add_requirement|modify_requirement|remove_requirement|rename_requirement
        op: String,
        /// Requirement id (req_id)
        req_id: String,
        /// Title (required for add/modify)
        #[arg(long)]
        title: Option<String>,
        /// Statement (required for add/modify; MUST/SHALL)
        #[arg(long)]
        statement: Option<String>,
        /// Rename source (required for rename)
        #[arg(long)]
        from: Option<String>,
        /// Rename target (required for rename)
        #[arg(long)]
        to: Option<String>,
        /// Name hint (optional for remove)
        #[arg(long)]
        name: Option<String>,
    },
    /// Add a delta op scenario row (add/modify ops only)
    AddScenario {
        /// Change id
        change_id: String,
        /// Capability / spec id
        capability: String,
        /// Requirement id (req_id)
        req_id: String,
        /// Scenario id
        scenario_id: String,
        /// GIVEN (optional)
        #[arg(long, default_value = "")]
        given: String,
        /// WHEN (required)
        #[arg(long = "when")]
        when_: String,
        /// THEN (required)
        #[arg(long = "then")]
        then_: String,
    },
}

#[derive(Subcommand)]
pub enum SddCommands {
    /// Initialize llmanspec in your project (use --update to refresh existing)
    Init {
        /// Target path (default: current directory)
        path: Option<PathBuf>,
        /// Locale for templates (default: en)
        #[arg(long)]
        lang: Option<String>,
        /// Update existing llmanspec instead of creating new
        #[arg(long)]
        update: bool,
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
        /// Emit compact JSON (no pretty whitespace). Requires `--json`.
        #[arg(long, requires = "json")]
        compact_json: bool,
    },
    /// Show a change or spec
    Show {
        /// Item name (change id or spec id)
        item: Option<String>,
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Emit compact JSON (no pretty whitespace). Requires `--json`.
        #[arg(long, requires = "json")]
        compact_json: bool,
        /// Spec-only: output metadata only (no `requirements`). Requires `--json`.
        #[arg(long, requires = "json")]
        meta_only: bool,
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
        /// Emit compact JSON (no pretty whitespace). Requires `--json`.
        #[arg(long, requires = "json")]
        compact_json: bool,
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
    },
    /// Archive workflow commands
    Archive {
        /// Legacy: change id (equivalent to `archive run <change-id>`)
        change: Option<String>,
        /// Legacy: skip updating specs
        #[arg(long)]
        skip_specs: bool,
        /// Legacy: dry run mode
        #[arg(long)]
        dry_run: bool,
        /// Legacy: force archive even if validation fails
        #[arg(long, hide = true)]
        force: bool,
        /// Disable interactive prompts (e.g. purpose input for new specs)
        #[arg(long)]
        no_interactive: bool,
        #[command(subcommand)]
        command: Option<ArchiveSubcommand>,
    },
    /// Spec authoring helpers
    Spec(SddSpecArgs),
    /// Delta authoring helpers
    Delta(SddDeltaArgs),
    /// Generate a change dependency graph
    Graph {
        /// Output format (default: mermaid)
        #[arg(long, default_value = "mermaid")]
        format: String,
        /// Scope: active, archived, all, or comma-separated (e.g. active,archived). Default expands level-1 depends_on targets.
        #[arg(long, default_value = "active")]
        scope: String,
        /// Recursion depth when a seed change is specified (default: 1)
        #[arg(long, default_value_t = 1)]
        depth: usize,
        /// Seed change ID to center the graph on
        change: Option<String>,
    },
    /// Import specs from OpenSpec markdown format into llmanspec
    Import {
        /// Source OpenSpec specs directory (default: openspec/specs)
        #[arg(long)]
        source: Option<PathBuf>,
        /// Glob pattern to filter spec names (e.g. 'config-*')
        #[arg(long)]
        scope: Option<String>,
        /// Parse and report without writing files
        #[arg(long)]
        dry_run: bool,
        /// Overwrite existing specs in target
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
pub enum ArchiveSubcommand {
    /// Archive a change and update main specs
    Run {
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
        /// Disable interactive prompts (e.g. purpose input for new specs)
        #[arg(long)]
        no_interactive: bool,
    },
    /// Freeze archived change directories into a single cold-backup archive
    Freeze {
        /// Freeze entries older than this date (YYYY-MM-DD)
        #[arg(long)]
        before: Option<String>,
        /// Keep N most recent candidates unfrozen
        #[arg(long, default_value_t = 0)]
        keep_recent: usize,
        /// Dry run mode
        #[arg(long)]
        dry_run: bool,
    },
    /// Thaw archived changes from the cold-backup archive
    Thaw {
        /// Restore only these archived change directories (repeatable)
        #[arg(long)]
        change: Vec<String>,
        /// Override thaw destination path
        #[arg(long)]
        dest: Option<PathBuf>,
    },
}

pub fn run(args: &SddArgs) -> Result<()> {
    match &args.command {
        SddCommands::Init { path, lang, update } => init::run(
            path.as_deref().unwrap_or_else(|| std::path::Path::new(".")),
            lang.as_deref(),
            *update,
        ),
        SddCommands::List {
            specs,
            changes,
            sort,
            json,
            compact_json,
        } => list::run(list::ListArgs {
            specs: *specs,
            changes: *changes,
            sort: sort.clone(),
            json: *json,
            compact_json: *compact_json,
        }),
        SddCommands::Show {
            item,
            json,
            compact_json,
            meta_only,
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
            compact_json: *compact_json,
            meta_only: *meta_only,
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
            compact_json,
            no_interactive,
        } => validate::run(validate::ValidateArgs {
            item: item.clone(),
            all: *all,
            changes: *changes,
            specs: *specs,
            item_type: item_type.clone(),
            strict: *strict,
            json: *json,
            compact_json: *compact_json,
            no_interactive: *no_interactive,
        }),
        SddCommands::Archive {
            change,
            skip_specs,
            dry_run,
            force,
            no_interactive,
            command,
        } => match command {
            Some(ArchiveSubcommand::Run {
                change,
                skip_specs,
                dry_run,
                force,
                no_interactive,
            }) => archive::run(archive::ArchiveArgs {
                change: change.clone(),
                skip_specs: *skip_specs,
                dry_run: *dry_run,
                force: *force,
                no_interactive: *no_interactive,
            }),
            Some(ArchiveSubcommand::Freeze {
                before,
                keep_recent,
                dry_run,
            }) => freeze::run_freeze(freeze::FreezeArgs {
                before: before.clone(),
                keep_recent: *keep_recent,
                dry_run: *dry_run,
            }),
            Some(ArchiveSubcommand::Thaw { change, dest }) => freeze::run_thaw(freeze::ThawArgs {
                change: change.clone(),
                dest: dest.clone(),
            }),
            None => archive::run(archive::ArchiveArgs {
                change: change.clone(),
                skip_specs: *skip_specs,
                dry_run: *dry_run,
                force: *force,
                no_interactive: *no_interactive,
            }),
        },
        SddCommands::Spec(args) => match &args.command {
            SddSpecCommands::Skeleton { capability, force } => authoring::spec::run_skeleton(
                std::path::Path::new("."),
                authoring::spec::SpecSkeletonArgs {
                    capability: capability.clone(),
                    force: *force,
                },
            ),
            SddSpecCommands::AddRequirement {
                capability,
                req_id,
                title,
                statement,
            } => authoring::spec::run_add_requirement(
                std::path::Path::new("."),
                authoring::spec::SpecAddRequirementArgs {
                    capability: capability.clone(),
                    req_id: req_id.clone(),
                    title: title.clone(),
                    statement: statement.clone(),
                },
            ),
            SddSpecCommands::AddScenario {
                capability,
                req_id,
                scenario_id,
                given,
                when_,
                then_,
            } => authoring::spec::run_add_scenario(
                std::path::Path::new("."),
                authoring::spec::SpecAddScenarioArgs {
                    capability: capability.clone(),
                    req_id: req_id.clone(),
                    scenario_id: scenario_id.clone(),
                    given: given.clone(),
                    when_: when_.clone(),
                    then_: then_.clone(),
                },
            ),
        },
        SddCommands::Delta(args) => match &args.command {
            SddDeltaCommands::Skeleton {
                change_id,
                capability,
                force,
            } => authoring::delta::run_skeleton(
                std::path::Path::new("."),
                authoring::delta::DeltaSkeletonArgs {
                    change_id: change_id.clone(),
                    capability: capability.clone(),
                    force: *force,
                },
            ),
            SddDeltaCommands::AddOp {
                change_id,
                capability,
                op,
                req_id,
                title,
                statement,
                from,
                to,
                name,
            } => authoring::delta::run_add_op(
                std::path::Path::new("."),
                authoring::delta::DeltaAddOpArgs {
                    change_id: change_id.clone(),
                    capability: capability.clone(),
                    op: op.clone(),
                    req_id: req_id.clone(),
                    title: title.clone(),
                    statement: statement.clone(),
                    from: from.clone(),
                    to: to.clone(),
                    name: name.clone(),
                },
            ),
            SddDeltaCommands::AddScenario {
                change_id,
                capability,
                req_id,
                scenario_id,
                given,
                when_,
                then_,
            } => authoring::delta::run_add_scenario(
                std::path::Path::new("."),
                authoring::delta::DeltaAddScenarioArgs {
                    change_id: change_id.clone(),
                    capability: capability.clone(),
                    req_id: req_id.clone(),
                    scenario_id: scenario_id.clone(),
                    given: given.clone(),
                    when_: when_.clone(),
                    then_: then_.clone(),
                },
            ),
        },
        SddCommands::Graph {
            format,
            scope,
            depth,
            change,
        } => graph::run(graph::GraphArgs {
            format: format.clone(),
            scope: scope.clone(),
            depth: *depth,
            change: change.clone(),
        }),
        SddCommands::Import {
            source,
            scope,
            dry_run,
            force,
        } => interop::run(
            std::path::Path::new("."),
            interop::ImportArgs {
                source: source.clone(),
                scope: scope.clone(),
                dry_run: *dry_run,
                force: *force,
            },
        ),
    }
}

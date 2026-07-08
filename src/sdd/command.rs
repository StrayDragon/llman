use crate::sdd::authoring;
use crate::sdd::change::archive;
use crate::sdd::change::freeze;
use crate::sdd::project::{init, interop, migrate, upgrade_guide};
use crate::sdd::shared::{graph, list, show, status, validate};
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
    #[command(alias = "add-requirement")]
    AddReq {
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
    /// Add a new requirement (extracted from add-op)
    #[command(alias = "add-op")]
    AddReq {
        /// Change id
        change_id: String,
        /// Capability / spec id
        capability: String,
        /// Requirement id (req_id)
        req_id: String,
        /// Requirement title
        #[arg(long)]
        title: String,
        /// Requirement statement (MUST/SHALL)
        #[arg(long)]
        statement: String,
    },
    /// Modify an existing requirement
    ModifyReq {
        /// Change id
        change_id: String,
        /// Capability / spec id
        capability: String,
        /// Requirement id (req_id)
        req_id: String,
        /// New title
        #[arg(long)]
        title: Option<String>,
        /// New statement (MUST/SHALL)
        #[arg(long)]
        statement: Option<String>,
    },
    /// Remove a requirement
    RemoveReq {
        /// Change id
        change_id: String,
        /// Capability / spec id
        capability: String,
        /// Requirement id (req_id)
        req_id: String,
        /// Name hint
        #[arg(long)]
        name: Option<String>,
    },
    /// Rename a requirement
    RenameReq {
        /// Change id
        change_id: String,
        /// Capability / spec id
        capability: String,
        /// Source requirement id
        #[arg(long)]
        from: String,
        /// Target requirement id
        #[arg(long)]
        to: String,
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
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
    },
    /// Show a change or spec
    Show {
        /// Item name (change id or spec id)
        item: Option<String>,
        /// Output format (e.g. json,compact,meta-only,deltas,reqs-only,no-scenarios)
        #[arg(long)]
        output: Option<String>,
        /// Output as JSON (deprecated: use --output json)
        #[arg(long, hide = true)]
        json: bool,
        /// Emit compact JSON (deprecated: use --output json,compact)
        #[arg(long, hide = true, requires = "json")]
        compact_json: bool,
        /// Spec-only: output metadata only (deprecated: use --output json,meta-only)
        #[arg(long, hide = true, requires = "json")]
        meta_only: bool,
        /// Specify item type when ambiguous: change|spec
        #[arg(long = "type")]
        item_type: Option<String>,
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
        /// Change-only: show only deltas (deprecated: use --output deltas)
        #[arg(long, hide = true)]
        deltas_only: bool,
        /// Change-only: alias for --deltas-only (deprecated)
        #[arg(long, hide = true)]
        requirements_only: bool,
        /// Spec-only: show only requirements (deprecated: use --output reqs-only)
        #[arg(long, hide = true)]
        requirements: bool,
        /// Spec-only: exclude scenarios (deprecated: use --output no-scenarios)
        #[arg(long, hide = true)]
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
        /// Force validation stage: draft, spec, or full (overrides auto-detection)
        #[arg(long, value_parser = ["draft", "spec", "full"])]
        stage: Option<String>,
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
    },
    /// Archive workflow commands
    Archive {
        /// Disable interactive prompts (e.g. purpose input for new specs)
        #[arg(long)]
        no_interactive: bool,
        #[command(subcommand)]
        command: ArchiveSubcommand,
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
    /// Show project status overview
    Status {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Get specs relevant to a task and/or file paths (agent-oriented)
    Context {
        /// Natural language description of the current change
        #[arg(long)]
        task: Option<String>,
        /// File paths involved in the change (comma-separated)
        #[arg(long, value_delimiter = ',')]
        paths: Vec<String>,
        /// Maximum number of specs to return (default: 10)
        #[arg(long, default_value_t = 10)]
        top: usize,
        /// Retrieval/index backend: `pageindex` (agentic tree search).
        ///
        /// Can also be preset via `LLMAN_SDD_INDEX_BACKEND`.
        #[arg(long)]
        backend: Option<String>,
    },
    /// Index management commands (rebuild, check freshness)
    Index(IndexCommands),
    /// Project management commands
    Project(SddProjectArgs),
}

/// Index management commands
#[derive(Args)]
pub struct IndexCommands {
    #[command(subcommand)]
    pub command: IndexSubcommand,
}

#[derive(Subcommand)]
pub enum IndexSubcommand {
    /// Rebuild the index (sync or async)
    Rebuild {
        /// Run rebuild in background and return immediately
        #[arg(long)]
        run_async: bool,
        /// Which backend's index to rebuild: `pageindex` (default).
        ///
        /// Can also be preset via `LLMAN_SDD_INDEX_BACKEND`.
        #[arg(long)]
        backend: Option<String>,
    },
    /// Check index freshness without rebuilding
    Check {},
}

#[derive(Args)]
pub struct SddProjectArgs {
    #[command(subcommand)]
    pub command: SddProjectCommands,
}

#[derive(Subcommand)]
pub enum SddProjectCommands {
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
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
    },
    /// Migrate specs to the current canonical format (one-shot, idempotent)
    Migrate {
        /// Scan and report without writing files (no confirmation prompt)
        #[arg(long)]
        dry_run: bool,
        /// Re-migrate even when both `spec.toon` and legacy `spec.md` exist
        #[arg(long)]
        force: bool,
        /// Skip the confirmation prompt and apply (for agents/scripts)
        #[arg(short = 'y', long)]
        yes: bool,
        /// Treat the terminal as non-interactive
        #[arg(long)]
        no_interactive: bool,
    },
    /// Output an upgrade guide prompt for the current SDD project
    UpgradeGuide,
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
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
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
            no_interactive,
        } => list::run(list::ListArgs {
            specs: *specs,
            changes: *changes,
            sort: sort.clone(),
            json: *json,
            compact_json: *compact_json,
            no_interactive: *no_interactive,
        }),
        SddCommands::Show {
            item,
            output,
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
        } => {
            // Parse --output option or fall back to legacy flags
            let (use_json, use_compact, use_meta_only, use_deltas, use_reqs_only, use_no_scenarios) =
                if let Some(output_str) = output {
                    let flags: Vec<&str> = output_str.split(',').map(|s| s.trim()).collect();
                    (
                        flags.contains(&"json"),
                        flags.contains(&"compact"),
                        flags.contains(&"meta-only"),
                        flags.contains(&"deltas"),
                        flags.contains(&"reqs-only"),
                        flags.contains(&"no-scenarios"),
                    )
                } else {
                    (
                        *json,
                        *compact_json,
                        *meta_only,
                        *deltas_only || *requirements_only,
                        *requirements,
                        *no_scenarios,
                    )
                };
            show::run(show::ShowArgs {
                item: item.clone(),
                json: use_json,
                compact_json: use_compact,
                meta_only: use_meta_only,
                item_type: item_type.clone(),
                no_interactive: *no_interactive,
                deltas_only: use_deltas,
                requirements_only: false,
                requirements: use_reqs_only,
                no_scenarios: use_no_scenarios,
                requirement: *requirement,
            })
        }
        SddCommands::Validate {
            item,
            all,
            changes,
            specs,
            item_type,
            strict,
            json,
            compact_json,
            stage,
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
            stage: stage.clone(),
            no_interactive: *no_interactive,
        }),
        SddCommands::Archive {
            no_interactive: _,
            command,
        } => match command {
            ArchiveSubcommand::Run {
                change,
                skip_specs,
                dry_run,
                force,
                no_interactive,
            } => archive::run(archive::ArchiveArgs {
                change: change.clone(),
                skip_specs: *skip_specs,
                dry_run: *dry_run,
                force: *force,
                no_interactive: *no_interactive,
            }),
            ArchiveSubcommand::Freeze {
                before,
                keep_recent,
                dry_run,
                no_interactive,
            } => freeze::run_freeze(freeze::FreezeArgs {
                before: before.clone(),
                keep_recent: *keep_recent,
                dry_run: *dry_run,
                no_interactive: *no_interactive,
            }),
            ArchiveSubcommand::Thaw { change, dest } => freeze::run_thaw(freeze::ThawArgs {
                change: change.clone(),
                dest: dest.clone(),
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
            SddSpecCommands::AddReq {
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
            SddDeltaCommands::AddReq {
                change_id,
                capability,
                req_id,
                title,
                statement,
            } => authoring::delta::run_add_op(
                std::path::Path::new("."),
                authoring::delta::DeltaAddOpArgs {
                    change_id: change_id.clone(),
                    capability: capability.clone(),
                    op: "add_requirement".to_string(),
                    req_id: req_id.clone(),
                    title: Some(title.clone()),
                    statement: Some(statement.clone()),
                    from: None,
                    to: None,
                    name: None,
                },
            ),
            SddDeltaCommands::ModifyReq {
                change_id,
                capability,
                req_id,
                title,
                statement,
            } => authoring::delta::run_add_op(
                std::path::Path::new("."),
                authoring::delta::DeltaAddOpArgs {
                    change_id: change_id.clone(),
                    capability: capability.clone(),
                    op: "modify_requirement".to_string(),
                    req_id: req_id.clone(),
                    title: title.clone(),
                    statement: statement.clone(),
                    from: None,
                    to: None,
                    name: None,
                },
            ),
            SddDeltaCommands::RemoveReq {
                change_id,
                capability,
                req_id,
                name,
            } => authoring::delta::run_add_op(
                std::path::Path::new("."),
                authoring::delta::DeltaAddOpArgs {
                    change_id: change_id.clone(),
                    capability: capability.clone(),
                    op: "remove_requirement".to_string(),
                    req_id: req_id.clone(),
                    title: None,
                    statement: None,
                    from: None,
                    to: None,
                    name: name.clone(),
                },
            ),
            SddDeltaCommands::RenameReq {
                change_id,
                capability,
                from,
                to,
            } => authoring::delta::run_add_op(
                std::path::Path::new("."),
                authoring::delta::DeltaAddOpArgs {
                    change_id: change_id.clone(),
                    capability: capability.clone(),
                    op: "rename_requirement".to_string(),
                    req_id: from.clone(),
                    title: None,
                    statement: None,
                    from: Some(from.clone()),
                    to: Some(to.clone()),
                    name: None,
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
        SddCommands::Status { json } => status::run(status::StatusArgs { json: *json }),
        SddCommands::Context {
            task,
            paths,
            top,
            backend,
        } => {
            let backend = crate::sdd::context::resolve_backend(backend.clone())?;
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(crate::sdd::context::context_run(
                task.clone(),
                paths.clone(),
                *top,
                backend,
            ))
        }
        SddCommands::Index(cmd) => match &cmd.command {
            IndexSubcommand::Check {} => crate::sdd::context::index_check(),
            IndexSubcommand::Rebuild { run_async, backend } => {
                let backend = crate::sdd::context::resolve_backend(backend.clone())?;
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(crate::sdd::context::index_rebuild(
                    None, None, None, *run_async, backend,
                ))
            }
        },
        SddCommands::Project(args) => match &args.command {
            SddProjectCommands::Import {
                source,
                scope,
                dry_run,
                force,
                no_interactive,
            } => interop::run(
                std::path::Path::new("."),
                interop::ImportArgs {
                    source: source.clone(),
                    scope: scope.clone(),
                    dry_run: *dry_run,
                    force: *force,
                    no_interactive: *no_interactive,
                },
            ),
            SddProjectCommands::Migrate {
                dry_run,
                force,
                yes,
                no_interactive,
            } => migrate::run(migrate::MigrateArgs {
                dry_run: *dry_run,
                force: *force,
                yes: *yes,
                no_interactive: *no_interactive,
            }),
            SddProjectCommands::UpgradeGuide => upgrade_guide::run(),
        },
    }
}

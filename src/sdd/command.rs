use crate::sdd::authoring;
use crate::sdd::change::archive;
use crate::sdd::change::freeze;
use crate::sdd::project::templates::TemplateStyle;
use crate::sdd::project::{init, interop, migrate, update, update_skills};
use crate::sdd::shared::{list, show, validate};
use anyhow::Result;
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
pub struct SddArgs {
    #[command(subcommand)]
    pub command: SddCommands,
}

#[derive(Args)]
pub struct SddLegacyArgs {
    #[command(subcommand)]
    pub command: SddLegacyCommands,
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
        /// Pretty-align ISON tables for review (whitespace-only)
        #[arg(long)]
        pretty_ison: bool,
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
        /// Pretty-align ISON tables for review (whitespace-only)
        #[arg(long)]
        pretty_ison: bool,
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
        /// Pretty-align ISON tables for review (whitespace-only)
        #[arg(long)]
        pretty_ison: bool,
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
        /// Pretty-align ISON tables for review (whitespace-only)
        #[arg(long)]
        pretty_ison: bool,
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
        /// Pretty-align ISON tables for review (whitespace-only)
        #[arg(long)]
        pretty_ison: bool,
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
        /// Pretty-align ISON tables for review (whitespace-only)
        #[arg(long)]
        pretty_ison: bool,
    },
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
        /// Generate skills for all tools (llman sdd workflow commands only for Claude)
        #[arg(long)]
        all: bool,
        /// Tool to generate skills for: claude,codex (repeatable; workflow commands only for claude)
        #[arg(long, value_delimiter = ',')]
        tool: Vec<String>,
        /// Override output path for generated skills
        #[arg(long)]
        path: Option<PathBuf>,
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
        /// Generate only llman sdd workflow commands for Claude (no skills)
        #[arg(long, conflicts_with = "skills_only")]
        commands_only: bool,
        /// Generate only skills (no llman sdd workflow commands)
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
        /// Emit old-vs-new style A/B evaluation report
        #[arg(long)]
        ab_report: bool,
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
        /// Pretty-align ISON tables for review (whitespace-only). Applies to `llman sdd` write paths.
        #[arg(long)]
        pretty_ison: bool,
        /// Legacy: force archive even if validation fails
        #[arg(long, hide = true)]
        force: bool,
        #[command(subcommand)]
        command: Option<ArchiveSubcommand>,
    },
    /// Import spec workflow content from external style
    Import {
        /// Source/target style (currently only: openspec)
        #[arg(long)]
        style: String,
        /// Project root path (default: current directory)
        path: Option<PathBuf>,
    },
    /// Export spec workflow content to external style
    Export {
        /// Source/target style (currently only: openspec)
        #[arg(long)]
        style: String,
        /// Project root path (default: current directory)
        path: Option<PathBuf>,
    },
    /// Migrate llmanspec specs to ISON payload containers
    #[command(hide = true)]
    Migrate {
        /// Execute migration to ISON containers
        #[arg(long)]
        to_ison: bool,
        /// Preview migrations without writing files
        #[arg(long)]
        dry_run: bool,
        /// Project root path (default: current directory)
        path: Option<PathBuf>,
    },
    /// Canonical spec authoring helpers (table/object ISON)
    Spec(SddSpecArgs),
    /// Canonical delta authoring helpers (table/object ISON)
    Delta(SddDeltaArgs),
}

#[derive(Subcommand)]
pub enum SddLegacyCommands {
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
        /// Generate skills for all tools (llman sdd workflow commands only for Claude)
        #[arg(long)]
        all: bool,
        /// Tool to generate skills for: claude,codex (repeatable; workflow commands only for claude)
        #[arg(long, value_delimiter = ',')]
        tool: Vec<String>,
        /// Override output path for generated skills
        #[arg(long)]
        path: Option<PathBuf>,
        /// Disable interactive prompts
        #[arg(long)]
        no_interactive: bool,
        /// Generate only llman sdd workflow commands for Claude (no skills)
        #[arg(long, conflicts_with = "skills_only")]
        commands_only: bool,
        /// Generate only skills (no llman sdd workflow commands)
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
        /// Emit old-vs-new style A/B evaluation report
        #[arg(long)]
        ab_report: bool,
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
        #[command(subcommand)]
        command: Option<ArchiveLegacySubcommand>,
    },
    /// Import spec workflow content from external style
    Import {
        /// Source/target style (currently only: openspec)
        #[arg(long)]
        style: String,
        /// Project root path (default: current directory)
        path: Option<PathBuf>,
    },
    /// Export spec workflow content to external style
    Export {
        /// Source/target style (currently only: openspec)
        #[arg(long)]
        style: String,
        /// Project root path (default: current directory)
        path: Option<PathBuf>,
    },
    /// Migrate llmanspec specs to ISON payload containers
    #[command(hide = true)]
    Migrate {
        /// Execute migration to ISON containers
        #[arg(long)]
        to_ison: bool,
        /// Preview migrations without writing files
        #[arg(long)]
        dry_run: bool,
        /// Project root path (default: current directory)
        path: Option<PathBuf>,
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
        /// Pretty-align ISON tables for review (whitespace-only). Applies to `llman sdd` write paths.
        #[arg(long)]
        pretty_ison: bool,
        /// Force archive even if validation fails
        #[arg(long, hide = true)]
        force: bool,
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

#[derive(Subcommand)]
pub enum ArchiveLegacySubcommand {
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
    run_with_style(args, TemplateStyle::New)
}

pub fn run_legacy(args: &SddLegacyArgs) -> Result<()> {
    run_with_style_legacy(args, TemplateStyle::Legacy)
}

fn run_with_style(args: &SddArgs, style: TemplateStyle) -> Result<()> {
    match &args.command {
        SddCommands::Init { path, lang } => init::run(
            path.as_deref().unwrap_or_else(|| std::path::Path::new(".")),
            lang.as_deref(),
            style,
        ),
        SddCommands::Update { path } => update::run(
            path.as_deref().unwrap_or_else(|| std::path::Path::new(".")),
            style,
        ),
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
            style,
        }),
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
            style,
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
            style,
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
            ab_report,
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
            style,
            ab_report: *ab_report,
        }),
        SddCommands::Archive {
            change,
            skip_specs,
            dry_run,
            pretty_ison,
            force,
            command,
        } => match command {
            Some(ArchiveSubcommand::Run {
                change,
                skip_specs,
                dry_run,
                pretty_ison,
                force,
            }) => archive::run(archive::ArchiveArgs {
                change: change.clone(),
                skip_specs: *skip_specs,
                dry_run: *dry_run,
                pretty_ison: *pretty_ison,
                force: *force,
                style,
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
                pretty_ison: *pretty_ison,
                force: *force,
                style,
            }),
        },
        SddCommands::Import { style, path } => interop::run_import(interop::InteropArgs {
            style: style.clone(),
            path: path.clone(),
        }),
        SddCommands::Export { style, path } => interop::run_export(interop::InteropArgs {
            style: style.clone(),
            path: path.clone(),
        }),
        SddCommands::Migrate {
            to_ison,
            dry_run,
            path,
        } => migrate::run(migrate::MigrateArgs {
            to_ison: *to_ison,
            dry_run: *dry_run,
            path: path.clone(),
        }),
        SddCommands::Spec(args) => match &args.command {
            SddSpecCommands::Skeleton {
                capability,
                force,
                pretty_ison,
            } => authoring::spec::run_skeleton(
                std::path::Path::new("."),
                authoring::spec::SpecSkeletonArgs {
                    capability: capability.clone(),
                    force: *force,
                    pretty_ison: *pretty_ison,
                },
                style,
            ),
            SddSpecCommands::AddRequirement {
                capability,
                req_id,
                title,
                statement,
                pretty_ison,
            } => authoring::spec::run_add_requirement(
                std::path::Path::new("."),
                authoring::spec::SpecAddRequirementArgs {
                    capability: capability.clone(),
                    req_id: req_id.clone(),
                    title: title.clone(),
                    statement: statement.clone(),
                    pretty_ison: *pretty_ison,
                },
                style,
            ),
            SddSpecCommands::AddScenario {
                capability,
                req_id,
                scenario_id,
                given,
                when_,
                then_,
                pretty_ison,
            } => authoring::spec::run_add_scenario(
                std::path::Path::new("."),
                authoring::spec::SpecAddScenarioArgs {
                    capability: capability.clone(),
                    req_id: req_id.clone(),
                    scenario_id: scenario_id.clone(),
                    given: given.clone(),
                    when_: when_.clone(),
                    then_: then_.clone(),
                    pretty_ison: *pretty_ison,
                },
                style,
            ),
        },
        SddCommands::Delta(args) => match &args.command {
            SddDeltaCommands::Skeleton {
                change_id,
                capability,
                force,
                pretty_ison,
            } => authoring::delta::run_skeleton(
                std::path::Path::new("."),
                authoring::delta::DeltaSkeletonArgs {
                    change_id: change_id.clone(),
                    capability: capability.clone(),
                    force: *force,
                    pretty_ison: *pretty_ison,
                },
                style,
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
                pretty_ison,
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
                    pretty_ison: *pretty_ison,
                },
                style,
            ),
            SddDeltaCommands::AddScenario {
                change_id,
                capability,
                req_id,
                scenario_id,
                given,
                when_,
                then_,
                pretty_ison,
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
                    pretty_ison: *pretty_ison,
                },
                style,
            ),
        },
    }
}

fn run_with_style_legacy(args: &SddLegacyArgs, style: TemplateStyle) -> Result<()> {
    match &args.command {
        SddLegacyCommands::Init { path, lang } => init::run(
            path.as_deref().unwrap_or_else(|| std::path::Path::new(".")),
            lang.as_deref(),
            style,
        ),
        SddLegacyCommands::Update { path } => update::run(
            path.as_deref().unwrap_or_else(|| std::path::Path::new(".")),
            style,
        ),
        SddLegacyCommands::UpdateSkills {
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
            style,
        }),
        SddLegacyCommands::List {
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
            style,
        }),
        SddLegacyCommands::Show {
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
            style,
        }),
        SddLegacyCommands::Validate {
            item,
            all,
            changes,
            specs,
            item_type,
            strict,
            json,
            compact_json,
            no_interactive,
            ab_report,
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
            style,
            ab_report: *ab_report,
        }),
        SddLegacyCommands::Archive {
            change,
            skip_specs,
            dry_run,
            force,
            command,
        } => match command {
            Some(ArchiveLegacySubcommand::Run {
                change,
                skip_specs,
                dry_run,
                force,
            }) => archive::run(archive::ArchiveArgs {
                change: change.clone(),
                skip_specs: *skip_specs,
                dry_run: *dry_run,
                pretty_ison: false,
                force: *force,
                style,
            }),
            Some(ArchiveLegacySubcommand::Freeze {
                before,
                keep_recent,
                dry_run,
            }) => freeze::run_freeze(freeze::FreezeArgs {
                before: before.clone(),
                keep_recent: *keep_recent,
                dry_run: *dry_run,
            }),
            Some(ArchiveLegacySubcommand::Thaw { change, dest }) => {
                freeze::run_thaw(freeze::ThawArgs {
                    change: change.clone(),
                    dest: dest.clone(),
                })
            }
            None => archive::run(archive::ArchiveArgs {
                change: change.clone(),
                skip_specs: *skip_specs,
                dry_run: *dry_run,
                pretty_ison: false,
                force: *force,
                style,
            }),
        },
        SddLegacyCommands::Import { style, path } => interop::run_import(interop::InteropArgs {
            style: style.clone(),
            path: path.clone(),
        }),
        SddLegacyCommands::Export { style, path } => interop::run_export(interop::InteropArgs {
            style: style.clone(),
            path: path.clone(),
        }),
        SddLegacyCommands::Migrate {
            to_ison,
            dry_run,
            path,
        } => migrate::run(migrate::MigrateArgs {
            to_ison: *to_ison,
            dry_run: *dry_run,
            path: path.clone(),
        }),
    }
}

use crate::x::arena::{contest, dataset, generate, models, report, vote};
use anyhow::Result;
use clap::{Args, Subcommand};

#[derive(Args)]
#[command(about = "Experimental arena commands (prompt/model challenge workflow)")]
#[command(subcommand_required = true)]
pub struct ArenaArgs {
    #[command(subcommand)]
    pub command: ArenaCommands,
}

#[derive(Subcommand)]
pub enum ArenaCommands {
    /// Discover and pick models from an OpenAI-compatible `/v1/models` endpoint
    Models {
        #[command(subcommand)]
        command: ModelsCommands,
    },
    /// Contest configuration management
    Contest {
        #[command(subcommand)]
        command: ContestCommands,
    },
    /// Dataset configuration management
    Dataset {
        #[command(subcommand)]
        command: DatasetCommands,
    },
    /// Generate a new run (matches + generations + objective signals for repo tasks)
    Gen(generate::GenArgs),
    /// Vote on a run (resumable)
    Vote(vote::VoteArgs),
    /// Compute Elo ratings and render a report for a run
    Report(report::ReportArgs),
}

#[derive(Subcommand)]
pub enum ModelsCommands {
    /// List available model ids
    List {
        /// Print JSON array instead of line-by-line output
        #[arg(long)]
        json: bool,
    },
    /// Interactively multi-select models and print JSON array
    Pick,
}

#[derive(Subcommand)]
pub enum ContestCommands {
    /// Create a contest template file
    Init(contest::ContestInitArgs),
}

#[derive(Subcommand)]
pub enum DatasetCommands {
    /// Create a dataset template file
    Init(dataset::DatasetInitArgs),
}

pub fn run(args: &ArenaArgs) -> Result<()> {
    match &args.command {
        ArenaCommands::Models { command } => match command {
            ModelsCommands::List { json } => models::run_list(*json),
            ModelsCommands::Pick => models::run_pick(),
        },
        ArenaCommands::Contest { command } => match command {
            ContestCommands::Init(args) => contest::run_init(args),
        },
        ArenaCommands::Dataset { command } => match command {
            DatasetCommands::Init(args) => dataset::run_init(args),
        },
        ArenaCommands::Gen(args) => generate::run(args),
        ArenaCommands::Vote(args) => vote::run(args),
        ArenaCommands::Report(args) => report::run(args),
    }
}

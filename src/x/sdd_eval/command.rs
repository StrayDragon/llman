use crate::x::sdd_eval::{paths, playbook, run};
use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use std::path::PathBuf;

#[derive(Args)]
#[command(
    author,
    version,
    about,
    long_about = "Playbook-driven SDD evaluation pipeline (experimental)",
    subcommand_required = true,
    arg_required_else_help = true
)]
pub struct SddEvalArgs {
    #[command(subcommand)]
    pub command: SddEvalCommand,
}

#[derive(Subcommand)]
pub enum SddEvalCommand {
    /// Create a playbook template under .llman/sdd-eval/playbooks/
    Init {
        /// Playbook name (without extension)
        #[arg(long)]
        name: String,
        /// Overwrite existing playbook file
        #[arg(long)]
        force: bool,
    },
    /// Run a playbook and create a new run directory
    Run {
        /// Path to the playbook YAML file
        #[arg(long)]
        playbook: PathBuf,
    },
    /// Generate (or re-generate) a report for a run
    Report {
        /// Run id (directory name under .llman/sdd-eval/runs/)
        #[arg(long)]
        run: String,
    },
    /// Import human scores and merge into run report data
    #[command(name = "import-human")]
    ImportHuman {
        /// Run id (directory name under .llman/sdd-eval/runs/)
        #[arg(long)]
        run: String,
        /// JSON file containing human scores
        #[arg(long)]
        file: PathBuf,
    },
}

pub fn run(args: &SddEvalArgs) -> Result<()> {
    let project_root = paths::project_root_from_cwd()?;

    match &args.command {
        SddEvalCommand::Init { name, force } => {
            let path = paths::playbook_path(&project_root, name);
            playbook::write_template(&path, *force)
                .with_context(|| format!("write playbook template {}", path.display()))?;
            println!("{}", path.display());
            Ok(())
        }
        SddEvalCommand::Run { playbook } => {
            let pb = playbook::load_from_path(playbook)?;
            pb.validate().context("validate playbook")?;
            let run_dir = run::create_run(&project_root, playbook, &pb)?;
            run::execute_run(&project_root, &run_dir, &pb)?;
            println!("{}", run_dir.display());
            Ok(())
        }
        SddEvalCommand::Report { run: run_id } => {
            run::generate_report(&project_root, run_id)?;
            Ok(())
        }
        SddEvalCommand::ImportHuman { run: run_id, file } => {
            run::import_human_scores(&project_root, run_id, file)?;
            Ok(())
        }
    }
}

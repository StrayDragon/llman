use clap::{Args, Subcommand};

use crate::x::collect::tree::TreeArgs;

#[derive(Args)]
#[command(
    author,
    version,
    about,
    long_about = "A collection of commands for collecting information"
)]
pub struct CollectArgs {
    #[command(subcommand)]
    pub command: CollectCommands,
}

#[derive(Subcommand)]
pub enum CollectCommands {
    /// Collect directory structure as a tree
    Tree(TreeArgs),
}

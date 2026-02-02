pub mod command;
mod config;
mod git;
mod hash;
mod interactive;
mod registry;
mod scan;
mod sync;
mod types;

pub use config::load_config;
pub use registry::Registry;
pub use sync::{
    ConflictResolver, InteractiveResolver, apply_target_link, apply_target_links, sync_sources,
};
pub use types::{
    ConfigEntry, ConflictOption, SkillCandidate, SkillsConfig, SkillsPaths, SyncSummary,
    TargetConflictStrategy, TargetMode,
};

pub(crate) use git::find_git_root;
pub(crate) use interactive::is_interactive;

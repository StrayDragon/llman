pub mod command;
mod config;
mod git;
mod interactive;
mod registry;
mod scan;
mod sync;
mod types;

pub use config::load_config;
pub use registry::Registry;
pub use sync::{apply_target_link, apply_target_links};
pub use types::{
    ConfigEntry, SkillCandidate, SkillsConfig, SkillsPaths, TargetConflictStrategy, TargetMode,
};

pub(crate) use git::find_git_root;
pub(crate) use interactive::is_interactive;

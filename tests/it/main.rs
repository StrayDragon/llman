//! Consolidated integration-test binary: every `tests/*_tests.rs` file used
//! to be its own crate statically linking the whole workspace (22 binaries);
//! merging them into one target removes the per-binary link cost while the
//! files stay individually browsable as modules. `bdd_steps.rs` stays a
//! separate auto-discovered target (opt-in `bdd` feature).

pub mod common;

mod claude_code_account_edit;
mod claude_code_account_env;
mod claude_code_forward_args;
mod config;
mod configuration;
mod error;
mod import_direction;
mod integration;
mod path_validation;
mod performance;
mod print_config_dir_path;
mod processor;
mod prompts_orchestrator;
mod rm_empty_dirs;
mod sdd_bdd_compat;
mod sdd_integration;
mod skills_integration;
mod skills_targets_sync;
mod tool;
mod tool_agents_md;
mod tree_sitter;

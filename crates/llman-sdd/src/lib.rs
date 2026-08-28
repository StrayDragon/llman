//! llman-sdd: the spec-driven development core of llman.
//!
//! Hosts the whole `sdd` module tree (command entry, change lifecycle,
//! authoring, project management, shared helpers, spec IR/validation,
//! context index) plus the llmanspec schema generation API.
//!
//! Import-path zero-drift: the facade crate re-exports this crate as
//! `sdd` (`pub use llman_sdd as sdd;`), so `crate::sdd::…` (inside the
//! facade) and `llman::sdd::…` (inside integration tests) keep compiling.
//! Internally the moved code refers to `crate::sdd::…` (this crate owns the
//! `sdd` module) and to `crate::{fs_utils,git_utils,…}` which resolve via
//! the `llman-core` re-exports below.
//!
//! `test_utils` is a `cfg(test)`-only copy of the facade helper (81 lines,
//! test infrastructure exempt from the SSOT rule — see design T12); if the
//! pain grows, extract an `llman-test-support` dev-dependency crate.

#[macro_use]
extern crate rust_i18n;

i18n!("../../locales");

pub use llman_core::{env_safety, fs_utils, git_utils, managed_block, path_utils, schema_utils};

pub mod sdd;

#[cfg(test)]
pub mod test_utils;

//! llman-core: the dependency-free utility layer of llman.
//!
//! Members: `fs_utils` / `path_utils` / `managed_block` / `env_safety` /
//! `git_utils` / `schema_utils`. These modules MUST NOT import llman feature
//! modules (sdd/skills/tool/x) — the direction is locked by
//! `tests/import_direction_tests.rs` on the facade side.
//!
//! The facade crate (`llman`) re-exports every member at its historical
//! path (`llman::fs_utils`, `crate::fs_utils`, …), so importing code does
//! not drift when this crate was split out (change src-cleanup-pre-split T11).

#[macro_use]
extern crate rust_i18n;

i18n!("../../locales");

pub mod env_safety;
pub mod fs_utils;
pub mod git_utils;
pub mod managed_block;
pub mod path_utils;
pub mod schema_utils;

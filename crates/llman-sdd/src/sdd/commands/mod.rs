//! Sdd subcommand implementations (reporting / listing commands).
//!
//! Split from `shared` (T-quick 2026-08-28): `shared` is now the pure
//! utility layer (constants / discovery / ids / interactive / json /
//! match_utils / tasks / types / graph-data helpers); the command bodies
//! for `sdd list|show|status|validate|graph` live here. Dispatch stays in
//! `crate::sdd::command`; these modules are crate-internal and invisible
//! to the facade (which only consumes `sdd::command` + schema API).

pub(crate) mod graph;
pub(crate) mod list;
pub(crate) mod show;
pub(crate) mod status;
pub(crate) mod validate;

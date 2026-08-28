//! Leaves shared verbatim between x providers (claude_code / codex).
//!
//! D-A scope note (change src-cleanup-pre-split): only byte-identical
//! leaves are hoisted here. Command-flow skeletons stay per-provider —
//! their differences are i18n key prefixes and template paths, and
//! parameterizing them is an explicit non-goal of this change.

/// Mask a secret value for display: short values fully starred, longer
/// values keep the first/last 4 chars.
pub(crate) fn mask_secret(value: &str) -> String {
    if value.len() <= 8 {
        "*".repeat(value.len())
    } else {
        format!("{}...{}", &value[..4], &value[value.len() - 4..])
    }
}

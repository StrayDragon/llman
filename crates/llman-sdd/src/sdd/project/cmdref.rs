//! r139: the CLI is the single source of truth for which commands exist, and
//! `--help` is the only command reference agents read (skills embed no
//! command tables). This module is the help-quality gate: every visible `sdd`
//! subcommand MUST carry a non-empty clap doc comment (en baseline), guarded
//! by tests.

use crate::sdd::command::SddCommands;

/// One visible leaf command: full invocation path (without the `llman`
/// prefix) and its clap about line (en baseline).
pub(crate) struct CmdRef {
    pub(crate) path: String,
    pub(crate) about: String,
}

/// Walk the `sdd` subcommand tree and collect every visible leaf.
pub(crate) fn visible_leaves() -> Vec<CmdRef> {
    let root = <SddCommands as clap::Subcommand>::augment_subcommands(clap::Command::new("sdd"));
    let mut out = Vec::new();
    walk(&root, &mut Vec::new(), &mut out);
    out
}

fn walk(cmd: &clap::Command, prefix: &mut Vec<String>, out: &mut Vec<CmdRef>) {
    for sub in cmd.get_subcommands() {
        if sub.is_hide_set() {
            continue;
        }
        prefix.push(sub.get_name().to_string());
        if sub.has_subcommands() {
            walk(sub, prefix, out);
        } else {
            let about = sub
                .get_about()
                .unwrap_or_default()
                .to_string()
                .trim()
                .to_string();
            out.push(CmdRef {
                path: prefix.join(" "),
                about,
            });
        }
        prefix.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_cover_known_subtrees_and_skip_hidden() {
        let paths: Vec<String> = visible_leaves().into_iter().map(|c| c.path).collect();
        for expected in [
            "review",
            "init",
            "list",
            "show",
            "validate",
            "graph",
            "context",
            "change diff",
            "change finalize",
            "spec next-req-id",
            "spec resolve-req",
            "archive freeze",
            "index rebuild",
            "project migrate",
        ] {
            assert!(
                paths.iter().any(|p| p == expected),
                "missing leaf `{expected}` in {paths:?}"
            );
        }
        // Hidden commands (removed surface, r115) must not leak.
        assert!(!paths.iter().any(|p| p.contains("delta")), "{paths:?}");
    }

    /// r139: agents read `--help` directly, so every visible leaf MUST have
    /// a non-empty doc comment as its command reference.
    #[test]
    fn leaves_have_about_baselines() {
        let empty: Vec<String> = visible_leaves()
            .into_iter()
            .filter(|c| c.about.is_empty())
            .map(|c| c.path)
            .collect();
        assert!(
            empty.is_empty(),
            "clap doc comments missing for: {empty:?} — every visible command needs an en baseline"
        );
    }
}

//! r139: managed-skill command reference generated from the clap command
//! tree at render time. The CLI is the single source of truth for which
//! commands exist — the hand-written `sdd-commands` unit drifted and shipped
//! a removed subcommand in 9 skills. One-liners come from i18n
//! (`sdd.cmdref.<dotted-path>`, en + zh-Hans) and fall back to the clap
//! `about` (en) when a key is missing; a missing fallback is not a render
//! failure (r141).

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

/// Locale-aware one-liner with clap-about fallback (r141: a missing key at
/// this level is not a missing render variable).
fn one_liner(locale: &str, path: &str, about: &str) -> String {
    let key = format!("sdd.cmdref.{}", path.replace(' ', "."));
    let translated = rust_i18n::t!(&key, locale = locale).to_string();
    if translated.is_empty() || translated == key {
        about.to_string()
    } else {
        translated
    }
}

/// Render the generated command reference block injected as the
/// `sdd_command_reference` template variable. Grouped format: one line per
/// top-level command; nested subcommands inline on the parent line (keeps
/// the per-skill token cost close to the retired static unit, r139).
pub(crate) fn sdd_command_reference(locale: &str) -> String {
    let mut lines = vec![rust_i18n::t!("sdd.cmdref.header", locale = locale).to_string()];
    let leaves = visible_leaves();
    let mut emitted_parent = Vec::new();
    for leaf in &leaves {
        let (parent, sub) = match leaf.path.split_once(' ') {
            Some((p, s)) => (p.to_string(), Some(s.to_string())),
            None => (leaf.path.clone(), None),
        };
        if sub.is_none() {
            lines.push(format!(
                "- `llman sdd {}` — {}",
                leaf.path,
                one_liner(locale, &leaf.path, &leaf.about)
            ));
        } else if !emitted_parent.contains(&parent) {
            // First leaf of a group: emit the parent line listing all its
            // subcommands with their one-liners.
            let subs: Vec<String> = leaves
                .iter()
                .filter(|l| l.path.starts_with(&format!("{parent} ")))
                .map(|l| {
                    let sub_name = &l.path[parent.len() + 1..];
                    format!("`{sub_name}` {}", one_liner(locale, &l.path, &l.about))
                })
                .collect();
            let parent_about = one_liner(locale, &parent, "");
            if parent_about.is_empty() {
                lines.push(format!("- `llman sdd {parent}` — {}", subs.join("；")));
            } else {
                lines.push(format!(
                    "- `llman sdd {parent}` — {parent_about}：{}",
                    subs.join("；")
                ));
            }
            emitted_parent.push(parent);
        }
    }
    lines.push(rust_i18n::t!("sdd.cmdref.footer", locale = locale).to_string());
    lines.join("\n")
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

    #[test]
    fn leaves_have_about_baselines() {
        let empty: Vec<String> = visible_leaves()
            .into_iter()
            .filter(|c| c.about.is_empty())
            .map(|c| c.path)
            .collect();
        assert!(
            empty.is_empty(),
            "clap doc comments missing for: {empty:?} — every generated one-liner needs an en baseline"
        );
    }

    #[test]
    fn one_liner_falls_back_to_clap_about() {
        let got = one_liner("en", "no.such.command", "clap baseline");
        assert_eq!(got, "clap baseline");
    }

    #[test]
    fn generated_block_is_bilingual_and_current() {
        let en = sdd_command_reference("en");
        // Grouped format: parent line + inline subcommand one-liners.
        assert!(en.contains("`llman sdd change`"), "{en}");
        assert!(en.contains("`diff`"), "{en}");
        // r139: removed syntax must never reappear.
        assert!(!en.contains("spec-md2toon"), "{en}");

        let zh = sdd_command_reference("zh-Hans");
        assert!(zh.contains("`llman sdd change`"), "{zh}");
        assert_ne!(en, zh, "zh-Hans one-liners should differ from en");
    }
}

use crate::sdd::change::lock_gate;
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::{discover_changes, list_specs};
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::backend::feature_backend;
use crate::sdd::spec::backend::feature_backend::compute_rule_morphology;
use crate::sdd::spec::staleness::evaluate_staleness_with_override;
use crate::sdd::spec::validation::ValidationLevel;
use crate::sdd::spec::validation::resolve_spec_file;
use anyhow::{Result, anyhow};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Aggregate review (feat-sdd-review-workflow-suite).
///
/// Five signals, all sourced from the same modules `list` / `show` / `validate`
/// use — no second statistics implementation (r38): pending + manual rules,
/// unbound (orphan) acceptance scenarios, staleness, locked-rule diff hints
/// (D-C: hint only), and a `validate --all` sweep.
#[derive(Debug, Clone)]
pub(crate) struct ReviewArgs {
    pub(crate) capability: Option<String>,
    pub(crate) json: bool,
    pub(crate) export_html: Option<PathBuf>,
    /// Accepted and ignored: this subcommand has no interactive mode.
    /// Kept so the flag matrix stays uniform across sibling subcommands.
    #[allow(dead_code)]
    pub(crate) no_interactive: bool,
}

#[derive(Debug, Clone, Serialize)]
struct Signal {
    kind: &'static str,
    capability: String,
    count: usize,
    detail: String,
}

struct Review {
    signals: Vec<Signal>,
    critical: usize,
    warning: usize,
}

impl Review {
    fn push(&mut self, kind: &'static str, capability: &str, count: usize, detail: String) {
        self.signals.push(Signal {
            kind,
            capability: capability.to_string(),
            count,
            detail,
        });
    }
}

pub(crate) fn run(root: &Path, args: &ReviewArgs) -> Result<()> {
    let llmanspec_dir = root.join(LLMANSPEC_DIR_NAME);
    if !llmanspec_dir.is_dir() {
        // r6: no silent empty result on a non-project directory.
        return Err(anyhow!(
            "no llmanspec project found at {} (run `llman sdd init` first)",
            llmanspec_dir.display()
        ));
    }
    let _config = load_required_config(&llmanspec_dir)?;

    let mut caps = list_specs(root)?;
    if let Some(want) = &args.capability {
        validate_sdd_id(want, "spec")?;
        caps.retain(|c| c == want);
        if caps.is_empty() {
            return Err(anyhow!("capability `{want}` not found"));
        }
    }

    let mut review = Review {
        signals: Vec::new(),
        critical: 0,
        warning: 0,
    };

    for cap in &caps {
        collect_spec_signals(root, cap, &mut review)?;
    }
    collect_locked_hints(root, &mut review);
    collect_validate_sweep(&mut review);

    if args.json {
        let value = serde_json::json!({
            "signals": review.signals,
            "summary": {
                "criticalCount": review.critical,
                "warningCount": review.warning,
            },
        });
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        print_text(&review);
    }

    if let Some(path) = &args.export_html {
        write_html(path, &review)?;
        println!("wrote {}", path.display());
    }

    if review.critical > 0 {
        // r20: CRITICAL findings fail the run for CI / agent gates.
        return Err(anyhow!(
            "review found {} CRITICAL finding(s); see the report above",
            review.critical
        ));
    }
    Ok(())
}

fn collect_spec_signals(root: &Path, cap: &str, review: &mut Review) -> Result<()> {
    let specs_root = root.join(LLMANSPEC_DIR_NAME).join("specs");
    let spec_path = resolve_spec_file(&specs_root, cap)?;
    let content = std::fs::read_to_string(&spec_path)?;
    let parsed =
        feature_backend::FEATURE_BACKEND.parse_content(&content, &format!("spec `{cap}`"))?;
    let morph = compute_rule_morphology(&parsed);

    review.push("pending", cap, morph.rule_pending_count, String::new());
    if morph.rule_pending_count > 0 {
        review.warning += morph.rule_pending_count;
    }
    review.push("manual", cap, morph.rule_manual_count, String::new());

    // r5: unbound acceptance scenarios = orphans (no @req link).
    let orphans = morph.orphan_acceptance_count;
    review.push("unbound", cap, orphans, String::new());
    if orphans > 0 {
        review.warning += orphans;
    }

    let stale = evaluate_staleness_with_override(root, cap, &spec_path, None, None);
    let status = stale.info.status.as_str().to_string();
    if matches!(status.as_str(), "ok" | "info" | "not_applicable") {
        review.push("stale", cap, 0, status);
    } else {
        review.push("stale", cap, 1, status.clone());
        review.warning += 1;
    }
    Ok(())
}

/// D-C: hint only — count locked-rule gate errors per bound active change and
/// point at `llman sdd change diff`; never render the diff inside review.
fn collect_locked_hints(root: &Path, review: &mut Review) {
    let mut bound = 0usize;
    let mut edits = 0usize;
    let Ok(changes) = discover_changes(root) else {
        review.push("locked", "-", 0, String::new());
        return;
    };
    for loc in changes {
        let dir = loc.abs_dir(root);
        let proposal = dir.join("proposal.md");
        let Ok(text) = std::fs::read_to_string(&proposal) else {
            continue;
        };
        let branch = frontmatter_value(&text, "branch");
        let base_sha =
            frontmatter_value(&text, "base_sha").or_else(|| frontmatter_value(&text, "baseSha"));
        let (Some(_branch), Some(base_sha)) = (branch, base_sha) else {
            continue;
        };
        bound += 1;
        let acked = frontmatter_value(&text, "rules_edit_acked")
            .map(|v| v == "true")
            .unwrap_or(false);
        let violations = lock_gate::check(root, &base_sha, acked);
        let errors = violations
            .iter()
            .filter(|i| i.level == ValidationLevel::Error)
            .count();
        edits += errors;
        review.critical += errors;
    }
    review.push(
        "locked",
        "-",
        edits,
        format!("{bound} bound change(s); inspect with `llman sdd change diff <id>`"),
    );
}

fn collect_validate_sweep(review: &mut Review) {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => {
            review.push("validate", "-", 0, "current_exe unavailable".into());
            return;
        }
    };
    let Ok(out) = Command::new(exe)
        .args([
            "sdd",
            "validate",
            "--all",
            "--strict",
            "--no-check",
            "--no-interactive",
        ])
        .output()
    else {
        review.push("validate", "-", 0, "spawn failed".into());
        return;
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let fails = text.lines().filter(|l| l.starts_with("FAIL ")).count();
    let warns = text.matches("WARNING").count();
    if !out.status.success() || fails > 0 {
        review.push(
            "validate",
            "-",
            fails.max(1),
            "validate --all failed; run `llman sdd validate --all` for details".into(),
        );
        review.critical += fails.max(1);
    } else {
        review.push("validate", "-", 0, "ok".into());
    }
    review.warning += warns;
}

fn print_text(review: &Review) {
    println!(
        "Review: critical={} warning={}",
        review.critical, review.warning
    );
    for s in &review.signals {
        println!("{}: {} ({})", s.kind, s.capability, s.count);
        if !s.detail.is_empty() {
            println!("  - {}", s.detail);
        }
    }
}

fn write_html(path: &Path, review: &Review) -> Result<()> {
    const TEMPLATE: &str = include_str!("../../../../templates/sdd/shared/review.html");
    let mut mermaid = String::from("graph TD\n");
    let mut sig_json: Vec<serde_json::Value> = Vec::new();
    for (idx, s) in review.signals.iter().enumerate() {
        let label = esc(&format!(
            "{} [{}] = {} — {}",
            s.capability,
            s.kind,
            s.count,
            if s.detail.is_empty() { "ok" } else { &s.detail }
        ));
        mermaid.push_str(&format!("    s{idx}[\"{label}\"]\n"));
        sig_json.push(serde_json::json!({
            "kind": s.kind,
            "capability": s.capability,
            "count": s.count,
            "detail": s.detail,
        }));
    }
    let html = TEMPLATE
        .replace("__CRITICAL__", &review.critical.to_string())
        .replace("__WARNING__", &review.warning.to_string())
        .replace("__SIGNALS__", &serde_json::to_string(&sig_json)?)
        .replace("__MERMAID__", &mermaid)
        .replace(
            "__GENERATED__",
            &OffsetDateTime::now_utc()
                .format(&Rfc3339)
                .expect("valid timestamp"),
        );
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, html)?;
    Ok(())
}

/// Minimal HTML/mermaid label escaping (r51).
fn esc(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn frontmatter_value(text: &str, key: &str) -> Option<String> {
    let mut in_front = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if in_front {
                break;
            }
            in_front = true;
            continue;
        }
        if !in_front {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(v) = rest.strip_prefix(':') {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

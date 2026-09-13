//! Unified Git-native change binding: branch + fork-point branch (base_branch)
//! + base SHA as the change anchor.
//!
//! Changes attach to a non-default Git branch via `change start` or `change attach`.
//! The only delta is `git diff <base>...HEAD`. Archive seals documentation
//! and merges back into the recorded fork-point branch (r113; squash by default).

use crate::fs_utils::atomic_write_with_mode;
use crate::git_utils::{
    branch_diff, branch_has_upstream, is_default_branch, merge_base_sha,
    resolve_default_branch_ref, run_git, working_tree_clean,
};
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::discovery::resolve_change_dir;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::frontmatter::split_frontmatter;
use anyhow::{Result, anyhow, bail};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Git binding recorded in `proposal.md` frontmatter (unified flow).
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChangeGitBinding {
    pub(crate) branch: String,
    pub(crate) base_sha: String,
    /// Fork-point branch (r111): where this change forked from. `change start`
    /// and `attach` resolve it to the local default branch; `attach --base`
    /// records a stacked fork source explicitly. Empty = legacy binding
    /// written before this field existed (merge target falls back to the
    /// local default branch). MUST NOT feed diff/lock-gate range math — it
    /// only resolves the finalize/archive merge target (sdd-workflow r113).
    pub(crate) base_branch: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AttachArgs {
    pub(crate) change: String,
    /// Re-bind even if already attached (updates branch/base to current HEAD state).
    pub(crate) force: bool,
    /// Record the fork-point branch explicitly (stacked workflows). Defaults
    /// to the local default branch when omitted.
    pub(crate) base: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct DiffArgs {
    pub(crate) change: String,
    pub(crate) export_patch: Option<PathBuf>,
    /// Machine-readable diff summary (r137: commitCount contract key).
    pub(crate) json: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct StartArgs {
    pub(crate) change: String,
    /// Create a linked worktree instead of switching branches in-place (r116).
    pub(crate) worktree: bool,
    /// Accepted and ignored; start has no interactive mode. Keeps the flag
    /// matrix uniform across change subcommands.
    #[allow(dead_code)]
    pub(crate) no_interactive: bool,
}

/// Current branch for change-binding flows: sdd owns the domain error here
/// (llman-core reports detached HEAD as `Ok(None)` — a state, not an error).
fn current_branch_bound(root: &Path) -> Result<String> {
    crate::git_utils::current_branch(root)?
        .ok_or_else(|| anyhow!("detached HEAD is not allowed for change binding"))
}

// Pure git plumbing (run_git / current_branch / current_head_sha /
// resolve_default_branch_ref / is_default_branch / working_tree_clean /
// merge_base_sha / branch_diff / branch_has_upstream) moved verbatim to
// `crate::git_utils` so tool/skills/prompts can share it without reaching
// into sdd internals.

/// Optional shared-mode gate from `bdd.shared` / future config.
/// For now: only enforced when `LLMAN_SDD_REQUIRE_UPSTREAM=1`.
pub(crate) fn shared_mode_required() -> bool {
    std::env::var("LLMAN_SDD_REQUIRE_UPSTREAM")
        .map(|v| matches!(v.trim(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn parse_yaml_string(doc: &serde_json::Value, key: &str) -> Option<String> {
    doc.get(key).and_then(|v| match v {
        serde_json::Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

/// Read Git binding fields from proposal frontmatter (best-effort).
pub(crate) fn read_binding(root: &Path, change_id: &str) -> Result<Option<ChangeGitBinding>> {
    let path = resolve_change_dir(root, change_id)?.join("proposal.md");
    if !path.exists() {
        bail!("change `{}` proposal.md not found", change_id);
    }
    let content = fs::read_to_string(&path)?;
    let (yaml_str, _) = split_frontmatter(&content);
    let Some(yaml_str) = yaml_str else {
        return Ok(None);
    };
    let parsed: serde_json::Value = serde_saphyr::from_str(&yaml_str)
        .map_err(|err| anyhow!("proposal frontmatter YAML invalid: {err}"))?;
    let branch = parse_yaml_string(&parsed, "branch");
    let base_sha = parse_yaml_string(&parsed, "base_sha");
    // Legacy bindings predate `base_branch`; empty = fall back to the local
    // default branch at merge-target resolution time.
    let base_branch = parse_yaml_string(&parsed, "base_branch").unwrap_or_default();
    match (branch, base_sha) {
        (Some(branch), Some(base_sha)) => Ok(Some(ChangeGitBinding {
            branch,
            base_sha,
            base_branch,
        })),
        _ => Ok(None),
    }
}

fn upsert_frontmatter_fields(content: &str, updates: &[(&str, String)]) -> Result<String> {
    let (yaml_str, body) = split_frontmatter(content);
    let mut map: serde_json::Map<String, serde_json::Value> = if let Some(yaml_str) = yaml_str {
        match serde_saphyr::from_str::<serde_json::Value>(&yaml_str)? {
            serde_json::Value::Object(m) => m,
            serde_json::Value::Null => serde_json::Map::new(),
            other => bail!("proposal frontmatter must be a mapping, got {other:?}"),
        }
    } else {
        serde_json::Map::new()
    };

    for (key, value) in updates {
        map.insert((*key).to_string(), serde_json::Value::String(value.clone()));
    }

    let yaml = serde_saphyr::to_string(&serde_json::Value::Object(map))?;
    // serde-saphyr adds a trailing newline; wrap as frontmatter.
    let yaml = yaml.trim_end();
    let body = body.trim_start_matches('\n');
    Ok(format!("---\n{yaml}\n---\n\n{body}"))
}

pub(crate) fn write_binding(
    root: &Path,
    change_id: &str,
    binding: &ChangeGitBinding,
) -> Result<()> {
    let path = resolve_change_dir(root, change_id)?.join("proposal.md");
    let content = fs::read_to_string(&path)?;
    let updates = vec![
        ("branch", binding.branch.clone()),
        ("base_sha", binding.base_sha.clone()),
        ("base_branch", binding.base_branch.clone()),
    ];
    let rebuilt = upsert_frontmatter_fields(&content, &updates)?;
    atomic_write_with_mode(&path, rebuilt.as_bytes(), None)?;
    Ok(())
}

/// Attach the current non-default branch + merge-base SHA to a change.
///
/// Coexists with `change start` (which auto-creates the branch). Use `attach`
/// when the user has already manually `git switch -c`'d to a branch, or wants
/// to bind a non-`sdd/` prefixed branch. Unified flow (r57): works regardless
/// of whether `bdd:` is configured.
pub(crate) fn run_attach(root: &Path, args: AttachArgs) -> Result<()> {
    let change_name = crate::sdd::shared::discovery::resolve_change_id_human(root, &args.change)?;
    validate_sdd_id(&change_name, "change")?;
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);
    let _config = load_required_config(&llmanspec)?;
    let dir = resolve_change_dir(root, &change_name)?;
    if !dir.exists() {
        bail!("change `{}` not found", change_name);
    }
    if !resolve_change_dir(root, &change_name)?
        .join("proposal.md")
        .exists()
    {
        bail!("change `{}` is missing proposal.md", change_name);
    }

    if let Some(existing) = read_binding(root, &change_name)?
        && !args.force
    {
        bail!(
            "change `{}` already attached to branch `{}` (base {}); pass --force to rebind",
            change_name,
            existing.branch,
            existing.base_sha
        );
    }

    let branch = current_branch_bound(root)?;
    if is_default_branch(root, &branch)? {
        bail!(
            "changes must not attach on the default branch (`{branch}`); create/switch to a feature branch first (or use `change start`)"
        );
    }
    let default_ref = resolve_default_branch_ref(root)?;
    let base_sha = merge_base_sha(root, &default_ref)?;
    let base_branch = match args.base.as_deref() {
        Some(raw) => {
            let trimmed = raw.trim();
            if let Err(reason) = crate::env_safety::validate_user_git_ref(trimmed) {
                bail!("invalid --base ref: {reason}");
            }
            if !crate::git_utils::git_ref_exists(root, &format!("refs/heads/{trimmed}")) {
                bail!(
                    "base branch `{trimmed}` does not exist; --base records the fork source branch for merge-target resolution (r111)"
                );
            }
            if trimmed == branch {
                bail!("--base must differ from the bound branch `{branch}`");
            }
            trimmed.to_string()
        }
        None => local_branch_name(&default_ref),
    };
    let binding = ChangeGitBinding {
        branch: branch.clone(),
        base_sha: base_sha.clone(),
        base_branch: base_branch.clone(),
    };
    write_binding(root, &change_name, &binding)?;
    println!(
        "attached change `{}` → branch `{branch}` base `{base_sha}` base-branch `{base_branch}`",
        change_name
    );
    Ok(())
}

/// Local checkout-able branch name for a resolved default ref
/// (`origin/main` → `main`; local refs pass through unchanged).
pub(crate) fn local_branch_name(default_ref: &str) -> String {
    default_ref
        .strip_prefix("origin/")
        .unwrap_or(default_ref)
        .to_string()
}

/// Count uncommitted entries in the working tree (`git status --porcelain`).
fn dirty_tree_count(root: &Path) -> Result<usize> {
    let status = run_git(root, &["status", "--porcelain"])?;
    Ok(status.lines().filter(|l| !l.trim().is_empty()).count())
}

/// Build the feature branch name for a change.
///
/// Format: `<prefix><change-id>` where prefix defaults to `sdd/` and can be
/// overridden via `sdd.branch_prefix` in config.yaml. Slice 2 default only;
/// worktree naming (r116) is handled separately in `start.rs`.
fn feature_branch_name(change_id: &str, config: &crate::sdd::project::config::SddConfig) -> String {
    let prefix = config
        .sdd
        .as_ref()
        .and_then(|s| s.branch_prefix.as_deref())
        .unwrap_or("sdd/");
    format!("{prefix}{change_id}")
}

/// `change start <id>`: the recommended Designed → Full entry point (r111).
///
/// Single-process: clean-tree gate → create feature branch → write attach
/// binding. Errors are terse and token-friendly (no stack traces, no advice
/// lists). `--worktree` (r116) routes to worktree creation (slice 3).
pub(crate) fn run_start(root: &Path, args: StartArgs) -> Result<()> {
    let change_name = crate::sdd::shared::discovery::resolve_change_id_human(root, &args.change)?;
    validate_sdd_id(&change_name, "change")?;
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec)?;
    let dir = resolve_change_dir(root, &change_name)?;
    if !dir.exists() {
        bail!("change `{}` not found", change_name);
    }
    if !resolve_change_dir(root, &change_name)?
        .join("proposal.md")
        .exists()
    {
        bail!("change `{}` is missing proposal.md", change_name);
    }
    if let Some(existing) = read_binding(root, &change_name)? {
        bail!(
            "change `{}` already attached to branch `{}` (base {}); pass --force to rebind via `change attach`",
            change_name,
            existing.branch,
            existing.base_sha
        );
    }
    // clean-tree gate (r111): terse, token-friendly error.
    let dirty = dirty_tree_count(root)?;
    if dirty > 0 {
        bail!("dirty tree: {dirty} uncommitted files; commit/stash before `change start`");
    }
    // Reject if already on a non-default branch the user may want to keep.
    let current = current_branch_bound(root)?;
    if !is_default_branch(root, &current)? {
        bail!(
            "already on non-default branch `{current}`; use `change attach` to bind it, or switch to the default branch before `change start`"
        );
    }
    let branch = feature_branch_name(&change_name, &config);
    let default_ref = resolve_default_branch_ref(root)?;
    let base_sha = merge_base_sha(root, &default_ref)?;
    // start always forks from the default branch (r111 requires being on it),
    // so base_branch is the resolved local default branch name.
    let binding = ChangeGitBinding {
        branch: branch.clone(),
        base_sha: base_sha.clone(),
        base_branch: local_branch_name(&default_ref),
    };
    if args.worktree {
        let wt_path = crate::sdd::change::start::run_start_worktree(
            root,
            &change_name,
            &branch,
            &base_sha,
            &config,
        )?;
        // Binding must land in the linked worktree checkout, not the main tree
        // (main stays on the default branch after `worktree add`).
        write_binding(&wt_path, &change_name, &binding)?;
    } else {
        // Create and switch to the feature branch from the default branch.
        run_git(root, &["checkout", "-b", &branch])?;
        write_binding(root, &change_name, &binding)?;
    }
    println!(
        "started change `{change_name}` → branch `{branch}` base `{base_sha}` base-branch `{}`",
        binding.base_branch
    );
    Ok(())
}

/// `change checkpoint` is removed (r25): any call fails with a single-line
/// pointer to `change finalize`, mirroring the r115 `change delta` precedent.
pub(crate) fn run_checkpoint_removed() -> Result<()> {
    anyhow::bail!("change checkpoint is removed; use change finalize")
}

/// Number of commits on the current branch since the attach base (r137).
pub(crate) fn commit_count_since_base(root: &Path, base_sha: &str) -> Result<i64> {
    let out = run_git(root, &["rev-list", "--count", &format!("{base_sha}..HEAD")])?;
    out.trim()
        .parse::<i64>()
        .map_err(|err| anyhow!("bad commit count `{out}`: {err}"))
}

/// Print the r137 commit-count line; hint (non-blocking) when discipline
/// suggests the history is drifting into step-log territory.
pub(crate) fn print_commit_count(root: &Path, base_sha: &str) -> Result<()> {
    let commit_count = commit_count_since_base(root, base_sha)?;
    println!(
        "{}",
        t!("sdd.change.commits_since_base", count = commit_count)
    );
    if commit_count > 1 {
        eprintln!(
            "{}",
            t!("sdd.change.multi_commit_hint", count = commit_count)
        );
    }
    Ok(())
}

pub(crate) fn run_diff(root: &Path, args: DiffArgs) -> Result<()> {
    let change_name = crate::sdd::shared::discovery::resolve_change_id_human(root, &args.change)?;
    validate_sdd_id(&change_name, "change")?;
    let Some(binding) = read_binding(root, &change_name)? else {
        bail!(
            "change `{}` has no Git binding; run `llman sdd change attach {}` first",
            change_name,
            change_name
        );
    };
    let branch = current_branch_bound(root)?;
    if branch != binding.branch {
        bail!(
            "current branch `{branch}` does not match attached branch `{}`",
            binding.branch
        );
    }
    if args.json {
        let range_base =
            crate::sdd::change::lock_gate::effective_range_base(root, Some(&binding.base_sha))
                .unwrap_or_else(|_| binding.base_sha.clone());
        let commit_count = commit_count_since_base(root, &range_base)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "change": change_name,
                "branch": binding.branch,
                "base": binding.base_sha,
                "commitCount": commit_count,
            }))?
        );
        return Ok(());
    }
    let range_base =
        crate::sdd::change::lock_gate::effective_range_base(root, Some(&binding.base_sha))
            .unwrap_or_else(|_| binding.base_sha.clone());
    print_commit_count(root, &range_base)?;
    let diff = branch_diff(root, &range_base)?;
    if let Some(path) = &args.export_patch {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write_with_mode(path, diff.as_bytes(), None)?;
        println!("wrote patch export → {}", path.display());
    } else {
        print!("{diff}");
        if !diff.ends_with('\n') && !diff.is_empty() {
            println!();
        }
    }
    // r135 report-only: surface locked-rule edits behind this change's diff.
    for issue in crate::sdd::change::lock_gate::check(root, &range_base) {
        let label = match issue.level {
            crate::sdd::spec::validation::ValidationLevel::Warning => "WARNING",
            crate::sdd::spec::validation::ValidationLevel::Info => "INFO",
            crate::sdd::spec::validation::ValidationLevel::Error => "ERROR",
        };
        println!("[{}] {}: {}", label, issue.path, issue.message);
    }
    Ok(())
}

/// Enforce archive preconditions: attached, on branch (strict variant for `change archive`).
///
/// This is the strict variant used by `change archive` — it requires a clean
/// working tree (archive itself does not commit anything, so a dirty tree
/// would get lost across the ff-merge). The `checkpointed` requirement is
/// removed (r25): `change checkpoint` no longer exists; archive seals whatever
/// the branch tip carries.
/// For the `finalize` path (which itself handles the dirty tree via the auto
/// commit), use [`enforce_bdd_archive_gates_relaxed`] instead.
pub(crate) fn enforce_bdd_archive_gates(root: &Path, change_id: &str) -> Result<ChangeGitBinding> {
    enforce_bdd_archive_gates_inner(root, change_id, /* require_clean_tree */ true)
}

/// Relaxed variant of [`enforce_bdd_archive_gates`] that skips the clean-tree
/// check. Used by `change finalize` so the implementation diff can stay dirty
/// and be committed together with the archive rename by the auto commit.
///
/// Caller is responsible for the auto commit (`archive(sdd): <change-id>`,
/// r25) after this returns.
pub(crate) fn enforce_bdd_archive_gates_relaxed(
    root: &Path,
    change_id: &str,
) -> Result<ChangeGitBinding> {
    enforce_bdd_archive_gates_inner(root, change_id, /* require_clean_tree */ false)
}

fn enforce_bdd_archive_gates_inner(
    root: &Path,
    change_id: &str,
    require_clean_tree: bool,
) -> Result<ChangeGitBinding> {
    let Some(binding) = read_binding(root, change_id)? else {
        bail!(
            "archive requires Git binding; run `llman sdd change attach {change_id}` (or `change start`) first"
        );
    };
    let branch = current_branch_bound(root)?;
    if branch != binding.branch {
        bail!(
            "archive must run on attached branch `{}` (current: `{branch}`)",
            binding.branch
        );
    }
    if is_default_branch(root, &branch)? {
        bail!("archive must not run on the default branch");
    }
    if require_clean_tree && !working_tree_clean(root)? {
        bail!("working tree must be clean before archive");
    }
    if shared_mode_required() && !branch_has_upstream(root)? {
        bail!("shared mode requires an upstream before archive");
    }
    Ok(binding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_utils::current_branch;
    use std::process::Command;
    use tempfile::tempdir;

    /// Frontmatter rewrites must be EOF-hook-clean: the rebuilt file ends
    /// with exactly one trailing newline even when the input lacked one, and
    /// rewriting twice is byte-identical (idempotent).
    #[test]
    fn upsert_frontmatter_preserves_and_canonicalizes_trailing_newline() {
        let with_newline = "---\ndepends_on: []\n---\n\n## Why\nx.\n";
        let without_newline = "---\ndepends_on: []\n---\n\n## Why\nx.";
        let updates = vec![("branch", "sdd/c1".to_string())];

        let rebuilt_a = upsert_frontmatter_fields(with_newline, &updates).unwrap();
        let rebuilt_b = upsert_frontmatter_fields(without_newline, &updates).unwrap();
        assert!(rebuilt_a.ends_with("x.\n"), "got: {rebuilt_a:?}");
        assert!(rebuilt_b.ends_with("x.\n"), "got: {rebuilt_b:?}");

        // Idempotent: rewriting the rewritten file changes nothing.
        let twice = upsert_frontmatter_fields(&rebuilt_a, &updates).unwrap();
        assert_eq!(twice, rebuilt_a);
    }

    fn git(root: &Path, args: &[&str]) {
        let out = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn init_repo(root: &Path) {
        git(root, &["init", "-b", "main"]);
        git(root, &["config", "user.name", "t"]);
        git(root, &["config", "user.email", "t@x"]);
        fs::write(root.join("README"), "hi").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "init"]);
    }

    #[test]
    fn attach_rejects_default_branch() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root);
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\nbdd:\n  run_command: \"true\"\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/proposal.md"), "## Why\nx\n").unwrap();
        let err = run_attach(
            root,
            AttachArgs {
                change: "c1".into(),
                force: false,
                base: None,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("default branch"), "got: {err}");
    }

    #[test]
    fn start_rejects_dirty_tree() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root);
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/proposal.md"), "## Why\nx\n").unwrap();
        // Uncommitted file → dirty tree gate (r111).
        fs::write(root.join("uncommitted"), "x").unwrap();
        let err = run_start(
            root,
            StartArgs {
                change: "c1".into(),
                worktree: false,
                no_interactive: false,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("dirty tree"), "got: {err}");
        // Must NOT be verbose: token-friendly.
        assert!(!err.contains("\n"), "error must be single-line: {err}");
    }

    #[test]
    fn start_creates_branch_and_binding_on_clean_tree() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root);
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/proposal.md"), "## Why\nx\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "seed"]);
        run_start(
            root,
            StartArgs {
                change: "c1".into(),
                worktree: false,
                no_interactive: false,
            },
        )
        .expect("start on clean tree");
        // Branch created.
        let branch = current_branch(root).unwrap().unwrap();
        assert_eq!(branch, "sdd/c1");
        // Binding written.
        let binding = read_binding(root, "c1").unwrap().unwrap();
        assert_eq!(binding.branch, "sdd/c1");
        assert!(!binding.base_sha.is_empty());
    }

    #[test]
    fn start_worktree_writes_binding_into_linked_tree_not_main() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root);
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/proposal.md"), "## Why\nx\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "seed"]);
        run_start(
            root,
            StartArgs {
                change: "c1".into(),
                worktree: true,
                no_interactive: false,
            },
        )
        .expect("start --worktree");

        // Main worktree stays on default and must NOT carry the binding.
        assert_eq!(current_branch(root).unwrap().unwrap(), "main");
        let main_proposal =
            fs::read_to_string(root.join("llmanspec/changes/c1/proposal.md")).unwrap();
        assert!(
            !main_proposal.contains("branch:"),
            "main tree must not get binding: {main_proposal}"
        );

        let wt = root.join(".git/sdd/worktrees/c1");
        assert!(wt.exists(), "worktree path missing");
        let wt_binding = read_binding(&wt, "c1")
            .unwrap()
            .expect("binding in worktree");
        assert_eq!(wt_binding.branch, "sdd/c1");
        assert!(!wt_binding.base_sha.is_empty());
    }

    #[test]
    fn start_rejects_already_attached() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root);
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/proposal.md"), "## Why\nx\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "seed"]);
        run_start(
            root,
            StartArgs {
                change: "c1".into(),
                worktree: false,
                no_interactive: false,
            },
        )
        .expect("first start");
        git(root, &["checkout", "main"]);
        git(root, &["branch", "-D", "sdd/c1"]);
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "post-start"]);
        // Already attached → reject without --force (start has no --force;
        // rebind goes via `change attach --force`).
        let err = run_start(
            root,
            StartArgs {
                change: "c1".into(),
                worktree: false,
                no_interactive: false,
            },
        )
        .unwrap_err()
        .to_string();
        assert!(err.contains("already attached"), "got: {err}");
    }

    #[test]
    fn attach_and_diff_on_feature_branch() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root);
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\nbdd:\n  run_command: \"true\"\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/proposal.md"), "## Why\nx\n").unwrap();
        git(root, &["checkout", "-b", "sdd/c1"]);
        fs::write(root.join("extra.txt"), "e").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "feat"]);

        run_attach(
            root,
            AttachArgs {
                change: "c1".into(),
                force: false,
                base: None,
            },
        )
        .unwrap();
        let binding = read_binding(root, "c1").unwrap().unwrap();
        assert_eq!(binding.branch, "sdd/c1");
        assert!(!binding.base_sha.is_empty());

        let diff = branch_diff(root, &binding.base_sha).unwrap();
        assert!(diff.contains("extra.txt") || !diff.is_empty());
    }

    #[test]
    fn start_records_base_branch_as_local_default() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root);
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/proposal.md"), "## Why\nx\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "seed"]);
        run_start(
            root,
            StartArgs {
                change: "c1".into(),
                worktree: false,
                no_interactive: false,
            },
        )
        .expect("start");
        let binding = read_binding(root, "c1").unwrap().unwrap();
        assert_eq!(binding.base_branch, "main");
    }

    #[test]
    fn attach_base_flag_records_custom_fork_source() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root);
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/proposal.md"), "## Why\nx\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "seed"]);
        git(root, &["branch", "stack-base"]);
        git(root, &["checkout", "-b", "feat/x"]);
        run_attach(
            root,
            AttachArgs {
                change: "c1".into(),
                force: false,
                base: Some("stack-base".into()),
            },
        )
        .expect("attach --base");
        let binding = read_binding(root, "c1").unwrap().unwrap();
        assert_eq!(binding.base_branch, "stack-base");
    }

    #[test]
    fn attach_base_rejects_missing_and_self_referential_branch() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root);
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/config.yaml"),
            "schema: spec-driven\nlocale: en\n",
        )
        .unwrap();
        fs::write(root.join("llmanspec/changes/c1/proposal.md"), "## Why\nx\n").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", "seed"]);
        git(root, &["checkout", "-b", "feat/x"]);

        let missing = run_attach(
            root,
            AttachArgs {
                change: "c1".into(),
                force: false,
                base: Some("no-such-branch".into()),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(missing.contains("does not exist"), "got: {missing}");

        let self_ref = run_attach(
            root,
            AttachArgs {
                change: "c1".into(),
                force: false,
                base: Some("feat/x".into()),
            },
        )
        .unwrap_err()
        .to_string();
        assert!(self_ref.contains("must differ"), "got: {self_ref}");
    }

    #[test]
    fn read_binding_legacy_without_base_branch_yields_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("llmanspec/changes/c1")).unwrap();
        fs::write(
            root.join("llmanspec/changes/c1/proposal.md"),
            "---\nbranch: feat/x\nbase_sha: abc123\n---\n## Why\nx\n",
        )
        .unwrap();
        let binding = read_binding(root, "c1").unwrap().unwrap();
        assert_eq!(binding.branch, "feat/x");
        assert_eq!(binding.base_sha, "abc123");
        assert!(
            binding.base_branch.is_empty(),
            "legacy binding must fall back via empty base_branch"
        );
    }

    #[test]
    fn resolve_merge_method_precedence_and_validation() {
        use crate::sdd::change::archive::{MergeMethod, resolve_merge_method};
        // Built-in default.
        assert_eq!(
            resolve_merge_method(None, None).unwrap(),
            MergeMethod::Squash
        );
        // Config wins over default.
        assert_eq!(
            resolve_merge_method(Some("ff"), None).unwrap(),
            MergeMethod::Ff
        );
        // Flag wins over config.
        assert_eq!(
            resolve_merge_method(Some("ff"), Some("squash")).unwrap(),
            MergeMethod::Squash
        );
        // Invalid values error with the legal set.
        let err = resolve_merge_method(Some("rebase"), None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("squash") && err.contains("ff"), "got: {err}");
    }

    #[test]
    fn resolve_merge_target_precedence() {
        use crate::sdd::change::archive::resolve_merge_target;
        let dir = tempdir().unwrap();
        let root = dir.path();
        init_repo(root); // repo has local `main`

        // 1. --into wins (validated ref).
        assert_eq!(
            resolve_merge_target(root, "stack-base", Some("main")).unwrap(),
            "main"
        );
        // 2. Recorded base_branch wins when the branch exists locally.
        git(root, &["branch", "stack-base"]);
        assert_eq!(
            resolve_merge_target(root, "stack-base", None).unwrap(),
            "stack-base"
        );
        // 3. Recorded base_branch missing locally → fall back to default.
        assert_eq!(resolve_merge_target(root, "ghost", None).unwrap(), "main");
        // 4. Legacy empty base_branch → default.
        assert_eq!(resolve_merge_target(root, "", None).unwrap(), "main");
        // 5. --into refuses option injection.
        assert!(resolve_merge_target(root, "", Some("-c")).is_err());
    }
}

//! Unified Git-native change binding: branch + base SHA as the change anchor.
//!
//! Changes attach to a non-default Git branch via `change start` or `change attach`.
//! The only delta is `git diff <base>...HEAD`. Archive seals documentation
//! and fast-forward merges into the default branch.

use crate::fs_utils::atomic_write_with_mode;
use crate::git_utils::{
    branch_diff, branch_has_upstream, current_branch, current_head_sha, is_default_branch,
    merge_base_sha, resolve_default_branch_ref, run_git, working_tree_clean,
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
    pub(crate) checkpointed: bool,
    pub(crate) checkpoint_sha: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AttachArgs {
    pub(crate) change: String,
    /// Re-bind even if already attached (updates branch/base to current HEAD state).
    pub(crate) force: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct CheckpointArgs {
    pub(crate) change: String,
    pub(crate) no_check: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct DiffArgs {
    pub(crate) change: String,
    pub(crate) export_patch: Option<PathBuf>,
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

fn parse_yaml_string(doc: &serde_yaml::Value, key: &str) -> Option<String> {
    doc.get(key).and_then(|v| match v {
        serde_yaml::Value::String(s) => {
            let t = s.trim();
            if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            }
        }
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn parse_yaml_bool(doc: &serde_yaml::Value, key: &str) -> bool {
    match doc.get(key) {
        Some(serde_yaml::Value::Bool(b)) => *b,
        Some(serde_yaml::Value::String(s)) => matches!(s.trim(), "true" | "yes" | "1"),
        _ => false,
    }
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
    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml_str)
        .map_err(|err| anyhow!("proposal frontmatter YAML invalid: {err}"))?;
    let branch = parse_yaml_string(&parsed, "branch");
    let base_sha =
        parse_yaml_string(&parsed, "base_sha").or_else(|| parse_yaml_string(&parsed, "baseSha"));
    match (branch, base_sha) {
        (Some(branch), Some(base_sha)) => Ok(Some(ChangeGitBinding {
            branch,
            base_sha,
            checkpointed: parse_yaml_bool(&parsed, "checkpointed"),
            checkpoint_sha: parse_yaml_string(&parsed, "checkpoint_sha")
                .or_else(|| parse_yaml_string(&parsed, "checkpointSha")),
        })),
        _ => Ok(None),
    }
}

fn upsert_frontmatter_fields(content: &str, updates: &[(&str, String)]) -> Result<String> {
    let (yaml_str, body) = split_frontmatter(content);
    let mut map: serde_yaml::Mapping = if let Some(yaml_str) = yaml_str {
        match serde_yaml::from_str::<serde_yaml::Value>(&yaml_str)? {
            serde_yaml::Value::Mapping(m) => m,
            serde_yaml::Value::Null => serde_yaml::Mapping::new(),
            other => bail!("proposal frontmatter must be a mapping, got {other:?}"),
        }
    } else {
        serde_yaml::Mapping::new()
    };

    for (key, value) in updates {
        map.insert(
            serde_yaml::Value::String((*key).to_string()),
            serde_yaml::Value::String(value.clone()),
        );
    }

    // Represent checkpointed as bool when possible.
    if let Some((_, v)) = updates.iter().find(|(k, _)| *k == "checkpointed") {
        let b = matches!(v.as_str(), "true" | "yes" | "1");
        map.insert(
            serde_yaml::Value::String("checkpointed".into()),
            serde_yaml::Value::Bool(b),
        );
    }

    let yaml = serde_yaml::to_string(&serde_yaml::Value::Mapping(map))?;
    // serde_yaml adds a trailing newline; wrap as frontmatter.
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
    let mut updates = vec![
        ("branch", binding.branch.clone()),
        ("base_sha", binding.base_sha.clone()),
        (
            "checkpointed",
            if binding.checkpointed {
                "true".into()
            } else {
                "false".into()
            },
        ),
    ];
    if let Some(sha) = &binding.checkpoint_sha {
        updates.push(("checkpoint_sha", sha.clone()));
    }
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

    let branch = current_branch(root)?;
    if is_default_branch(root, &branch)? {
        bail!(
            "changes must not attach on the default branch (`{branch}`); create/switch to a feature branch first (or use `change start`)"
        );
    }
    let default_ref = resolve_default_branch_ref(root)?;
    let base_sha = merge_base_sha(root, &default_ref)?;
    let binding = ChangeGitBinding {
        branch: branch.clone(),
        base_sha: base_sha.clone(),
        checkpointed: false,
        checkpoint_sha: None,
    };
    write_binding(root, &change_name, &binding)?;
    println!(
        "attached change `{}` → branch `{branch}` base `{base_sha}`",
        change_name
    );
    Ok(())
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
    let current = current_branch(root)?;
    if !is_default_branch(root, &current)? {
        bail!(
            "already on non-default branch `{current}`; use `change attach` to bind it, or switch to the default branch before `change start`"
        );
    }
    let branch = feature_branch_name(&change_name, &config);
    let default_ref = resolve_default_branch_ref(root)?;
    let base_sha = merge_base_sha(root, &default_ref)?;
    let binding = ChangeGitBinding {
        branch: branch.clone(),
        base_sha: base_sha.clone(),
        checkpointed: false,
        checkpoint_sha: None,
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
    println!("started change `{change_name}` → branch `{branch}` base `{base_sha}`");
    Ok(())
}

/// Require a clean tree, matching branch binding, and (optionally) full BDD check.
pub(crate) fn run_checkpoint(root: &Path, args: CheckpointArgs) -> Result<()> {
    let change_name = crate::sdd::shared::discovery::resolve_change_id_human(root, &args.change)?;
    validate_sdd_id(&change_name, "change")?;
    let _llmanspec = root.join(LLMANSPEC_DIR_NAME);
    let Some(mut binding) = read_binding(root, &change_name)? else {
        bail!(
            "change `{}` has no Git binding; run `llman sdd change attach {}` first",
            change_name,
            change_name
        );
    };

    let branch = current_branch(root)?;
    if branch != binding.branch {
        bail!(
            "current branch `{branch}` does not match attached branch `{}`",
            binding.branch
        );
    }
    if is_default_branch(root, &branch)? {
        bail!("cannot checkpoint on the default branch");
    }
    // Locked-rule integrity (spec-format r135).
    {
        let acked = crate::sdd::change::lock_gate::rules_edit_acked_for(root, &change_name);
        let lock_issues = crate::sdd::change::lock_gate::check(root, &binding.base_sha, acked);
        for issue in &lock_issues {
            match issue.level {
                crate::sdd::spec::validation::ValidationLevel::Error => {
                    eprintln!("{}", issue.message);
                    anyhow::bail!("locked-rule gate failed");
                }
                _ => eprintln!("{}", issue.message),
            }
        }
    }
    if !working_tree_clean(root)? {
        bail!("working tree is dirty; commit all changes before checkpoint");
    }

    if shared_mode_required() && !branch_has_upstream(root)? {
        bail!(
            "shared mode requires an upstream (set LLMAN_SDD_REQUIRE_UPSTREAM=0 to skip, or `git push -u`)"
        );
    }

    // Fast + optional full validation of the live branch tree.
    crate::sdd::commands::validate::run(
        root,
        crate::sdd::commands::validate::ValidateArgs {
            item: None,
            all: false,
            changes: false,
            specs: true,
            item_type: None,
            strict: true,
            json: false,
            compact_json: false,
            stage: None,
            no_interactive: true,
            check: !args.no_check,
            no_check: args.no_check,
        },
    )?;

    // Also validate the change documentation itself (proposal/tasks stage).
    crate::sdd::commands::validate::run(
        root,
        crate::sdd::commands::validate::ValidateArgs {
            item: Some(change_name.clone()),
            all: false,
            changes: false,
            specs: false,
            item_type: Some("change".into()),
            strict: true,
            json: false,
            compact_json: false,
            stage: None,
            no_interactive: true,
            check: false,
            no_check: true,
        },
    )?;

    let head = current_head_sha(root)?;
    binding.checkpointed = true;
    binding.checkpoint_sha = Some(head.clone());
    write_binding(root, &change_name, &binding)?;
    println!(
        "checkpointed change `{}` at `{head}` on branch `{}`",
        change_name, binding.branch
    );
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
    let branch = current_branch(root)?;
    if branch != binding.branch {
        bail!(
            "current branch `{branch}` does not match attached branch `{}`",
            binding.branch
        );
    }
    let diff = branch_diff(root, &binding.base_sha)?;
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
    Ok(())
}

/// Enforce archive preconditions: attached, checkpointed, clean, on branch (strict variant for `change archive`).
///
/// This is the strict variant used by `change archive` — it requires a clean
/// working tree (because archive itself does not write the checkpoint frontmatter,
/// so a clean tree guarantees `checkpoint_sha` still points to a real commit).
/// For the `finalize` path (which writes the frontmatter itself and intentionally
/// leaves the tree dirty for a single commit), use
/// [`enforce_bdd_archive_gates_relaxed`] instead.
pub(crate) fn enforce_bdd_archive_gates(root: &Path, change_id: &str) -> Result<ChangeGitBinding> {
    enforce_bdd_archive_gates_inner(root, change_id, /* require_clean_tree */ true)
}

/// Relaxed variant of [`enforce_bdd_archive_gates`] that skips the clean-tree
/// AND `checkpointed` checks. Used by `change finalize` so:
/// (1) the implementation diff can stay dirty and be committed together with
///     the finalize metadata in a single commit; and
/// (2) finalize itself is responsible for writing the `checkpointed` field
///     (and `checkpoint_sha`), so we must not reject a pre-checkpoint binding.
///
/// Caller is responsible for persisting `checkpointed: true` (and
/// `checkpoint_sha`) on the change binding after this returns.
pub(crate) fn enforce_bdd_archive_gates_relaxed(
    root: &Path,
    change_id: &str,
) -> Result<ChangeGitBinding> {
    let Some(binding) = read_binding(root, change_id)? else {
        bail!(
            "archive requires Git binding; run `llman sdd change attach {change_id}` then checkpoint"
        );
    };
    let branch = current_branch(root)?;
    if branch != binding.branch {
        bail!(
            "archive must run on attached branch `{}` (current: `{branch}`)",
            binding.branch
        );
    }
    if is_default_branch(root, &branch)? {
        bail!("archive must not run on the default branch");
    }
    if shared_mode_required() && !branch_has_upstream(root)? {
        bail!("shared mode requires an upstream before archive");
    }
    Ok(binding)
}

fn enforce_bdd_archive_gates_inner(
    root: &Path,
    change_id: &str,
    require_clean_tree: bool,
) -> Result<ChangeGitBinding> {
    let Some(binding) = read_binding(root, change_id)? else {
        bail!(
            "archive requires Git binding; run `llman sdd change attach {change_id}` then checkpoint"
        );
    };
    let branch = current_branch(root)?;
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
    if !binding.checkpointed {
        bail!(
            "change `{change_id}` is not checkpointed; run `llman sdd change checkpoint {change_id}`"
        );
    }
    if shared_mode_required() && !branch_has_upstream(root)? {
        bail!("shared mode requires an upstream before archive");
    }
    Ok(binding)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use tempfile::tempdir;

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
        let branch = current_branch(root).unwrap();
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
        assert_eq!(current_branch(root).unwrap(), "main");
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
            },
        )
        .unwrap();
        let binding = read_binding(root, "c1").unwrap().unwrap();
        assert_eq!(binding.branch, "sdd/c1");
        assert!(!binding.base_sha.is_empty());
        assert!(!binding.checkpointed);

        let diff = branch_diff(root, &binding.base_sha).unwrap();
        assert!(diff.contains("extra.txt") || !diff.is_empty());
    }
}

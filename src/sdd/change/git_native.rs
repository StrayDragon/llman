//! Git-native BDD-on change binding: branch + base SHA as the change delta.
//!
//! BDD-on changes attach to a non-canonical Git branch. The only delta is
//! `git diff <base>...HEAD`. Archive seals documentation only; Git merge
//! promotes the real specs/features to the default branch.

use crate::fs_utils::atomic_write_with_mode;
use crate::sdd::project::config::load_required_config;
use crate::sdd::shared::constants::LLMANSPEC_DIR_NAME;
use crate::sdd::shared::ids::validate_sdd_id;
use crate::sdd::spec::frontmatter::split_frontmatter;
use anyhow::{Result, anyhow, bail};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git binding recorded in `proposal.md` frontmatter for BDD-on changes.
#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangeGitBinding {
    pub branch: String,
    pub base_sha: String,
    pub checkpointed: bool,
    pub checkpoint_sha: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AttachArgs {
    pub change: String,
    /// Re-bind even if already attached (updates branch/base to current HEAD state).
    pub force: bool,
}

#[derive(Debug, Clone)]
pub struct CheckpointArgs {
    pub change: String,
    pub no_check: bool,
}

#[derive(Debug, Clone)]
pub struct DiffArgs {
    pub change: String,
    pub export_patch: Option<PathBuf>,
}

fn change_dir(root: &Path, change_id: &str) -> PathBuf {
    root.join(LLMANSPEC_DIR_NAME)
        .join("changes")
        .join(change_id)
}

fn proposal_path(root: &Path, change_id: &str) -> PathBuf {
    change_dir(root, change_id).join("proposal.md")
}

fn run_git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|err| anyhow!("git {:?} failed to spawn: {err}", args))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            bail!("git {:?} failed", args);
        }
        bail!("{stderr}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn current_branch(root: &Path) -> Result<String> {
    let branch = run_git(root, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    if branch.is_empty() || branch == "HEAD" {
        bail!("detached HEAD is not allowed for BDD-on change binding");
    }
    Ok(branch)
}

pub fn current_head_sha(root: &Path) -> Result<String> {
    run_git(root, &["rev-parse", "HEAD"])
}

pub fn resolve_default_branch_ref(root: &Path) -> Result<String> {
    if let Ok(sym) = run_git(root, &["symbolic-ref", "refs/remotes/origin/HEAD"])
        && let Some(name) = sym.strip_prefix("refs/remotes/origin/")
    {
        let remote = format!("origin/{name}");
        if git_ref_exists(root, &remote) {
            return Ok(remote);
        }
        if git_ref_exists(root, name) {
            return Ok(name.to_string());
        }
    }
    for candidate in ["origin/main", "origin/master", "main", "master"] {
        if git_ref_exists(root, candidate) {
            return Ok(candidate.to_string());
        }
    }
    bail!("unable to resolve default branch (tried origin/main, origin/master, main, master)");
}

fn git_ref_exists(root: &Path, reference: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .current_dir(root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn is_default_branch(root: &Path, branch: &str) -> Result<bool> {
    let default_ref = resolve_default_branch_ref(root)?;
    let default_name = default_ref
        .strip_prefix("origin/")
        .unwrap_or(default_ref.as_str());
    Ok(branch == default_name || branch == default_ref)
}

pub fn working_tree_clean(root: &Path) -> Result<bool> {
    let status = run_git(root, &["status", "--porcelain"])?;
    Ok(status.trim().is_empty())
}

pub fn merge_base_sha(root: &Path, base_ref: &str) -> Result<String> {
    run_git(root, &["merge-base", base_ref, "HEAD"])
}

pub fn branch_diff(root: &Path, base_sha: &str) -> Result<String> {
    run_git(
        root,
        &["diff", "--find-renames", &format!("{base_sha}...HEAD")],
    )
}

pub fn branch_has_upstream(root: &Path) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"])
        .current_dir(root)
        .output()
        .map_err(|err| anyhow!("git upstream check failed: {err}"))?;
    Ok(output.status.success())
}

/// Optional shared-mode gate from `bdd.shared` / future config.
/// For now: only enforced when `LLMAN_SDD_REQUIRE_UPSTREAM=1`.
pub fn shared_mode_required() -> bool {
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
pub fn read_binding(root: &Path, change_id: &str) -> Result<Option<ChangeGitBinding>> {
    let path = proposal_path(root, change_id);
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
    let path = proposal_path(root, change_id);
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
pub fn run_attach(root: &Path, args: AttachArgs) -> Result<()> {
    let change_name = crate::sdd::shared::discovery::resolve_change_id(root, &args.change)?;
    validate_sdd_id(&change_name, "change")?;
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec)?;
    if config.bdd.is_none() {
        bail!("`sdd change attach` requires BDD-on (`bdd:` in config.yaml)");
    }
    let dir = change_dir(root, &change_name);
    if !dir.exists() {
        bail!("change `{}` not found", change_name);
    }
    if !proposal_path(root, &change_name).exists() {
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
            "BDD-on changes must not attach on the default branch (`{branch}`); create/switch to a feature branch first"
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

/// Require a clean tree, matching branch binding, and (optionally) full BDD check.
pub fn run_checkpoint(root: &Path, args: CheckpointArgs) -> Result<()> {
    let change_name = crate::sdd::shared::discovery::resolve_change_id(root, &args.change)?;
    validate_sdd_id(&change_name, "change")?;
    let llmanspec = root.join(LLMANSPEC_DIR_NAME);
    let config = load_required_config(&llmanspec)?;
    if config.bdd.is_none() {
        bail!("`sdd change checkpoint` requires BDD-on (`bdd:` in config.yaml)");
    }

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
    if !working_tree_clean(root)? {
        bail!("working tree is dirty; commit all changes before checkpoint");
    }

    if shared_mode_required() && !branch_has_upstream(root)? {
        bail!(
            "shared mode requires an upstream (set LLMAN_SDD_REQUIRE_UPSTREAM=0 to skip, or `git push -u`)"
        );
    }

    // Fast + optional full validation of the live branch tree.
    crate::sdd::shared::validate::run(
        root,
        crate::sdd::shared::validate::ValidateArgs {
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
    crate::sdd::shared::validate::run(
        root,
        crate::sdd::shared::validate::ValidateArgs {
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

pub fn run_diff(root: &Path, args: DiffArgs) -> Result<()> {
    let change_name = crate::sdd::shared::discovery::resolve_change_id(root, &args.change)?;
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

/// Enforce BDD-on archive preconditions: attached, checkpointed, clean, on branch.
///
/// This is the strict variant used by `change archive` — it requires a clean
/// working tree (because archive itself does not write the checkpoint frontmatter,
/// so a clean tree guarantees `checkpoint_sha` still points to a real commit).
/// For the `finalize` path (which writes the frontmatter itself and intentionally
/// leaves the tree dirty for a single commit), use
/// [`enforce_bdd_archive_gates_relaxed`] instead.
pub fn enforce_bdd_archive_gates(root: &Path, change_id: &str) -> Result<ChangeGitBinding> {
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
pub fn enforce_bdd_archive_gates_relaxed(root: &Path, change_id: &str) -> Result<ChangeGitBinding> {
    let Some(binding) = read_binding(root, change_id)? else {
        bail!(
            "BDD-on archive requires Git binding; run `llman sdd change attach {change_id}` then checkpoint"
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
        bail!("BDD-on archive must not run on the default branch");
    }
    if shared_mode_required() && !branch_has_upstream(root)? {
        bail!("shared mode requires an upstream before archive");
    }
    // Reject leftover feature_delta files (legacy model).
    let change_specs = change_dir(root, change_id).join("specs");
    if change_specs.exists() {
        for entry in fs::read_dir(&change_specs)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            for file in fs::read_dir(entry.path())? {
                let file = file?;
                let name = file.file_name().to_string_lossy().to_string();
                if name.ends_with(".feature.delta.toon") || name == "feature.delta.toon" {
                    bail!(
                        "legacy feature_delta found at {}; migrate to branch-local .feature files before archive",
                        file.path().display()
                    );
                }
            }
        }
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
            "BDD-on archive requires Git binding; run `llman sdd change attach {change_id}` then checkpoint"
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
        bail!("BDD-on archive must not run on the default branch");
    }
    if require_clean_tree && !working_tree_clean(root)? {
        bail!("working tree must be clean before BDD-on archive");
    }
    if !binding.checkpointed {
        bail!(
            "change `{change_id}` is not checkpointed; run `llman sdd change checkpoint {change_id}`"
        );
    }
    if shared_mode_required() && !branch_has_upstream(root)? {
        bail!("shared mode requires an upstream before archive");
    }
    // Reject leftover feature_delta files (legacy model).
    let change_specs = change_dir(root, change_id).join("specs");
    if change_specs.exists() {
        for entry in fs::read_dir(&change_specs)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            for file in fs::read_dir(entry.path())? {
                let file = file?;
                let name = file.file_name().to_string_lossy().to_string();
                if name.ends_with(".feature.delta.toon") || name == "feature.delta.toon" {
                    bail!(
                        "legacy feature_delta found at {}; migrate to branch-local .feature files before archive",
                        file.path().display()
                    );
                }
            }
        }
    }
    Ok(binding)
}

#[cfg(test)]
mod tests {
    use super::*;
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

//! `llman tool agents-md` — manage agent init files (AGENTS.md / CLAUDE.md / .cursor/ etc.).
//!
//! Three subcommands form a loop: `scan` (discover + register) → `clean`
//! (remove, with default-branch guard) → `revert` (restore from default branch).
//! The manifest of recorded paths lives in project `.llman/config.yaml` under
//! `tools.agents-md.files`. Directory entries (e.g. `.cursor/`) are expanded to
//! their git-tracked files at clean/revert time.

use crate::fs_utils::atomic_write_with_mode;
use crate::path_utils::{relative_path_from_dir, safe_parent_for_creation};
use crate::sdd::change::git_native::{
    self, current_branch, is_default_branch, resolve_default_branch_ref,
};
use crate::skills::shared::git::find_git_root;
use crate::tool::command::{AgentsMdCleanArgs, AgentsMdRevertArgs, AgentsMdScanArgs};
use crate::tool::config::{ToolConfig, default_agent_init_names};
use anyhow::{Context, Result, anyhow, bail};
use rust_i18n::t;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Commit message used by `clean --commit`.
const CLEAN_COMMIT_MSG: &str = "chore(agents-md): clean stale agent init files";
/// Commit message used by `revert --commit`.
const REVERT_COMMIT_MSG: &str = "chore(agents-md): restore agent init files from default branch";

// ---------------------------------------------------------------------------
// scan
// ---------------------------------------------------------------------------

pub fn run_scan(args: &AgentsMdScanArgs) -> Result<()> {
    println!("{}", t!("tool.agents_md.scan.start"));

    let cwd = std::env::current_dir().context("get current directory")?;
    let root = resolve_project_root(&cwd)?;

    // Scan name list source: global config override of the built-in default.
    let names = resolve_scan_names(args.config.as_deref())?;
    if args.verbose {
        println!(
            "{}",
            t!("tool.agents_md.scan.names", names = names.join(", "))
        );
    }

    let discovered = discover_paths(&root, &names)?;
    println!("{}", t!("tool.agents_md.scan.found_title"));
    if discovered.is_empty() {
        println!("  - {}", t!("tool.agents_md.scan.none"));
    } else {
        for rel in &discovered {
            println!("  - {}", rel.display());
        }
    }

    if args.upsert_project_configs {
        let config_path = ensure_project_config_path(&root)?;
        upsert_manifest(&config_path, &discovered, &root)?;
        println!(
            "{}",
            t!(
                "tool.agents_md.scan.upsert_done",
                path = config_path.display()
            )
        );
    }

    Ok(())
}

/// Resolve the names to scan for: global config `tools.agents-md` overrides the
/// built-in default (replacement, not union).
fn resolve_scan_names(explicit_config: Option<&Path>) -> Result<Vec<String>> {
    // Use load_with_priority_or_default so an explicit config or local project
    // config is honored; global config is consulted for the override.
    let global = global_tool_config()?;
    if let Some(agents) = global
        .as_ref()
        .and_then(|c| c.get_agents_md_config())
        .filter(|c| !c.files.is_empty())
    {
        return Ok(agents.files.clone());
    }
    let _ = explicit_config; // local/project config does not redefine scan names
    Ok(default_agent_init_names())
}

/// Load the global config (best-effort). Returns None if absent or invalid.
fn global_tool_config() -> Result<Option<ToolConfig>> {
    let config_dir =
        crate::config::resolve_config_dir(None).context("resolve global config dir")?;
    let path = config_dir.join("config.yaml");
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(ToolConfig::load(&path)?))
}

/// Recursively discover paths under `root` matching the given names.
/// Returns paths relative to `root`, sorted. A name may be a file or directory.
fn discover_paths(root: &Path, names: &[String]) -> Result<Vec<PathBuf>> {
    let name_set: BTreeSet<String> = names.iter().cloned().collect();
    let mut found: BTreeSet<PathBuf> = BTreeSet::new();

    for entry in walkdir(root)? {
        let file_name = match entry.file_name().and_then(|n| n.to_str()) {
            Some(s) => s,
            None => continue,
        };
        if name_set.contains(file_name)
            && let Some(rel) = relative_path_from_dir(root, &entry)
        {
            found.insert(rel);
        }
    }

    Ok(found.into_iter().collect())
}

/// Minimal recursive directory walk honoring `.git` skip. Returns all entries
/// (files and dirs) under `root`.
fn walkdir(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let read_dir = match fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            let is_dir = entry
                .file_type()
                .map(|t| t.is_dir())
                .unwrap_or(path.is_dir());
            out.push(path.clone());
            if is_dir {
                stack.push(path);
            }
        }
    }
    Ok(out)
}

/// Project `.llman/config.yaml` path, creating the `.llman` dir if needed.
fn ensure_project_config_path(root: &Path) -> Result<PathBuf> {
    let dir = root.join(".llman");
    fs::create_dir_all(&dir)
        .with_context(|| t!("tool.agents_md.error.create_dir", path = dir.display()).to_string())?;
    Ok(dir.join("config.yaml"))
}

/// Write/merge the discovered manifest into project config `tools.agents-md.files`.
/// Preserves all other existing keys by operating on the raw YAML value.
fn upsert_manifest(config_path: &Path, discovered: &[PathBuf], root: &Path) -> Result<()> {
    let existing_content = if config_path.exists() {
        fs::read_to_string(config_path).ok()
    } else {
        None
    };

    let mut doc: serde_yaml::Value = match &existing_content {
        Some(text) => serde_yaml::from_str(text)
            .unwrap_or_else(|_| serde_yaml::Value::Mapping(Default::default())),
        None => serde_yaml::Value::Mapping({
            let mut m = serde_yaml::Mapping::new();
            m.insert("version".into(), "0.1".into());
            m
        }),
    };

    let rel_strings: Vec<serde_yaml::Value> = discovered
        .iter()
        .map(|p| {
            serde_yaml::Value::String(
                p.to_str()
                    .map(|s| s.trim_end_matches('/').to_string())
                    .unwrap_or_default(),
            )
        })
        .collect();

    ensure_child_mapping(&mut doc, "tools");
    let tools_val = doc.get_mut("tools").expect("tools present");
    ensure_child_mapping(tools_val, "agents-md");
    let agents_md = tools_val
        .get_mut("agents-md")
        .and_then(|v| v.as_mapping_mut())
        .expect("agents-md is mapping");
    agents_md.insert("files".into(), serde_yaml::Value::Sequence(rel_strings));

    let yaml = serde_yaml::to_string(&doc)
        .map_err(|e| anyhow!(t!("tool.agents_md.error.serialize", error = e)))?;
    let mut content = yaml;
    if !content.ends_with('\n') {
        content.push('\n');
    }
    if let Some(parent) = safe_parent_for_creation(config_path) {
        fs::create_dir_all(parent).with_context(|| {
            t!("tool.agents_md.error.create_dir", path = parent.display()).to_string()
        })?;
    }
    atomic_write_with_mode(config_path, content.as_bytes(), None).with_context(|| {
        t!(
            "tool.agents_md.error.write_failed",
            path = config_path.display()
        )
        .to_string()
    })?;
    let _ = root;
    Ok(())
}

/// Ensure `parent[key]` exists and is a mapping (creating it if needed).
fn ensure_child_mapping(parent: &mut serde_yaml::Value, key: &str) {
    if !parent.is_mapping() {
        *parent = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
    }
    let map = parent.as_mapping_mut().expect("parent is mapping");
    if !map.contains_key(key) {
        map.insert(
            serde_yaml::Value::String(key.to_string()),
            serde_yaml::Value::Mapping(serde_yaml::Mapping::new()),
        );
    }
}

// ---------------------------------------------------------------------------
// clean
// ---------------------------------------------------------------------------

pub fn run_clean(args: &AgentsMdCleanArgs) -> Result<()> {
    println!("{}", t!("tool.agents_md.clean.start"));

    let cwd = std::env::current_dir().context("get current directory")?;
    let root = resolve_project_root(&cwd)?;

    let manifest = load_manifest(args.config.as_deref(), &root)?;
    if manifest.is_empty() {
        println!("{}", t!("tool.agents_md.clean.empty_manifest"));
        return Ok(());
    }
    if args.verbose {
        println!(
            "{}",
            t!("tool.agents_md.clean.manifest", paths = manifest.join(", "))
        );
    }

    let targets = expand_to_tracked_files(&root, &manifest)?;
    if targets.is_empty() {
        println!("{}", t!("tool.agents_md.clean.no_targets"));
        return Ok(());
    }

    print_clean_preview(&root, &targets);

    // Default-branch guard for --commit.
    if args.commit {
        let branch = current_branch(&root)?;
        if is_default_branch(&root, &branch)? && !args.force {
            bail!(
                "{}",
                t!("tool.agents_md.clean.error.default_branch", branch = branch)
            );
        }
    }

    if !args.yes && !args.commit {
        println!("{}", t!("tool.agents_md.clean.dry_run_hint"));
        return Ok(());
    }

    for file in &targets {
        if file.exists() {
            fs::remove_file(file).with_context(|| {
                t!("tool.agents_md.error.delete_failed", path = file.display()).to_string()
            })?;
        }
    }
    println!(
        "{}",
        t!("tool.agents_md.clean.deleted_count", count = targets.len())
    );

    if args.commit {
        git_commit_files(&root, &targets, CLEAN_COMMIT_MSG)?;
        println!("{}", t!("tool.agents_md.clean.committed"));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// revert
// ---------------------------------------------------------------------------

pub fn run_revert(args: &AgentsMdRevertArgs) -> Result<()> {
    println!("{}", t!("tool.agents_md.revert.start"));

    let cwd = std::env::current_dir().context("get current directory")?;
    let root = resolve_project_root(&cwd)?;

    let manifest = load_manifest(args.config.as_deref(), &root)?;
    if manifest.is_empty() {
        println!("{}", t!("tool.agents_md.revert.empty_manifest"));
        return Ok(());
    }

    let default_ref = resolve_default_branch_ref(&root)?;
    let targets = expand_to_files_for_checkout(&root, &manifest, &default_ref)?;
    if targets.is_empty() {
        println!("{}", t!("tool.agents_md.revert.no_targets"));
        return Ok(());
    }

    if args.verbose {
        println!(
            "{}",
            t!("tool.agents_md.revert.from_ref", r#ref = default_ref)
        );
    }
    print_revert_preview(&root, &targets, &default_ref);

    // For --commit on the default branch, create a recovery branch first.
    let mut created_branch: Option<String> = None;
    if args.commit {
        let branch = current_branch(&root)?;
        if is_default_branch(&root, &branch)? {
            let new_branch = format!("agents-md/revert-{}", timestamp_suffix());
            git_native::run_git(&root, &["checkout", "-b", &new_branch])?;
            created_branch = Some(new_branch);
        }
    } else if !args.yes {
        println!("{}", t!("tool.agents_md.revert.dry_run_hint"));
        return Ok(());
    }

    for file in &targets {
        let rel = relative_path_from_dir(&root, file).unwrap_or_else(|| file.clone());
        if let Err(e) = git_native::run_git(
            &root,
            &["checkout", &default_ref, "--", &rel.to_string_lossy()],
        ) {
            // A manifest path absent on the default branch is skipped (warn).
            eprintln!(
                "{}",
                t!(
                    "tool.agents_md.revert.skip_missing",
                    path = rel.display(),
                    error = e
                )
            );
        }
    }
    println!(
        "{}",
        t!(
            "tool.agents_md.revert.restored_count",
            count = targets.len()
        )
    );

    if args.commit {
        let abs: Vec<PathBuf> = targets.iter().map(|rel| root.join(rel)).collect();
        git_commit_files(&root, &abs, REVERT_COMMIT_MSG)?;
        println!("{}", t!("tool.agents_md.revert.committed"));
        if let Some(b) = created_branch {
            println!("{}", t!("tool.agents_md.revert.branch_created", branch = b));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

fn resolve_project_root(cwd: &Path) -> Result<PathBuf> {
    find_git_root(cwd).ok_or_else(|| anyhow!(t!("tool.agents_md.error.no_git_repo").to_string()))
}

/// Load the manifest (recorded paths) from project `.llman/config.yaml`.
fn load_manifest(explicit_config: Option<&Path>, root: &Path) -> Result<Vec<String>> {
    let path = explicit_config
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| root.join(".llman").join("config.yaml"));
    if !path.exists() {
        return Ok(Vec::new());
    }
    let config = ToolConfig::load(&path)?;
    Ok(config
        .get_agents_md_config()
        .map(|c| c.files.clone())
        .unwrap_or_default())
}

/// Expand manifest entries (which may be directories) into git-tracked file
/// paths relative to `root`. Uses `git ls-files` so .gitignore is honored.
fn expand_to_tracked_files(root: &Path, manifest: &[String]) -> Result<Vec<PathBuf>> {
    let tracked = git_ls_files(root, manifest)?;
    Ok(tracked)
}

/// For revert we need the paths as they exist on the *default branch* (the
/// files may already be deleted from the working tree by a prior `clean`).
/// We list the default branch tree with `git ls-tree`, then keep the manifest
/// entries (files used directly; directories expanded to their contained files).
fn expand_to_files_for_checkout(
    root: &Path,
    manifest: &[String],
    default_ref: &str,
) -> Result<Vec<PathBuf>> {
    let tree_files = git_ls_tree(root, default_ref)?;
    let mut out = Vec::new();
    for entry in manifest {
        let trimmed = entry.trim_end_matches('/');
        let entry_path = Path::new(trimmed);
        let parent_name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(trimmed);
        // If the manifest entry is a file name present in the default tree, use it.
        if tree_files.iter().any(|p| p == entry_path) {
            out.push(PathBuf::from(trimmed));
            continue;
        }
        // Otherwise treat as a directory prefix: include all tree files under it.
        let prefix = format!("{}/", trimmed.trim_start_matches("./"));
        for tf in &tree_files {
            if let Some(s) = tf.to_str()
                && (s.starts_with(&prefix) || tf.to_string_lossy() == *parent_name)
            {
                out.push(tf.clone());
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// `git ls-tree -r --name-only <ref>` → relative paths on that ref.
fn git_ls_tree(root: &Path, reference: &str) -> Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["ls-tree", "-r", "--name-only", reference])
        .current_dir(root)
        .output()
        .map_err(|e| anyhow!("git ls-tree failed to spawn: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(if stderr.is_empty() {
            format!("git ls-tree {reference} failed")
        } else {
            stderr
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect())
}

/// Run `git ls-files -- <entries>` and return the relative tracked paths.
fn git_ls_files(root: &Path, manifest: &[String]) -> Result<Vec<PathBuf>> {
    let mut cmd = Command::new("git");
    cmd.args(["ls-files", "-z", "--"]).current_dir(root);
    for entry in manifest {
        cmd.arg(entry);
    }
    let output = cmd
        .output()
        .map_err(|e| anyhow!("git ls-files failed to spawn: {e}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(if stderr.is_empty() {
            "git ls-files failed".to_string()
        } else {
            stderr
        });
    }
    let mut files = Vec::new();
    for entry in output.stdout.split(|b| *b == 0) {
        if entry.is_empty() {
            continue;
        }
        let rel = PathBuf::from(String::from_utf8_lossy(entry).to_string());
        files.push(rel);
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// `git add <files>` + single commit on the current branch.
fn git_commit_files(root: &Path, files: &[PathBuf], message: &str) -> Result<()> {
    if files.is_empty() {
        return Ok(());
    }
    let mut add = Command::new("git");
    add.args(["add", "--"]).current_dir(root);
    for f in files {
        let rel = relative_path_from_dir(root, f).unwrap_or_else(|| f.clone());
        add.arg(rel);
    }
    let add_out = add
        .output()
        .map_err(|e| anyhow!("git add failed to spawn: {e}"))?;
    if !add_out.status.success() {
        bail!(
            "git add failed: {}",
            String::from_utf8_lossy(&add_out.stderr).trim()
        );
    }
    // Skip commit entirely when nothing is staged (revert may produce no diff
    // if the file already matches the default branch — restore goal still met).
    let diff_cached = Command::new("git")
        .args(["diff", "--cached", "--quiet"])
        .current_dir(root)
        .status()
        .map_err(|e| anyhow!("git diff --cached failed to spawn: {e}"))?;
    // exit 0 = no staged changes; skip commit instead of failing.
    if diff_cached.success() {
        return Ok(());
    }
    git_native::run_git(root, &["commit", "-m", message])?;
    Ok(())
}

fn print_clean_preview(root: &Path, targets: &[PathBuf]) {
    use comfy_table::{ContentArrangement, Table};
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        t!("tool.agents_md.preview.header.path").to_string(),
        t!("tool.agents_md.preview.header.exists").to_string(),
    ]);
    for rel in targets {
        let abs = root.join(rel);
        let exists = if abs.exists() {
            t!("tool.agents_md.preview.yes").to_string()
        } else {
            t!("tool.agents_md.preview.no").to_string()
        };
        table.add_row(vec![rel.display().to_string(), exists]);
    }
    println!("{}", t!("tool.agents_md.clean.preview_title"));
    println!("{table}");
}

fn print_revert_preview(root: &Path, targets: &[PathBuf], default_ref: &str) {
    use comfy_table::{ContentArrangement, Table};
    let mut table = Table::new();
    table.load_preset(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.set_header(vec![
        t!("tool.agents_md.preview.header.path").to_string(),
        t!("tool.agents_md.preview.header.ref").to_string(),
    ]);
    for rel in targets {
        let _ = root;
        table.add_row(vec![rel.display().to_string(), default_ref.to_string()]);
    }
    println!("{}", t!("tool.agents_md.revert.preview_title"));
    println!("{table}");
}

fn timestamp_suffix() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_scan_names_nonempty() {
        let names = default_agent_init_names();
        assert!(!names.is_empty());
    }

    #[test]
    fn test_ensure_child_mapping_creates_nested() {
        let mut doc = serde_yaml::Value::Mapping(serde_yaml::Mapping::new());
        ensure_child_mapping(&mut doc, "tools");
        let tools_val = doc.get_mut("tools").unwrap();
        ensure_child_mapping(tools_val, "agents-md");
        assert!(doc.get("tools").unwrap().is_mapping());
        assert!(
            doc.get("tools")
                .unwrap()
                .get("agents-md")
                .unwrap()
                .is_mapping()
        );
    }
}

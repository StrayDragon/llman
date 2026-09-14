//! External subcommand discovery: `llman <name>` resolves `llman-<name>` on
//! `PATH` (git-style plugins; cli.feature r56).
//!
//! The contract is language-agnostic — any executable file (ELF binary,
//! shebang script, npm sh shim) qualifies on Unix: regular file with any
//! execute bit. Directory order wins (first PATH hit), matching `which`.
//!
//! The `*_in` variants take explicit directories and are the pure,
//! env-free test surface; the public functions read the real `PATH`.

use std::fs;
use std::path::{Path, PathBuf};

/// Executable-name prefix shared by the hub (`llman`) and its plugins.
const PLUGIN_PREFIX: &str = "llman-";

/// Resolve executable `llman-<name>` across `PATH`; first hit wins.
pub fn resolve(name: &str) -> Option<PathBuf> {
    resolve_in(&path_dirs(), name)
}

/// Discover all `llman-*` command names on `PATH` (deduped, sorted).
pub fn discover() -> Vec<String> {
    discover_in(&path_dirs())
}

fn path_dirs() -> Vec<PathBuf> {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default()
}

fn resolve_in(dirs: &[PathBuf], name: &str) -> Option<PathBuf> {
    if name.is_empty() {
        return None;
    }
    let file_name = format!("{PLUGIN_PREFIX}{name}");
    dirs.iter().find_map(|dir| {
        let candidate = dir.join(&file_name);
        is_executable_file(&candidate).then_some(candidate)
    })
}

fn discover_in(dirs: &[PathBuf]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_name) = entry.file_name().into_string() else {
                continue;
            };
            let Some(name) = file_name.strip_prefix(PLUGIN_PREFIX) else {
                continue;
            };
            if name.is_empty() || !is_executable_file(&entry.path()) {
                continue;
            }
            if !names.iter().any(|seen| seen == name) {
                names.push(name.to_string());
            }
        }
    }
    names.sort();
    names
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    meta.is_file() && meta.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    // Windows: PATHEXT-aware resolution (.exe/.cmd/.bat/.ps1) is a reserved
    // follow-up (cli.feature r56 targets Unix semantics only for now).
    let _ = path;
    false
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn write_exec(dir: &Path, file_name: &str) -> PathBuf {
        let path = dir.join(file_name);
        fs::write(&path, "#!/bin/sh\nexit 0\n").expect("write plugin");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        path
    }

    fn write_plain(dir: &Path, file_name: &str) -> PathBuf {
        let path = dir.join(file_name);
        fs::write(&path, "not executable").expect("write file");
        path
    }

    #[test]
    fn resolve_finds_executable_in_first_dir() {
        let first = TempDir::new().unwrap();
        let second = TempDir::new().unwrap();
        let expected = write_exec(first.path(), "llman-foo");
        write_exec(second.path(), "llman-foo");
        let dirs = vec![first.path().to_path_buf(), second.path().to_path_buf()];
        assert_eq!(resolve_in(&dirs, "foo"), Some(expected));
    }

    #[test]
    fn resolve_ignores_non_executable_and_missing() {
        let dir = TempDir::new().unwrap();
        write_plain(dir.path(), "llman-bar");
        let dirs = vec![dir.path().to_path_buf()];
        assert_eq!(resolve_in(&dirs, "bar"), None);
        assert_eq!(resolve_in(&dirs, "nope"), None);
        assert_eq!(resolve_in(&dirs, ""), None);
    }

    #[test]
    fn resolve_ignores_directories_named_like_plugins() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("llman-dir")).unwrap();
        let dirs = vec![dir.path().to_path_buf()];
        assert_eq!(resolve_in(&dirs, "dir"), None);
    }

    #[test]
    fn discover_collects_sorted_deduped_plugin_names() {
        let first = TempDir::new().unwrap();
        let second = TempDir::new().unwrap();
        write_exec(first.path(), "llman-beta");
        write_exec(first.path(), "llman-alpha");
        write_plain(first.path(), "llman-muted");
        write_exec(first.path(), "unrelated");
        write_exec(second.path(), "llman-alpha");
        write_exec(second.path(), "llman-gamma");
        let dirs = vec![first.path().to_path_buf(), second.path().to_path_buf()];
        assert_eq!(discover_in(&dirs), vec!["alpha", "beta", "gamma"]);
    }
}

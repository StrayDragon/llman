//! External subcommand discovery: `llman <name>` resolves `llman-<name>` on
//! `PATH` (git-style plugins; cli.feature r56).
//!
//! The contract is language-agnostic — any executable file (ELF binary,
//! shebang script, npm sh shim) qualifies on Unix: regular file with any
//! execute bit. Directory order wins (first PATH hit), matching `which`.
//!
//! Beyond `PATH`, well-known package-manager global-bin dirs (bun, pnpm,
//! npm-custom, user-local) are appended as a fallback so linked dev installs
//! (e.g. `bun link` shims under `~/.bun/bin`) resolve even when the dir is
//! absent from `PATH`. `PATH` hits keep precedence.
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
    let from_path: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).collect())
        .unwrap_or_default();
    let non_empty = |key: &str| -> Option<PathBuf> {
        std::env::var_os(key)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
    };
    let home = non_empty("HOME");
    let bun_install = non_empty("BUN_INSTALL");
    let pnpm_home = non_empty("PNPM_HOME");
    let extras = extra_bin_dirs(
        home.as_deref(),
        bun_install.as_deref(),
        pnpm_home.as_deref(),
    );
    merge_dirs(from_path, extras)
}

/// Well-known package-manager global-bin dirs appended after `PATH`; empty
/// without a home dir. `$BUN_INSTALL/bin` and `$PNPM_HOME` win over their
/// hardcoded defaults when set.
fn extra_bin_dirs(
    home: Option<&Path>,
    bun_install: Option<&Path>,
    pnpm_home: Option<&Path>,
) -> Vec<PathBuf> {
    let Some(home) = home else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = Vec::new();
    let mut push = |dir: PathBuf| {
        if !dirs.contains(&dir) {
            dirs.push(dir);
        }
    };
    if let Some(bun) = bun_install {
        push(bun.join("bin"));
    }
    push(home.join(".bun").join("bin"));
    if let Some(pnpm) = pnpm_home {
        push(pnpm.to_path_buf());
    }
    push(home.join(".local/share/pnpm"));
    push(home.join(".npm-global/bin"));
    push(home.join(".local/bin"));
    dirs
}

fn merge_dirs(mut base: Vec<PathBuf>, extra: Vec<PathBuf>) -> Vec<PathBuf> {
    for dir in extra {
        if !base.contains(&dir) {
            base.push(dir);
        }
    }
    base
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

    #[test]
    fn extra_bin_dirs_lists_canonical_package_manager_bins() {
        let home = Path::new("/home/u");
        assert_eq!(
            extra_bin_dirs(Some(home), None, None),
            vec![
                home.join(".bun/bin"),
                home.join(".local/share/pnpm"),
                home.join(".npm-global/bin"),
                home.join(".local/bin"),
            ]
        );
    }

    #[test]
    fn extra_bin_dirs_honors_bun_and_pnpm_env_overrides() {
        let home = Path::new("/home/u");
        let dirs = extra_bin_dirs(
            Some(home),
            Some(Path::new("/opt/bun")),
            Some(Path::new("/pnpm-home")),
        );
        assert_eq!(dirs.first(), Some(&PathBuf::from("/opt/bun/bin")));
        assert!(dirs.contains(&PathBuf::from("/pnpm-home")));
    }

    #[test]
    fn extra_bin_dirs_dedupes_bun_install_matching_default() {
        let home = Path::new("/home/u");
        let dirs = extra_bin_dirs(Some(home), Some(home), None);
        let bun_bin = home.join(".bun/bin");
        assert_eq!(dirs.iter().filter(|dir| **dir == bun_bin).count(), 1);
    }

    #[test]
    fn extra_bin_dirs_empty_without_home() {
        assert!(extra_bin_dirs(None, Some(Path::new("/opt/bun")), None).is_empty());
    }

    #[test]
    fn merge_dirs_appends_extras_after_base_without_duplicates() {
        let base = vec![PathBuf::from("/a"), PathBuf::from("/b")];
        let merged = merge_dirs(base, vec![PathBuf::from("/b"), PathBuf::from("/c")]);
        assert_eq!(
            merged,
            vec![
                PathBuf::from("/a"),
                PathBuf::from("/b"),
                PathBuf::from("/c")
            ]
        );
    }

    #[test]
    fn resolve_falls_back_to_bun_bin_when_absent_from_path_dirs() {
        let home = TempDir::new().unwrap();
        let bun_bin = home.path().join(".bun/bin");
        fs::create_dir_all(&bun_bin).unwrap();
        let expected = write_exec(&bun_bin, "llman-sdd");
        // Only an unrelated dir is on "PATH"; the plugin lives under <home>/.bun/bin.
        let path = vec![TempDir::new().unwrap().path().to_path_buf()];
        let dirs = merge_dirs(path, extra_bin_dirs(Some(home.path()), None, None));
        assert_eq!(resolve_in(&dirs, "sdd"), Some(expected));
    }
}

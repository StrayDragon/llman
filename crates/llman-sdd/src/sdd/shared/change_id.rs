//! Machine-readable change id convention helpers (sdd-workflow r29).
//!
//! Covers the two scopes the spec keeps separate:
//! - `compile_change_id_pattern`: full-match regex gate over **active** ids only.
//! - number harvesting for `llman_sdd_unique_id`: a **whole-tree** recursive scan
//!   of `llmanspec/` (missing a nested dir means re-issuing a live number).

use anyhow::{Result, anyhow};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

/// Compile a user-configured pattern into a full-match regex.
///
/// The user pattern is wrapped as `^(?:...)$` so top-level alternation keeps
/// working and matching is anchored at both ends regardless of user anchors.
pub(crate) fn compile_change_id_pattern(pattern: &str) -> Result<Regex> {
    let anchored = format!("^(?:{pattern})$");
    Regex::new(&anchored).map_err(|err| anyhow!("regex compile failed: {err}"))
}

/// Extract the unique-group extractor from a user pattern: only patterns that
/// carry a named `(?P<unique>[0-9]+)` group drive number extraction; anything
/// else (no pattern, or pattern without the group) falls back to the built-in
/// `c<digits>` heuristic (design D2 of add-change-id-convention-config).
pub(crate) fn unique_group_regex(pattern: Option<&str>) -> Result<Option<Regex>> {
    let Some(pattern) = pattern else {
        return Ok(None);
    };
    if !pattern.contains("(?P<unique>") {
        return Ok(None);
    }
    compile_change_id_pattern(pattern).map(Some)
}

/// Extract the number carried by a change-id-shaped directory name.
///
/// When the project configures `change_id.pattern` with a named capture group
/// `(?P<unique>[0-9]+)`, that group wins; otherwise a built-in heuristic picks
/// the first `c<digits>` run at a token boundary (matches `c2790-…` and the
/// `…-c20-…` date-prefixed archive shape).
pub(crate) fn extract_unique_number(name: &str, unique_group: Option<&Regex>) -> Option<u64> {
    if let Some(re) = unique_group {
        return re
            .captures(name)
            .and_then(|caps| caps.name("unique"))
            .and_then(|m| m.as_str().parse().ok());
    }
    let re = c_token_regex();
    re.captures(name)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

fn c_token_regex() -> &'static Regex {
    static RE: std::sync::OnceLock<Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        // 'c' not preceded/followed by [a-z0-9], then digits: c2790, -c20-, ^c10…
        // (rust `regex` has no look-around; the trailing class backtracks safely —
        // glued-letter names like `cab12` match no position.)
        Regex::new("(?i)(?:^|[^a-z0-9])c([0-9]+)(?:$|[^a-z0-9])").expect("static regex")
    })
}

/// Outcome of the whole-tree unique-number scan (`llman_sdd_unique_id` basis).
#[derive(Debug, Clone)]
pub(crate) struct IdHarvest {
    /// Highest number found anywhere under the tree (None = no numbered names).
    pub(crate) max_number: Option<u64>,
    /// Next free number: max + 1, or 1 when the tree carries no numbers.
    pub(crate) next_number: u64,
    /// Frozen archives that could not be inspected (tool missing / read failed).
    /// Callers MUST surface these as warnings so humans can cross-check manually.
    pub(crate) warnings: Vec<String>,
}

/// Harvest change-id numbers from the whole `llmanspec/` tree, recursively.
///
/// Scans directory names at any depth (covers `changes/`, `changes/archive/`,
/// downstream dirs like `delayed-changes/…`). Skips symlinks (no cycles) and
/// dot-dirs. Frozen archives (`*.7z`, `*.zip`, `*.tar*`, `*.archived` shapes)
/// are inspected best-effort; unreadable ones become warnings, never errors.
pub(crate) fn harvest_unique_numbers(
    llmanspec_dir: &Path,
    pattern: Option<&Regex>,
) -> Result<IdHarvest> {
    let mut numbers = Vec::new();
    let mut warnings = Vec::new();
    walk_id_names(llmanspec_dir, pattern, &mut numbers, &mut warnings)?;
    let max_number = numbers.iter().copied().max();
    let next_number = max_number.map_or(1, |max| max + 1);
    Ok(IdHarvest {
        max_number,
        next_number,
        warnings,
    })
}

fn walk_id_names(
    dir: &Path,
    pattern: Option<&Regex>,
    numbers: &mut Vec<u64>,
    warnings: &mut Vec<String>,
) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            warnings.push(format!("cannot read {}: {err}", dir.display()));
            return Ok(());
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                warnings.push(format!("cannot list {}: {err}", dir.display()));
                continue;
            }
        };
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                warnings.push(format!("cannot stat {}: {err}", path.display()));
                continue;
            }
        };
        if file_type.is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if file_type.is_dir() {
            if name.starts_with('.') {
                continue;
            }
            if let Some(n) = extract_unique_number(&name, pattern) {
                numbers.push(n);
            }
            walk_id_names(&path, pattern, numbers, warnings)?;
        } else if is_frozen_archive(&name) {
            inspect_frozen_archive(&path, &name, numbers, warnings);
        }
    }
    Ok(())
}

/// Frozen-archive shapes by loose suffix substring, so double extensions like
/// `freezed_changes.7z.archived` still hit their real container format.
fn is_frozen_archive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    [".7z", ".zip", ".tar", ".tgz", ".archived"]
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

fn inspect_frozen_archive(
    path: &Path,
    name: &str,
    numbers: &mut Vec<u64>,
    warnings: &mut Vec<String>,
) {
    let lower = name.to_ascii_lowercase();
    let listing = if lower.contains(".7z") || lower.contains(".archived") {
        list_with_7z(path)
    } else if lower.contains(".zip") {
        list_with(["unzip", "-l"], path)
    } else {
        list_with(["tar", "-tf"], path)
    };
    match listing {
        Ok(stdout) => {
            for line in stdout.lines() {
                if let Some(n) = extract_unique_number(line.trim(), None) {
                    numbers.push(n);
                }
            }
        }
        Err(err) => warnings.push(format!(
            "cannot inspect frozen archive {} ({err}); numbers inside still count as taken — verify manually",
            path.display()
        )),
    }
}

fn list_with<const N: usize>(argv: [&str; N], path: &Path) -> std::result::Result<String, String> {
    let program = argv[0];
    let output = Command::new(program)
        .args(&argv[1..])
        .arg(path)
        .output()
        .map_err(|err| format!("{program} unavailable: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "{program} exited with {}",
            output.status.code().unwrap_or(-1)
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn list_with_7z(path: &Path) -> std::result::Result<String, String> {
    list_with(["7z", "l", "-ba"], path).or_else(|_| list_with(["7za", "l", "-ba"], path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_wraps_user_pattern_for_full_match() {
        let re = compile_change_id_pattern("^c[0-9]+-(add|fix)-[a-z0-9-]+$").unwrap();
        assert!(re.is_match("c10-add-login"));
        assert!(!re.is_match("x c10-add-login"));
        // Top-level alternation must not be split by the anchors.
        let re = compile_change_id_pattern("a|b").unwrap();
        assert!(re.is_match("a"));
        assert!(re.is_match("b"));
        assert!(!re.is_match("ab"));
        assert!(!re.is_match("a c"));
    }

    #[test]
    fn compile_rejects_invalid_regex() {
        assert!(compile_change_id_pattern("c([0-9]+").is_err());
    }

    #[test]
    fn extract_heuristic_c_token() {
        assert_eq!(
            extract_unique_number("c2790-refactor-core", None),
            Some(2790)
        );
        assert_eq!(
            extract_unique_number("2026-09-13-c20-legacy", None),
            Some(20)
        );
        assert_eq!(extract_unique_number("fix-sdd-command-safety", None), None);
        // Letters glued to the number disqualify the token.
        assert_eq!(extract_unique_number("cab12-driver", None), None);
    }

    #[test]
    fn extract_prefers_named_unique_group() {
        let re = compile_change_id_pattern(r"^c(?P<unique>[0-9]+)-.*$").unwrap();
        assert_eq!(extract_unique_number("c42-fix-x", Some(&re)), Some(42));
        assert_eq!(extract_unique_number("nope", Some(&re)), None);
    }

    #[test]
    fn harvest_scans_whole_tree_including_deep_delayed_dirs() {
        // Issue #20 topology: active max < delayed/frozen max must not collide.
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        for rel in [
            "changes/c10-add-login",
            "changes/archive/2026-09-13-c20-legacy",
            "delayed-changes/tools/c2620-tool-x",
            "delayed-changes/tui/deeper/c9-y",
        ] {
            fs::create_dir_all(root.join(rel)).unwrap();
        }
        fs::create_dir_all(root.join("changes/grouped/no-number-dir")).unwrap();

        let harvest = harvest_unique_numbers(root, None).unwrap();
        assert_eq!(harvest.max_number, Some(2620));
        assert_eq!(harvest.next_number, 2621);
        assert!(harvest.warnings.is_empty());
    }

    #[test]
    fn harvest_empty_tree_starts_at_one() {
        let tmp = tempfile::TempDir::new().unwrap();
        let harvest = harvest_unique_numbers(tmp.path(), None).unwrap();
        assert_eq!(harvest.max_number, None);
        assert_eq!(harvest.next_number, 1);
    }

    #[test]
    fn harvest_skips_symlinks_and_dot_dirs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("changes/c1-a")).unwrap();
        fs::create_dir_all(root.join(".hidden/c99-b")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("changes/c1-a"), root.join("changes/c77-link"))
            .unwrap();

        let harvest = harvest_unique_numbers(root, None).unwrap();
        assert_eq!(harvest.max_number, Some(1));
    }

    #[test]
    fn harvest_warns_on_unreadable_frozen_archive() {
        let tmp = tempfile::TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("changes/archive")).unwrap();
        // Invalid container: any real/unavailable tool fails → warning path.
        fs::write(root.join("changes/archive/freezed.7z.archived"), b"junk").unwrap();

        let harvest = harvest_unique_numbers(root, None).unwrap();
        assert!(
            harvest
                .warnings
                .iter()
                .any(|warning| warning.contains("freezed.7z.archived")),
            "warnings: {:?}",
            harvest.warnings
        );
    }
}

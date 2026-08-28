//! Import-direction lock (seam S2, change `src-cleanup-pre-split` / T5).
//!
//! Freezes the module dependency direction table from the change design:
//!
//! | source                | forbidden `crate::` targets      |
//! |-----------------------|----------------------------------|
//! | `src/sdd/**`          | skills, tool, x                  |
//! | `src/skills/**`       | sdd, tool, x                     |
//! | `src/tool/**`         | sdd, skills, x                   |
//! | `src/x/**`            | sdd                              |
//! | top-level util layer  | sdd, skills, tool, x             |
//!
//! The utility layer lives in the `crates/llman-core` workspace member
//! (`fs_utils` / `path_utils` / `managed_block` / `env_safety` / `git_utils`
//! / `schema_utils`, since T11) and must stay dependency-free; it is the
//! seed of the future `llman-core` published crate.
//!
//! The facade layer (`cli`, `config`, `config_schema`, `self_command`,
//! `prompts`, `arg_utils`, `editor`, `error`, `main`, `lib`, `bin`) is
//! intentionally unasserted. `test_utils` is not asserted either.
//!
//! This test only reads source files (via `CARGO_MANIFEST_DIR`) and writes
//! nothing.

use std::fs;
use std::path::{Path, PathBuf};

const SRC_DIR: &str = "src";
const CORE_SRC_DIR: &str = "crates/llman-core/src";

/// Feature-module directories and the `crate::` first segments they forbid.
const FORBIDDEN_FOR_MODULE_DIRS: &[(&str, &[&str])] = &[
    ("sdd", &["skills", "tool", "x"]),
    ("skills", &["sdd", "tool", "x"]),
    ("tool", &["sdd", "skills", "x"]),
    ("x", &["sdd"]),
];

/// Top-level utility-layer files that must not reference any feature module.
const FORBIDDEN_FOR_ALL_MODULES: &[&str] = &["sdd", "skills", "tool", "x"];

const UTILITY_LAYER_FILES: &[&str] = &[
    "fs_utils.rs",
    "path_utils.rs",
    "managed_block.rs",
    "env_safety.rs",
    "git_utils.rs",
    "schema_utils.rs",
];

fn src_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(SRC_DIR)
}

fn core_src_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(CORE_SRC_DIR)
}

/// Recursively collect `.rs` files under `dir`.
fn rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match fs::read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

/// Extract the first path segment after every `crate::` occurrence,
/// ignoring line comments. Returns segments like `sdd`, `skills`, `config`.
fn crate_path_segments(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (line_no, line) in text.lines().enumerate() {
        let code = line.split("//").next().unwrap_or("");
        let mut cursor = 0;
        while let Some(pos) = code[cursor..].find("crate::") {
            let start = cursor + pos + "crate::".len();
            let ident_end = code[start..]
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .map_or(code.len(), |rel| start + rel);
            if ident_end > start {
                found.push((line_no + 1, code[start..ident_end].to_string()));
            }
            cursor = ident_end.max(start);
        }
    }
    found
}

/// All violations in `dir` (relative to `src/`) referencing forbidden segments.
fn direction_violations(dir: &str, forbidden: &[&str]) -> Vec<String> {
    let root = src_root().join(dir);
    let mut violations = Vec::new();
    for file in rs_files(&root) {
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(_) => continue,
        };
        for (line_no, segment) in crate_path_segments(&text) {
            if forbidden.contains(&segment.as_str()) {
                let rel = file
                    .strip_prefix(src_root())
                    .unwrap_or(&file)
                    .display()
                    .to_string();
                violations.push(format!(
                    "src/{rel}:{line_no}: `crate::{segment}` violates direction rule for `{dir}/`"
                ));
            }
        }
    }
    violations.sort();
    violations
}

#[test]
fn sdd_must_not_reference_sibling_feature_modules() {
    let violations = direction_violations("sdd", &["skills", "tool", "x"]);
    assert!(
        violations.is_empty(),
        "src/sdd/** must not depend on skills/tool/x:\n{}",
        violations.join("\n")
    );
}

#[test]
fn skills_must_not_reference_sibling_feature_modules() {
    let violations = direction_violations("skills", &["sdd", "tool", "x"]);
    assert!(
        violations.is_empty(),
        "src/skills/** must not depend on sdd/tool/x:\n{}",
        violations.join("\n")
    );
}

#[test]
fn tool_must_not_reference_sibling_feature_modules() {
    let violations = direction_violations("tool", &["sdd", "skills", "x"]);
    assert!(
        violations.is_empty(),
        "src/tool/** must not depend on sdd/skills/x (git merge made this hold, keep it):\n{}",
        violations.join("\n")
    );
}

#[test]
fn x_must_not_reference_sdd() {
    let violations = direction_violations("x", &["sdd"]);
    assert!(
        violations.is_empty(),
        "src/x/** must not depend on sdd:\n{}",
        violations.join("\n")
    );
}

#[test]
fn utility_layer_must_stay_dependency_free() {
    let mut violations = Vec::new();
    for file in UTILITY_LAYER_FILES {
        let path = core_src_root().join(file);
        let text = fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!(
                "utility-layer module {file} is missing under {CORE_SRC_DIR}; update the direction table if it was renamed"
            )
        });
        for (line_no, segment) in crate_path_segments(&text) {
            if FORBIDDEN_FOR_ALL_MODULES.contains(&segment.as_str()) {
                violations.push(format!(
                    "{CORE_SRC_DIR}/{file}:{line_no}: `crate::{segment}` violates the utility-layer rule"
                ));
            }
        }
    }
    assert!(
        violations.is_empty(),
        "top-level utility layer must not depend on feature modules:\n{}",
        violations.join("\n")
    );
}

/// Sanity guard: every rule in the table maps to a module that actually exists,
/// so a rename cannot silently void the direction lock.
#[test]
fn direction_table_covers_existing_modules() {
    let src = src_root();
    assert!(
        !FORBIDDEN_FOR_MODULE_DIRS.is_empty(),
        "direction table must not be emptied silently"
    );
    for (module, _) in FORBIDDEN_FOR_MODULE_DIRS {
        assert!(
            src.join(module).is_dir(),
            "src/{module} missing but referenced by the direction table"
        );
    }
}

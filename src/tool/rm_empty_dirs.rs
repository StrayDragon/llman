use crate::tool::command::RmUselessDirsArgs;
use crate::tool::config::{Config, DirListConfig, ListMode};
use anyhow::{Result, anyhow};
use ignore::Match;
use ignore::gitignore::Gitignore;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(args: &RmUselessDirsArgs) -> Result<()> {
    let target = match &args.path {
        Some(path) => path.clone(),
        None => std::env::current_dir()?,
    };

    if !target.exists() {
        return Err(anyhow!(t!(
            "tool.rm_empty_dirs.error.target_not_exist",
            path = target.display()
        )));
    }

    if !target.is_dir() {
        return Err(anyhow!(
            "{}",
            t!(
                "tool.rm_empty_dirs.error.target_not_dir",
                path = target.display()
            )
        ));
    }

    let config = Config::load_with_priority_or_default(args.config.as_deref())?;
    let rm_config = config.get_rm_useless_dirs_config();
    let protected_dirs = resolve_dir_names(
        DEFAULT_PROTECTED_DIRS,
        rm_config.and_then(|cfg| cfg.protected.as_ref()),
    );
    let useless_dirs = resolve_dir_names(
        DEFAULT_USELESS_DIRS,
        rm_config.and_then(|cfg| cfg.useless.as_ref()),
    );

    let dry_run = !args.yes;
    let gitignore_path = resolve_gitignore_path(&target, args)?;
    let gitignore = match gitignore_path.as_ref() {
        Some(path) => Some(load_gitignore(path)?),
        None => None,
    };

    if is_protected_target(&target, &protected_dirs) {
        eprintln!(
            "{}",
            t!(
                "tool.rm_empty_dirs.skipping_protected_path",
                path = target.display()
            )
        );
        return Ok(());
    }

    println!("{}", t!("tool.rm_empty_dirs.start_title"));
    println!(
        "{}",
        t!("tool.rm_empty_dirs.target_label", path = target.display())
    );
    if dry_run {
        println!("{}", t!("tool.rm_empty_dirs.dry_run_enabled"));
    } else {
        println!("{}", t!("tool.rm_empty_dirs.live_mode_enabled"));
    }
    if args.prune_ignored {
        println!("{}", t!("tool.rm_empty_dirs.prune_ignored_enabled"));
    }
    if args.verbose {
        if let Some(path) = gitignore_path.as_ref() {
            println!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.verbose_gitignore_using",
                    path = path.display()
                )
            );
        } else {
            println!("{}", t!("tool.rm_empty_dirs.verbose_gitignore_none"));
        }
    }

    let mut report = RemovalReport::default();
    let options = Options {
        dry_run,
        gitignore,
        prune_ignored: args.prune_ignored,
        verbose: args.verbose,
        protected_dirs,
        useless_dirs,
    };

    let root_empty = process_dir(&target, false, &options, &mut report)?;

    if root_empty {
        if dry_run {
            println!("{}", t!("tool.rm_empty_dirs.note_root_would_be_empty"));
        } else {
            println!("{}", t!("tool.rm_empty_dirs.note_root_empty"));
        }
    }

    println!("\n{}", t!("tool.rm_empty_dirs.summary_title"));
    println!(
        "{}",
        t!(
            "tool.rm_empty_dirs.summary_empty_dirs_found",
            count = report.useless_dirs_found
        )
    );
    if !dry_run {
        println!(
            "{}",
            t!(
                "tool.rm_empty_dirs.summary_empty_dirs_removed",
                count = report.targets.len()
            )
        );
        if !report.failed.is_empty() {
            println!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.summary_empty_dirs_failed",
                    count = report.failed.len()
                )
            );
        }
    }
    println!(
        "{}",
        t!(
            "tool.rm_empty_dirs.summary_dirs_scanned",
            count = report.dirs_scanned
        )
    );
    println!(
        "{}",
        t!(
            "tool.rm_empty_dirs.summary_files_scanned",
            count = report.files_scanned
        )
    );
    let ignored_entries = report.ignored_dirs + report.ignored_files;
    if ignored_entries == 0 {
        println!("{}", t!("tool.rm_empty_dirs.summary_ignored_entries_zero"));
    } else {
        println!(
            "{}",
            t!(
                "tool.rm_empty_dirs.summary_ignored_entries",
                total = ignored_entries,
                dirs = report.ignored_dirs,
                files = report.ignored_files
            )
        );
    }
    println!(
        "{}",
        t!("tool.rm_empty_dirs.summary_errors", count = report.errors)
    );

    if !report.targets.is_empty() {
        let label = if dry_run {
            t!("tool.rm_empty_dirs.list_to_remove")
        } else {
            t!("tool.rm_empty_dirs.list_removed")
        };
        println!("\n{}:", label);
        for dir in &report.targets {
            println!("  - {}", dir.display());
        }
    }

    if !report.failed.is_empty() {
        println!("\n{}:", t!("tool.rm_empty_dirs.list_failed_title"));
        for dir in &report.failed {
            println!("  - {}", dir.display());
        }
    }

    Ok(())
}

#[derive(Default)]
struct RemovalReport {
    targets: Vec<PathBuf>,
    failed: Vec<PathBuf>,
    errors: usize,
    useless_dirs_found: usize,
    dirs_scanned: usize,
    files_scanned: usize,
    ignored_dirs: usize,
    ignored_files: usize,
}

struct Options {
    dry_run: bool,
    gitignore: Option<Gitignore>,
    prune_ignored: bool,
    verbose: bool,
    protected_dirs: HashSet<String>,
    useless_dirs: HashSet<String>,
}

const DEFAULT_PROTECTED_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".bzr",
    ".idea",
    ".vscode",
    "node_modules",
    ".yarn",
    ".pnpm-store",
    ".pnpm",
    ".npm",
    ".cargo",
    ".venv",
    "venv",
    ".tox",
    ".nox",
    "__pypackages__",
    "target",
    "vendor",
];

const DEFAULT_USELESS_DIRS: &[&str] = &[
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".basedpyright",
    ".pytype",
    ".pyre",
    ".ty",
    ".ty_cache",
    ".ty-cache",
];

fn resolve_dir_names(defaults: &[&str], config: Option<&DirListConfig>) -> HashSet<String> {
    let mode = config.map(|cfg| cfg.mode).unwrap_or(ListMode::Extend);
    let mut names = HashSet::new();
    match mode {
        ListMode::Extend => {
            for name in defaults {
                names.insert((*name).to_string());
            }
            if let Some(cfg) = config {
                for name in &cfg.names {
                    names.insert(name.clone());
                }
            }
        }
        ListMode::Override => {
            if let Some(cfg) = config {
                for name in &cfg.names {
                    names.insert(name.clone());
                }
            }
        }
    }
    names
}

fn is_protected_target(target: &Path, protected_dirs: &HashSet<String>) -> bool {
    target.iter().any(|component| {
        component
            .to_str()
            .is_some_and(|name| protected_dirs.contains(name))
    })
}

fn process_dir(
    dir: &Path,
    parent_ignored: bool,
    options: &Options,
    report: &mut RemovalReport,
) -> Result<bool> {
    report.dirs_scanned += 1;
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            report.errors += 1;
            eprintln!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.error.read_dir_failed",
                    path = dir.display(),
                    error = err
                )
            );
            return Ok(false);
        }
    };

    let mut is_empty = true;
    let mut ignored_files = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                report.errors += 1;
                eprintln!(
                    "{}",
                    t!(
                        "tool.rm_empty_dirs.error.read_entry_failed",
                        path = dir.display(),
                        error = err
                    )
                );
                is_empty = false;
                continue;
            }
        };

        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                report.errors += 1;
                eprintln!(
                    "{}",
                    t!(
                        "tool.rm_empty_dirs.error.read_file_type_failed",
                        path = path.display(),
                        error = err
                    )
                );
                is_empty = false;
                continue;
            }
        };

        let is_dir = file_type.is_dir();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if is_dir && options.protected_dirs.contains(name_str.as_ref()) {
            if options.verbose {
                println!(
                    "{}",
                    t!(
                        "tool.rm_empty_dirs.skipping_protected_path",
                        path = path.display()
                    )
                );
            }
            is_empty = false;
            continue;
        }

        if is_dir && options.useless_dirs.contains(name_str.as_ref()) {
            report.useless_dirs_found += 1;
            if try_remove_useless_dir(&path, options, report) {
                report.targets.push(path.clone());
            } else {
                is_empty = false;
            }
            continue;
        }

        let match_result = match options.gitignore.as_ref() {
            Some(gitignore) => gitignore.matched(&path, is_dir),
            None => Match::None,
        };
        let ignored_by_match = matches!(match_result, Match::Ignore(_));
        let is_whitelisted = matches!(match_result, Match::Whitelist(_));
        let ignored = if parent_ignored {
            !is_whitelisted
        } else {
            ignored_by_match
        };
        if ignored {
            if is_dir {
                report.ignored_dirs += 1;
            } else {
                report.ignored_files += 1;
            }
            if !options.prune_ignored {
                if options.verbose {
                    println!(
                        "{}",
                        t!(
                            "tool.rm_empty_dirs.skipping_ignored_path",
                            path = path.display()
                        )
                    );
                }
                is_empty = false;
                continue;
            }

            if !is_dir {
                ignored_files.push(path);
                continue;
            }
        }

        if is_dir {
            let child_parent_ignored = if is_whitelisted {
                false
            } else {
                parent_ignored || ignored_by_match
            };
            let child_empty = process_dir(&path, child_parent_ignored, options, report)?;
            if child_empty {
                report.useless_dirs_found += 1;
                if try_remove_dir(&path, options, report) {
                    report.targets.push(path.clone());
                } else {
                    is_empty = false;
                }
            } else {
                is_empty = false;
            }
        } else {
            report.files_scanned += 1;
            is_empty = false;
        }
    }

    if options.prune_ignored && is_empty && !ignored_files.is_empty() {
        for file in ignored_files {
            if !try_remove_ignored_file(&file, options, report) {
                is_empty = false;
            }
        }
    }

    Ok(is_empty)
}

fn try_remove_dir(path: &Path, options: &Options, report: &mut RemovalReport) -> bool {
    if options.verbose {
        if options.dry_run {
            println!(
                "{}",
                t!("tool.rm_empty_dirs.would_remove_dir", path = path.display())
            );
        } else {
            println!(
                "{}",
                t!("tool.rm_empty_dirs.removing_dir", path = path.display())
            );
        }
    }

    if options.dry_run {
        return true;
    }

    match fs::remove_dir(path) {
        Ok(()) => true,
        Err(err) => {
            report.errors += 1;
            report.failed.push(path.to_path_buf());
            eprintln!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.error.remove_dir_failed",
                    path = path.display(),
                    error = err
                )
            );
            false
        }
    }
}

fn try_remove_useless_dir(path: &Path, options: &Options, report: &mut RemovalReport) -> bool {
    if contains_protected_dir(path, options, report) {
        return false;
    }

    if options.verbose {
        if options.dry_run {
            println!(
                "{}",
                t!("tool.rm_empty_dirs.would_remove_dir", path = path.display())
            );
        } else {
            println!(
                "{}",
                t!("tool.rm_empty_dirs.removing_dir", path = path.display())
            );
        }
    }

    if options.dry_run {
        return true;
    }

    match fs::remove_dir_all(path) {
        Ok(()) => true,
        Err(err) => {
            report.errors += 1;
            report.failed.push(path.to_path_buf());
            eprintln!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.error.remove_dir_failed",
                    path = path.display(),
                    error = err
                )
            );
            false
        }
    }
}

fn contains_protected_dir(path: &Path, options: &Options, report: &mut RemovalReport) -> bool {
    if options.protected_dirs.is_empty() {
        return false;
    }

    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(err) => {
            report.errors += 1;
            eprintln!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.error.read_dir_failed",
                    path = path.display(),
                    error = err
                )
            );
            return true;
        }
    };

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                report.errors += 1;
                eprintln!(
                    "{}",
                    t!(
                        "tool.rm_empty_dirs.error.read_entry_failed",
                        path = path.display(),
                        error = err
                    )
                );
                return true;
            }
        };

        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(err) => {
                report.errors += 1;
                eprintln!(
                    "{}",
                    t!(
                        "tool.rm_empty_dirs.error.read_file_type_failed",
                        path = entry.path().display(),
                        error = err
                    )
                );
                return true;
            }
        };

        if !file_type.is_dir() {
            continue;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if options.protected_dirs.contains(name_str.as_ref()) {
            return true;
        }

        if contains_protected_dir(&entry.path(), options, report) {
            return true;
        }
    }

    false
}

fn try_remove_ignored_file(path: &Path, options: &Options, report: &mut RemovalReport) -> bool {
    if options.verbose {
        if options.dry_run {
            println!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.would_remove_ignored_file",
                    path = path.display()
                )
            );
        } else {
            println!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.removing_ignored_file",
                    path = path.display()
                )
            );
        }
    }

    if options.dry_run {
        return true;
    }

    match fs::remove_file(path) {
        Ok(()) => true,
        Err(err) => {
            report.errors += 1;
            eprintln!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.error.remove_ignored_file_failed",
                    path = path.display(),
                    error = err
                )
            );
            false
        }
    }
}

fn resolve_gitignore_path(target: &Path, args: &RmUselessDirsArgs) -> Result<Option<PathBuf>> {
    if let Some(path) = &args.gitignore {
        if !path.exists() {
            return Err(anyhow!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.error.gitignore_not_exist",
                    path = path.display()
                )
            ));
        }
        if !path.is_file() {
            return Err(anyhow!(
                "{}",
                t!(
                    "tool.rm_empty_dirs.error.gitignore_not_file",
                    path = path.display()
                )
            ));
        }
        return Ok(Some(path.clone()));
    }

    let default_path = target.join(".gitignore");
    if default_path.is_file() {
        Ok(Some(default_path))
    } else {
        Ok(None)
    }
}

fn load_gitignore(path: &Path) -> Result<Gitignore> {
    let (gitignore, err) = Gitignore::new(path);
    if let Some(err) = err {
        eprintln!(
            "{}",
            t!(
                "tool.rm_empty_dirs.error.gitignore_parse_warning",
                path = path.display(),
                error = err
            )
        );
    }
    Ok(gitignore)
}

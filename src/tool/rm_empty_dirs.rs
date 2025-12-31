use crate::tool::command::RmEmptyDirsArgs;
use anyhow::{Result, anyhow};
use ignore::Match;
use ignore::gitignore::Gitignore;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(args: &RmEmptyDirsArgs) -> Result<()> {
    let target = match &args.path {
        Some(path) => path.clone(),
        None => std::env::current_dir()?,
    };

    if !target.exists() {
        return Err(anyhow!("Target path does not exist: {}", target.display()));
    }

    if !target.is_dir() {
        return Err(anyhow!(
            "Target path is not a directory: {}",
            target.display()
        ));
    }

    let dry_run = !args.yes;
    let gitignore_path = resolve_gitignore_path(args)?;
    let gitignore = match gitignore_path.as_ref() {
        Some(path) => Some(load_gitignore(path)?),
        None => None,
    };

    println!("Remove empty directories");
    println!("Target: {}", target.display());
    if dry_run {
        println!("Dry run mode enabled (use -y to delete).");
    } else {
        println!("Live mode enabled (empty directories will be removed).");
    }
    if args.verbose {
        if let Some(path) = gitignore_path.as_ref() {
            println!("Using .gitignore: {}", path.display());
        } else {
            println!("No .gitignore configured.");
        }
    }

    let mut report = RemovalReport::default();
    let options = Options {
        dry_run,
        gitignore,
        verbose: args.verbose,
    };

    let root_empty = process_dir(&target, &options, &mut report)?;

    if root_empty {
        if dry_run {
            println!(
                "Note: target directory would be empty after removal; root directory is not removed."
            );
        } else {
            println!("Note: target directory is empty; root directory is not removed.");
        }
    }

    println!("\n=== Summary ===");
    if dry_run {
        println!("Empty directories found: {}", report.targets.len());
    } else {
        println!("Empty directories removed: {}", report.targets.len());
    }
    println!("Errors: {}", report.errors);

    if !report.targets.is_empty() {
        println!("\nDirectories:");
        for dir in &report.targets {
            println!("  - {}", dir.display());
        }
    }

    Ok(())
}

#[derive(Default)]
struct RemovalReport {
    targets: Vec<PathBuf>,
    errors: usize,
}

struct Options {
    dry_run: bool,
    gitignore: Option<Gitignore>,
    verbose: bool,
}

fn process_dir(dir: &Path, options: &Options, report: &mut RemovalReport) -> Result<bool> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) => {
            report.errors += 1;
            eprintln!(
                "Warning: failed to read directory {}: {}",
                dir.display(),
                err
            );
            return Ok(false);
        }
    };

    let mut is_empty = true;

    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                report.errors += 1;
                eprintln!(
                    "Warning: failed to read entry in {}: {}",
                    dir.display(),
                    err
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
                    "Warning: failed to read file type for {}: {}",
                    path.display(),
                    err
                );
                is_empty = false;
                continue;
            }
        };

        let is_dir = file_type.is_dir();
        if is_ignored(&path, is_dir, options) {
            if options.verbose {
                println!("Skipping ignored path: {}", path.display());
            }
            is_empty = false;
            continue;
        }

        if is_dir {
            let child_empty = process_dir(&path, options, report)?;
            if child_empty {
                if try_remove_dir(&path, options, report) {
                    report.targets.push(path.clone());
                } else {
                    is_empty = false;
                }
            } else {
                is_empty = false;
            }
        } else {
            is_empty = false;
        }
    }

    Ok(is_empty)
}

fn try_remove_dir(path: &Path, options: &Options, report: &mut RemovalReport) -> bool {
    if options.verbose {
        if options.dry_run {
            println!("Would remove: {}", path.display());
        } else {
            println!("Removing: {}", path.display());
        }
    }

    if options.dry_run {
        return true;
    }

    match fs::remove_dir(path) {
        Ok(()) => true,
        Err(err) => {
            report.errors += 1;
            eprintln!("Warning: failed to remove {}: {}", path.display(), err);
            false
        }
    }
}

fn resolve_gitignore_path(args: &RmEmptyDirsArgs) -> Result<Option<PathBuf>> {
    if let Some(path) = &args.gitignore {
        if !path.exists() {
            return Err(anyhow!(
                ".gitignore path does not exist: {}",
                path.display()
            ));
        }
        if !path.is_file() {
            return Err(anyhow!(".gitignore path is not a file: {}", path.display()));
        }
        return Ok(Some(path.clone()));
    }

    let default_path = std::env::current_dir()?.join(".gitignore");
    if default_path.is_file() {
        Ok(Some(default_path))
    } else {
        Ok(None)
    }
}

fn load_gitignore(path: &Path) -> Result<Gitignore> {
    let (gitignore, err) = Gitignore::new(path);
    if let Some(err) = err {
        eprintln!("Warning: failed to fully parse {}: {}", path.display(), err);
    }
    Ok(gitignore)
}

fn is_ignored(path: &Path, is_dir: bool, options: &Options) -> bool {
    match options.gitignore.as_ref() {
        Some(gitignore) => matches!(gitignore.matched(path, is_dir), Match::Ignore(_)),
        None => false,
    }
}

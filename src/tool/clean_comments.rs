use crate::tool::command::CleanUselessCommentsArgs;
use crate::tool::config::Config;
use crate::tool::processor::CommentProcessor;
use anyhow::{Result, anyhow};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

pub fn run(args: &CleanUselessCommentsArgs) -> Result<()> {
    println!("Clean useless comments command");

    // Load configuration with local-first priority
    let config = Config::load_with_priority_or_default(args.config.as_deref())?;
    let safety = config
        .get_clean_comments_config()
        .and_then(|clean_config| clean_config.safety.as_ref());
    let effective_dry_run = effective_dry_run(args);

    if args.verbose {
        // Show which config was loaded
        if let Some(ref config_path) = args.config {
            println!("Using explicit config: {}", config_path.display());
        } else {
            let local_config = std::env::current_dir()?.join(".llman/config.yaml");
            if local_config.exists() {
                println!("Using local config: .llman/config.yaml");
            } else {
                println!("Using global config");
            }
        }

        if let Some(clean_config) = config.get_clean_comments_config() {
            println!("Scope includes: {:?}", clean_config.scope.include);
            println!("Scope excludes: {:?}", clean_config.scope.exclude);
        }
    }

    if effective_dry_run {
        println!("Dry run mode enabled - no files will be modified");
        if !args.yes {
            println!("Use -y to apply changes.");
        }
    }

    if args.interactive {
        println!("Interactive mode enabled");
    }

    warn_if_not_git_repo(args)?;

    if let Some(safety) = safety {
        if safety.dry_run_first.unwrap_or(false) && !effective_dry_run && !args.force {
            println!("Safety: dry-run-first is enabled; running in dry-run mode only.");
            println!("Re-run with -y --force to apply changes.");

            let mut dry_args = args.clone();
            dry_args.dry_run = true;
            dry_args.yes = false;
            dry_args.force = true;

            let mut processor = CommentProcessor::new(config.clone(), dry_args);
            let result = processor.process()?;
            print_processing_results(&result);
            return Ok(());
        }

        if safety.require_git_commit.unwrap_or(false) && !effective_dry_run && !args.force {
            match is_git_repo_clean()? {
                Some(true) => {}
                Some(false) => {
                    return Err(anyhow!(
                        "Working tree has uncommitted changes; commit before running or use --force"
                    ));
                }
                None => {
                    eprintln!(
                        "Warning: require-git-commit enabled but no git repository detected."
                    );
                }
            }
        }
    }

    // Check if we need confirmation before proceeding
    if !args.force && !effective_dry_run && args.interactive {
        let should_proceed = ask_for_confirmation(args)?;
        if !should_proceed {
            println!("Operation cancelled by user.");
            return Ok(());
        }
    }

    // Process files
    let mut processor = CommentProcessor::new(config, args.clone());
    let result = processor.process()?;

    // Display results
    print_processing_results(&result);

    Ok(())
}

fn print_processing_results(result: &crate::tool::processor::ProcessingResult) {
    println!("\n=== Processing Complete ===");
    println!("Files changed: {}", result.files_changed.len());
    println!("Comments removed: {}", result.comments_removed);
    println!("Errors: {}", result.errors);

    if !result.files_changed.is_empty() {
        println!("\nModified files:");
        for file in &result.files_changed {
            println!("  - {}", file.display());
        }
    }
}

fn ask_for_confirmation(args: &CleanUselessCommentsArgs) -> Result<bool> {
    use inquire::Confirm;

    println!("\n=== Clean Useless Comments ===");
    println!("This operation will remove comments from your source code files.");

    if let Some(config_path) = &args.config {
        println!("Using configuration: {}", config_path.display());
    }

    if !args.files.is_empty() {
        println!("Files to process:");
        for file in &args.files {
            println!("  - {}", file.display());
        }
    }

    if effective_dry_run(args) {
        println!("Mode: Dry run (no files will be modified)");
    } else {
        println!("Mode: Live (files will be modified)");
    }

    let answer = Confirm::new("Do you want to continue?")
        .with_default(false)
        .with_help_message("This will start the comment cleaning process")
        .prompt()?;

    Ok(answer)
}

fn is_git_repo_clean() -> Result<Option<bool>> {
    let output = match Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) => output,
        Err(_) => return Ok(None),
    };

    if !output.status.success() {
        return Ok(None);
    }

    let repo_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if repo_root.is_empty() {
        return Ok(None);
    }

    let status = match Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&repo_root)
        .output()
    {
        Ok(status) => status,
        Err(_) => return Ok(None),
    };

    if !status.status.success() {
        return Ok(None);
    }

    Ok(Some(status.stdout.is_empty()))
}

fn warn_if_not_git_repo(args: &CleanUselessCommentsArgs) -> Result<()> {
    let targets = if args.files.is_empty() {
        vec![std::env::current_dir()?]
    } else {
        args.files.clone()
    };

    let mut checked = HashSet::new();
    let mut missing = Vec::new();

    for target in targets {
        let path = if target.is_dir() {
            target
        } else {
            target
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or(target.clone())
        };

        if !path.exists() || !checked.insert(path.clone()) {
            continue;
        }

        if !is_git_repo(&path)? {
            missing.push(path);
        }
    }

    if !missing.is_empty() {
        eprintln!("Warning: target path is not inside a git repository:");
        for path in missing {
            eprintln!("  - {}", path.display());
        }
    }

    Ok(())
}

fn is_git_repo(path: &Path) -> Result<bool> {
    let output = match Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        Ok(output) => output,
        Err(_) => return Ok(false),
    };

    Ok(output.status.success())
}

fn effective_dry_run(args: &CleanUselessCommentsArgs) -> bool {
    args.dry_run || !args.yes
}

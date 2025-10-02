use crate::tool::command::CleanUselessCommentsArgs;
use crate::tool::config::Config;
use crate::tool::processor::CommentProcessor;
use anyhow::Result;

pub fn run(args: &CleanUselessCommentsArgs) -> Result<()> {
    println!("Clean useless comments command");

    // Load configuration
    let config_path = args.config.as_deref()
        .unwrap_or_else(|| ".llman/config.yaml".as_ref());

    let config = Config::load_or_default(config_path)?;

    if args.verbose {
        println!("Configuration loaded successfully");
        if let Some(clean_config) = config.get_clean_comments_config() {
            println!("Scope includes: {:?}", clean_config.scope.include);
            println!("Scope excludes: {:?}", clean_config.scope.exclude);
        }
    }

    if args.dry_run {
        println!("Dry run mode enabled - no files will be modified");
    }

    if args.interactive {
        println!("Interactive mode enabled");
    }

    // Check if we need confirmation before proceeding
    if !args.force && !args.dry_run {
        let should_proceed = if args.interactive {
            ask_for_confirmation(args)?
        } else {
            // Default behavior: don't continue unless explicitly confirmed
            ask_for_confirmation_with_default_no(args)?
        };

        if !should_proceed {
            println!("Operation cancelled by user.");
            return Ok(());
        }
    }

    // Process files
    let mut processor = CommentProcessor::new(config, args.clone());
    let result = processor.process()?;

    // Display results
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

    Ok(())
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

    if args.dry_run {
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

fn ask_for_confirmation_with_default_no(args: &CleanUselessCommentsArgs) -> Result<bool> {
    use inquire::Select;

    println!("\n=== Clean Useless Comments ===");
    println!("This operation will remove comments from your source code files.");
    println!("By default, this operation will NOT proceed without explicit confirmation.");

    if let Some(config_path) = &args.config {
        println!("Using configuration: {}", config_path.display());
    }

    if !args.files.is_empty() {
        println!("Files to process:");
        for file in &args.files {
            println!("  - {}", file.display());
        }
    }

    if args.dry_run {
        println!("Mode: Dry run (no files will be modified)");
    } else {
        println!("Mode: Live (files will be modified)");
        println!("⚠️  WARNING: This will modify your source files!");
    }

    let options = vec![
        "No, cancel operation",           // Default option (index 0)
        "Yes, proceed with cleaning comments",
        "Show detailed information first",
    ];

    let choice = Select::new("Choose an option:", options)
        .with_starting_cursor(0) // Default to first option "No"
        .with_help_message("Select whether to proceed with the operation")
        .prompt()?;

    match choice {
        "Yes, proceed with cleaning comments" => Ok(true),
        "No, cancel operation" => Ok(false),
        "Show detailed information first" => {
            show_detailed_information(args)?;
            // After showing details, ask again with default to "No"
            ask_for_confirmation_with_default_no(args)
        }
        _ => Ok(false),
    }
}

fn show_detailed_information(args: &CleanUselessCommentsArgs) -> Result<()> {
    println!("\n=== Detailed Information ===");
    println!("This tool will scan and remove comments from source code files based on rules:");
    println!("- Comments shorter than the minimum length will be removed");
    println!("- Comments matching preservation patterns will be kept");
    println!("- The operation respects file scope includes/excludes from configuration");

    if let Some(config_path) = &args.config {
        println!("\nConfiguration file: {}", config_path.display());

        // Try to load and show config details
        if let Ok(config) = crate::tool::config::Config::load(config_path.clone()) {
            if let Some(clean_config) = config.get_clean_comments_config() {
                println!("Include patterns: {:?}", clean_config.scope.include);
                println!("Exclude patterns: {:?}", clean_config.scope.exclude);

                if let Some(lang_rules) = &clean_config.lang_rules.python {
                    println!("Python rules: {:?}", lang_rules);
                }
                if let Some(lang_rules) = &clean_config.lang_rules.javascript {
                    println!("JavaScript rules: {:?}", lang_rules);
                }
                if let Some(lang_rules) = &clean_config.lang_rules.rust {
                    println!("Rust rules: {:?}", lang_rules);
                }
                if let Some(lang_rules) = &clean_config.lang_rules.go {
                    println!("Go rules: {:?}", lang_rules);
                }
            }
        }
    }

    if !args.files.is_empty() {
        println!("\nSpecific files to process:");
        for file in &args.files {
            println!("  - {}", file.display());
        }
    } else {
        println!("\nWill process files based on configuration scope patterns.");
    }

    if args.dry_run {
        println!("\n🛡️  DRY RUN MODE: No files will actually be modified.");
        println!("   You can safely review what would be changed.");
    } else {
        println!("\n⚠️  LIVE MODE: Files will be permanently modified!");
        println!("   Make sure you have backups or use version control.");
    }

    println!("\n💡 Tip: You can use --force to skip this confirmation in the future.");
    println!("   Or use --dry-run to preview changes without modifying files.");

    Ok(())
}
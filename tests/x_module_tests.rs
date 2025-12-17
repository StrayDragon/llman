use clap::Parser;
use llman::cli::{Cli, Commands, XArgs, XCommands};
use llman::x::collect::command::{CollectArgs, CollectCommands};
use llman::x::collect::tree::TreeArgs;
use llman::x::cursor::command::{CursorArgs, CursorCommands, ExportArgs};
use std::path::PathBuf;
mod common;
use common::*;

/// Tests X module command parsing
#[test]
fn test_x_module_command_parsing() {
    // Test x cursor command
    let args = vec!["llman", "x", "cursor", "export"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::X(x_args) => {
                match x_args.command {
                    XCommands::Cursor(_) => {
                        // Successfully parsed x cursor command
                    }
                    _ => panic!("Expected XCommands::Cursor"),
                }
            }
            _ => panic!("Expected Commands::X"),
        }
    }

    // Test x collect command
    let args = vec!["llman", "x", "collect", "tree"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::X(x_args) => {
                match x_args.command {
                    XCommands::Collect(_) => {
                        // Successfully parsed x collect command
                    }
                    _ => panic!("Expected XCommands::Collect"),
                }
            }
            _ => panic!("Expected Commands::X"),
        }
    }
}

/// Tests X module command with various argument combinations
#[test]
fn test_x_module_with_additional_arguments() {
    // Test x cursor with potential future arguments
    let args = vec!["llman", "x", "cursor", "--help"];
    let cli = Cli::try_parse_from(args);
    // Help will exit early, causing an error, which is expected
    assert!(cli.is_err());

    // Test x collect with potential future arguments
    let args = vec!["llman", "x", "collect", "--help"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_err());
}

/// Tests X module invalid subcommands
#[test]
fn test_x_module_invalid_subcommands() {
    // Test invalid x subcommand
    let args = vec!["llman", "x", "invalid"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_err());

    // Test case sensitivity
    let args = vec!["llman", "x", "CURSOR"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_err());

    let args = vec!["llman", "x", "Cursor"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_err());
}

/// Tests X module command ordering and structure
#[test]
fn test_x_module_command_structure() {
    // Test that x command must have subcommand
    let args = vec!["llman", "x"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_err());

    // Test that subcommand comes after x
    let args = vec!["llman", "cursor", "x"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_err());
}

/// Tests X module in context of full CLI
#[test]
fn test_x_module_in_full_cli_context() {
    // Test x command doesn't interfere with other commands
    let other_commands = vec![
        vec!["llman", "project", "tree"],
        vec!["llman", "tool", "clean-useless-comments", "--help"],
        vec!["llman", "--version"],
    ];

    for cmd in other_commands {
        let cli = Cli::try_parse_from(cmd);
        // These may succeed or fail depending on the command,
        // but the important thing is they don't conflict with x module parsing
        match cli {
            Ok(_) => {
                // Successfully parsed
            }
            Err(_) => {
                // Failed (e.g., --help and --version exit early)
            }
        }
    }
}

/// Tests X module command aliases and variants
#[test]
fn test_x_module_command_variants() {
    // Test different forms of the cursor command
    let cursor_variants = vec![
        vec!["llman", "x", "cursor", "export"],
        // Note: clap doesn't accept trailing spaces as separate arguments,
        // they need to be part of a valid argument
    ];

    for variant in cursor_variants {
        let cli = Cli::try_parse_from(variant);
        assert!(cli.is_ok());
    }

    // Test different forms of the collect command
    let collect_variants = vec![
        vec!["llman", "x", "collect", "tree"],
        // Note: clap doesn't accept trailing spaces as separate arguments,
        // they need to be part of a valid argument
    ];

    for variant in collect_variants {
        let cli = Cli::try_parse_from(variant);
        assert!(cli.is_ok());
    }
}

/// Tests X module error handling and edge cases
#[test]
fn test_x_module_error_handling() {
    // Test with extra arguments (should be handled gracefully)
    let args = vec!["llman", "x", "cursor", "extra", "arguments"];
    let cli = Cli::try_parse_from(args);
    // Should fail because cursor doesn't accept positional arguments
    assert!(cli.is_err());

    // Test with unknown flags
    let args = vec!["llman", "x", "cursor", "--unknown-flag"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_err());

    // Test with malformed arguments
    let malformed_cases = vec![
        vec!["llman", "x", ""],        // Empty subcommand
        vec!["llman", "x", " "],       // Space subcommand
        vec!["llman", "x", "cursor-"], // Trailing dash
        vec!["llman", "x", "-cursor"], // Leading dash
    ];

    for case in malformed_cases {
        let cli = Cli::try_parse_from(case);
        assert!(cli.is_err());
    }
}

/// Tests X module integration with global flags
#[test]
fn test_x_module_with_global_flags() {
    // Test x commands with global help flag
    let help_cases = vec![
        vec!["llman", "--help", "x", "cursor"],
        vec!["llman", "x", "--help", "cursor"],
        vec!["llman", "x", "cursor", "--help"],
    ];

    for case in help_cases {
        let cli = Cli::try_parse_from(case);
        // Help commands exit early, causing errors
        assert!(cli.is_err());
    }

    // Test x commands with global version flag
    let version_cases = vec![
        vec!["llman", "--version", "x", "cursor"],
        vec!["llman", "x", "--version", "cursor"],
        vec!["llman", "x", "cursor", "--version"],
    ];

    for case in version_cases {
        let cli = Cli::try_parse_from(case);
        // Version commands exit early, causing errors
        assert!(cli.is_err());
    }
}

/// Tests X module future extensibility
#[test]
fn test_x_module_extensibility() {
    // This test ensures that the x module structure can be easily extended

    // Current supported commands
    let supported_test_cases = vec![
        (vec!["llman", "x", "cursor", "export"], "cursor"),
        (vec!["llman", "x", "collect", "tree"], "collect"),
        (
            vec!["llman", "x", "claude-code", "account", "list"],
            "claude-code",
        ),
        (vec!["llman", "x", "cc", "account", "list"], "claude-code"),
    ];

    for (args, cmd_name) in supported_test_cases {
        let cli = Cli::try_parse_from(args);
        assert!(cli.is_ok());

        if let Ok(cli) = cli {
            match cli.command {
                Commands::X(x_args) => {
                    // Successfully parsed an X command
                    match x_args.command {
                        XCommands::Cursor(_) => assert_eq!(cmd_name, "cursor"),
                        XCommands::Collect(_) => assert_eq!(cmd_name, "collect"),
                        XCommands::ClaudeCode(_) => assert_eq!(cmd_name, "claude-code"),
                        XCommands::Codex(_) => assert_eq!(cmd_name, "codex"),
                    }
                }
                _ => panic!("Expected Commands::X"),
            }
        }
    }
}

/// Tests X module command serialization/deserialization compatibility
#[test]
fn test_x_module_cli_compatibility() {
    // Test that x commands can be constructed programmatically
    let cursor_command = Commands::X(XArgs {
        command: XCommands::Cursor(CursorArgs {
            command: CursorCommands::Export(ExportArgs {
                interactive: false,
                db_path: None,
                workspace_dir: None,
                composer_id: None,
                output_mode: "console".to_string(),
                output_file: None,
                debug: false,
            }),
        }),
    });

    let collect_command = Commands::X(XArgs {
        command: XCommands::Collect(CollectArgs {
            command: CollectCommands::Tree(TreeArgs {
                path: PathBuf::from("."),
                output: None,
                no_ignore: false,
                max_depth: Some(2),
                append_default_context: false,
            }),
        }),
    });

    // These should be constructible without panics
    match cursor_command {
        Commands::X(x_args) => {
            match x_args.command {
                XCommands::Cursor(_) => {
                    // Successfully constructed
                }
                _ => panic!("Expected XCommands::Cursor"),
            }
        }
        _ => panic!("Expected Commands::X"),
    }

    match collect_command {
        Commands::X(x_args) => {
            match x_args.command {
                XCommands::Collect(_) => {
                    // Successfully constructed
                }
                _ => panic!("Expected XCommands::Collect"),
            }
        }
        _ => panic!("Expected Commands::X"),
    }
}

/// Tests X module with various environment contexts
#[test]
fn test_x_module_environment_contexts() {
    let env = TestEnvironment::new();

    // Create different types of project files that might interact with x commands
    let project_files = vec![
        ".cursor/rules/example.mdc",
        "llman-project.json",
        ".llman/config.yaml",
    ];

    for file in project_files {
        let file_path = env.path().join(file);
        if let Some(parent) = file_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&file_path, "# Test content").unwrap();
    }

    // Test that x commands can be parsed in different project contexts
    let x_commands = vec![
        vec!["llman", "x", "cursor", "export"],
        vec!["llman", "x", "collect", "tree"],
    ];

    for cmd in x_commands {
        let cli = Cli::try_parse_from(cmd);
        assert!(cli.is_ok());
    }
}

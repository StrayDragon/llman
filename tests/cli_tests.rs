use clap::Parser;
use llman::cli::{Cli, Commands, ProjectCommands, XCommands};
use llman::tool::command::ToolCommands;
use std::path::PathBuf;

/// Tests that the CLI can parse basic commands correctly
#[test]
fn test_cli_basic_command_parsing() {
    // Test help command
    let args = vec!["llman", "--help"];
    let cli = Cli::try_parse_from(args);
    // Help command will exit early, so we expect an error
    assert!(cli.is_err());

    // Test version command
    let args = vec!["llman", "--version"];
    let cli = Cli::try_parse_from(args);
    // Version command also exits early
    assert!(cli.is_err());
}

/// Tests project command parsing
#[test]
fn test_project_command_parsing() {
    // Test project tree command
    let args = vec!["llman", "project", "tree"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::Project(project_args) => {
                match project_args.command {
                    ProjectCommands::Tree(_) => {
                        // Successfully parsed project tree command
                    }
                }
            }
            _ => panic!("Expected Commands::Project"),
        }
    }
}

/// Tests tool command parsing
#[test]
fn test_tool_command_parsing() {
    // Test tool clean-useless-comments command
    let args = vec![
        "llman",
        "tool",
        "clean-useless-comments",
        "--dry-run",
        "--verbose",
    ];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::Tool(tool_args) => match tool_args.command {
                ToolCommands::CleanUselessComments(args) => {
                    assert!(args.dry_run);
                    assert!(args.verbose);
                    assert!(args.files.is_empty());
                }
            },
            _ => panic!("Expected Commands::Tool"),
        }
    }
}

/// Tests tool command with file arguments
#[test]
fn test_tool_command_with_files() {
    let args = vec![
        "llman",
        "tool",
        "clean-useless-comments",
        "--dry-run",
        "file1.py",
        "file2.js",
    ];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::Tool(tool_args) => match tool_args.command {
                ToolCommands::CleanUselessComments(args) => {
                    assert_eq!(args.files.len(), 2);
                    assert_eq!(args.files[0], PathBuf::from("file1.py"));
                    assert_eq!(args.files[1], PathBuf::from("file2.js"));
                }
            },
            _ => panic!("Expected Commands::Tool"),
        }
    }
}

/// Tests tool command with configuration file
#[test]
fn test_tool_command_with_config() {
    let args = vec![
        "llman",
        "tool",
        "clean-useless-comments",
        "--config",
        "custom.yaml",
    ];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::Tool(tool_args) => match tool_args.command {
                ToolCommands::CleanUselessComments(args) => {
                    assert!(args.config.is_some());
                    assert_eq!(args.config.unwrap(), PathBuf::from("custom.yaml"));
                }
            },
            _ => panic!("Expected Commands::Tool"),
        }
    }
}

/// Tests tool command with all options
#[test]
fn test_tool_command_with_all_options() {
    let args = vec![
        "llman",
        "tool",
        "clean-useless-comments",
        "--config",
        "test.yaml",
        "--dry-run",
        "--interactive",
        "--force",
        "--verbose",
        "--git-only",
        "test.py",
    ];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::Tool(tool_args) => match tool_args.command {
                ToolCommands::CleanUselessComments(args) => {
                    assert!(args.config.is_some());
                    assert!(args.dry_run);
                    assert!(args.interactive);
                    assert!(args.force);
                    assert!(args.verbose);
                    assert!(args.git_only);
                    assert_eq!(args.files.len(), 1);
                    assert_eq!(args.files[0], PathBuf::from("test.py"));
                }
            },
            _ => panic!("Expected Commands::Tool"),
        }
    }
}

/// Tests X command parsing
#[test]
fn test_x_command_parsing() {
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

/// Tests invalid command handling
#[test]
fn test_invalid_command_handling() {
    // Test completely invalid command
    let args = vec!["llman", "invalid-command"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_err());

    // Test invalid subcommand
    let args = vec!["llman", "project", "invalid-subcommand"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_err());
}

/// Tests CLI argument edge cases
#[test]
fn test_cli_argument_edge_cases() {
    // Test empty files list with tool command
    let args = vec!["llman", "tool", "clean-useless-comments"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test conflicting flags (should still parse, let application handle logic)
    let args = vec![
        "llman",
        "tool",
        "clean-useless-comments",
        "--dry-run",
        "--force",
    ];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::Tool(tool_args) => match tool_args.command {
                ToolCommands::CleanUselessComments(args) => {
                    assert!(args.dry_run);
                    assert!(args.force);
                }
            },
            _ => panic!("Expected Commands::Tool"),
        }
    }
}

/// Tests CLI with relative and absolute paths
#[test]
fn test_cli_with_path_arguments() {
    // Test with relative path
    let args = vec!["llman", "tool", "clean-useless-comments", "./src/main.py"];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::Tool(tool_args) => match tool_args.command {
                ToolCommands::CleanUselessComments(args) => {
                    assert_eq!(args.files.len(), 1);
                    assert_eq!(args.files[0], PathBuf::from("./src/main.py"));
                }
            },
            _ => panic!("Expected Commands::Tool"),
        }
    }

    // Test with absolute path (this is a dummy absolute path for testing)
    let absolute_path = "/home/user/project/main.py";
    let args = vec!["llman", "tool", "clean-useless-comments", absolute_path];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::Tool(tool_args) => match tool_args.command {
                ToolCommands::CleanUselessComments(args) => {
                    assert_eq!(args.files.len(), 1);
                    assert_eq!(args.files[0], PathBuf::from(absolute_path));
                }
            },
            _ => panic!("Expected Commands::Tool"),
        }
    }
}

/// Tests CLI with special characters in file paths
#[test]
fn test_cli_with_special_characters() {
    let special_file = "file with spaces.py";
    let args = vec!["llman", "tool", "clean-useless-comments", special_file];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    if let Ok(cli) = cli {
        match cli.command {
            Commands::Tool(tool_args) => match tool_args.command {
                ToolCommands::CleanUselessComments(args) => {
                    assert_eq!(args.files.len(), 1);
                    assert_eq!(args.files[0], PathBuf::from(special_file));
                }
            },
            _ => panic!("Expected Commands::Tool"),
        }
    }
}

/// Tests CLI configuration argument with various formats
#[test]
fn test_cli_config_argument_formats() {
    // Test with YAML extension
    let args = vec![
        "llman",
        "tool",
        "clean-useless-comments",
        "--config",
        "config.yaml",
    ];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test with relative path
    let args = vec![
        "llman",
        "tool",
        "clean-useless-comments",
        "--config",
        "./config/config.yaml",
    ];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());

    // Test with absolute path
    let args = vec![
        "llman",
        "tool",
        "clean-useless-comments",
        "--config",
        "/home/user/.llman/config.yaml",
    ];
    let cli = Cli::try_parse_from(args);
    assert!(cli.is_ok());
}

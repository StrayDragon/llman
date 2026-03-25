use clap::Parser;
use llman::cli::{Cli, Commands, XArgs, XCommands};
use llman::x::claude_code::command::{ClaudeCodeArgs, ClaudeCodeCommands};

#[test]
fn parses_cc_main_command_trailing_args_after_double_dash() {
    let cli = Cli::try_parse_from(["llman", "x", "cc", "--", "--version"]).expect("parse");

    match cli.command {
        Some(Commands::X(XArgs {
            command: XCommands::ClaudeCode(ClaudeCodeArgs { command, args }),
        })) => {
            assert!(command.is_none());
            assert_eq!(args, vec!["--version"]);
        }
        _ => panic!("unexpected parse result"),
    }
}

#[test]
fn parses_cc_run_subcommand_args_without_leaking_to_parent() {
    let cli = Cli::try_parse_from([
        "llman",
        "x",
        "cc",
        "run",
        "--group",
        "test",
        "--",
        "--version",
    ])
    .expect("parse");

    match cli.command {
        Some(Commands::X(XArgs {
            command:
                XCommands::ClaudeCode(ClaudeCodeArgs {
                    command:
                        Some(ClaudeCodeCommands::Run {
                            interactive,
                            group,
                            args,
                        }),
                    args: parent_args,
                }),
        })) => {
            assert!(!interactive);
            assert_eq!(group.as_deref(), Some("test"));
            assert_eq!(args, vec!["--version"]);
            assert!(parent_args.is_empty());
        }
        _ => panic!("unexpected parse result"),
    }
}

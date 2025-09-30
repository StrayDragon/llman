# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`llman` is a Rust-based command-line tool for managing LLM application rules and prompts, with special focus on Cursor editor integration. The tool helps developers create, manage, and deploy prompt rules files for various LLM applications.

## Build System and Development Commands

### Primary Commands (using just)

```bash
# Build and development
just build                    # Debug build
just build-release            # Release build
just run [args]              # Run with test config (uses ./artifacts/testing_config_home)
just run-prod [args]         # Run with production config
just test                    # Run tests
just check                   # Format, lint, and test (complete check)
just fmt                     # Format code
just lint                    # Run clippy with warnings as errors
just clean                   # Clean build artifacts
just install                 # Install locally

# Development utilities
just create-dev-template <name> <content>  # Create test template
just check-i18n             # Check internationalization status
```

### Direct Cargo Commands

```bash
cargo build                 # Build debug version
cargo build --release       # Build release version
cargo test                  # Run tests
cargo fmt                   # Format code
cargo clippy -- -D warnings # Lint with deny warnings
cargo install --path .       # Install from source
```

## Architecture Overview

### Core Components

- **`src/main.rs`**: Entry point with internationalization setup
- **`src/cli.rs`**: Command-line interface definition and command routing
- **`src/config.rs`**: Configuration management and file system operations
- **`src/prompt.rs`**: Core prompt management functionality
- **`src/error.rs`**: Error handling and custom error types

### Command Structure

The application follows a hierarchical command structure:

```
llman <command> [subcommand] [options]

Main commands:
- prompt/rule: Manage prompts and rules
  - gen: Generate new prompts (interactive or template-based)
  - list: List existing prompts
  - upsert: Create or update prompts
  - rm: Remove prompts
- project: Project utilities
  - tree: Generate directory tree structure
- x: Experimental commands
  - cursor: Cursor-specific functionality
  - collect: Information collection utilities
```

### Key Design Patterns

1. **Configuration Management**: Uses `Config` struct for centralized config handling with support for custom config directories via `LLMAN_CONFIG_DIR` environment variable
2. **Command Separation**: Each major command group has its own module and handler
3. **Internationalization**: Built-in i18n support using `rust-i18n` with English and Chinese locales
4. **Error Handling**: Comprehensive error handling with `anyhow` and custom error types

### File Structure

```
src/
├── main.rs              # Application entry point
├── cli.rs               # CLI definition and command routing
├── config.rs            # Configuration management
├── error.rs             # Error handling
├── prompt.rs            # Prompt management logic
└── x/                   # Experimental features
    ├── mod.rs
    ├── cursor/          # Cursor-specific functionality
    │   ├── mod.rs
    │   ├── command.rs   # Cursor command implementations
    │   ├── database.rs  # Database operations
    │   └── models.rs    # Data models
    └── collect/         # Data collection utilities
        ├── mod.rs
        ├── command.rs   # Collection commands
        └── tree.rs      # Directory tree generation
```

## Development Environment

### Configuration

- **Test Configuration**: Use `just run` which automatically sets `LLMAN_CONFIG_DIR=./artifacts/testing_config_home`
- **Production Configuration**: Use `just run-prod` or direct cargo commands
- **Language**: Set `LLMAN_LANG=zh-CN` or `LLMAN_LANG=zh` for Chinese, defaults to English

### Testing

- Test fixtures are stored in `./artifacts/testing_config_home/`
- Use `just create-dev-template` to create test prompt templates
- The application validates that commands are run in project directories (Git repositories)

### Cursor Integration

- Cursor rules are stored in `.cursor/rules/` directory with `.mdc` extension
- The tool can generate and manage Cursor-specific prompt rules
- Supports interactive template selection and rule generation

## Key Dependencies

- **CLI**: `clap` (with derive, cargo, env features)
- **Configuration**: `directories`, `once_cell`
- **Database**: `diesel` with SQLite backend
- **Serialization**: `serde`, `serde_json`, `toml`
- **Interactive UI**: `inquire`
- **Internationalization**: `rust-i18n`
- **Error Handling**: `anyhow`, `thiserror`
- **File Operations**: `glob`, `walkdir`, `ignore`

## Development Notes

- The application enforces running in Git project directories for safety
- Interactive commands provide user-friendly prompts and confirmations
- Template system allows for reusable prompt generation
- Experimental features are organized under the `x` command namespace
- Comprehensive error messages with internationalization support
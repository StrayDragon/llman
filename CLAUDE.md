# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**llman** is a Rust-based CLI tool for managing LLM application rules and prompts, with special focus on Cursor editor integration. Written in Rust 2024 edition (requires nightly toolchain).

## Build System and Commands

### Using just (recommended)

```bash
# Build and development
just build                    # Debug build
just build-release            # Release build
just run [args]              # Run with test config (./artifacts/testing_config_home)
just run-prod [args]         # Run with production config
just test                    # Run tests
just check                   # Format check + lint + test
just check-all               # Full check (doc + release build)
just fmt                     # Format code
just lint                    # Clippy with warnings as errors
just clean                   # Clean build artifacts
just install                 # Install locally

# Development utilities
just create-dev-template <name> <content>  # Create test template
just check-i18n             # Check internationalization status
```

### Direct Cargo Commands

```bash
cargo +nightly build                 # Debug build
cargo +nightly build --release       # Release build
cargo +nightly test                  # Run tests
cargo +nightly fmt                   # Format code
cargo +nightly clippy -- -D warnings # Lint with deny warnings
```

**Important**: The project uses Rust nightly toolchain for edition 2024 support.

## Architecture Overview

### Core Components

- **`src/main.rs`**: Entry point with i18n setup
- **`src/cli.rs`**: CLI definition and command routing (clap)
- **`src/config.rs`**: Configuration management and file system operations
- **`src/error.rs`**: Error handling with `thiserror`
- **`src/prompt.rs`**: Core prompt management functionality
- **`src/path_utils.rs`**: Path validation utilities

### Experimental Features (`src/x/`)

- **`cursor/`**: Cursor-specific functionality (conversations export, database operations)
- **`claude_code/`**: Claude Code API configuration management (alias: `cc`)
- **`codex/`**: OpenAI Codex configuration management

### Developer Tools (`src/tool/`)

- **`clean-comments`**: Clean useless code comments using tree-sitter

### Command Structure

```
llman <command> [subcommand] [options]

Main commands:
- prompt/rule: Manage prompts and rules
  - gen: Generate new prompts (interactive or template-based)
  - list: List existing prompts
  - upsert: Create or update prompts
  - rm: Remove prompts
- x: Experimental commands
  - cursor: Cursor functionality (export conversations)
  - claude-code/cc: Claude Code API configuration
  - codex: OpenAI Codex configuration
- tool: Developer tools
  - clean-comments: Clean code comments
```

## Key Design Patterns

1. **Configuration Management**: `Config` struct for centralized config handling with `LLMAN_CONFIG_DIR` environment variable override
2. **Error Handling**: `LlmanError` enum with `thiserror`, uses `display_localized()` for i18n error messages
3. **Internationalization**: `rust-i18n` with YAML translations in `locales/app.yml`; use `t!("key.subkey", param = value)` pattern
4. **Safety Checks**: Prevents running in home directory, requires Git repo
5. **Interactive/Non-interactive Modes**: Commands support both modes via `inquire` crate

## Configuration

- **Default config dir**: `~/.config/llman/`
- **Test config dir**: `./artifacts/testing_config_home/` (use `just run`)
- **Claude Code config**: `~/.config/llman/claude-code.toml`

Environment variables:
- `LLMAN_CONFIG_DIR`: Custom configuration directory
- `LLMAN_LANG`: Language (`zh-CN`, `zh`, or default English)

## Development Notes

- **i18n changes**: Must `cargo clean` after modifying translations
- **Prefer batch edits**: Reduce tool calls by batching text updates
- **Don't change deps**: Ask user before modifying `Cargo.toml`
- **Use `context7` MCP**: For crate API documentation instead of guessing
- **Use justfile**: For project-level actions instead of Makefile
- **Test fixtures**: Stored in `./artifacts/testing_config_home/`

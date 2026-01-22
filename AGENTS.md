<!-- OPENSPEC:START -->
# OpenSpec 说明文档

这些说明文档适用于在此项目中工作的 AI 助手。

当请求满足以下条件时，始终打开 `@/openspec/AGENTS.md`：
- 提及规划或提案（诸如 proposal、spec、change、plan 之类的词）
- 引入新功能、破坏性变更、架构转变或大型性能/安全工作
- 听起来模棱两可，您需要权威规范才能进行编码

使用 `@/openspec/AGENTS.md` 了解：
- 如何创建和应用变更提案
- 规范格式和约定
- 项目结构和指南

保持此托管块，以便 'openspec update' 可以刷新说明文档。

<!-- OPENSPEC:END -->

# Repository Guidelines

## Project Structure and Module Organization
- `src/` holds the Rust library and CLI code; `src/main.rs` wires the CLI and i18n.
- `src/x/` contains experimental integrations (cursor, claude_code, codex).
- `src/tool/` contains developer utilities used by the CLI.
- `tests/` contains integration tests; files are named `*_tests.rs`.
- `templates/` stores prompt templates; `locales/` stores i18n YAML files.
- `artifacts/testing_config_home/` is the test fixture config root used by dev commands.
- `scripts/` has helper scripts; `docs/` has design and planning notes.

## Build, Test, and Development Commands
This project targets Rust edition 2024 and uses the nightly toolchain.

- `just build` / `just build-release`: debug or release builds.
- `just run -- <args>`: run with test config (`LLMAN_CONFIG_DIR=./artifacts/testing_config_home`).
- `just run-prod -- <args>`: run with production config.
- `just test`: run the full test suite.
- `just check`: format check, lint, and tests.
- `just check-all`: check plus docs and release build.
- `just fmt` / `just lint`: rustfmt and clippy.

Cargo equivalents use `cargo +nightly ...`.

## Coding Style and Naming Conventions
- Use rustfmt defaults (4-space indentation) and keep code warning-free; clippy runs with `-D warnings`.
- Use `snake_case` for file and module names; keep CLI subcommands lowercase with hyphens for multi-word names.
- Prefer small, focused functions and reuse shared helpers in `src/path_utils.rs` and `src/config.rs`.

## Testing Guidelines
- Add unit tests near the code when possible, and integration tests under `tests/`.
- Name new integration test files `*_tests.rs` and keep test names descriptive.
- Use `LLMAN_CONFIG_DIR=./artifacts/testing_config_home` to avoid touching real user config.

## Commit and Pull Request Guidelines
- Commit messages use a short type prefix such as `feat:`, `fix:`, `refactor:`, `doc:`, or `bump:` with an optional scope, for example `fix(security): ...`.
- Keep commits focused and in present tense.
- PRs should include a clear summary, testing commands run, and links to related issues. Include sample CLI output when user-visible behavior changes.

## Configuration Notes
- Default config lives in `~/.config/llman/` unless overridden by `LLMAN_CONFIG_DIR`.
- `LLMAN_LANG` is reserved; only `en` is supported unless explicitly requested otherwise.
- i18n strings are placeholders; English-only is required unless explicitly requested otherwise.

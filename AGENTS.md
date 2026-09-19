<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解 SDD 命令行为配置，以及 `llmanspec/AGENTS.md` 获取项目附加规则。

## SDD 流水线

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman-sdd init --update` 刷新。
<!-- LLMANSPEC:END -->

# Repository Guidelines

## Project Structure and Module Organization
- `src/` holds the Rust library and CLI code; `src/main.rs` wires the CLI and i18n.
- `src/x/` contains experimental integrations (cursor, claude_code, codex).
- `src/tool/` contains developer utilities used by the CLI.
- `tests/it/` holds the consolidated integration-test target (single `it` binary; modules registered in `tests/it/main.rs`).
- `templates/` stores prompt templates (codex/claude-code); `locales/` holds the i18n strings (embedded at compile time via `build.rs`).
- `artifacts/testing_config_home/` is the test fixture config root used by dev commands.
- `scripts/` has helper scripts. `llmanspec/` holds this repo's spec data (read-only for llman itself; managed by the external llman-sdd v2 tool).

## Build, Test, and Development Commands
This project targets Rust edition 2024 and uses the nightly toolchain.

- `just build` / `just build-release`: debug or release builds.
- `just run -- <args>`: run with test config (`LLMAN_CONFIG_DIR=./artifacts/testing_config_home`).
- `just run-prod -- <args>`: run with production config.
- `just release`: git-tag 分发——打 `v<version>` tag 并推送（安装方式：`cargo install --git https://github.com/StrayDragon/llman --tag v<version>`）。
- `just test`: run the full test suite (`cargo nextest run --profile ci` when `cargo-nextest` is installed; otherwise `cargo test`). Config: `.config/nextest.toml`.
- `just check`: format check, lint, and tests.
- `just check-all`: check plus docs (`RUSTDOCFLAGS=-D warnings`), release build, and README managed-section checks.
- `just readme` / `just check-readme`: regenerate / verify the README sections marked `<!-- README:GENERATED ... -->` (install version + command tables, sourced from CLI `--help` and `Cargo.toml`); run `just readme` after changing the CLI surface or bumping the version.
- `just fmt` / `just lint`: rustfmt and clippy.

Cargo equivalents use `cargo +nightly ...`.

## Coding Style and Naming Conventions
- Use rustfmt defaults (4-space indentation) and keep code warning-free; clippy runs with `-D warnings`.
- Use `snake_case` for file and module names; keep CLI subcommands lowercase with hyphens for multi-word names.
- Prefer small, focused functions and reuse shared helpers in `crates/llman-core/src/path_utils.rs` (re-exported as `llman::path_utils`) and `src/config.rs`.

## Testing Guidelines
- Add unit tests near the code when possible, and integration tests as modules under `tests/it/` (register them in `tests/it/main.rs`); keep test names descriptive.
- Interactive CLI flows (e.g. `inquire` prompts) do not require automated tests; test the core, non-interactive logic instead.
- Use `LLMAN_CONFIG_DIR=./artifacts/testing_config_home` to avoid touching real user config.
- Avoid workspace pollution: tests that may create files/dirs MUST use `tempfile::TempDir` (or `TestEnvironment`) and write only inside it so everything is auto-cleaned.
- Avoid parallel test collisions: don’t use fixed relative paths/identifiers in the repo root (e.g. `config`, `config.yaml`); prefer unique temp paths and guard env/cwd changes with `crate::test_utils::TestProcess`.
- Editing `locales/*.yml` triggers rebuild automatically (`build.rs` declares `rerun-if-changed`).

## 规约与变更流程（llman-sdd v2）

本仓库的行为规格数据在 `llmanspec/specs/*.feature`，由外部工具 llman-sdd v2 读取和管理
（`llman-sdd validate` / `llman-sdd review` / `llman-sdd show` 等）。llman 仓库内的任何代码
都不得写入或校验该目录——它是用户数据格式，归 llman-sdd v2 所有。

命令入口约定：日常用 `llman-sdd <cmd>`（旧别名 `llmanspec <cmd>` 已废弃，将随下一个发布移除）。`llman sdd <cmd>` 不是
内置命令——它经 `llman-*` 外部子命令发现自动委托给 PATH 上的 `llman-sdd`（退出码透传）；
未安装 llman-sdd 时会报找不到可执行文件。

## Commit and Pull Request Guidelines
- Commit messages use a short type prefix such as `feat:`, `fix:`, `refactor:`, `doc:`, or `bump:` with an optional scope, for example `fix(security): ...`.
- Keep commits focused and in present tense.
- PRs should include a clear summary, testing commands run, and links to related issues. Include sample CLI output when user-visible behavior changes.

## Configuration Notes
- Default config lives in `~/.config/llman/` unless overridden by `LLMAN_CONFIG_DIR`.
- `LLMAN_LANG` is reserved; only `en` is supported unless explicitly requested otherwise.
- i18n strings are placeholders; English-only is required unless explicitly requested otherwise.

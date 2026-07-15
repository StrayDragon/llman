<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解项目上下文与规则。

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman sdd update` 刷新。
<!-- LLMANSPEC:END -->


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
- `just test`: run the full test suite (`cargo nextest run --profile ci` when `cargo-nextest` is installed; otherwise `cargo test`). Config: `.config/nextest.toml`.
- `just check`: format check, lint, and tests.
- `just check-all`: check plus docs (`RUSTDOCFLAGS=-D warnings`), release build, and SDD template checks.
- `just check-sdd-templates`: verify SDD template version headers and locale parity.
- `just fmt` / `just lint`: rustfmt and clippy.

Cargo equivalents use `cargo +nightly ...`.

## Coding Style and Naming Conventions
- Use rustfmt defaults (4-space indentation) and keep code warning-free; clippy runs with `-D warnings`.
- Use `snake_case` for file and module names; keep CLI subcommands lowercase with hyphens for multi-word names.
- Prefer small, focused functions and reuse shared helpers in `src/path_utils.rs` and `src/config.rs`.

## Testing Guidelines
- Add unit tests near the code when possible, and integration tests under `tests/`.
- Name new integration test files `*_tests.rs` and keep test names descriptive.
- Interactive CLI flows (e.g. `inquire` prompts) do not require automated tests; test the core, non-interactive logic instead.
- Use `LLMAN_CONFIG_DIR=./artifacts/testing_config_home` to avoid touching real user config.
- Avoid workspace pollution: tests that may create files/dirs MUST use `tempfile::TempDir` (or `TestEnvironment`) and write only inside it so everything is auto-cleaned.
- Avoid parallel test collisions: don’t use fixed relative paths/identifiers in the repo root (e.g. `config`, `config.yaml`); prefer unique temp paths and guard env/cwd changes with `crate::test_utils::TestProcess`.
- When editing `templates/sdd/**`, run `just check-sdd-templates` (also in `just check-all`).

## BDD-on (Feature-as-Spec) Conventions
本项目已启用 BDD-on 模式（`llmanspec/config.yaml` 含 `bdd:` 段）。spec 行为规格由
`llmanspec/specs/<name>/*.feature` 承载；每个 `spec.toon` 仅保留 `kind`/`name`/`purpose`。

- **fast mode（默认）**：`llman sdd validate <spec> --strict` 只做 Gherkin 语法解析（结构合法性），
  不执行任何 runner。所有 spec 的 feature 在 fast mode 下都应通过。
- **full mode（执行验证，可选）**：`llman sdd validate <spec> --check` 或 `cargo test --features bdd`
  实际运行 step 绑定。full mode 需要在 `tests/bdd_steps.rs` 中有对应的 `#[scenario]` 绑定 +
  可匹配的 step 定义。
- **泛化 step 库**：`tests/bdd_steps.rs` 提供一组可复用的「运行 llman → 断言输出」step
  （如 `当 运行 llman {args}`、`那么 退出码为 {code:i32}`、`那么 stderr 包含 {text}`）。
  新增可执行场景时优先复用这些泛化 step；仅在确实需要新断言模式时才添加新 step 定义。
- **判定一个 spec 是否可启用 full mode**：该 spec 的 feature 触发步骤应能由 CLI 子进程驱动
  （即 `假如`/`当` 步骤实际调用 `llman` 命令，而非描述内部状态如「管理器扫描」）。
  描述内部行为的 feature（约占全库 86%）不适合 full mode，保持 fast mode 即可。
- **新增 scenario 绑定**：在 `tests/bdd_steps.rs` 底部用 `#[scenario(path=..., name=...)]`
  绑定；`name` 必须与 `.feature` 中 `场景:` 标题**精确匹配**（字节级，含中文）。
- rstest-bdd **不支持正则**（字面文本经 `regex::escape` 全锚定 `^...$`）；泛化靠 `{name}`
  /`{name:type}` 占位符实现，「包含」语义在 step 函数体内用 Rust 子串断言表达。

## Commit and Pull Request Guidelines
- Commit messages use a short type prefix such as `feat:`, `fix:`, `refactor:`, `doc:`, or `bump:` with an optional scope, for example `fix(security): ...`.
- Keep commits focused and in present tense.
- PRs should include a clear summary, testing commands run, and links to related issues. Include sample CLI output when user-visible behavior changes.

## Configuration Notes
- Default config lives in `~/.config/llman/` unless overridden by `LLMAN_CONFIG_DIR`.
- `LLMAN_LANG` is reserved; only `en` is supported unless explicitly requested otherwise.
- i18n strings are placeholders; English-only is required unless explicitly requested otherwise.

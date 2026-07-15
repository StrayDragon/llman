<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解 SDD 命令行为配置，以及 `llmanspec/AGENTS.md` 获取项目附加规则。

## SDD 流水线

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman sdd init --update` 刷新。
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

### 如何启用/关闭 BDD-on 模式
**启用**：在 `llmanspec/config.yaml` 加 `bdd:` 段，`run_command` 按测试框架选：
```yaml
bdd:
  run_command: "cargo test --features bdd"                      # rstest-bdd
  # run_command: "pytest {feature_dir} -k {feature_name} -v"    # pytest-bdd
```
agent 在 propose 阶段遇到可执行行为场景时会主动询问是否启用（见 `llman-sdd-propose` 4a）。
**关闭**：删除 `bdd:` 段。注意：已有的 `.feature` 文件**不会被自动删除**——`validate`/`index`
会忽略它们。若确定不再需要，手动删除或保留作文档；solidify 在 BDD-off 时会提示残留 `.feature`
但不删除（见 `llman-sdd-solidify` 硬约束）。

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
- **占位符引号陷阱**：`.feature` 里写 `bdd 配置为 "on"` 时，rstest-bdd 会把引号也捕获进
  `{mode}`（值为 `"on"` 而非 `on`）。step 函数体内比较前必须 `trim_matches('"')`。

## BDD 模式兼容性测试维护规则

`llmanspec/specs/sdd-bdd-mode-compat/` 承载 BDD on/off 切换的行为合约（合约层，
`*.feature` + `tests/bdd_steps.rs` 的 `#[scenario]` 绑定）；`tests/sdd_bdd_compat_tests.rs`
承载实现细节层（init 结构、serde 向后兼容、13 子命令 smoke）。当 llman sdd 流程变动时，
以下变更**必须**同步适配这些测试：

- **改 validate 的 `--check`/`--no-check` 语义** → 同步 `validate-check.feature`（runner
  触发条件、降级行为）。
- **改 solidify 行为**（no-op 条件、`.feature` 输出位置/措辞）→ 同步 `solidify-mode.feature`
  的断言文本。
- **改 index rebuild 的 `.feature` embed 逻辑** → 同步 `index-embed.feature` +
  `tests/sdd_bdd_compat_tests.rs::test_index_rebuild_backward_compat_old_tree_loads`。
- **新增/移除 sdd 子命令** → 更新 `tests/sdd_bdd_compat_tests.rs::test_all_subcommands_smoke_bdd_on_and_off`
  的 `read_only` 命令列表。
- **改 step 库** → 确保 `已初始化 sdd 项目且 bdd 配置为 {mode}` step 仍能驱动所有场景
  （注意引号陷阱：`{mode}` 含引号，需 `trim_matches('"')`）。

判定新增断言的归属：
- 描述「MUST/SHALL 用户可见行为」（合约）→ 写 `.feature` + `#[scenario]` 绑定。
- 描述内部实现（serde、字段结构、smoke 兜底）→ 写 `tests/sdd_bdd_compat_tests.rs`。

## Commit and Pull Request Guidelines
- Commit messages use a short type prefix such as `feat:`, `fix:`, `refactor:`, `doc:`, or `bump:` with an optional scope, for example `fix(security): ...`.
- Keep commits focused and in present tense.
- PRs should include a clear summary, testing commands run, and links to related issues. Include sample CLI output when user-visible behavior changes.

## Configuration Notes
- Default config lives in `~/.config/llman/` unless overridden by `LLMAN_CONFIG_DIR`.
- `LLMAN_LANG` is reserved; only `en` is supported unless explicitly requested otherwise.
- i18n strings are placeholders; English-only is required unless explicitly requested otherwise.

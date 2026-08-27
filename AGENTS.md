<!-- LLMANSPEC:START -->
# LLMAN 规范驱动开发

本项目使用 llman SDD。阅读 `llmanspec/config.yaml` 了解 SDD 命令行为配置，以及 `llmanspec/AGENTS.md` 获取项目附加规则。

## SDD 流水线

使用 `/llman-sdd-explore` 开始，然后按照 pipeline：`/llman-sdd-propose` → `/llman-sdd-apply` → `/llman-sdd-verify` → `/llman-sdd-archive`。

保留此托管块，便于 `llman sdd init --update` 刷新。
<!-- LLMANSPEC:END -->

## SDD 可选增强能力

主 pipeline（explore→propose→apply→verify→archive）之外的**可选增强**，按需触发，默认行为不变。能力借鉴自 [mattpocock/skills](https://github.com/mattpocock/skills)（MIT，见下方致谢），经内化重写以 llman 的单 SSOT（单轨 feature-as-spec）为根，不引入 `CONTEXT.md`。

### pipeline 阶段内增强（触发词进入分支）

| 阶段 | 增强能力 | 触发词 | 说明 |
|------|---------|--------|------|
| explore | 逐问深挖 | 「深挖」「逐个问」 | 一次只问一个问题并附推荐答案；能查到的事实不问用户，只有决策才问；术语冲突时回写 live `.feature`（不另建词表） |
| propose | 测试边界前置 + 垂直切片 | 写 tasks 前自动 | 先列将测试的边界（seam，来自 `*.feature` GWT）并确认；tasks 按垂直切片拆 + `[blocked-by]` 依赖 |
| apply | 紧反馈诊断 | 自修复失败且判定为难定位 bug | 先建一个能复现失败的命令，再排查；禁止没有复现命令就猜原因 |
| verify | 双轴审查 | 用户要求或规范疑似 | 合约轴（`.feature` 中 @human 规约与 @executable 验收）+ 标准轴（AGENTS.md 编码规范 + 12 项代码坏味）分离呈现 |

### 独立可选 skill（不属于线性 pipeline）

| skill | invocation | 用途 |
|-------|-----------|------|
| `llman-sdd-arch-review` | model-invoked | 扫描薄模块，找出可加深（藏更多行为到更小接口后）的候选 |
| `llman-sdd-wayfinder` | user-invoked | 把大型、一团乱的工作拆成决策地图，逐个解决决策 |
| `llman-sdd-research` | model-invoked | 后台 agent 委托查一手资料（官方文档/源码/API） |

> 注：上述独立 skill 已列入 `OPTIONAL_SKILL_NAMES`；经 `llmanspec/config.yaml` 的 `extra_skills` 启用后，`init --update` 会写入/刷新。未列入 candidate 的 `llman-sdd-*` 目录会被清理——启用前先配 `extra_skills`。

### 设计词汇

下面这组关于模块形状的词，在 arch-review / verify 标准轴 / propose 测试边界中使用。MUST NOT 替换为 component/service/API/boundary（它们含义更宽、不够精确）：

- **Module（模块）** — 有接口和实现的东西（函数/类/包都算）。
- **Interface（接口）** — 调用者为正确使用所须知道的一切（签名 + 不变量 + 错误模式 + 性能）。
- **Depth（厚度）** — 接口背后的行为量；厚 = 小接口后藏大量行为，薄 = 接口 ≈ 实现（调用者没省事）。
- **Seam（接缝）** — 不改调用处就能换实现的位置；在 llman 中接缝 = `*.feature` GWT 驱动的公共边界（CLI 子进程或 public 函数）。
- **删除验证** — 想象删除模块：复杂度直接消失（只是透传，无价值）还是在 N 处重新冒出来（在扛事，有价值）。

> 上述能力的借鉴来源与第三方许可声明见根目录 `NOTICE` 文件。


# Repository Guidelines

## Project Structure and Module Organization
- `src/` holds the Rust library and CLI code; `src/main.rs` wires the CLI and i18n.
- `src/x/` contains experimental integrations (cursor, claude_code, codex).
- `src/tool/` contains developer utilities used by the CLI.
- `tests/` contains integration tests; files are named `*_tests.rs`.
- `templates/` stores prompt templates; `locales/` stores i18n YAML files.
- `artifacts/testing_config_home/` is the test fixture config root used by dev commands.
- `scripts/` has helper scripts. SDD workflow SSOT is root `AGENTS.md` + `llmanspec/` (not a parallel `docs/sdd` tree).

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
- Editing `locales/*.yml` triggers rebuild automatically (`build.rs` declares `rerun-if-changed`); no need to touch sources after translation edits.
- When editing `templates/sdd/**`, run `just check-sdd-templates` (also in `just check-all`).

## 统一 Git-native 变更流程（单轨 feature-as-spec）

标准术语（禁止用「车道」等隐喻替代）：

| 标准说法 | 是什么 | 不是什么 |
|----------|--------|---------|
| **Skill 导航** | explore → propose → apply → verify → archive 的 agent 技能顺序 | **不是** Git-native 生命周期 |
| **Git-native 生命周期** | Draft → Designed → Branch binding → Specs landing → apply → verify → finalize/archive | Specs landing 不是 skill |
| **CLI 三态 `stage`** | `draft` / `designed` / `full` | full 仍可能 `readyToImplement=false` |
| **Branch binding** | `change start`/`attach` 绑定非默认 `sdd/<id>` 分支 + `base_sha` | 不等于 Specs landing，不等于可 apply |
| **Specs landing** | 在绑定分支编辑 `llmanspec/specs/**/<capability>.feature` 并留相对 base_sha 的 diff | 不是在默认分支改 live specs |
| **`skip_specs_landing`** | frontmatter 豁免：本次无 live 合约变更 | 不是跳过 Branch binding |
| **`readyToImplement`** | apply 门禁：`Full ∧ (specsLanded ∨ skip_specs_landing)` | 用 `show`/`status --json` 查 |
| **Locked rules（@human）** | 人拥有的约束场景；哈希锁定于 base_sha | 新增规则无需 ack；改/删须 `rules_edit_acked: true` |

线性流程：

```
draft [proposal.md]
  → designed [+design+tasks]
  → bound [change start|attach]
  → specs-landed [绑定分支编辑 <capability>.feature 并 commit]
  → apply → verify → finalize/archive
```

### 单轨格式（spec-format r131-r136）

- 每个 capability 目录只有**一个** `<capability>.feature`；`spec.toon` 已废除（出现即 ERROR，跑 toon2features）。
- 头注释 `# capability:` / `# purpose:` / `# scope:` 必填（scope 驱动 staleness）。
- 约束 = `@req:<id> @human` 场景（statement 全文放描述）；验收 = `@executable` 场景（用 `@req:<id>` 挂回）。
- 三态分级：enforced / manual(`@manual`) / pending —— `list --specs` 与 `show` 输出。
- 禁止把场景嵌进 `Rule:` 块（rstest-bdd scenarios! 会静默跳过）。
- `change delta` / solidify / `*.feature.delta.toon` / `bdd.bindings` 均已移除。



## BDD 兼容测试维护规则

`tests/sdd_bdd_compat_tests.rs` 承载实现细节层（init 结构、serde 向后兼容、子命令 smoke）；
行为合约在 `llmanspec/specs/sdd-bdd-mode-compat/*.feature`。改动以下内容必须同步适配：
validate `--check` 语义、change 生命周期命令面、锁定门禁、index rebuild embed、
sdd 子命令增删（smoke 列表）、step 库（保持泛化 step 可驱动全部 @executable 场景；
注意 rstest-bdd 占位符引号陷阱：`{mode}` 含引号需 trim）。

判定新增断言归属：用户可见 MUST/SHALL 行为 → `.feature` + `@executable`；
内部实现（serde、字段结构、smoke 兜底）→ Rust 测试文件。


## Commit and Pull Request Guidelines
- Commit messages use a short type prefix such as `feat:`, `fix:`, `refactor:`, `doc:`, or `bump:` with an optional scope, for example `fix(security): ...`.
- Keep commits focused and in present tense.
- PRs should include a clear summary, testing commands run, and links to related issues. Include sample CLI output when user-visible behavior changes.

## Configuration Notes
- Default config lives in `~/.config/llman/` unless overridden by `LLMAN_CONFIG_DIR`.
- `LLMAN_LANG` is reserved; only `en` is supported unless explicitly requested otherwise.
- i18n strings are placeholders; English-only is required unless explicitly requested otherwise.

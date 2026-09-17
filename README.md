<div align="center">

  # llman

  [![](https://img.shields.io/crates/v/llman?style=flat-square&logo=rust&label=crates.io)](https://crates.io/crates/llman)
  [![](https://img.shields.io/crates/dr/llman?style=flat-square&logo=rust)](https://crates.io/crates/llman)
  [![](https://img.shields.io/docsrs/llman?style=flat-square&logo=docsdotrs&label=docs.rs)](https://docs.rs/llman)
  [![](https://img.shields.io/github/stars/straydragon/llman?style=flat-square&logo=github)](https://github.com/straydragon/llman/stargazers)
  [![](https://img.shields.io/github/actions/workflow/status/straydragon/llman/ci.yaml?style=flat-square&logo=github&label=CI)](https://github.com/straydragon/llman/actions)
  [![](https://img.shields.io/crates/l/llman?style=flat-square&color=blue)](https://github.com/straydragon/llman/blob/main/LICENSE)

  **llm**an — 管 LLM 应用的规则（prompts）与 AI 编码工作流小工具


</div>

---

llman 是一个面向 AI 编码工作流的 Rust CLI，解决两个日常痛点：

1. **规则漂移** — Cursor 用 `.mdc`、Claude Code 用 `CLAUDE.md`、Codex 用 TOML，提示词各管各的，改一处忘三处。llman 把规则和 skills 收进一个配置库，一条命令生成、同步到各个工具。
2. **AI 写码没合约** — 让智能体改代码，改得对不对全靠肉眼。这由姊妹项目 **[llman-sdd](https://github.com/StrayDragon/llman-sdd)** 解决：规格是可执行的 Gherkin `.feature`，变更走 draft → apply → verify → archive 的生命周期，每一步都有 CLI 门禁。

> **迁移说明（v0.0.79）**：原内置的 `llman sdd` 子系统已移出本仓库，重写为独立项目
> [llman-sdd v2](https://github.com/StrayDragon/llman-sdd)（TypeScript + Bun，
> npm 包 [`@llman-sdd/cli`](https://www.npmjs.com/package/@llman-sdd/cli)）。
> 安装后提供 `llman-sdd` / `llmanspec` 两个命令；既有 `llmanspec/` 目录零迁移直接可用。
> 旧入口 `llman sdd` 已不再内置：未命中内置命令时经 `llman-*` 外部子命令发现
> **自动委托**给 PATH 上的 `llman-sdd`（argv 原样转发、退出码透传）。

### 三个命令入口怎么选

| 入口 | 状态 | 何时使用 |
|---|---|---|
| `llman-sdd <cmd>` | ✅ 主命令 | 日常 SDD 操作一律用它（llman-sdd v2 提供） |
| `llmanspec <cmd>` | ✅ 等价别名 | 与 `llman-sdd` 同一二进制，按习惯选用 |
| `llman sdd <cmd>` | 🔁 自动委托 | 非内置命令，经 `llman-*` 发现转发给 `llman-sdd`；未安装时提示找不到 |

> 说明：`llman sdd` 走的是 git 风格外部子命令机制（cli.feature r56）——argv 原样转发、
> stdio/cwd 继承、退出码透传、`-C/--config-dir` 以 `LLMAN_CONFIG_DIR` 注入子进程。
> 未安装 llman-sdd 时报 `unrecognized command 'sdd': no llman-sdd executable found on PATH`。

本仓库自身也用 llman-sdd 管理开发流程——规格在 [llmanspec/specs/](llmanspec/specs/)，AI 与人共用同一套合约。

## 提示词与技能管理

```bash
llman prompts                        # 交互式编排：选规则 → 选目标应用
llman x cursor prompts list          # 列出可用的规则模板
llman x cursor prompts gen my-rule   # 从模板生成 Cursor 规则
llman x claude-code account          # Claude Code 多账号 API 配置切换
llman x codex agents                 # 管理 Codex 自定义 agent 配置
llman skills                         # 把技能同步安装到各 AI 工具
```

支持 **Cursor** / **Claude Code** / **Codex** 三个应用；`sync-ignore` 还能在它们之间同步 ignore 规则，`llman tool` 另带清理无用注释、清理空目录、管理 AGENTS.md 等开发小工具。

## 规格驱动开发（SDD）→ 已迁移

SDD 工作流不再随 llman 分发，请改用独立项目 llman-sdd v2：

```bash
npm install -g @llman-sdd/cli

llman-sdd init              # 在项目里初始化 llmanspec/（--update 刷新既有）
llman-sdd change new        # 起草一个变更
llman-sdd validate          # 校验规格
llman-sdd review            # 人工评审检查点
```

命令面与数据格式兼容：`llmanspec/` 目录布局（config.yaml / specs/*.feature / changes/）不变，无需任何迁移。详见 <https://github.com/StrayDragon/llman-sdd>。

## 外部子命令（llman-* 插件）

`llman` 会自动发现 `PATH` 上名为 `llman-*` 的可执行文件（git 外部子命令模式）：`llman <name>` 未命中内置命令时委托给 `llman-<name>` 执行——参数原样转发、stdio/cwd/环境继承、退出码透传，`-C/--config-dir` 会以 `LLMAN_CONFIG_DIR` 形式注入子进程。任何语言编写的可执行文件（二进制、Node/Python/Shell 脚本）放入 `PATH` 即接入，例如 `cargo install llman-foo` 或 `npm install -g` 后立即可用 `llman foo`；内置命令始终优先。

## 安装

<!-- README:GENERATED version START -->
```bash
# crates.io（llman-core 依赖一并安装）
cargo install llman --version 0.0.79
```
<!-- README:GENERATED END -->

从源码：

```bash
git clone https://github.com/StrayDragon/llman && cd llman
cargo install --path . --bin llman
```

也可从 GitHub tag 直接构建：`cargo install --git https://github.com/StrayDragon/llman --tag v<version>`。需要 nightly 工具链（`rust-toolchain.toml` 会自动切换）。

## 命令一览

<!-- README:GENERATED commands START -->
**`llman`** 顶层命令组：

| 命令 | 说明 |
|---|---|
| `prompts` | Prompt orchestrator (interactive-only) |
| `skills` | Manage skills |
| `x` | Experimental commands |
| `tool` | Developer tools |
| `self` | Self-management commands |

**`llman x`**：

| 子命令 | 说明 |
|---|---|
| `cursor` | Commands for interacting with Cursor |
| `claude-code` | Commands for managing Claude Code configurations |
| `codex` | Commands for managing Codex configurations |

**`llman tool`**：

| 子命令 | 说明 |
|---|---|
| `clean-useless-comments` | Clean useless comments from source code |
| `rm-useless-dirs` | Remove useless directories recursively |
| `sync-ignore` | Sync ignore rules across OpenCode/Cursor/Claude Code |
| `agents-md` | Manage agent init files (AGENTS.md / CLAUDE.md / .cursor/ etc.) |

> SDD 工作流（原 `llman sdd`）已迁移至独立项目 llman-sdd v2：`npm install -g @llman-sdd/cli`（命令 `llman-sdd` / `llmanspec`）；`llman self` 提供 schema 生成与 shell 补全。
<!-- README:GENERATED END -->

## Workspace crates

| crate | 说明 |
|---|---|
| [`llman`](https://crates.io/crates/llman) | 主 CLI |
| [`llman-core`](https://crates.io/crates/llman-core) | 配置解析、路径等共享基础件 |

## 开发

```bash
just build     # debug 构建
just test      # 全量测试（nextest 优先，含 doctest）
just check     # fmt-check + clippy + test
just qa        # CI 全量：check-all + i18n 审计 + 依赖扫描 + 供应链审计
```

README 中 `<!-- README:GENERATED ... -->` 标记的段落由 `just readme` 从 CLI `--help` 与 workspace 版本自动生成；改动命令面或版本号后跑一次，`just check-readme` 会拦截过期（已接入 `just check-all` 与 CI）。

## 文档

- [AGENTS.md](AGENTS.md) — 项目约定与开发流程
- [llmanspec/](llmanspec/) — 本仓库的活规格（`llman-sdd list --specs` 查看，工具来自 llman-sdd v2）
- [NOTICE](NOTICE) — 第三方致谢沿革与许可声明

## 许可证

MIT

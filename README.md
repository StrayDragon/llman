<div align="center">

  # llman

  [![](https://img.shields.io/crates/v/llman?style=flat-square&logo=rust&label=crates.io)](https://crates.io/crates/llman)
  [![](https://img.shields.io/crates/dr/llman?style=flat-square&logo=rust)](https://crates.io/crates/llman)
  [![](https://img.shields.io/docsrs/llman?style=flat-square&logo=docsdotrs&label=docs.rs)](https://docs.rs/llman)
  [![](https://img.shields.io/github/stars/straydragon/llman?style=flat-square&logo=github)](https://github.com/straydragon/llman/stargazers)
  [![](https://img.shields.io/github/actions/workflow/status/straydragon/llman/ci.yaml?style=flat-square&logo=github&label=CI)](https://github.com/straydragon/llman/actions)
  [![](https://img.shields.io/crates/l/llman?style=flat-square&color=blue)](https://github.com/straydragon/llman/blob/main/LICENSE)

  **llm**an — 管 LLM 应用的规则（prompts），也管 AI 时代的开发流程（SDD）


</div>

---

llman 是一个面向 AI 编码工作流的 Rust CLI，解决两个日常痛点：

1. **规则漂移** — Cursor 用 `.mdc`、Claude Code 用 `CLAUDE.md`、Codex 用 TOML，提示词各管各的，改一处忘三处。llman 把规则和 skills 收进一个配置库，一条命令生成、同步到各个工具。
2. **AI 写码没合约** — 让智能体改代码，改得对不对全靠肉眼。`llman sdd`（独立二进制 `llmanspec`）提供 Git-native 的规格驱动开发（SDD）：规格是可执行的 Gherkin `.feature`，变更走 draft → apply → verify → archive 的生命周期，每一步都有 CLI 门禁。

本仓库自己就是用 `llman sdd` 开发的——规格在 [llmanspec/specs/](llmanspec/specs/)，AI 与人共用同一套合约。

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

## 规格驱动开发（SDD）

一次功能变更被建模成有生命周期的「change」，规格（specs）是单轨的 Gherkin `.feature` 文件——人读得懂，`rstest-bdd` 能执行：

```
draft [proposal.md]
  → designed [+design+tasks]
  → bound [change start]            # 自动绑 sdd/<id> 分支 + base_sha
  → specs-landed [编辑 .feature]     # 规格先落地，代码后实施
  → apply → verify → archive
```

- `@human` 规则由人拥有、哈希锁定，改动需显式确认；
- `llmanspec validate --all` / `llmanspec review` 是 AI 与人的共同门禁；
- `llmanspec context` 按任务/文件检索相关规格，给智能体喂准确上下文。

```bash
llmanspec init              # 在项目里初始化 llmanspec/（可一并安装 SDD skills）
llmanspec change new        # 起草一个变更
llmanspec validate --all    # 校验规格与变更
llmanspec review            # 人工评审检查点
```

## 安装

<!-- README:GENERATED version START -->
```bash
# crates.io（llman-core / llman-sdd / gherkin-zh 依赖一并安装）
cargo install llman --version 0.0.75
```
<!-- README:GENERATED END -->

从源码：

```bash
git clone https://github.com/StrayDragon/llman && cd llman
cargo install --path . --bin llman --bin llmanspec
```

也可从 GitHub tag 直接构建：`cargo install --git https://github.com/StrayDragon/llman --tag v<version>`。需要 nightly 工具链（`rust-toolchain.toml` 会自动切换）。

## 命令一览

<!-- README:GENERATED commands START -->
**`llman`** 顶层命令组：

| 命令 | 说明 |
|---|---|
| `prompts` | Prompt orchestrator (interactive-only) |
| `skills` | Manage skills |
| `sdd` | Spec-driven development workflow |
| `x` | Experimental commands |
| `tool` | Developer tools |
| `self` | Self-management commands |

**`llman sdd`**：

| 子命令 | 说明 |
|---|---|
| `review` | Aggregate review: pending/manual rules, unbound scenarios, staleness, locked-rule hints and a validate --all sweep (sdd-review r5-r51) |
| `init` | Initialize llmanspec in your project (use --update to refresh existing) |
| `list` | List changes or specs |
| `show` | Show a change or spec |
| `validate` | Validate changes and specs |
| `archive` | Archive workflow commands (cold backup). Prefer `sdd change archive` to seal a change |
| `change` | Change lifecycle: new / start / attach / finalize / archive |
| `spec` | Spec authoring helpers |
| `graph` | Generate a change dependency graph |
| `worktree` | Worktree management commands |
| `context` | Get specs relevant to a task and/or file paths (agent-oriented) |
| `index` | Index management commands (rebuild, check freshness) |
| `config` | Project configuration commands (view/edit config.yaml) |
| `project` | Project management commands |

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

> 独立二进制 `llmanspec` ≡ `llman sdd`（参数一致，供不带主 CLI 的场景使用）；`llman self` 提供 schema 生成与 shell 补全。
<!-- README:GENERATED END -->

## Workspace crates

| crate | 说明 |
|---|---|
| [`llman`](https://crates.io/crates/llman) | 主 CLI（`llman` + `llmanspec` 两个二进制） |
| [`llman-core`](https://crates.io/crates/llman-core) | 配置解析、路径等共享基础件 |
| [`llman-sdd`](https://crates.io/crates/llman-sdd) | SDD 工作流实现（`llman sdd` / `llmanspec`） |
| [`gherkin-zh`](https://crates.io/crates/gherkin-zh) | gherkin 0.16 的 fork，补 zh-CN「规则」关键字 |

## 开发

```bash
just build     # debug 构建
just test      # 全量测试（nextest 优先，含 doctest）
just check     # fmt-check + clippy + test
just qa        # CI 全量：check-all + i18n 审计 + 依赖扫描 + 供应链审计
```

README 中 `<!-- README:GENERATED ... -->` 标记的段落由 `just readme` 从 CLI `--help` 与 workspace 版本自动生成；改动命令面或版本号后跑一次，`just check-readme` 会拦截过期（已接入 `just check-all` 与 CI）。

## 文档

- [AGENTS.md](AGENTS.md) — 项目约定与 SDD 流程
- [llmanspec/](llmanspec/) — 本仓库的活规格（`llmanspec list --specs` 查看）
- [NOTICE](NOTICE) — 第三方致谢与许可声明

## 许可证

MIT

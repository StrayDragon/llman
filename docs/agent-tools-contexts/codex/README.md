# Codex CLI（OpenAI）配置与上下文

本页聚焦 Codex CLI 的**配置入口**、`AGENTS.md`/project-doc/skills 的**加载与优先级**、以及 MCP/权限（sandbox + approvals）机制，并对接本仓库结构。

证据：`docs/agent-tools-contexts/_raw/codex/`

## 1) 配置文件位置与覆盖顺序

官方说明的配置文件：

- 用户配置：`~/.codex/config.toml`
- 项目配置：`codex.toml`

覆盖优先级：CLI 参数 → 项目 `codex.toml` → 用户 `~/.codex/config.toml`，并可用 `--config` 指定自定义配置路径。参见 `docs/agent-tools-contexts/_raw/codex/developers.openai.com__codex__config__basics__2026-02-05.md`。

## 2) 项目文档（project doc）与 AGENTS.md

Codex 会寻找“项目文档”用于上下文注入；默认回退文件名列表包含 `AGENTS.md` 与 `CLAUDE.md` 等。参见 `docs/agent-tools-contexts/_raw/codex/developers.openai.com__codex__config__agents-md__2026-02-05.md`。

可显式指定项目文档：

- `--project-doc path/to/file.md`
- `CODEX_PROJECT_DOC=path/to/file.md`

建议：

- 复用同一份项目指令文件：优先维护 `AGENTS.md`（Codex/Cursor）并提供 `CLAUDE.md`（Claude Code）做兼容入口。
- 把“高变更、只对本机有效”的内容放到本机配置（避免污染团队共享指令）。

补充（来自当前运行时的 `AGENTS.md` 规范，非官网网页摘录）：

- `AGENTS.md` 对其所在目录树生效；更深层目录的 `AGENTS.md` 优先级更高。
- 当你修改某文件时，需要遵守该文件路径所在 scope 的 `AGENTS.md` 指令。

## 3) approvals / sandbox（命令执行权限）

Codex 提供 `approval_policy`（是否需要用户批准）：

- `untrusted`：每条命令都询问
- `on-request`：仅在你明确要求时询问
- `on-failure`：仅当命令失败时询问
- `never`：从不询问

参见 `docs/agent-tools-contexts/_raw/codex/developers.openai.com__codex__config__rules__2026-02-05.md`。

命令规则匹配要点：

- 命令会按 `|`、`&&`、`||`、`;`、`(...)`、`$(...)` 等控制操作符拆成多个 segment；每个 segment 独立评估。
- “prefix rule” 只匹配某个 segment 的起始前缀。

这点对编写“自动批准某类命令”的规则很关键（避免误把 `git pull | tee ...` 当成一条）。

## 4) MCP（外部工具）

Codex 的 MCP servers 在配置里声明（示例节选）：

```toml
[mcp_servers.server-name]
command = "uvx"
args = ["mcp-server-git"]
env = { "GIT_SIGNING_KEY" = "${GIT_SIGNING_KEY}" }
```

参见 `docs/agent-tools-contexts/_raw/codex/developers.openai.com__codex__config__mcp__2026-02-05.md`。

## 5) Skills（技能库）

官方给出的 skills 搜索路径：

- 项目：`.codex/skills/`
- 用户：`~/.codex/skills/`
- 管理员：`/etc/codex/skills/`

参见 `docs/agent-tools-contexts/_raw/codex/developers.openai.com__codex__skills__2026-02-05.md`。

与本仓库对接建议：

- 本仓库把 skills 统一收敛在 `skills/`，投放目标由 `config.toml` target 与目标目录实时链接状态共同决定（不再依赖 `skills/registry.json`）。
- 由 llman（或你的同步脚本）把需要投放给 Codex 的技能同步到 `~/.codex/skills/`（用户级）或业务仓库的 `.codex/skills/`（项目级）。

## 6) 与本仓库现状对应

- `prompt/codex/`：已提供 Codex CLI 的输出风格提示（中/英）；可作为 project doc 的候选内容（通过 `--project-doc` 或 `CODEX_PROJECT_DOC` 注入）。
- `config.yaml`：`skills.dir: $LLMAN_CONFIG_DIR/skills`（llman 侧的 skills 目录约定）。

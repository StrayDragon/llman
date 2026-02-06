# Cursor 配置与上下文（Rules / MCP / CLI）

本页梳理 Cursor 的 Rules（`.mdc`）、项目/用户配置位置、解析/优先级顺序，以及 MCP servers 的配置方式，并对接本仓库的 `prompt/cursor/`。

证据：`docs/agent-tools-contexts/_raw/cursor/`

## 1) Rules：类型、位置、格式

官方列出的规则类型与位置：

- Project Rules：放在 `.cursor/rules`，对整个项目生效；并且可在子目录继续放置 `.cursor/rules` 形成 Nested Rules（仅在该子目录工作时注入上下文）。
- User Rules：全局规则，作用于所有项目。
- `.cursorrules`：legacy 方式。
- `AGENTS.md` / `CLAUDE.md`：Cursor 也支持把它们作为规则文件。

参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__context__rules__zh__2026-02-05.md`。

补充（User rules）：

- User rules 在 Cursor Settings > Rules 中配置；
- 对所有项目生效，并且总是包含在 model context；
- 不支持 `.mdc`（纯文本）。

参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__context__rules__en__2026-02-05.md`。

扩展名与格式：

- Cursor 支持 `.md` 与 `.mdc`；使用 `.mdc` + frontmatter 可更精细控制应用条件。参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__en__context__agents-md__2026-02-05.md`。

`.mdc` 文件格式（frontmatter）：

- `description`：描述
- `globs`：匹配哪些文件
- `alwaysApply`：是否总是应用

当规则命中时，其正文会被放到 model context 的开头。参见同一 raw 文件。

Rule Type（Cursor UI 中的类型，影响字段/行为）：

- `Always`
- `Auto Attached`
- `Agent Requested`
- `Manual`

参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__context__rules__en__2026-02-05.md`。

关于“多条规则同时命中时的拼接顺序”：官方只保证“注入到上下文开头”，未公开稳定顺序；建议避免依赖顺序，通过更具体的 `globs` 与 Nested Rules 来拆分责任。

AGENTS.md 的限制（适合作为“最简单的一份项目指令”）：

- 纯 Markdown，无 `.mdc` 元数据、不可拆分多文件、无 scoping；
- 仅支持放在项目根目录（文档标注 v1.5），子目录嵌套支持计划在 v1.6。

参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__en__context__agents-md__2026-02-05.md`。

## 2) MCP：配置文件、字段、插值与优先级

配置文件位置：

- 项目：`.cursor/mcp.json`
- 全局：`~/.cursor/mcp.json`

参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__context__mcp__zh__2026-02-05.md`。

模板插值：

- `${env:VAR_NAME}`（环境变量）
- `${input:variable_name}`（用户输入）
- `${workspaceFolder}`（工作区目录）
- `${userHome}`（用户目录，文档为西语页面节选）

参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__context__mcp__zh__2026-02-05.md` 与 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__es__context__mcp__2026-02-05.md`。

插值生效字段（文档节选）：`command`、`args`、`env`、`url`、`headers`。参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__es__context__mcp__2026-02-05.md`。

字段说明（节选）：`command`、`args`、`env`、`cwd`、`timeout`、`transport`、`envFile`、`url`、`headers` 等，参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__context__mcp__fields__fr__2026-02-05.md`。

CLI 场景的配置优先级（cursor-agent）：

- `project → global → nested`

参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__cli__reference__mcp__2026-02-05.md` 与 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__cli__mcp__2026-02-05.md`。

## 3) Cursor CLI 读取哪些上下文

官方说明 Cursor CLI 与 IDE 使用相同规则，会读取：

- `.cursor/rules`
- `.cursor/mcp.json`
- `AGENTS.md`
- `CLAUDE.md`

参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__cli__using__2026-02-05.md`。

另外，CLI 的 MCP 配置会从父目录自动发现（文档节选），参见 `docs/agent-tools-contexts/_raw/cursor/docs.cursor.com__cli__mcp__2026-02-05.md`。

## 4) 与本仓库的对接

- `prompt/cursor/*.mdc` 可直接作为 Project Rules 或 User Rules 投放：
  - 项目级：复制/同步到业务仓库 `.cursor/rules/`
  - 用户级：放到 Cursor 的 User Rules（Cursor UI 或其对应的全局规则目录；按 Cursor 官方文档为准）
- 建议把“团队共识”放项目规则，把“个人偏好”放用户规则/本机规则。

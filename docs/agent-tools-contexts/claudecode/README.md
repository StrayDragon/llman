# Claude Code（Anthropic）配置与上下文

本页关注 Claude Code 的**配置入口**、**上下文/指令文件**、**解析/优先级顺序**、**MCP 扩展**，并说明如何与本仓库的 `prompt/`、`skills/` 对接。

证据：`docs/agent-tools-contexts/_raw/claudecode/`

## 1) 配置入口与优先级（Settings）

Claude Code 的 settings 支持多 scope；高 scope 覆盖低 scope，优先级为：Managed → CLI args → Local project → Shared project → User。参见 `docs/agent-tools-contexts/_raw/claudecode/docs.claude.com__claude-code__settings__2026-02-05.md`。

常见配置文件位置（官方表格节选）：

- 用户：`~/.claude/settings.json`
- 项目（共享）：`.claude/settings.json`
- 项目（本机，不提交）：`.claude/settings.local.json`
- 内部状态：`~/.claude.json`（包含部分 MCP/approved-tools/plugins 的用户/本机存储）

建议落地（仓库内）：

- 提交到 git：`.claude/settings.json`
- 不提交到 git：`.claude/settings.local.json`（放机密、个人偏好）

## 2) 记忆/指令文件（CLAUDE.md）

Memory 会在 Claude Code 启动时注入到对话开头，并且**会从当前工作目录向上递归查找** `CLAUDE.md`（允许子项目有自己的记忆）。参见 `docs/agent-tools-contexts/_raw/claudecode/docs.anthropic.com__claude-code__memory__2026-02-05.md`。

官方列出的 3 类 memory 位置：

- 项目：`CLAUDE.md`（项目根，团队共享）
- 用户：`~/.claude/CLAUDE.md`（所有项目共享）
- 项目本机：`CLAUDE.local.md`（不提交）

另外，memory 文件内可用 `# <path>` 形式导入其它文件（原文见上面的 raw 文件）。

## 3) Slash Commands（自定义命令）

自定义命令目录（Markdown 文件）：

- 项目命令：`.claude/commands/`（团队共享）
- 个人命令：`~/.claude/commands/`（仅自己）

参见 `docs/agent-tools-contexts/_raw/claudecode/docs.anthropic.com__claude-code__slash-commands__2026-02-05.md`。

## 4) MCP（外部工具）

Claude Code 通过 MCP 连接外部工具，提供 `claude mcp add/list/remove` 等命令，并可用 `--scope project` 写入项目级配置。参见 `docs/agent-tools-contexts/_raw/claudecode/docs.anthropic.com__claude-code__mcp__2026-02-05.md`。

与 settings 结合时，官方说明 MCP server 的存储分布为：

- 用户 + 本机：`~/.claude.json`
- 项目：`.mcp.json`
- Managed：`managed-mcp.json`

参见 `docs/agent-tools-contexts/_raw/claudecode/docs.claude.com__claude-code__settings__2026-02-05.md`。

## 5) 与本仓库的对接

- `prompt/claude-code/`：当前仅占位。建议在你的“业务仓库”补齐 `CLAUDE.md` + `.claude/settings.json` + `.claude/commands/`，并把可复用片段同步回这里维护。
- `claude-code.toml`：当前包含多组 `ANTHROPIC_*` / `CLAUDE_CODE_*` 环境变量 profiles（其中包含敏感 token）。建议：
  - 仅把**键名/语义**写入文档；不要把值提交到公开仓库；
  - 在运行 `claude` 前，通过脚本/direnv/密码管理器注入对应 env。

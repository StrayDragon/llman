## Why
- `llman prompts` 目前只支持为 Cursor 生成规则文件；想在同一套 prompt 管理工作流里，把 Codex 与 Claude Code 也纳入可注入目标。
- 文档与提示信息仍混用 `llman prompt` 与 `llman prompts`；需要统一对外呈现并保留兼容别名，降低学习成本与误用概率。

## What Changes
- 将 `llman prompts` 作为对外主命令，并保证 `llman prompt` 作为别名继续可用；更新帮助文本与 i18n hint，默认展示 `llman prompts`。
- 扩展 `llman prompts` 的 `--app` 支持范围：`cursor`（保持现状）、`codex`、`claude-code`。
- 为新增 app 提供“注入到目标工具”能力：
  - Codex：将模板导出为 Codex Custom Prompts（Markdown），支持写入：
    - 用户目录：`~/.codex/prompts/`
    - 项目目录：`<repo_root>/.codex/prompts/`
  - Claude Code：将模板内容注入到 Claude Code 的 memory file（以托管块方式更新，保留用户自定义内容），支持写入：
    - 用户目录：`~/.claude/CLAUDE.md`
    - 项目目录：`<repo_root>/CLAUDE.md`
- 明确冲突与覆盖策略：目标文件存在时，默认需要交互确认；非交互模式需显式 `--force` 或等价策略。
 - 增加目标范围选择：为 `codex` / `claude-code` 的注入新增 `--scope user|project|all`（默认 `project`），以便同时覆盖用户与仓库级配置。

## Impact
- Specs：新增 `prompts-management` 规范增量（命令别名、app 支持、Codex/Claude Code 注入行为）。
- Code：扩展 prompt 管理的 app 列表、输出路径与文件格式映射；完善覆盖/冲突处理；更新 CLI help 与 i18n hint。
- Docs：README 与计划文档中的示例命令统一为 `llman prompts`（保留 `llman prompt` 作为兼容别名说明）。

## Goals
- 让 `llman prompts` 成为“模板存储 + 目标工具注入”的统一入口，支持 Cursor / Codex / Claude Code 三类目标。
- 保持行为可预期：默认不破坏用户已有文件；重复执行应幂等（尤其是 Claude Code 的 `CLAUDE.md` 注入）。

## Non-goals
- 不在本变更中引入 Claude Code 的自定义 slash commands / plugins 体系。
- 不将 Codex Custom Prompts 与 llman skills 管理强绑定（尽管 Codex 官方更推荐 skills）。

## Key Decisions
### 1) Canonical command name
- 对外文档与提示统一使用 `llman prompts`。
- 保留 `llman prompt` 作为别名以兼容历史文档与用户习惯。

### 2) Template storage vs injection targets
llman 内部模板继续存放于 `LLMAN_CONFIG_DIR/prompt/<app>/`，`gen` 的职责是把模板“注入”到各工具期望的位置：
- **Cursor**：保持现有 `.cursor/rules/*.mdc` 行为不变。
- **Codex**：写入 Custom Prompts（显式调用的 `/prompts:<name>`），支持两种 scope：
  - user scope：`~/.codex/prompts/*.md`
  - project scope：`<repo_root>/.codex/prompts/*.md`
  - 约束：Codex 只扫描该目录顶层 Markdown 文件；llman 不应创建子目录或写入非 `.md` 文件。
  - 备注：Codex Custom Prompts 已被标记 deprecated；本实现只作为兼容能力，推荐用户使用 llman skills 管理输出到 `.codex/skills`。
- **Claude Code**：写入 memory file（启动加载附加指令），支持两种 scope：
  - user scope：`~/.claude/CLAUDE.md`
  - project scope：`<repo_root>/CLAUDE.md`
  - 采用托管块（managed block）策略：仅更新 llman 管理的区段，保留用户手写内容；重复执行不产生重复区段。

### 3) Ownership and conflict strategy
- 对 Codex：目标 `.md` 文件视为“单文件托管”；存在冲突时提示覆盖或要求 `--force`。
- 对 Claude Code：以托管块边界为准；不覆盖文件中非托管内容。

## Confirmed During Review
- `claude-code` 注入 MUST 同时支持 user scope 与 project scope。
- Codex prompts 注入 MUST 同时支持 user scope 与 project scope。

## Why

Codex 的 subagent/custom agents 配置分散在用户的 Codex 配置目录（例如 `~/.codex/agents/` 或 `CODEX_HOME/agents/`）中，难以在多机器、多项目间统一管理与复用。

我们需要把这些 agent 配置纳入 llman 配置目录集中托管，并提供安全、可重复的同步能力，同时复用 llman prompts 的模板片段来注入到 agent 的 `developer_instructions`，以便统一团队/个人的工作姿态与约束。

## What Changes

- 新增 `llman x codex agents` 命令组，用于管理 Codex `agents/*.toml`：
  - `import`：从目标 Codex agents 目录导入（纳入 llman 托管目录）
  - `sync`：从 llman 托管目录同步到目标 Codex agents 目录（默认逐文件软链接；可选复制覆盖）
  - `inject`：把 llman prompts 模板片段注入到托管的 agent TOML 的 `developer_instructions`（marker 管理）
  - `status`：只读展示托管/目标差异与可注入性提示
- 安全开关：为写操作提供 `--dry-run` 预览计划，以及在非交互环境下用 `--yes/--force` 显式确认执行。
- 交互向导：在交互环境下运行 `llman x codex agents`（无子命令）时使用 inquire 引导选择操作与参数。
- 在 llman 配置目录新增托管位置：`$LLMAN_CONFIG_DIR/codex/agents/`（source of truth）。
- 冲突处理：当导入/同步遇到已有文件且不是预期链接/来源时，默认先备份再覆盖，以降低误操作风险。
- 新增与更新相关的单元测试，确保在 `TempDir`/`CODEX_HOME` 下运行，不触碰真实用户配置。

## Capabilities

### New Capabilities
- `codex-agents-management`: 在 llman 中集中托管并同步 Codex `agents/*.toml`，支持 import/sync 与基于 llman prompts 的模板注入。

### Modified Capabilities
- （无）

## Impact

- CLI：新增 `llman x codex agents ...` 子命令与帮助文案。
- 文件系统：读写/创建 `agents/*.toml`，以及创建 symlink 或执行复制覆盖；需要谨慎处理备份与路径校验。
- prompts：读取 `$LLMAN_CONFIG_DIR/prompt/codex/*.md` 模板片段，并以 marker 注入到 TOML 字符串字段。
- 测试：需要覆盖 Unix 下的 symlink 行为与跨平台的 copy 行为；通过 `CODEX_HOME` 覆盖目标目录避免污染。

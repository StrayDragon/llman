## Context

当前 llman 已支持：
- `llman x codex` 的 provider 配置管理与运行（写入/更新 `~/.codex/config.toml` 的 provider 段落）。
- `llman prompts` 的模板管理与注入（包括 Codex 的 `AGENTS.md` / `AGENTS.override.md` 以及 `prompts/*.md`）。

但 Codex 的 subagent/custom agents 配置（`<codex_home>/agents/*.toml`）仍需要用户手动维护，且常常需要在多台机器、多项目间复用一致的 agent 配置与 `developer_instructions` 片段（例如统一的工作姿态、工具使用约束、风格约束）。

此变更引入一个新的命令组：在 llman 配置目录内集中托管 Codex agents，并提供 import/sync/inject，最大化复用现有 llman prompts 模板体系。

约束：
- 不能在测试/开发命令中触碰真实用户配置：测试需使用 `TempDir` + `CODEX_HOME` 覆盖目标目录。
- Windows 支持是 partial：设计上不强行依赖 symlink，提供 copy 作为兜底路径。

## Goals / Non-Goals

**Goals:**
- 在 `$LLMAN_CONFIG_DIR/codex/agents/` 集中托管 `agents/*.toml`（source of truth）。
- 提供 `import` 把现有 `agents/*.toml` 纳入托管目录。
- 提供 `status` 只读检查，展示托管/目标差异与注入可行性提示。
- 提供 `sync` 把托管目录同步到目标 Codex agents 目录：
  - 默认逐文件 symlink（不替换整个目录）
  - 冲突时默认“备份后覆盖”
  - 可选 copy 模式（便于不支持 symlink 的环境）
- 提供 `inject`：把 llman prompts（`prompt/codex/*.md`）片段注入到 agent TOML 的 `developer_instructions` 字符串中，并用 marker 进行幂等更新。
- 提供安全开关：`--dry-run` 预览计划；非交互环境下用 `--yes/--force` 显式确认写操作。
- 提供交互向导：在交互环境下用 inquire 选择操作与参数并执行。

**Non-Goals:**
- 不接管 Codex 的 `~/.codex/config.toml` 全量管理（仍由现有 `llman x codex account/run` 负责 provider）。
- 不尝试解析/修改 Codex 运行时生成的 state/log/sqlite 文件。
- 不引入新的模板体系；复用现有 llman prompts 的 codex 模板文件即可。

## Decisions

1) **CLI 入口：`llman x codex agents`**

- 在 `llman x codex` 下增加 `agents` 子命令组：
  - `agents import`
  - `agents sync`
  - `agents inject`
  - `agents status`
- 与现有 `llman x codex account/run/stats` 保持并列，避免破坏现有用户习惯。

2) **托管目录与目标目录的路径约定**

- 托管目录（source of truth）：`$LLMAN_CONFIG_DIR/codex/agents/`
- 目标目录默认：`$CODEX_HOME/agents/`；若未设置 `CODEX_HOME`，则使用 `~/.codex/agents/`
- 提供参数覆盖：
  - `--codex-home` 覆盖 `CODEX_HOME` 推导的 home
  - `--agents-dir` 直接指定目标 agents 目录（优先级最高）
  - `--managed-dir` 覆盖托管目录（便于测试与高级用法）

3) **同步默认策略：逐文件 symlink + 冲突备份覆盖**

- 默认 `sync` 使用逐文件 symlink（仅影响托管列表中的 `*.toml`，不替换整个目标目录）。
- 当目标存在同名普通文件（或 symlink 指向非托管路径）：
  - 先备份为 `*.llman.bak.<timestamp>`
  - 再替换为正确的 symlink / 或 copy 覆盖

4) **模板注入策略：注入到 `developer_instructions`，marker 幂等管理**

- 仅在 `developer_instructions = \"\"\"...\"\"\"` 中注入 llman prompts 片段（真正影响 agent 行为）。
- 使用与现有 prompt 注入一致的 marker：
  - `<!-- LLMAN-PROMPTS:START -->`
  - `<!-- LLMAN-PROMPTS:END -->`
- 若 `developer_instructions` 已有 marker，则替换 marker 区块；若没有则追加。
- 若某些 toml（例如 `agents/defaults.toml`）不包含 `developer_instructions`，则跳过并提示。

5) **避免整文件 TOML 重写：基于文本的局部更新**

- 选择在 `inject` 中做“字符串区块替换”而不是 toml parse + pretty-print：
  - 优点：最大限度保留用户原始格式、注释与字段顺序。
  - 风险：对极端格式（非三引号字符串、拼接、转义异常）更敏感。
  - 缓解：限定支持范围为常见的 `developer_instructions = \"\"\"...\"\"\"`，并通过测试覆盖 marker 更新与插入。

6) **安全开关：dry-run + 显式确认**

- 所有会产生写操作的命令（import/sync/inject）都提供 `--dry-run`：输出计划但不落盘。
- 在非交互环境下，写操作需要 `--yes/--force` 显式确认；交互环境下未确认时弹出确认提示。

7) **交互向导：无子命令进入 inquire 流程**

- 当用户在交互环境运行 `llman x codex agents`（无子命令）时，进入向导：
  - 选择操作（status/import/inject/sync）
  - 根据操作 MultiSelect 选择 agents 文件与/或 prompts 模板，并选择 sync 模式（link/copy）
  - 在执行写操作前展示计划并确认

## Risks / Trade-offs

- [误覆盖用户手写配置] → 默认备份再覆盖；提供 `--only` 精确选择；输出清晰日志说明。
- [symlink 在部分环境不可用] → 提供 copy 模式；在不支持的平台自动报错并建议使用 copy。
- [基于文本的注入对格式敏感] → 明确支持范围，出现无法解析时跳过并提示；通过单元测试覆盖主路径。

## Migration Plan

- 无现有行为迁移（新增命令组）。
- 用户可选择：
  1) `import` 将现有 agents 纳入托管目录；
  2) 在托管目录内编辑；
  3) `inject` 注入模板片段；
  4) `sync` 发布到目标 Codex 目录。
- 回滚：删除目标 symlink 并恢复备份文件即可（备份默认同目录保存）。

## Open Questions

- 是否需要在 `sync` 中提供“删除目标多余文件”的收敛能力？（本变更先不做，避免误删风险）
- 是否需要支持更复杂的 `developer_instructions` 形态（非三引号字符串）？（先保持最小可用范围）

## Why
当前 `llman skills` 交互入口直接暴露 `target id`（如 `claude_user`、`codex_user`），对用户来说不直观，也无法直接按“工具 + 范围”去管理技能。

在实际使用中，用户希望把高频技能下沉到项目范围，降低全局技能数量对上下文的影响；同时 Claude Code 与 Codex 的 scope 术语并不完全一致（如 Personal/User 与 Project/Repo），需要在交互层明确表达。

另外，Codex 与 Claude 的官方技能文档都强调“个人范围 + 项目范围”并存的工作方式，因此 `llman skills` 需要提供对应的可管理入口。

## What Changes
- 将 `llman skills` 交互流程从“按 target 选择”调整为“按 agent 工具 → scope 范围 → skills 列表”。
- 新增按 agent 语义展示 scope 文案：
  - Claude: `Personal (All your projects)` / `Project (This project only)`
  - Codex: `User (All your projects)` / `Repo (This project only)`
  - AgentSkills: `_agentskills_` 下的全局范围
- 在缺省配置（无 `config.toml`）下，补齐 Claude/Codex 的项目范围默认 target（仓库内可写，非仓库只读）。
- Codex 用户级路径优先对齐 `.agents/skills`，并为已有 `.codex/skills` 保留兼容回退。
- 保持 `config.toml` v2 与 `registry.json` 数据结构不变，避免引入迁移成本。

## Scope Boundaries
- 包含：`llman skills` 交互式入口、默认 target 解析、相关文案与测试。
- 不包含：
  - Admin/System 级 skills 管理（如 `/etc/codex/skills`、内置 SYSTEM）
  - `skills` 扫描源模型重构（仍保持 `<skills_root>` 单一来源扫描）
  - 非交互模式新参数设计

## Impact
- 受影响规范：`skills-management`
- 受影响代码（预期）：
  - `src/skills/cli/command.rs`
  - `src/skills/config/mod.rs`
  - `src/skills/shared/git.rs`（复用仓库根解析）
  - `locales/app.yml`
  - `tests/skills_integration_tests.rs`
  - `src/skills/config/mod.rs`（单元测试）

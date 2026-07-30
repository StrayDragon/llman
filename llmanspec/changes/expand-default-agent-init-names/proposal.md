---
depends_on: []
---

## Why

`tool-agents-md-management` (r121) 的内置默认扫描清单只覆盖 5 项，结合 2025 年主流 AI coding agent 的 init 文件约定调研，存在 2 个明显缺口和 1 个需补的目录项：

1. **`.cursorrules` 已被 Cursor 官方标记 deprecated**，当前推荐 `.cursor/rules/*.mdc`（[来源](https://forum.cursor.com/t/46934)）。我们的命令已支持目录扫描，应把 `.cursor/` 目录纳入默认清单。
2. **Windsurf/Codeium 用 `.windsurfrules`**（项目根单文件，[来源](https://www.claudemdeditor.com/windsurfrules-guide)），与 `.cursorrules` 同类，当前完全未覆盖。
3. **Claude Code 除根 `CLAUDE.md` 外，还在 `.claude/` 下生成 agents/commands 文件**，这些正是个人 PR 开发时易被 agent 改写的，应纳入。

## What Changes

- 扩展 `default_agent_init_names()` 默认清单，新增 `.cursor/`、`.claude/`、`.windsurfrules` 三项（保留 `.cursorrules` 兼容存量项目）。
- 更新 r121 statement 的内置默认清单枚举。
- 补单测断言新增项存在。

## Capabilities

- `tool-agents-md-management`（r121：默认清单枚举精确化）

## Impact

- **改**：`src/tool/config.rs` 的 `default_agent_init_names()`、r121 statement。
- **测试**：`src/tool/config.rs` 单测 `test_default_agent_init_names_includes_common` 补断言。
- **不改**：scan/clean/revert 行为逻辑、config schema 结构、命令接口。

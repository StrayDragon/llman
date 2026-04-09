## Why

`llman skills` 当前支持三种 agent tools: `claude`、`codex` 和 `agent`（显示为 `_agentskills_`）。其中 `agent` 对应的 `_agentskills_` 目录结构是一个遗留设计，目前仅提供"全局"单一作用域，与 `claude` 和 `codex` 的用户/项目双层作用域设计不一致。移除 `_agentskills_` 支持可以简化代码，降低维护成本，同时不影响核心功能。

## What Changes

- **删除** `skills-management` 规范中关于 `agent` / `_agentskills_` 的所有 requirement 和 scenario
- **删除** `src/skills/config/mod.rs` 中 `agent_global` target 的默认配置
- **删除** `src/skills/cli/command.rs` 中所有 `agent` 特殊处理逻辑：
  - `display_agent_label()` 中的 `"agent" => "_agentskills_"` 分支
  - `display_scope_label()` 中的 `("agent", "global")` 分支
  - `agent_order()` 中的 `"agent" => 2` 分支
  - `scope_order()` 中的 `("agent", "global")` 分支
  - `selectable_agents()` 中的 `agent == "agent"` 特殊判断
  - `scopes_for_agent()` 中跳过 scope 选择直接返回的逻辑
- **删除** 相关单元测试
- **BREAKING** 用户如果使用了 `~/.skills` 目录下的技能链接，将不再被 `llman skills` 管理

## Capabilities

### New Capabilities

（无新功能）

### Modified Capabilities

- `skills-management`: 移除所有 `agent` / `_agentskills_` 相关 requirement（agent 菜单项、global scope 展示等），保留 `claude` 和 `codex` 的完整交互流程

## Impact

- **受影响规范**: `openspec/specs/skills-management/spec.md`
- **受影响代码**:
  - `src/skills/config/mod.rs` - 移除 `agent_global` 默认 target
  - `src/skills/cli/command.rs` - 移除 `agent` 相关的 display label、ordering、selection 逻辑及测试
  - `src/skills/cli/command.rs:449` - 移除 `agent == "agent" && scope_choices.len() == 1` 特殊路径

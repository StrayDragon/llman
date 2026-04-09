## Context

`llman skills` 命令支持管理三种 agent tools 的技能链接：`claude`、`codex`、`agent`（显示为 `_agentskills_`）。其中 `agent` 对应 `~/.skills` 全局目录，仅有 `global` 一个 scope，与 `claude`/`codex` 的 user/project 双层作用域设计不一致，且该功能使用率极低。

## Goals / Non-Goals

**Goals:**
- 移除 `agent` / `_agentskills_` 相关逻辑，简化代码
- 保留 `claude` 与 `codex` 的完整双 scope 支持
- 保持 `skills-management` 规范的内部一致性

**Non-Goals:**
- 不改变 `claude` 与 `codex` 的任何行为
- 不修改 `llman x claude-code` 与 `llman x codex` 命令

## Decisions

1. **移除 `agent` 默认 target**
   - `src/skills/config/mod.rs` 中的 `default_targets()` 函数在构建默认 targets 时不再包含 `agent_global`
   - 理由：`~/.skills` 目录结构是遗留设计，无实际使用场景

2. **移除 `agent` 特殊 display 逻辑**
   - `display_agent_label()` 不再处理 `"agent"` → `"_agentskills_"` 的映射
   - `display_scope_label()` 不再处理 `("agent", "global")` 分支
   - `agent_order()` / `scope_order()` 移除 `agent` 相关条目
   - 理由：这些逻辑仅服务于已删除的 agent target，无需保留

3. **移除 `agent` 选择流程特殊路径**
   - `src/skills/cli/command.rs:449` 的 `if agent == "agent" && scope_choices.len() == 1` 分支删除
   - 理由：该分支用于 agent 的"跳过 scope 选择直接进入技能多选"的 UX，与保留的 claude/codex 流程不同

4. **更新规范文档**
   - `openspec/specs/skills-management/spec.md` 中移除所有 `_agentskills_` / `agent` / `global` 相关 requirement 和 scenario
   - 理由：规范必须与实现保持同步

## Risks / Trade-offs

- **风险**: 现有 `~/.skills` 下有技能链接的用户会受到影响
- **缓解**: 这是一个破坏性变更，但 `~/.skills` 本身已极少使用，且规范文档中已标记为 BREAKING

- **风险**: 相关单元测试需要同步修改
- **缓解**: 测试修改范围明确（仅涉及 agent 相关的 assertion）

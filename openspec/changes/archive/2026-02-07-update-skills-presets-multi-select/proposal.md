# Proposal: update-skills-presets-multi-select

## Why

`llman skills` 预设能力已经可用，但在真实使用中仍有三个高频痛点：

1. 预设入口被单独放在第一层模式菜单，打断原本“agent -> scope -> skills”的主流程；
2. 分组行不可选，无法一键批量勾选一组技能；
3. 技能多选列表中只展示单一标识时，可读性不足，用户不易确认“技能 id”与“目录来源”的对应关系。

## What Changes

- 移除 `Select mode` 第一层菜单，恢复为直接 `agent -> scope -> skills`；
- 在 skills 多选列表内引入可选分组节点：
  - 来自 `registry.presets` 的配置化预设；
  - 来自目录名分组（`<group>.<name>`）的自动分组预设；
- 选择分组行时自动展开为该分组下的技能集合，实现批量勾选效果；
- 使用 `ratatui` 替代原 `inquire` 的 skills 多选界面，支持 preset 三态显示（`[ ]` / `[x]` / `[-]`）；
- 默认状态按“全集命中”推导分组项是否勾选，不再盲目全勾；
- 在树形选择中支持关键字搜索过滤，提升技能较多时的定位效率；
- 保持取消/空选择为安全 no-op（不产生任何变更）；
- 在交互式技能条目中统一展示 `skill_id (directory_name)`，例如 `brainstorming (superpowers.brainstorming)`。

## Impact

- 受影响 specs: `skills-management`
- 受影响代码（预期）:
  - `src/skills/cli/command.rs`
  - `locales/app.yml`

## Non-Goals

- 不引入新的 presets CLI 参数；
- 不新增 presets 的 CRUD 命令；
- 不改变 `registry.json` 的写入策略。

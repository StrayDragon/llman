# 更新 Skills 管理器默认 Agent Targets

## Why
- "codex" 作为技能管理器的 agent 类别过于绑定特定工具，而 `.agents` 目录约定更通用
- 移除 codex 默认 target，加入 `.agents/skills` 作为项目级默认 target
- Scope 选择应依据上下文动态排序：在 git 仓库内 project scope 优先，否则 global scope 优先

## What Changes
1. 移除 codex 默认 target（`codex_user`、`codex_repo`）及相关代码
2. 新增 `.agents/skills` 作为 agents/project 默认 target
3. Scope 选项根据 `env::current_dir()` 动态排序（git repo 内 project 优先，否则 user 优先）
4. 更新交互菜单 label：移除 codex 选项，显示 `.agents` 选项
5. 更新默认 target 测试用例

## Capabilities
- `skills-management`

## Impact
- 默认 targets 从 4 个减少到 3 个（claude user/project + agents project）
- 用户若依赖旧 codex 默认 target，需自行在 `config.toml` 配置
- 交互式 scope 选择顺序动态变化，但所有 scope 仍可访问

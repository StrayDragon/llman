# Design: update-skills-agent-targets

## 权衡
- 移除 codex 默认 target：用户若仍需要 `~/.codex/skills` 同步，需在 `config.toml` 中手动配置；默认不再支持 codex agent
- `.agents/skills` 使用 Copy mode（与 claude project 一致），确保技能文件在项目目录中持久化
- 动态 scope 排序仅影响交互菜单的选项顺序，不影响实际功能；用户始终可访问所有 scope

## Scope 排序策略
- `should_prefer_project_scope()` 检测逻辑：
  1. 若 `env::current_dir() == home_dir` → 返回 false（user 优先）
  2. 若 `find_git_root(cwd).is_some()` → 返回 true（project 优先）
  3. 否则返回 false（user 优先）

## 迁移
- 无向后兼容性问题；codex 默认 target 仅影响新安装/重置配置的用户

# Tasks: update-skills-agent-targets

- [x] 移除 `default_codex_user_dir_with()` 函数和 `codex_home` 参数
- [x] 从 `default_targets_with()` 中移除 `codex_user`、`codex_repo` targets
- [x] 新增 `agents_project` 默认 target（`.agents/skills`）
- [x] 更新交互菜单 label（`display_scope_label`）移除 codex、添加 agents
- [x] 更新 `agent_order` 和 `scope_order` 支持 agents
- [x] 实现动态 scope 排序（`should_prefer_project_scope()`）
- [x] 更新 unit tests 适配变更
- [x] 更新 integration tests 适配变更
- [x] `cargo +nightly test` 全部通过
- [x] 创建 delta spec 并校验
- [x] 归档变更并合并到主 spec
- [x] `cargo +nightly test` 最终校验
- [x] git commit

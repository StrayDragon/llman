## 1. Specification
- [x] 1.1 更新 `skills-management` 规范：交互入口改为 agent -> scope -> skills
- [x] 1.2 明确默认 target 扩展（Claude/Codex 项目范围）与 Codex `.agents/skills` 路径语义

## 2. Interactive Flow Refactor
- [x] 2.1 重构 `llman skills` 交互菜单，新增 agent 选择与 scope 选择步骤
- [x] 2.2 为 Claude/Codex/AgentSkills 提供 scope 展示文案映射与回退策略
- [x] 2.3 保持确认前 no-op、确认后仅同步单 target 差异

## 3. Default Targets and Path Resolution
- [x] 3.1 扩展默认 target：Claude(user/project)、Codex(user/repo)、AgentSkills(global)
- [x] 3.2 实现 repo 根目录解析下的项目路径决策（非 repo 时 project/repo 只读）
- [x] 3.3 Codex user 路径优先 `.agents/skills`，兼容 `.codex/skills` 回退

## 4. Tests and Messages
- [x] 4.1 更新 `locales/app.yml` 的交互文案（agent/scope/skills 三段提示）
- [x] 4.2 新增/更新 `src/skills/config/mod.rs` 单元测试覆盖默认 target 与路径解析
- [x] 4.3 新增/更新 `tests/skills_integration_tests.rs` 覆盖非交互回归与关键交互路径

## 5. Verification
- [x] 5.1 `cargo +nightly test --all skills`（或等价子集）
- [x] 5.2 `cargo +nightly test --all`
- [x] 5.3 `openspec validate update-skills-agent-scope-selection --strict --no-interactive`

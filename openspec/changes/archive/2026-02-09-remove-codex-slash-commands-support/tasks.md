## 1. CLI 行为与错误语义收敛

- [x] 1.1 调整 `src/sdd/project/update_skills.rs`：commands 生成阶段仅对 Claude 生效，Codex 不再写入 `.codex/prompts/opsx-*.md`。
- [x] 1.2 为 `--commands-only` + `--tool codex`（或不含 Claude 的等价组合）增加显式错误分支与本地化错误文案，提示改用 `--skills-only` 或改选 Claude。
- [x] 1.3 更新 `src/sdd/command.rs` 中 `update-skills` 相关参数帮助文本，去除“Codex commands/prompts”暗示，保持与新语义一致。

## 2. 模板与规范同步

- [x] 2.1 更新 `templates/sdd/en/agents.md` 与 `templates/sdd/zh-Hans/agents.md`，移除“Codex slash commands”推荐，改为“Codex 使用 skills”。
- [x] 2.2 更新 `templates/sdd/en/skills/shared.md` 与 `templates/sdd/zh-Hans/skills/shared.md` 的 OPSX quickstart：仅保留 Claude commands 路径，补充 Codex 无 commands 绑定说明。
- [x] 2.3 同步 `openspec/specs/sdd-workflow/spec.md`（归档前由本变更 delta 驱动）以反映 Codex commands 移除后的行为基线。

## 3. 测试回归与验证

- [x] 3.1 修改 `src/sdd/project/update_skills.rs` 相关单元测试：删除/改造“Codex 写入 `.codex/prompts`”断言，新增“Codex 不生成 commands”与“commands-only + codex 报错”覆盖。
- [x] 3.2 更新 `tests/sdd_integration_tests.rs` 中与 `update-skills --tool codex` 相关断言，确保仅检查 skills 输出。
- [x] 3.3 运行最小相关测试（`cargo +nightly test sdd_update_skills` 或等价）并记录结果；如有必要再跑 `just test` 做全量回归。

# Tasks

- [x] 1. `ScenarioEntry` 新增 `feature` 字段（默认 true），追加到 IR + parser + backend
- [x] 2. spec.toon 结构回归：移除 BDD-on valid_scope 豁免、空 requirements 特判，统一 spec 校验路径
- [x] 3. 移除 `spec_dir_as_scope()`（validate.rs）
- [x] 4. 新增 `llman sdd solidify <change-id> [--dry-run]` 命令
- [x] 5. Solidify 核心逻辑：自指检测 + feature=false 过滤 + Gherkin 按场景名写入
- [x] 6. 新增 `llman sdd project solidify-migrate [--dry-run]` 迁移命令
- [x] 7. Archive：删除 `FeatureUpdate`/`find_feature_updates`/`copy_feature_files` 全部代码
- [x] 8. 新建 `.agents/skills/llman-sdd-solidify/SKILL.md`（已由 `llman sdd init --update` 生成）
- [x] 9. 更新 propose/archive/compact/graph skills：移除 `feature_refs` 引导（模板源已更新，skills 已重新生成）
- [x] 10. locales：移除 `feature_as_spec_mode`/`bdd_empty_spec_guardrail` 等废弃 key
- [x] 11. 运行 `solidify-migrate` 迁移现有 BDD-on spec（24 个已迁移，5 个已在前面手动迁移）
- [x] 12. 验证全量 `llman sdd validate --all --strict --no-interactive`（29 passed, 仅本 change 因未归档 tasks pending 报 fail）
- [x] 13. `just check-all` — 480 tests passed, fmt/lint/release/schema/sdd-templates all passed

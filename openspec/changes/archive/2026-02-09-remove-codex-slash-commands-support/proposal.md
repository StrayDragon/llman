## Why

Codex 已将 Custom Prompts（slash commands）标记为弃用，继续在 `llman sdd update-skills` 中生成 `.codex/prompts/opsx-*.md` 会让用户依赖过时入口，并增加维护与迁移成本。我们需要让 llman SDD 的 Codex 集成与上游方向保持一致：以 skills 作为唯一推荐入口。

## What Changes

- 在 `llman sdd update-skills` 中取消 Codex OPSX slash commands（prompts）生成能力。
- 保留并强化 Codex skills 生成；`codex` 目标仅输出 skills，不再写入 `.codex/prompts/`。
- 调整 `--commands-only` / `--skills-only` 与 `--tool codex` 的行为与提示，使其符合“Codex 无 commands 可生成”的新语义。
- 更新 SDD 模板与文案，移除对 Codex slash commands / prompts 路径的指导，避免误导用户。
- 清理或改造与 Codex OPSX prompts 相关的测试与旧迁移逻辑。

## Capabilities

### New Capabilities

（无）

### Modified Capabilities

- `sdd-workflow`: `update-skills` 对 Codex 不再生成 slash commands/prompts，仅生成 skills，并提供清晰的参数约束与错误提示。

## Impact

- 受影响规范：`sdd-workflow`
- 受影响代码：`src/sdd/project/update_skills.rs`, `src/sdd/command.rs`, `templates/sdd/**`, `tests/sdd_integration_tests.rs`, `locales/app.yml`

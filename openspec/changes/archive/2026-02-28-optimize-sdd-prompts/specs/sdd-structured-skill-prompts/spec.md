## ADDED Requirements

### Requirement: 生成提示不得包含占位块或无效引导
SDD 的 new/legacy 双轨技能与 spec-driven 模板渲染结果 MUST 不包含会诱导不稳定行为的占位块或无效引导（例如 “Options / <option …> / What would you like to do?”）。

#### Scenario: update-skills 产物无占位块
- **WHEN** 维护者在同一代码版本下分别运行 new 与 legacy 风格的 `llman sdd update-skills --no-interactive --tool codex`
- **THEN** 生成的任意 `SKILL.md` 不包含子串 `Options:` 或 `<option`
- **AND** 生成的任意 `SKILL.md` 不包含子串 `What would you like to do?`

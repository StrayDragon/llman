## ADDED Requirements

### Requirement: 模板必须避免“影子真源”并保持单一事实来源
SDD 模板体系 MUST 避免保留不参与渲染/生成但容易被误认为“共享真源”的文件（例如历史遗留的 `templates/sdd/*/skills/shared.md`）。共享内容的真源 MUST 位于 `templates/**/units/**` 并通过 MiniJinja 的 `unit()` 注入使用。

#### Scenario: 共享内容仅由 units 承载
- **WHEN** 维护者需要更新多个 SDD skills/spec-driven 模板共享的一段提示内容
- **THEN** 该改动在单个 unit 文件中完成（位于 `templates/**/units/**`）
- **AND** 不要求维护者在多个模板或“共享页”中重复拷贝同一段内容

### Requirement: 渲染后产物不应包含未展开的注入标记
渲染后的 SDD skills/spec-driven 产物 MUST 不包含未展开的模板注入标记（例如 `{{ unit("...") }}`）。

#### Scenario: 生成产物无未展开标记
- **WHEN** 用户运行 `llman sdd update-skills --no-interactive --all`
- **THEN** 生成的 `SKILL.md` 不包含子串 `{{ unit(`

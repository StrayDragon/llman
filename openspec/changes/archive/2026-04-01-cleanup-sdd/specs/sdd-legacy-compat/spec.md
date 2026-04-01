# sdd-legacy-compat Specification (Delta)

## ADDED Requirements

### Requirement: Legacy Track Is Retired
The SDD workflow MUST NOT provide a legacy track (`sdd-legacy`) for templates/skills/prompts.

#### Scenario: User tries to use legacy track
- **WHEN** a user looks for or attempts to use legacy-style SDD commands or templates
- **THEN** the system fails loudly
- **AND** the error explains that only the canonical new-style workflow is supported

## REMOVED Requirements

### Requirement: Legacy Track Must Remain Available
The SDD workflow MUST keep a legacy style track for templates/skills/prompts that users can explicitly select.

**Reason**: 团队已完成迁移并验证 new track。继续保留 legacy track 会造成双轨维护成本与语义漂移风险。

**Migration**: 删除 legacy 轨道后，仅支持 `llman sdd ...` 的 new-style workflow。若仓库仍包含 legacy JSON-in-` ```ison `` payload，必须将其重写为 canonical table/object ISON（`object.spec`/`table.requirements`/`table.scenarios` 与 `object.delta`/`table.ops`/`table.op_scenarios`）。

### Requirement: New Style Default with Explicit Legacy Override
The SDD workflow MUST default to new style generation and behavior unless legacy is explicitly requested.

**Reason**: legacy track 被移除后，不再存在 “显式 legacy override” 的分流需求。

**Migration**: `llman sdd` 始终使用 new-style 语义与 `templates/sdd/**` 模板。

### Requirement: legacy 轨道必须与优化工作流一并维护
当维护者对 new 风格 SDD prompts 做出会影响执行行为的优化时，legacy 轨道 MUST 同步获得等价优化，或 MUST 显式记录两者分歧与理由（避免无意漂移）。

**Reason**: legacy track 被移除后，不再存在双轨同步维护的要求。

**Migration**: 维护者只需维护 `templates/sdd/**` 单轨模板与生成产物。

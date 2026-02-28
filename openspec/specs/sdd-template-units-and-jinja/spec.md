# sdd-template-units-and-jinja Specification

## Purpose
TBD - created by archiving change refactor-sdd-template-units-minijinja. Update Purpose after archive.
## Requirements
### Requirement: SDD Template Units Must Be Independent and Discoverable
SDD prompt composition MUST split reusable prompt fragments into independent template-unit files, with explicit unit identifiers and deterministic lookup rules by locale.

#### Scenario: Unit file can be edited independently
- **WHEN** a maintainer updates one prompt unit used by multiple SDD templates
- **THEN** the change is made in a single unit file without editing unrelated templates

#### Scenario: Locale-scoped unit resolution is deterministic
- **WHEN** the renderer resolves a unit for `zh-Hans` with fallback to `en`
- **THEN** it follows a documented deterministic fallback chain and returns exactly one resolved unit source

### Requirement: SDD Rendering Must Use MiniJinja Injection Contracts
SDD template rendering MUST use MiniJinja-based injection for unit composition, and rendering MUST fail fast on missing unit references or missing required variables.

#### Scenario: Missing unit reference fails rendering
- **WHEN** a template references a unit identifier that does not exist
- **THEN** render operation exits non-zero with a clear missing-unit error

#### Scenario: Missing required render variable fails rendering
- **WHEN** a template requires a render variable that is not provided
- **THEN** render operation exits non-zero and identifies the missing variable

### Requirement: 模板必须避免“影子真源”并保持单一事实来源
SDD 模板体系 MUST 避免保留不参与渲染/生成但容易被误认为“共享真源”的文件（例如历史遗留的 `templates/sdd/*/skills/shared.md`）。共享内容的真源 MUST 位于 `templates/**/units/**` 并通过 MiniJinja 的 `unit()` 注入使用。

#### Scenario: 共享内容仅由 units 承载
- **WHEN** 维护者需要更新多个 SDD skills/spec-driven 模板共享的一段提示内容
- **THEN** 该改动在单个 unit 文件中完成（位于 `templates/**/units/**`）
- **AND** 不要求维护者在多个模板或“共享页”中重复拷贝同一段内容

### Requirement: Rendered Outputs Must Stay Stable and Self-Contained
Rendered SDD templates and skills MUST remain self-contained text artifacts and MUST preserve stable output ordering to reduce maintenance diff noise.

#### Scenario: Generated SKILL output remains self-contained
- **WHEN** user runs `llman sdd update-skills --no-interactive --tool codex`
- **THEN** generated `SKILL.md` files include fully rendered content without unresolved injection markers (for example, `{{ unit(`)

#### Scenario: Stable generation order
- **WHEN** user runs the same generation command twice without source changes
- **THEN** generated file content order is identical across runs

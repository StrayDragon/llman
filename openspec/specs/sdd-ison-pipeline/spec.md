# sdd-ison-pipeline Specification

## Purpose
TBD - created by archiving change add-ison-first-sdd-pipeline. Update Purpose after archive.
## Requirements
### Requirement: ISON-First SDD Template Sources
The SDD template system MUST support ISON source templates as the primary authoring format for the new style track.

#### Scenario: New style template generation reads ISON source
- **WHEN** a maintainer runs SDD template refresh for new style
- **THEN** the system reads ISON source templates
- **AND** renders Markdown outputs used by generated instructions and skills

### Requirement: ISON Validation Before Render
The system MUST validate ISON source templates before rendering outputs.

#### Scenario: Invalid ISON source blocks rendering
- **WHEN** a new style ISON template has structural or type errors
- **THEN** SDD template generation fails with non-zero exit
- **AND** no partial rendered output is written for that failed template

### Requirement: Runtime Spec Parsing Uses ISON Container
The SDD runtime MUST parse `llmanspec` main specs according to the project’s configured `spec_style`, rather than assuming all `spec.md` payloads are ISON.

- for `spec_style: ison`, the runtime MUST parse canonical table/object ISON from fenced ` ```ison ` blocks
- for `spec_style: toon`, the runtime MUST parse one canonical TOON document from a fenced ` ```toon ` block
- for `spec_style: yaml`, the runtime MUST parse one canonical YAML document from a fenced ` ```yaml ` block

The parser MUST select the backend from project config only. It MUST NOT auto-detect another style as a fallback once `spec_style` is declared.

Files MAY include Markdown headings/prose around the canonical payload, but runtime semantics MUST come from the style-matched fenced payload only.

#### Scenario: Show/list/validate parse YAML main spec by configured style
- **WHEN** a user runs SDD commands that read `llmanspec/specs/<capability>/spec.md` in a project with `spec_style: yaml`
- **THEN** the parser extracts and parses the ` ```yaml ` payload as canonical semantic source
- **AND** command behavior does not depend on Markdown heading conventions

#### Scenario: Style mismatch is rejected without fallback
- **WHEN** a project declares `spec_style: yaml`
- **AND** a main spec contains only ` ```ison ` canonical payload
- **THEN** validation fails with non-zero exit
- **AND** the error explains that `yaml` was expected and `ison` was found

#### Scenario: Validation rejects legacy JSON payloads in ison projects
- **WHEN** a user runs validation in a project with `spec_style: ison` on a main spec whose ` ```ison ` payload is JSON
- **THEN** validation fails with non-zero exit
- **AND** the error message includes a concrete hint to rewrite the payload into canonical table/object ISON

### Requirement: Runtime Delta Parsing Uses ISON Ops
The SDD runtime MUST parse change delta specs according to the project’s configured `spec_style`, rather than assuming all delta specs use ISON ops blocks.

- for `spec_style: ison`, the runtime MUST read delta ops from `table.ops` and scenarios from `table.op_scenarios`
- for `spec_style: toon` and `spec_style: yaml`, the runtime MUST read delta ops from canonical `ops` collections and scenarios from canonical `op_scenarios` collections

The runtime MUST key add/modify/remove/rename semantics by structured fields (`req_id`, `id`, `from`, `to`, etc.), not by Markdown section headers or style-specific free-form text.

#### Scenario: Change validation parses YAML ops collection
- **WHEN** a user validates a change in a project with `spec_style: yaml`
- **THEN** delta operations are read from the YAML `ops` collection
- **AND** scenarios for add/modify are read from the YAML `op_scenarios` collection

#### Scenario: Validation rejects legacy delta JSON payloads in ison projects
- **WHEN** a user runs validation in a project with `spec_style: ison` on a delta spec whose ` ```ison ` payload is JSON
- **THEN** validation fails with non-zero exit
- **AND** the error message includes a concrete hint to rewrite the payload into canonical table/object ISON

### Requirement: 多风格解析必须先归一化到共享语义模型
SDD runtime MUST 在风格相关解析完成后，先将主 spec 与 delta spec 归一化到共享语义模型，再驱动：

- `llman sdd list`
- `llman sdd show`
- `llman sdd validate`
- `llman sdd archive`
- `llman sdd spec`
- `llman sdd delta`

命令实现 MUST NOT 为不同风格复制三套独立的需求/场景/op 业务逻辑；风格差异 MUST 仅停留在 envelope parsing 与 serialization 层。

#### Scenario: 不同风格共享同一验证语义
- **WHEN** 同一份 requirement/scenario 语义分别以 `ison` 与 `yaml` 表达
- **THEN** strict validation 对“缺失 scenario”或“重复 `(req_id, id)`”给出相同语义结论


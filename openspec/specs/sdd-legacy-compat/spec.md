# sdd-legacy-compat Specification

## Purpose
TBD - created by archiving change add-ison-first-sdd-pipeline. Update Purpose after archive.
## Requirements
### Requirement: Legacy Track Must Remain Available
The SDD workflow MUST keep a legacy style track for templates/skills/prompts that users can explicitly select.

#### Scenario: User selects legacy style explicitly
- **WHEN** a user runs SDD generation with legacy style option
- **THEN** output is generated from legacy templates
- **AND** no new-style-only validation constraints are enforced on that legacy output path

### Requirement: New Style Default with Explicit Legacy Override
The SDD workflow MUST default to new style generation and behavior unless legacy is explicitly requested.

#### Scenario: No style flag uses new track
- **WHEN** a user runs SDD generation commands without a style selector
- **THEN** the system uses the new style templates by default
- **AND** generated outputs include the new structured governance behavior

### Requirement: legacy 轨道必须与优化工作流一并维护
当维护者对 new 风格 SDD prompts 做出会影响执行行为的优化时，legacy 轨道 MUST 同步获得等价优化，或 MUST 显式记录两者分歧与理由（避免无意漂移）。

#### Scenario: new 与 legacy 同步或显式分歧
- **WHEN** 维护者对 `templates/sdd/**` 做出会影响 workflow 行为的提示词变更（例如 STOP 条件、验证步骤、约束表达）
- **THEN** 维护者同步更新 `templates/sdd-legacy/**` 中的等价提示词
- **OR** 在模板头注释与增量规范中显式记录分歧点与理由

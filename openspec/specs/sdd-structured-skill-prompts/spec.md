# sdd-structured-skill-prompts Specification

## Purpose
TBD - created by archiving change upgrade-sdd-archive-freeze-and-structured-prompts. Update Purpose after archive.
## Requirements
### Requirement: SDD 技能结构化提示协议
llman SDD 的技能模板与 spec-driven 模板 MUST 采用统一结构化提示协议，并通过模板单元注入方式组装协议块，以降低重复维护成本并保持内容一致性。

协议至少包含以下逻辑层：
- `Context`
- `Goal`
- `Constraints`
- `Workflow`
- `Decision Policy`
- `Output Contract`

#### Scenario: 结构化协议由共享单元注入
- **WHEN** 维护者检查 `templates/sdd/{locale}/skills/*.md` 与 `templates/sdd/{locale}/spec-driven/*.md`
- **THEN** 协议章节通过共享模板单元注入而不是手工重复拷贝

#### Scenario: 注入后结构化章节仍完整可见
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** 生成产物中可见完整结构化章节且顺序一致

### Requirement: 协议自包含且无外部技能硬依赖
结构化提示协议 MUST 以内置规则表述，不得要求调用外部技能作为前置依赖。

#### Scenario: 生成内容不引用外部技能作为必需步骤
- **WHEN** 用户执行 `llman sdd update-skills --all`
- **THEN** 生成的 `SKILL.md` 不包含“先调用外部技能再执行”的硬依赖指令

### Requirement: Structured Protocol Includes Ethics Governance Fields
The structured skill prompt protocol for new style MUST include enforceable ethics governance fields.

#### Scenario: New style structured protocol includes governance block
- **WHEN** new style SDD skills are generated
- **THEN** generated content includes governance fields for risk level, prohibited actions, required evidence, refusal contract, and escalation policy

### Requirement: Missing Governance Fields Fail New-Style Validation
New-style validation MUST fail when required ethics governance fields are missing.

#### Scenario: Validation fails on missing governance key
- **WHEN** a new style skill/protocol artifact omits a required ethics governance field
- **THEN** validation returns non-zero with explicit missing-field diagnostics

### Requirement: 生成提示不得包含占位块或无效引导
SDD 的 new/legacy 双轨技能与 spec-driven 模板渲染结果 MUST 不包含会诱导不稳定行为的占位块或无效引导（例如 “Options / <option …> / What would you like to do?”）。

#### Scenario: update-skills 产物无占位块
- **WHEN** 维护者在同一代码版本下分别运行 new 与 legacy 风格的 `llman sdd update-skills --no-interactive --tool codex`
- **THEN** 生成的任意 `SKILL.md` 不包含子串 `Options:` 或 `<option`
- **AND** 生成的任意 `SKILL.md` 不包含子串 `What would you like to do?`

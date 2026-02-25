## ADDED Requirements

### Requirement: SDD 技能结构化提示协议
llman SDD 的技能模板与 spec-driven 模板 MUST 采用统一结构化提示协议，以降低歧义并提升可验证性。

协议至少包含以下逻辑层：
- `Context`
- `Goal`
- `Constraints`
- `Workflow`
- `Decision Policy`
- `Output Contract`

#### Scenario: 结构化章节在模板中存在
- **WHEN** 维护者检查 `templates/sdd/{locale}/skills/*.md`
- **THEN** 每个 workflow skill 模板包含上述结构化章节或等价命名

#### Scenario: spec-driven 命令模板使用同协议
- **WHEN** 维护者检查 `templates/sdd/{locale}/spec-driven/*.md`
- **THEN** 关键命令模板（如 archive/explore/ff/apply）遵循同一结构化层次

### Requirement: 协议自包含且无外部技能硬依赖
结构化提示协议 MUST 以内置规则表述，不得要求调用外部技能作为前置依赖。

#### Scenario: 生成内容不引用外部技能作为必需步骤
- **WHEN** 用户执行 `llman sdd update-skills --all`
- **THEN** 生成的 `SKILL.md` 不包含“先调用外部技能再执行”的硬依赖指令

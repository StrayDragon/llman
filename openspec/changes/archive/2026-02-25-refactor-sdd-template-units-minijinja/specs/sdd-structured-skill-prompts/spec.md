## MODIFIED Requirements

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

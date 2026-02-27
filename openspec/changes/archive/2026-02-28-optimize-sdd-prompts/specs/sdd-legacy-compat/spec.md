## ADDED Requirements

### Requirement: legacy 轨道必须与优化工作流一并维护
当维护者对 new 风格 SDD prompts 做出会影响执行行为的优化时，legacy 轨道 MUST 同步获得等价优化，或 MUST 显式记录两者分歧与理由（避免无意漂移）。

#### Scenario: new 与 legacy 同步或显式分歧
- **WHEN** 维护者对 `templates/sdd/**` 做出会影响 workflow 行为的提示词变更（例如 STOP 条件、验证步骤、约束表达）
- **THEN** 维护者同步更新 `templates/sdd-legacy/**` 中的等价提示词
- **OR** 在模板头注释与增量规范中显式记录分歧点与理由

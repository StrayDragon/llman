## ADDED Requirements
### Requirement: Skills 重构保持行为一致
Skills 模块重构 MUST 保持技能发现、目标链接、冲突处理、registry 记录与 CLI 输出行为一致，且不得改变配置解析优先级与数据格式。

#### Scenario: Skills 重构后回归
- **WHEN** `src/skills/` 的模块结构被重组
- **THEN** `llman skills` 的扫描、链接与 registry 行为保持不变

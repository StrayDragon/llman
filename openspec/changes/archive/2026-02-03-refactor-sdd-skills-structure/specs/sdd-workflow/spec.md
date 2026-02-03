## ADDED Requirements
### Requirement: SDD 重构保持行为一致
SDD 模块重构 MUST 保持所有 `llman sdd` 子命令的行为、输出与退出码一致，并且不得改变模板内容与配置生成路径。

#### Scenario: SDD 重构后回归
- **WHEN** `src/sdd/` 的模块结构被重组
- **THEN** `sdd init/update/update-skills/list/show/validate/archive` 的行为与输出保持不变

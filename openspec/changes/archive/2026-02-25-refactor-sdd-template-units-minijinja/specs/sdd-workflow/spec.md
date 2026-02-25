## MODIFIED Requirements

### Requirement: SDD 模板区域复用
SDD 模板与 skills MUST 使用基于 MiniJinja 的模板单元注入机制进行复用。系统 MUST 通过显式模板单元标识符完成注入渲染，并在缺失单元、重复注册或渲染失败时报错并中止。

#### Scenario: 引用模板单元并成功渲染
- **WHEN** 模板声明注入一个已注册的共享单元
- **THEN** 生成结果中包含该单元的渲染内容且无未解析占位符

#### Scenario: 模板单元缺失
- **WHEN** 模板声明的单元标识符在当前 locale 与回退链中都不存在
- **THEN** 命令报错并退出非零

#### Scenario: 模板单元注册冲突
- **WHEN** 同一渲染上下文中存在重复的单元标识符定义
- **THEN** 命令报错并拒绝继续渲染

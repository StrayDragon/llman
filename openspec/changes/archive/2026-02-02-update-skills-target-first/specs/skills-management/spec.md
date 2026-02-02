## MODIFIED Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 在交互式终端扫描 `<skills_root>` 并进入交互式选择流程：先选择单个 target（`mode=skip` 目标必须展示为不可选），然后为该 target 展示技能多选列表；默认勾选来自该 target 目录内的实际软链接状态。用户确认后，管理器 MUST 对该 target 执行差异同步：新增项创建软链接、取消项移除软链接。命令 MUST NOT 创建或更新 `store/` 快照。

#### Scenario: 交互式先选目标再选技能
- **WHEN** 用户在交互式终端运行 `llman skills`
- **THEN** 管理器先要求选择一个 target，再展示技能多选列表

#### Scenario: 默认勾选来自目标链接
- **WHEN** 目标目录已有指向技能目录的 `<skill_id>` 软链接
- **THEN** 该技能在列表中默认勾选

#### Scenario: 确认后仅同步差异
- **WHEN** 用户确认选择
- **THEN** 管理器仅对该 target 增删变更项

#### Scenario: 取消不产生变更
- **WHEN** 用户在确认前退出或返回
- **THEN** 不修改任何目标链接且不写入 registry

### Requirement: 启用状态持久化
管理器 MUST 在 `<skills_root>/registry.json` 记录用户确认后的技能/目标启用状态。交互模式 MUST 使用目标目录中的链接状态作为默认选择来源，并且在用户确认之前不得写入或更新 registry。非交互模式下，若 `registry.json` 缺失则回退到 `config.toml` 里的 `enabled` 默认值。

#### Scenario: 交互默认来自文件系统
- **WHEN** 交互模式选择某 target，且 `registry.json` 存在不同状态
- **THEN** 默认勾选仍以目标目录链接状态为准

#### Scenario: 交互取消不写入 registry
- **WHEN** 用户退出而未确认
- **THEN** `registry.json` 不被创建或修改

#### Scenario: 确认后持久化状态
- **WHEN** 用户确认应用
- **THEN** `registry.json` 更新为确认后的状态

#### Scenario: 非交互缺省回退配置默认值
- **WHEN** 非交互模式且 `registry.json` 不存在
- **THEN** 使用 `config.toml` 的 `enabled` 默认值

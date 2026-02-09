## ADDED Requirements

### Requirement: 启用状态实时计算
管理器 MUST 不再依赖任何 registry 文件作为技能启用状态来源。交互与非交互流程都 MUST 基于目标目录真实链接状态实时计算技能启用状态，并仅将 `config.toml` 的 `target.enabled` 作为“当前未链接时”的默认回退值。

#### Scenario: 交互默认来自文件系统
- **WHEN** 交互模式选择某 target
- **THEN** 默认勾选基于目标目录的真实链接状态计算

#### Scenario: 非交互已链接优先
- **WHEN** 非交互模式下某技能在某 target 已存在正确链接
- **THEN** 管理器保持该技能在该 target 启用，不因配置默认值覆盖

#### Scenario: 非交互未链接回退配置默认值
- **WHEN** 非交互模式下某技能在某 target 当前未链接
- **THEN** 管理器使用该 target 的 `enabled` 默认值决定是否创建链接

#### Scenario: 运行不写持久化状态文件
- **WHEN** 管理器完成交互或非交互会话
- **THEN** 管理器不会创建或更新任何 registry 状态文件

## MODIFIED Requirements

### Requirement: 交互式技能管理入口
`llman skills` MUST 在交互式终端扫描 `<skills_root>` 后，直接进入既有交互主流程：先选择 agent，再选择 scope，最后进入 skills 多选。`mode=skip` 的 target 必须展示为只读不可切换。用户确认后，管理器 MUST 仅对选定 target 执行差异同步：新增项创建软链接、取消项移除软链接。命令 MUST NOT 创建或更新 `store/` 快照。

#### Scenario: 直接进入 agent 菜单
- **WHEN** 用户运行 `llman skills`
- **THEN** 管理器直接展示 `Select which agent tools to manage`，不再出现 `Select mode`

#### Scenario: Select individually 既有流程保留
- **WHEN** 用户进入交互流程
- **THEN** 管理器按 agent → scope → skills 流程执行

#### Scenario: 取消不产生变更
- **WHEN** 用户在确认前退出或返回
- **THEN** 不修改任何目标链接且不写入持久化状态文件

### Requirement: Skills 重构保持行为一致
Skills 模块重构 MUST 保持技能发现、目标链接、冲突处理与 CLI 输出行为一致，且不得改变配置解析优先级。

#### Scenario: Skills 重构后回归
- **WHEN** `src/skills/` 的模块结构被重组
- **THEN** `llman skills` 的扫描与链接行为保持不变

### Requirement: 冲突提示取消必须是安全 no-op
在交互模式下，如果用户取消冲突处理提示，管理器 MUST 将其视为安全的整体 abort（安全退出），并 MUST NOT 产生部分变更。

#### Scenario: 取消冲突提示
- **WHEN** target 存在冲突条目且用户在 overwrite/skip 提示中取消
- **THEN** 命令整体 abort 且不应用任何变更，并以成功状态退出

### Requirement: 预设来源与默认推断
管理器 MUST 仅支持运行时目录推断预设：MUST 从技能目录名按 `<preset>.<skill>` 规则自动推断分组。推断得到的预设 MUST 仅存在于运行时，不得依赖或写入任何 registry 持久化字段。

#### Scenario: 自动推断默认预设
- **WHEN** 技能目录名为 `superpowers.brainstorming`
- **THEN** 该目录被归入预设 `superpowers`，并以完整目录名作为该预设成员

#### Scenario: 无分段目录归入 ungrouped
- **WHEN** 技能目录名不包含 `.`
- **THEN** 该技能归入 `ungrouped` 分组

### Requirement: skills 列表中的分组节点
技能多选列表 MUST 以树形结构展示可选项：父节点为分组节点，子节点为该分组覆盖的具体技能。分组来源 MUST 仅为基于目录名 `<group>.<name>` 自动推断的分组预设。

选择分组项时，管理器 MUST 将其展开为对应技能集合并去重，最终按技能集合应用到目标。
分组项的默认勾选状态 MUST 由当前默认技能集合推导：仅当该分组覆盖的技能集合全部已在默认集合中时，才显示为勾选；否则 MUST 不勾选。

分组项的可视状态 MUST 支持三态：`[ ]`（未选）、`[x]`（全集选中）、`[-]`（部分选中）。
树形选择 MUST 支持关键字过滤搜索：用户输入关键字后，列表仅展示匹配的分组与技能（匹配技能时其父分组必须保留显示）。

#### Scenario: 选择分组自动展开
- **WHEN** 用户在 skills 列表中选择 `dakesan (3 skills)`
- **THEN** 管理器将 `dakesan` 对应技能集合加入最终选择集合

#### Scenario: 树形父子联动
- **WHEN** 用户在树形列表中切换分组父节点
- **THEN** 该父节点下所有子技能同步选中或取消

#### Scenario: 重叠技能去重
- **WHEN** 用户同时选择多个分组，且它们包含同一技能
- **THEN** 最终应用集合中该技能只保留一份

#### Scenario: 搜索过滤保留父节点
- **WHEN** 用户在树形选择中输入关键字，仅匹配到某个技能
- **THEN** 该技能所属分组仍显示，且仅显示匹配到的子技能

#### Scenario: preset 默认勾选为“全集命中”
- **WHEN** 当前默认集合仅包含某 preset 的部分技能
- **THEN** 该分组默认状态不勾选

## REMOVED Requirements

### Requirement: 启用状态持久化
**Reason**: 技能启用状态改为运行时实时计算，registry 不再作为状态事实来源。
**Migration**: 删除对 `<skills_root>/registry.json` 的依赖；若该文件仍存在，将其视为历史残留并忽略。

### Requirement: Registry 更新必须原子化
**Reason**: 管理器不再写入 registry 文件，因此不再需要 registry 原子写约束。
**Migration**: 无需迁移；删除实现中的 registry 写入路径即可。

### Requirement: 预设继承与解析
**Reason**: 配置化 presets（如 `extends`）被下线，预设来源统一为目录名推断。
**Migration**: 将原有预设组织迁移为目录命名约定（`<group>.<skill>`）。

### Requirement: 启动前预设校验与失败策略
**Reason**: 不再存在用户配置化 presets 输入，启动前的继承/引用校验不再适用。
**Migration**: 保留目录分组推断与运行时展示校验，不再执行 `extends/skill_dirs` 相关校验流程。

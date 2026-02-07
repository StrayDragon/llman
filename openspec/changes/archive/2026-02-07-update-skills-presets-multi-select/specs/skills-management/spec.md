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
- **THEN** 不修改任何目标链接且不写入 registry

## ADDED Requirements
### Requirement: skills 列表中的分组节点
技能多选列表 MUST 以树形结构展示可选项：父节点为分组节点，子节点为该分组覆盖的具体技能。分组来源包含两类：
1) `registry.presets` 中定义的配置化预设；
2) 基于目录名 `<group>.<name>` 自动推断的分组预设。

选择分组项时，管理器 MUST 将其展开为对应技能集合并去重，最终按技能集合应用到目标。
分组项的默认勾选状态 MUST 由当前默认技能集合推导：仅当该分组覆盖的技能集合全部已在默认集合中时，才显示为勾选；否则 MUST 不勾选。

分组项的可视状态 MUST 支持三态：`[ ]`（未选）、`[x]`（全集选中）、`[-]`（部分选中）。
树形选择 MUST 支持关键字过滤搜索：用户输入关键字后，列表仅展示匹配的分组与技能（匹配技能时其父分组必须保留显示）。

#### Scenario: 选择分组自动展开
- **WHEN** 用户在 skills 列表中选择 `dakesan (3 skills)`
- **THEN** 管理器将 `dakesan` 对应技能集合加入最终选择集合

#### Scenario: 配置预设与分组预设并存
- **WHEN** `registry.presets` 包含 `daily`，且目录分组包含 `dakesan`
- **THEN** 列表中同时展示 `daily (...)` 与 `dakesan (...)`

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

### Requirement: 技能条目展示应同时包含 skill_id 与目录名
交互式技能列表中的每个技能项 MUST 展示为 `skill_id (directory_name)`，用于明确用户可选标识与其目录来源。当技能目录分组与 skill_id 不一致时，管理器 MUST 仍按该格式展示。

#### Scenario: 展示 skill_id 与目录名
- **WHEN** 技能 `skill_id` 为 `brainstorming`，目录名为 `superpowers.brainstorming`
- **THEN** 交互选项显示为 `brainstorming (superpowers.brainstorming)`

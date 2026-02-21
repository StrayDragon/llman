# skills-management Specification (Change: add-agent-presets)

## MODIFIED Requirements

### Requirement: 预设来源与默认推断
管理器 MUST 支持两类运行时预设来源：
1) **目录分组预设**：MUST 从技能目录名按 `<preset>.<skill>` 规则自动推断分组。
2) **Agent preset**：MUST 从 `LLMAN_CONFIG_DIR/agents/*/agent.toml` 读取 agent manifest，并将其作为可在交互式 skills 多选中使用的 preset。

两类预设都 MUST 仅存在于运行时：管理器 MUST NOT 依赖或写入任何 registry 持久化字段，且 MUST NOT 写回任何 `agent.toml` 文件。

#### Scenario: 自动推断默认预设
- **WHEN** 技能目录名为 `superpowers.brainstorming`
- **THEN** 该目录被归入预设 `superpowers`，并以完整目录名作为该预设成员

#### Scenario: 无分段目录归入 ungrouped
- **WHEN** 技能目录名不包含 `.`
- **THEN** 该技能归入 `ungrouped` 分组

#### Scenario: agent manifest 作为 preset 来源
- **WHEN** `LLMAN_CONFIG_DIR/agents/foo/agent.toml` 存在且可解析
- **THEN** 交互式 skills 多选列表中出现一个 agent preset 选项 `foo`

#### Scenario: manifest 解析失败时跳过
- **WHEN** `LLMAN_CONFIG_DIR/agents/foo/agent.toml` 存在但不可解析
- **THEN** 命令继续运行且跳过该 agent preset，并输出明确警告

### Requirement: skills 列表中的分组节点
技能多选列表 MUST 以树形结构展示可选项：父节点包括：
- 基于目录名 `<group>.<name>` 自动推断的分组节点
- 基于 `LLMAN_CONFIG_DIR/agents/*/agent.toml` 的 agent preset 节点

选择任一父节点时，管理器 MUST 将其展开为对应技能集合并去重，最终按技能集合应用到目标。
分组/agent preset 节点的默认勾选状态 MUST 由当前默认技能集合推导：仅当该节点覆盖的技能集合全部已在默认集合中时，才显示为勾选；否则 MUST 不勾选。

分组/agent preset 节点的可视状态 MUST 支持三态：`[ ]`（未选）、`[x]`（全集选中）、`[-]`（部分选中）。
树形选择 MUST 支持关键字过滤搜索：用户输入关键字后，列表仅展示匹配的节点与技能（匹配到技能时其父节点必须保留显示）。

#### Scenario: 选择分组自动展开
- **WHEN** 用户在 skills 列表中选择 `dakesan (3 skills)`
- **THEN** 管理器将 `dakesan` 对应技能集合加入最终选择集合

#### Scenario: 选择 agent preset 自动展开
- **WHEN** 用户在 skills 列表中选择 `[agent] foo (3 skills)`
- **THEN** 管理器将 `foo` 与其 manifest `includes` 对应技能集合加入最终选择集合并去重

#### Scenario: agent preset 引用缺失技能时跳过
- **WHEN** 某 agent preset 的 `includes` 包含不存在的 skill_id
- **THEN** 管理器跳过缺失 skill_id，仍对存在的技能正常展开并输出明确警告

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

当某 skill_id 同时存在于 `LLMAN_CONFIG_DIR/agents/*/agent.toml` 的 `id` 集合中时（即 agent-skill），该技能条目 MUST 使用前缀 `[agent]` 标识，但仍 MUST 保留 `skill_id (directory_name)` 信息。

#### Scenario: 展示 skill_id 与目录名
- **WHEN** 技能 `skill_id` 为 `brainstorming`，目录名为 `superpowers.brainstorming`
- **THEN** 交互选项显示为 `brainstorming (superpowers.brainstorming)`

#### Scenario: agent-skill 增加标识
- **WHEN** skill_id 为 `foo` 且存在 `LLMAN_CONFIG_DIR/agents/foo/agent.toml`
- **THEN** 交互选项显示为 `[agent] foo (foo)`

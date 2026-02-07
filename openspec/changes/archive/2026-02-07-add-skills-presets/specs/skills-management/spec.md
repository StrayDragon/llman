# skills-management Specification (Delta)

## MODIFIED Requirements

### Requirement: 交互式技能管理入口
`llman skills` MUST 在交互式终端扫描 `<skills_root>` 后进入“模式选择 + 执行流程”：先选择模式（`Apply preset` 或 `Select individually`），再进入对应流程。`Select individually` 模式 MUST 保持原三段式交互（agent → scope → skills）；`Apply preset` 模式 MUST 先选择预设，再选择 agent 与 scope，并以预设解析结果作为默认勾选。`mode=skip` 的 target 必须展示为只读不可切换。用户确认后，管理器 MUST 仅对选定 target 执行差异同步：新增项创建软链接、取消项移除软链接。命令 MUST NOT 创建或更新 `store/` 快照。

#### Scenario: 有预设时显示模式选择
- **WHEN** 运行时存在可用预设
- **THEN** 管理器先展示 `Apply preset` / `Select individually` / `Exit` 模式菜单

#### Scenario: 无预设时隐藏 Apply preset
- **WHEN** 运行时不存在可用预设
- **THEN** 模式菜单不展示 `Apply preset`

#### Scenario: Select individually 保持既有流程
- **WHEN** 用户选择 `Select individually`
- **THEN** 管理器按既有 agent → scope → skills 流程执行

#### Scenario: Apply preset 进入预设流程
- **WHEN** 用户选择 `Apply preset`
- **THEN** 管理器先选择预设，再进入 agent/scope/确认流程，并以预设结果作为默认勾选

#### Scenario: 取消不产生变更
- **WHEN** 用户在确认前退出或返回
- **THEN** 不修改任何目标链接且不写入 registry

## ADDED Requirements

### Requirement: 预设来源与默认推断
管理器 MUST 支持运行时预设目录：当 `registry.json` 中 `presets` 非空时，MUST 使用其作为预设来源；当 `presets` 为空或不存在时，MUST 从技能目录名按 `<preset>.<skill>` 规则自动推断默认预设。自动推断得到的预设 MUST 仅存在于运行时，MUST NOT 写回 `registry.json`。

#### Scenario: 优先使用 registry 预设
- **WHEN** `registry.json` 包含非空 `presets`
- **THEN** 管理器使用该预设集合，不使用自动推断结果覆盖

#### Scenario: 自动推断默认预设
- **WHEN** `registry.json` 不包含 `presets` 或其为空
- **THEN** 目录名 `superpowers.brainstorming` 被归入预设 `superpowers`，并将完整目录名加入该预设的 `skill_dirs`

#### Scenario: 推断预设不落盘
- **WHEN** 管理器使用自动推断得到默认预设并完成一次会话
- **THEN** `registry.json` 不新增或修改 `presets` 字段

### Requirement: 预设继承与解析
管理器 MUST 支持预设通过 `extends` 继承父预设。解析时 MUST 先递归合并父预设，再合并当前预设的 `skill_dirs`，并对结果去重。

#### Scenario: 预设继承
- **WHEN** 预设 `full-stack` 定义 `extends = "daily"`
- **THEN** 解析 `full-stack` 时先包含 `daily` 的技能，再添加 `full-stack` 自身技能并去重

### Requirement: 启动前预设校验与失败策略
每次执行 `llman skills` 时，管理器 MUST 在进入任何交互 prompt 前完成预设校验。校验 MUST 至少包括：`extends` 父预设存在性、继承无环、`skill_dirs` 引用存在性、解析后结果非空。任一校验失败时，命令 MUST 立即报错并中止。

#### Scenario: 父预设不存在
- **WHEN** 预设 `full-stack` 的 `extends` 指向不存在的预设
- **THEN** 命令在进入交互前报错并退出

#### Scenario: 继承循环
- **WHEN** 预设 A extends B，且 B extends A
- **THEN** 命令在进入交互前报错并退出

#### Scenario: 引用不存在技能目录
- **WHEN** 某预设 `skill_dirs` 包含不存在的目录名
- **THEN** 命令在进入交互前报错并退出

#### Scenario: 预设解析为空
- **WHEN** 某预设解析后的技能集合为空
- **THEN** 命令在进入交互前报错并退出

### Requirement: 预设功能仅限交互模式
本变更中，预设能力 MUST 仅通过交互流程提供。`llman skills` MUST NOT 新增任何 presets 专用命令参数。

#### Scenario: 命令行帮助不包含预设参数
- **WHEN** 用户查看 `llman skills --help`
- **THEN** 帮助信息不包含 `--preset`、`--save-preset`、`--list-presets` 等 presets 参数

### Requirement: 技能分组推断与展示
管理器 MUST 根据技能目录名中的 `.` 推断分组：`<group>.<name>` 归入 `<group>`，不含 `.` 的目录归入 `ungrouped`。交互式技能列表 MUST 按分组聚合展示。

#### Scenario: 分组推断
- **WHEN** 技能目录名为 `superpowers.brainstorming`
- **THEN** 该技能归入 `superpowers` 分组

#### Scenario: 无分组技能
- **WHEN** 技能目录名为 `mermaid-expert`
- **THEN** 该技能归入 `ungrouped` 分组

#### Scenario: 分组展示
- **WHEN** 用户进入技能多选列表
- **THEN** 技能按分组聚合显示，并展示分组标题

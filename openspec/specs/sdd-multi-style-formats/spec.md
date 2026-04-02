# sdd-multi-style-formats Specification

## Purpose
TBD - created by archiving change support-multi-style-sdd-specs. Update Purpose after archive.
## Requirements
### Requirement: 项目必须显式声明唯一的 SDD 主风格
`llmanspec/config.yaml` MUST 显式声明 `spec_style`，且值 MUST 仅允许 `ison`、`toon`、`yaml` 三者之一。

`llman sdd init` MUST 为新项目写入显式值 `spec_style: ison`。任何读取或改写以下文件的 `llman sdd` 命令：

- `llmanspec/specs/<capability>/spec.md`
- `llmanspec/changes/<change>/specs/<capability>/spec.md`

都 MUST 先读取 `spec_style`。若该字段缺失、为空或不是受支持值，命令 MUST 失败，并明确提示用户先在 `llmanspec/config.yaml` 中设置主风格。

#### Scenario: 初始化新项目时写入显式主风格
- **WHEN** 用户执行 `llman sdd init`
- **THEN** 生成的 `llmanspec/config.yaml` 包含 `spec_style: ison`

#### Scenario: 缺失主风格会阻止 spec 命令继续
- **WHEN** 用户在 `llmanspec/config.yaml` 缺少 `spec_style` 的项目中执行 `llman sdd show sample --type spec`
- **THEN** 命令返回非零
- **AND** 错误提示要求先在 `llmanspec/config.yaml` 中声明 `spec_style`

### Requirement: 主 spec 与 delta spec 必须严格匹配项目主风格
一旦项目声明了 `spec_style`，仓库中的主 spec 与 delta spec MUST 全部使用该风格的 canonical payload，而 MUST NOT 混入其他风格。

风格与 fenced payload 的对应关系 MUST 为：

- `ison` → ` ```ison `
- `toon` → ` ```toon `
- `yaml` → ` ```yaml `

当项目声明主风格后，运行时 MUST NOT 自动探测、猜测或兼容其他风格。若文件缺少期望 fence、出现了其他风格 fence、或 payload 结构不属于配置风格，命令 MUST 失败，并同时指出“期望风格”和“实际内容/实际 fence”。

#### Scenario: TOON 项目中的 ISON spec 被拒绝
- **WHEN** `llmanspec/config.yaml` 声明 `spec_style: toon`
- **AND** `llmanspec/specs/sample/spec.md` 仅包含 ` ```ison ` canonical payload
- **THEN** `llman sdd validate sample --type spec` 返回非零
- **AND** 错误明确指出项目期望 `toon` 而文件实际为 `ison`

#### Scenario: 主 spec 与 delta spec 混用风格被拒绝
- **WHEN** 项目声明 `spec_style: yaml`
- **AND** 主 spec 使用 ` ```yaml ` payload
- **AND** 某个 change delta spec 使用 ` ```toon ` payload
- **THEN** `llman sdd validate --changes` 返回非零
- **AND** 错误明确指出不允许在同一项目中混用多种 SDD 风格

### Requirement: 三种风格必须共享同一语义模型
无论项目使用 `ison`、`toon` 或 `yaml`，系统 MUST 将主 spec 与 delta spec 解析为同一个语义模型，然后再驱动 `show`、`list`、`validate`、`archive` 与 authoring helpers。

主 spec 语义模型 MUST 至少包含：

- `kind`
- `name`
- `purpose`
- requirements：`req_id`、`title`、`statement`
- scenarios：`req_id`、`id`、`given`、`when`、`then`

delta spec 语义模型 MUST 至少包含：

- `kind`
- ops：`op`、`req_id`、`title`、`statement`、`from`、`to`、`name`
- op_scenarios：`req_id`、`id`、`given`、`when`、`then`

不同风格之间仅允许“承载格式”不同，不允许语义结果不同。相同语义内容在三种风格中经过 `show --json`、`validate`、archive merge 后，语义结果 MUST 保持一致。

#### Scenario: 三种风格的 show JSON 语义一致
- **WHEN** 三个项目分别以 `ison`、`toon`、`yaml` 表达同一份主 spec 语义
- **THEN** `llman sdd show sample --type spec --json` 的 `name`、`overview`、requirements 与 scenarios 语义结果一致

#### Scenario: archive merge 结果不因风格而改变
- **WHEN** 两个项目分别以 `ison` 与 `yaml` 表达同一组主 spec 与 delta spec 语义
- **AND** 用户分别执行 `llman sdd archive run <change>`
- **THEN** archive 之后的主 spec 语义结果一致

### Requirement: 风格转换必须显式、可审计、可验证
系统 MUST 提供显式的 SDD 风格转换能力，允许用户在 `ison`、`toon`、`yaml` 之间进行迁移，而 MUST NOT 在普通读写路径中偷偷转换。

转换能力 MUST 同时覆盖：

- 项目范围：转换全部主 spec 与 active change delta spec，并在成功后更新 `llmanspec/config.yaml`
- 单文件范围：转换单个主 spec 或 delta spec 文件，供用户审阅或分阶段迁移

任何转换写入之前，系统 MUST 验证源文档可被当前风格正确解析；写入之后，系统 MUST 重新解析目标文档并确认其语义等价。若任一步失败，命令 MUST 返回非零，并且 MUST NOT 把项目配置更新到新风格。

#### Scenario: 项目范围转换成功后更新主风格
- **WHEN** 用户对 `spec_style: ison` 的项目执行“转换整个项目到 `yaml`”
- **THEN** 所有主 spec 与 active change delta spec 被重写为 ` ```yaml ` payload
- **AND** `llmanspec/config.yaml` 被更新为 `spec_style: yaml`

#### Scenario: 转换失败时不更新项目配置
- **WHEN** 用户执行“转换整个项目到 `toon`”
- **AND** 某个目标文件在重解析校验阶段失败
- **THEN** 命令返回非零
- **AND** `llmanspec/config.yaml` 仍保留原有 `spec_style`

### Requirement: 多风格支持必须被标记为实验性
`toon` 与 `yaml` 风格 MUST 在帮助文本、错误提示、模板示例与转换命令说明中被标记为 experimental，避免用户将其误解为“可在同一项目内自由混用”的稳定兼容层。

#### Scenario: 帮助文本标记实验性风格
- **WHEN** 用户查看多风格相关帮助或转换说明
- **THEN** 文本中明确标注 `toon` 与 `yaml` 为 experimental


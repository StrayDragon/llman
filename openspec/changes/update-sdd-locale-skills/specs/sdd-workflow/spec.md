## ADDED Requirements

### Requirement: SDD 本地化配置与模板加载
`llman sdd` MUST 使用项目级配置 `llmanspec/config.yaml` 解析 locale，并基于 `templates/sdd/<locale>/` 加载 `llmanspec/AGENTS.md`、`llmanspec/templates/**` 与 sdd skills 内容。locale 仅影响模板与 skills 输出，不影响 CLI 文本。

#### Scenario: 初始化写入 locale 配置
- **WHEN** 用户执行 `llman sdd init --lang zh-Hans`
- **THEN** `llmanspec/config.yaml` 写入 `locale: zh-Hans` 且包含版本字段

#### Scenario: locale 回退链
- **WHEN** 配置 locale 为 `zh-Hans` 但缺少 `templates/sdd/zh-Hans/...`
- **THEN** 按 `zh-Hans` → `zh` → `en` 顺序回退

#### Scenario: locale 仅影响模板与 skills
- **WHEN** `llmanspec/config.yaml` 设置 locale 为 `zh-Hans`
- **THEN** `llmanspec/AGENTS.md` 与 sdd skills 使用中文模板
- **AND** CLI 输出仍保持英文

### Requirement: SDD Skills 生成与更新
`llman sdd update-skills` MUST 生成并更新 Claude Code 与 Codex 的技能目录（不生成 slash commands）。默认输出路径来自 `llmanspec/config.yaml`，交互模式允许输入覆盖路径；非交互模式必须通过 `--all` 或 `--tool` 指定目标，并可选 `--path` 覆盖输出路径。若目标技能已存在，命令 MUST 刷新托管内容以保持一致性。生成的 skills MUST 包含 sdd 校验修复提示与最小示例。

#### Scenario: 交互式技能生成
- **WHEN** 用户在可交互终端执行 `llman sdd update-skills`
- **THEN** 可选择 Claude Code 或 Codex，并使用默认路径或输入自定义路径生成技能

#### Scenario: 非交互技能生成
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude`
- **THEN** 命令在 Claude Code 技能路径下生成/更新技能

#### Scenario: 非交互更新全部
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** 命令生成/更新 Claude Code 与 Codex 的技能

#### Scenario: 更新已有技能
- **WHEN** 目标路径中存在同名技能目录
- **THEN** `SKILL.md` 被托管内容刷新

### Requirement: SDD 校验提示增强
`llman sdd validate` 在非 JSON 模式下 MUST 为常见结构错误提供可执行修复提示，包含最小示例片段与推荐命令。

#### Scenario: 缺少必需章节
- **WHEN** spec 缺少 `## Purpose` 或 `## Requirements`
- **THEN** 输出提示包含期望标题与最小示例片段

#### Scenario: 场景标题格式错误
- **WHEN** requirement 使用了非 `#### Scenario:` 标题
- **THEN** 输出提示包含标准 `#### Scenario:` 示例

#### Scenario: 无 delta 变更
- **WHEN** change 目录中未找到任何 delta
- **THEN** 输出提示包含 delta section 示例与文件路径提示

### Requirement: SDD 模板区域复用
SDD 模板与 skills MUST 支持 `{{region: <path>#<name>}}` 引用，系统 MUST 使用源文件中与文件类型匹配的 `region` 块替换占位符内容。若 region 缺失或重复，命令 MUST 报错并中止。

#### Scenario: 引用 Markdown region
- **WHEN** 模板包含 `{{region: docs/sdd.md#overview}}` 且目标文件存在 `<!-- region: overview -->` 块
- **THEN** 生成结果中替换为对应 region 内容

#### Scenario: region 缺失
- **WHEN** 模板引用的 region 在目标文件中不存在
- **THEN** 命令报错并退出非零

## MODIFIED Requirements

### Requirement: SDD 初始化脚手架
`llman sdd init [path]` 命令 MUST 在目标路径创建 `llmanspec/` 目录结构，包括 `llmanspec/AGENTS.md`、`llmanspec/project.md`、`llmanspec/specs/`、`llmanspec/changes/` 与 `llmanspec/changes/archive/`，以及 `llmanspec/templates/spec-driven/` 下的 `proposal.md`、`spec.md`、`design.md`、`tasks.md`。命令 MUST 生成 `llmanspec/config.yaml` 并写入 locale 配置。命令 MUST 创建或刷新 repo 根目录下的 `AGENTS.md` 受管块以指向 `llmanspec/AGENTS.md`。当 `llmanspec/` 已存在时，命令 MUST 报错并且不修改任何文件。生成的 `llmanspec/AGENTS.md` MUST 包含 LLMANSPEC 受管提示块且包含完整 llman sdd 方法论说明。

#### Scenario: 初始化新项目
- **WHEN** 用户在不存在 `llmanspec/` 的目录执行 `llman sdd init`
- **THEN** 必要的目录结构与模板文件被创建

#### Scenario: 初始化指定路径
- **WHEN** 用户执行 `llman sdd init <path>`
- **THEN** 在 `<path>` 下创建 `llmanspec/` 结构与模板文件

#### Scenario: 初始化时生成提示块
- **WHEN** `llman sdd init` 生成 `llmanspec/AGENTS.md`
- **THEN** 文件中包含 `<!-- LLMANSPEC:START -->` 与 `<!-- LLMANSPEC:END -->` 包裹的提示块

#### Scenario: 初始化时写入配置
- **WHEN** `llman sdd init --lang en` 运行
- **THEN** `llmanspec/config.yaml` 被写入且 locale 为 `en`

#### Scenario: 初始化时生成根 AGENTS
- **WHEN** `llman sdd init` 运行
- **THEN** repo 根目录 `AGENTS.md` 被创建或刷新受管块并指向 `llmanspec/AGENTS.md`

#### Scenario: 已存在 llmanspec 目录
- **WHEN** 用户在已有 `llmanspec/` 的目录执行 `llman sdd init`
- **THEN** 命令返回错误且不做任何更改

#### Scenario: openspec 共存
- **WHEN** `openspec/` 已存在但 `llmanspec/` 不存在
- **THEN** `llman sdd init` 仅创建 `llmanspec/` 且不修改 `openspec/`

### Requirement: SDD 指令与提示词刷新
`llman sdd update [path]` MUST 刷新 `llmanspec/AGENTS.md` 与 `llmanspec/templates/spec-driven/` 内置模板，同时 MUST 保持 `llmanspec/specs/**` 与 `llmanspec/changes/**` 不被修改。命令 MUST 刷新 repo 根目录 `AGENTS.md` 的 LLMANSPEC 受管块并保留非受管内容。更新 `llmanspec/AGENTS.md` 时 MUST 仅替换 LLMANSPEC 受管提示块，并保留受管块以外的用户内容。更新时必须使用 `llmanspec/config.yaml` 的 locale 选择模板。

#### Scenario: 更新指令文件
- **WHEN** 用户执行 `llman sdd update`
- **THEN** 指令/模板文件被刷新且现有 specs 与 changes 内容保持不变

#### Scenario: 更新指定路径
- **WHEN** 用户执行 `llman sdd update <path>`
- **THEN** 仅更新 `<path>/llmanspec/` 下的指令与模板文件

#### Scenario: 未初始化时更新
- **WHEN** 目标路径下不存在 `llmanspec/`
- **THEN** 命令返回错误并提示先执行 `llman sdd init`

#### Scenario: 保留用户自定义内容
- **WHEN** `llmanspec/AGENTS.md` 含有用户自定义内容且包含 LLMANSPEC 受管块
- **THEN** update 仅替换受管块并保留其他内容

#### Scenario: 更新根 AGENTS
- **WHEN** repo 根目录 `AGENTS.md` 存在且包含 LLMANSPEC 受管块
- **THEN** update 仅替换受管块并保留其他内容

#### Scenario: openspec 共存
- **WHEN** `openspec/` 与 `llmanspec/` 同时存在
- **THEN** `llman sdd update` 不修改 `openspec/`

### Requirement: SDD 命令范围
`llman sdd` MUST 仅暴露 OpenSpec 工作流的核心命令：`init`、`update`、`update-skills`、`list`、`show`、`validate`、`archive`。在 SDD 子命令组中 MUST 不提供 `change`、`spec`、`view`、`completion`、`config` 等额外子命令。

#### Scenario: 帮助文本仅包含核心命令
- **WHEN** 用户执行 `llman sdd --help`
- **THEN** 帮助文本仅包含 `init`、`update`、`update-skills`、`list`、`show`、`validate`、`archive`
- **AND** 不包含 `change`、`spec`、`view`、`completion`、`config`

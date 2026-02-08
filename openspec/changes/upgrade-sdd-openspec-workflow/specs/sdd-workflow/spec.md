## MODIFIED Requirements

### Requirement: SDD Skills 生成与更新
`llman sdd update-skills` MUST 生成并更新 Claude Code 与 Codex 的技能目录（不生成 slash commands）。默认输出路径来自 `llmanspec/config.yaml`，交互模式允许输入覆盖路径；非交互模式必须通过 `--all` 或 `--tool` 指定目标，并可选 `--path` 覆盖输出路径。若目标技能已存在，命令 MUST 刷新托管内容以保持一致性。生成的 skills MUST 包含 sdd 校验修复提示与最小示例。

Skills 集合 MUST 包含完整的工作流技能：
- `llman-sdd-onboard`: 引导新用户完成首次 SDD 工作流
- `llman-sdd-new-change`: 创建新的变更提案
- `llman-sdd-show`: 显示变更或规范内容
- `llman-sdd-validate`: 验证变更或规范格式
- `llman-sdd-archive`: 归档已完成的变更
- `llman-sdd-explore`: 探索模式，用于问题分析和设计思考
- `llman-sdd-continue`: 继续创建下一个 artifact
- `llman-sdd-apply`: 实施 tasks.md 中的任务
- `llman-sdd-ff`: 快速创建所有 artifacts
- `llman-sdd-verify`: 验证实现与 artifacts 匹配
- `llman-sdd-sync`: 同步 delta specs 到主 specs

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

#### Scenario: 生成完整工作流技能集
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** 生成的技能目录包含 `llman-sdd-explore`、`llman-sdd-continue`、`llman-sdd-apply`、`llman-sdd-ff`、`llman-sdd-verify`、`llman-sdd-sync` 等完整工作流技能

## ADDED Requirements

### Requirement: SDD Prompt 注入机制
SDD 模板系统 MUST 支持 `{{prompt: <name>}}` 占位符语法，用于从 `prompts/<name>.md` 加载可复用的 prompt 片段。加载顺序（高 → 低）MUST 为：配置的自定义路径 → 项目级 `templates/sdd/<locale>/prompts/` → 嵌入式模板。若 prompt 不存在，命令 MUST 报错并中止。

#### Scenario: 引用 prompt 模板
- **WHEN** skills 模板包含 `{{prompt: workflow-guardrails}}`
- **THEN** 生成结果中替换为 `prompts/workflow-guardrails.md` 的内容

#### Scenario: prompt 缺失报错
- **WHEN** 模板引用的 prompt 在任何路径中均不存在
- **THEN** 命令报错并退出非零

#### Scenario: locale 回退
- **WHEN** 配置 locale 为 `zh-Hans` 但 `templates/sdd/zh-Hans/prompts/<name>.md` 不存在
- **THEN** 按 `zh-Hans` → `zh` → `en` 顺序回退加载

#### Scenario: custom_path 覆盖项目与内置
- **WHEN** `llmanspec/config.yaml` 配置了 `prompts.custom_path`，且 custom/project/embedded 均存在同名 prompt
- **THEN** 使用 custom_path 下的 prompt 内容

### Requirement: SDD Explore 模式 Skill
`llman-sdd-explore` skill MUST 提供探索模式指导，允许 AI 助手在问题分析、设计思考阶段提供帮助。探索模式 MUST 明确禁止直接实现代码，仅允许阅读代码、创建 artifacts、提出问题。skill 内容 MUST 包含：stance 定义、可执行的操作列表、与 llmanspec 的交互方式、结束探索的引导。

#### Scenario: 探索模式进入
- **WHEN** 用户调用 `llman-sdd-explore` skill
- **THEN** AI 助手进入探索模式，可阅读代码和创建 artifacts，但不实现功能

#### Scenario: 探索模式退出
- **WHEN** 用户在探索模式中准备开始实现
- **THEN** skill 引导用户使用 `llman-sdd-new-change` 或 `llman-sdd-ff` 开始正式工作流

### Requirement: SDD Continue Skill
`llman-sdd-continue` skill MUST 指导 AI 助手继续未完成的变更，创建下一个待完成的 artifact。skill MUST 检查当前变更状态，识别已完成和待创建的 artifacts，按依赖顺序创建下一个 artifact。若所有 artifacts 已完成，skill MUST 引导用户进入 apply 阶段或 archive。

#### Scenario: 继续创建 artifact
- **WHEN** 用户调用 `llman-sdd-continue` 且变更存在未完成的 artifacts
- **THEN** skill 指导创建下一个按依赖顺序应完成的 artifact

#### Scenario: artifacts 全部完成
- **WHEN** 用户调用 `llman-sdd-continue` 且所有 artifacts 已完成
- **THEN** skill 提示可使用 `llman-sdd-apply` 或 `llman-sdd-archive`

### Requirement: SDD Apply Skill
`llman-sdd-apply` skill MUST 指导 AI 助手实施 tasks.md 中的任务。skill MUST 读取变更的上下文文件（proposal、specs、design、tasks），按顺序实施未完成的任务，每完成一个任务后更新 tasks.md 中的 checkbox 状态。实施过程中遇到问题时 skill MUST 暂停并请求用户指导。

#### Scenario: 实施任务
- **WHEN** 用户调用 `llman-sdd-apply` 且存在未完成的任务
- **THEN** skill 指导按顺序实施任务并更新 checkbox 状态

#### Scenario: 任务全部完成
- **WHEN** 用户调用 `llman-sdd-apply` 且所有任务已完成
- **THEN** skill 提示可使用 `llman-sdd-archive`

#### Scenario: 实施遇阻
- **WHEN** 实施任务过程中遇到不明确的需求或技术问题
- **THEN** skill 指导暂停并请求用户指导

### Requirement: SDD Fast-Forward Skill
`llman-sdd-ff` skill MUST 指导 AI 助手快速创建变更的所有 artifacts（proposal → specs → design → tasks），无需逐步确认。skill MUST 在创建前询问用户对变更的描述，然后依次创建所有 artifacts，最后显示完成状态和后续可用操作。

#### Scenario: 快速创建变更
- **WHEN** 用户调用 `llman-sdd-ff <change-name>`
- **THEN** skill 询问变更描述后依次创建 proposal、specs、design、tasks

#### Scenario: 变更已存在
- **WHEN** 用户调用 `llman-sdd-ff <change-name>` 但该变更已存在
- **THEN** skill 提示使用 `llman-sdd-continue` 继续

### Requirement: SDD Verify Skill
`llman-sdd-verify` skill MUST 指导 AI 助手验证实现与变更 artifacts 的一致性。skill MUST 读取 specs 和 design，检查代码实现是否符合规范，识别不一致之处并提供修复建议。验证通过后 skill MUST 引导用户进行 archive。

#### Scenario: 验证实现
- **WHEN** 用户调用 `llman-sdd-verify`
- **THEN** skill 指导检查实现与 specs/design 的一致性

#### Scenario: 验证发现问题
- **WHEN** 验证发现实现与 artifacts 不一致
- **THEN** skill 列出不一致之处并提供修复建议

#### Scenario: 验证通过
- **WHEN** 验证确认实现与 artifacts 一致
- **THEN** skill 提示可使用 `llman-sdd-archive`

### Requirement: SDD Sync Skill
`llman-sdd-sync` skill MUST 指导 AI 助手以可复现的人工作业协议将变更中的 delta specs 同步到主 specs，而不归档变更目录。skill MUST 指导用户检查 delta、手动应用 ADDED/MODIFIED/REMOVED/RENAMED 到对应主 specs，并在完成后运行验证命令。

#### Scenario: 同步 delta specs
- **WHEN** 用户调用 `llman-sdd-sync <change-name>`
- **THEN** skill 提供可复现步骤，指导手动将 delta specs 同步到主 specs 目录

#### Scenario: 同步后验证
- **WHEN** delta specs 已合并到主 specs
- **THEN** skill 运行 `llman sdd validate --specs` 验证合并结果

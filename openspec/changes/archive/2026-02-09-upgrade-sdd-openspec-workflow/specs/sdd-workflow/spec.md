## MODIFIED Requirements

### Requirement: SDD Skills 生成与更新
`llman sdd update-skills` MUST 支持为 Claude Code 与 Codex 生成/更新 workflow skills 与 OPSX 命令绑定（slash commands / prompts），用于把用户入口统一到新的 `/opsx:*` 动作工作流。

- 默认行为 MUST 同时生成 skills 与 OPSX commands。
- `--skills-only` MUST 仅生成 skills（不生成 OPSX commands）。
- `--commands-only` MUST 仅生成 OPSX commands（不生成 skills）。

默认 skills 输出路径来自 `llmanspec/config.yaml`；交互模式允许输入覆盖路径；非交互模式必须通过 `--all` 或 `--tool` 指定目标，并可选 `--path` 覆盖 skills 输出路径（仅对 skills 生效）。若目标技能已存在，命令 MUST 刷新托管内容以保持一致性。

生成的 skills MUST 包含完整工作流技能（含 OPSX 动作覆盖）：
- `llman-sdd-onboard`
- `llman-sdd-new-change`
- `llman-sdd-archive`
- `llman-sdd-explore`
- `llman-sdd-continue`
- `llman-sdd-ff`
- `llman-sdd-apply`
- `llman-sdd-verify`
- `llman-sdd-sync`
- `llman-sdd-bulk-archive`

（`llman-sdd-show` 与 `llman-sdd-validate` MAY 保留作为辅助技能，但不作为 OPSX commands 的必需绑定目标。）

生成的 OPSX commands MUST 仅包含新的 OPSX 命令集合：
- `explore`
- `onboard`
- `new`
- `continue`
- `ff`
- `apply`
- `verify`
- `sync`
- `archive`
- `bulk-archive`

命令绑定输出位置（V1）MUST 为：
- Claude Code：`.claude/commands/opsx/<command>.md`（命令语法 `/opsx:<command>`）
- Codex：`.codex/prompts/opsx-<command>.md`（项目级）

对 Codex，命令绑定 MUST 仅写入项目级 `.codex/prompts/`；实现 MUST NOT 写入 user-global（例如 `$CODEX_HOME/prompts` 或 `~/.codex/prompts`）。

#### Scenario: 交互式技能生成
- **WHEN** 用户在可交互终端执行 `llman sdd update-skills`
- **THEN** 可选择 Claude Code 或 Codex，并使用默认路径或输入自定义路径生成 skills

#### Scenario: 非交互技能生成
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude`
- **THEN** 命令在 Claude Code 技能路径下生成/更新 skills

#### Scenario: 非交互更新全部
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** 命令生成/更新 Claude Code 与 Codex 的 skills

#### Scenario: 更新已有技能
- **WHEN** 目标路径中存在同名技能目录
- **THEN** `SKILL.md` 被托管内容刷新

#### Scenario: 生成 OPSX commands
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** `.claude/commands/opsx/` 与 `.codex/prompts/` 下生成/刷新与 OPSX 动作集合一致的命令文件

#### Scenario: 仅生成 OPSX commands（非交互）
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude --commands-only`
- **THEN** `.claude/commands/opsx/` 下生成/刷新 OPSX 命令文件，且 MUST NOT 写入 `.claude/skills/`

#### Scenario: 仅生成 skills（非交互）
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude --skills-only`
- **THEN** `.claude/skills/` 下生成/刷新 workflow skills，且 MUST NOT 写入 `.claude/commands/opsx/`

#### Scenario: legacy commands 迁移（交互）
- **WHEN** 工作区存在 legacy 命令绑定（例如 `.claude/commands/openspec/` 或 `.codex/prompts/openspec-*.md`），且用户在可交互终端执行 `llman sdd update-skills`
- **THEN** 命令展示将被迁移/删除的 legacy 路径并要求二次确认；确认后删除 legacy 并生成 OPSX commands

#### Scenario: legacy commands 迁移（非交互）
- **WHEN** 工作区存在 legacy 命令绑定且用户执行 `llman sdd update-skills --no-interactive ...`
- **THEN** 命令 MUST 报错并提示改用交互模式完成迁移，且 MUST NOT 删除任何 legacy 文件

## ADDED Requirements

### Requirement: SDD OPSX Slash Command Bindings
SDD MUST 提供 OPSX slash commands 的工具适配文件，并由 `llman sdd update-skills` 负责生成/刷新。命令绑定内容 MUST 引导用户进入 llman sdd 的工作流（`llmanspec/`）并与 skills 的动作集合保持一致。命令绑定 MUST 仅包含 OPSX 命令集合，不得生成 legacy commands（例如旧式 `/openspec:*` 体系）。

#### Scenario: 仅生成 OPSX commands
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude --commands-only`
- **THEN** `.claude/commands/opsx/` 下仅存在 OPSX 命令文件（`new/continue/ff/apply/verify/sync/archive/bulk-archive/explore/onboard`）

#### Scenario: 命令绑定指向 llman sdd 工作流
- **WHEN** 用户调用任一 `/opsx:<command>` 触发对应命令绑定
- **THEN** 命令绑定文本引导其在 `llmanspec/` 下执行对应动作（创建 artifacts / 实施 tasks / 归档等），并引用 `llman sdd` 命令用于验证闭环

### Requirement: SDD Bulk-Archive Skill
`llman-sdd-bulk-archive` skill MUST 提供批量归档协议：列出活动 changes、让用户选择要归档的 change IDs、按顺序执行归档，并在完成后运行一次全量校验。批量归档 MUST 默认遵循与单个归档一致的护栏（需要确认目标、失败时停止并报告）。

#### Scenario: 批量归档多个变更
- **WHEN** 用户调用 `llman-sdd-bulk-archive` 并提供多个 change IDs
- **THEN** skill 指导依次运行 `llman sdd archive <id>`，并在结束后运行 `llman sdd validate --strict --no-interactive`

### Requirement: SDD Explore 模式 Skill
`llman-sdd-explore` skill MUST 提供探索模式指导，允许 AI 助手在问题分析、设计思考阶段提供帮助。探索模式 MUST 明确禁止直接实现代码，仅允许阅读代码、创建 artifacts、提出问题。skill 内容 MUST 包含：stance 定义、可执行的操作列表、与 llmanspec 的交互方式、结束探索的引导。

#### Scenario: 探索模式进入
- **WHEN** 用户调用 `llman-sdd-explore`（或通过 `/opsx:explore` 进入）
- **THEN** AI 助手进入探索模式，可阅读代码和创建 artifacts，但不实现功能

#### Scenario: 探索模式退出
- **WHEN** 用户在探索模式中准备开始实现
- **THEN** skill 引导用户使用 `llman-sdd-new-change`、`llman-sdd-ff` 或 `/opsx:new` 开始正式工作流

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

## MODIFIED Requirements

### Requirement: SDD Skills 生成与更新
`llman sdd update-skills` MUST 支持为 Claude Code 与 Codex 生成/更新 workflow skills，并在可用工具上生成 OPSX 命令绑定，以把用户入口统一到 `/opsx:*` 动作工作流。

- 默认行为 MUST 生成 skills，并在支持命令绑定的工具上生成 OPSX commands。
- `--skills-only` MUST 仅生成 skills（不生成 OPSX commands）。
- `--commands-only` MUST 仅生成 OPSX commands（不生成 skills）；若所选工具均不支持 OPSX commands，命令 MUST 返回非零错误并给出可操作提示。

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

对 Codex，`llman sdd update-skills` MUST NOT 生成或刷新 `.codex/prompts/opsx-<command>.md` 这类 slash command/custom prompt 绑定文件。Codex 在本能力下 MUST 仅生成 workflow skills。

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

#### Scenario: 默认模式仅为 Claude 生成 OPSX commands
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** `.claude/commands/opsx/` 下生成/刷新与 OPSX 动作集合一致的命令文件
- **AND** 命令 MUST NOT 写入 `.codex/prompts/opsx-*.md`

#### Scenario: 仅生成 OPSX commands（非交互）
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude --commands-only`
- **THEN** `.claude/commands/opsx/` 下生成/刷新 OPSX 命令文件，且 MUST NOT 写入 `.claude/skills/`

#### Scenario: 仅生成 skills（非交互）
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude --skills-only`
- **THEN** `.claude/skills/` 下生成/刷新 workflow skills，且 MUST NOT 写入 `.claude/commands/opsx/`

#### Scenario: Codex commands-only 被拒绝
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool codex --commands-only`
- **THEN** 命令返回非零错误并提示 Codex 不支持 OPSX commands，建议改用 `--skills-only` 或改选 Claude

#### Scenario: legacy commands 迁移（交互）
- **WHEN** 工作区存在 legacy 命令绑定（例如 `.claude/commands/openspec/` 或 `.codex/prompts/openspec-*.md`），且用户在可交互终端执行 `llman sdd update-skills`
- **THEN** 命令展示将被迁移/删除的 legacy 路径并要求二次确认；确认后删除 legacy 并生成 OPSX commands

#### Scenario: legacy commands 迁移（非交互）
- **WHEN** 工作区存在 legacy 命令绑定且用户执行 `llman sdd update-skills --no-interactive ...`
- **THEN** 命令 MUST 报错并提示改用交互模式完成迁移，且 MUST NOT 删除任何 legacy 文件

### Requirement: SDD OPSX Slash Command Bindings
SDD MUST 提供 OPSX slash commands 的工具适配文件，并由 `llman sdd update-skills` 负责生成/刷新。命令绑定内容 MUST 引导用户进入 llman sdd 的工作流（`llmanspec/`）并与 skills 的动作集合保持一致。命令绑定 MUST 仅包含 OPSX 命令集合，不得生成 legacy commands（例如旧式 `/openspec:*` 体系）。

当前版本中，OPSX slash command bindings MUST 仅为 Claude Code 生成。实现 MUST NOT 为 Codex 生成 `.codex/prompts/opsx-*.md` 绑定文件。

#### Scenario: 仅生成 OPSX commands
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude --commands-only`
- **THEN** `.claude/commands/opsx/` 下仅存在 OPSX 命令文件（`new/continue/ff/apply/verify/sync/archive/bulk-archive/explore/onboard`）

#### Scenario: 命令绑定指向 llman sdd 工作流
- **WHEN** 用户调用任一 `/opsx:<command>` 触发对应命令绑定
- **THEN** 命令绑定文本引导其在 `llmanspec/` 下执行对应动作（创建 artifacts / 实施 tasks / 归档等），并引用 `llman sdd` 命令用于验证闭环

#### Scenario: Codex 不生成 OPSX prompts
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool codex`
- **THEN** 命令仅生成/刷新 Codex skills，且 MUST NOT 在 `.codex/prompts/` 下生成 `opsx-*.md`

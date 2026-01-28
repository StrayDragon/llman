## RENAMED Requirements
- FROM: `### Requirement: 自动托管与来源重链接`
- TO: `### Requirement: 自动托管与来源只读`

## MODIFIED Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 在交互式终端执行只读同步（不修改来源目录），并在同步完成后进入交互式管理器；不需要 `--relink-sources` 授权。

#### Scenario: 交互式默认进入管理器
- **WHEN** 用户在交互式终端运行 `llman skills` 且未传入任何 flags
- **THEN** 命令执行只读同步并进入交互式管理器

#### Scenario: 交互式传入 --relink-sources
- **WHEN** 用户在交互式终端运行 `llman skills --relink-sources`
- **THEN** 命令返回错误并提示该参数已移除

### Requirement: 可配置来源与目标并提供默认值
技能管理器 MUST 从 `LLMAN_CONFIG_DIR/skills/config.toml` 加载 version=1 的 source/target 配置；若配置缺失，则使用默认来源与目标。repo scope MUST 由运行时基于 git 根目录自动发现。配置路径 MUST 支持 `~` 与环境变量展开。target 配置 MAY 指定 `mode`（`link`、`copy`、`skip`），缺省为 `link`；`skip` 目标必须被跳过且在交互式管理器中以只读状态展示。

默认来源/目标：
- `claude_user`: `~/.claude/skills`（或 `CLAUDE_HOME/skills`）
- `codex_user`: `~/.codex/skills`（或 `CODEX_HOME/skills`）
- `agent_global`: `~/.skills`

repo 级别来源/目标：
- `<repo>/.claude/skills`
- `<repo>/.codex/skills`

#### Scenario: 缺省配置时使用默认值并自动发现项目级来源
- **WHEN** 技能配置文件不存在且当前目录位于 git 仓库内
- **THEN** 管理器使用默认来源与目标，并自动加入仓库根目录下的 `.claude/skills` 与 `.codex/skills`

#### Scenario: 不支持的配置版本
- **WHEN** `config.toml` 中 `version` 不是 1
- **THEN** 命令返回错误并中止

#### Scenario: 目标 mode 缺省为 link
- **WHEN** target 配置缺失 `mode`
- **THEN** 该目标使用 `link` 模式

#### Scenario: 目标 mode=skip 被跳过
- **WHEN** target 配置为 `mode = "skip"`
- **THEN** 管理器不同步该目标且在交互式管理器中以只读状态展示

### Requirement: 自动托管与来源只读
管理器 MUST 扫描所有启用的来源，将技能目录复制到 `LLMAN_CONFIG_DIR/skills/store/<skill_id>/versions/<hash>`，更新 `LLMAN_CONFIG_DIR/skills/registry.json`，并确保 `LLMAN_CONFIG_DIR/skills/store/<skill_id>/current` 指向选定版本。管理器 MUST NOT 替换或重链接来源目录。

#### Scenario: 导入并保持来源不变
- **WHEN** 来源目录包含尚未托管的 `SKILL.md` 技能目录
- **THEN** 管理器将其复制到托管仓库并更新 registry，但来源目录保持不变且不被软链接替换

### Requirement: 按 agent 目标启用/禁用
管理器 MUST 支持按 agent 目标启用或禁用技能；启用在目标目录下创建 `<skill_id>` 链接或复制（取决于目标 `mode`），禁用仅移除该链接或受管副本。`mode = skip` 的目标 MUST NOT 被管理器更新或展示。目标路径不存在时 MUST 创建；若目标路径存在但不是目录，必须记录错误并跳过该目标。

#### Scenario: 为单个 agent 禁用技能（link）
- **WHEN** 用户禁用某个技能在 `mode=link` 的目标下
- **THEN** 仅移除该目标下的软链接，托管副本仍保留

#### Scenario: 为单个 agent 禁用技能（copy）
- **WHEN** 用户禁用某个技能在 `mode=copy` 的目标下且该目标为 llman 管理副本
- **THEN** 仅移除该目标下的受管副本，托管副本仍保留

#### Scenario: 目标路径非法
- **WHEN** 目标路径存在但不是目录
- **THEN** 记录错误并不创建链接或复制

## ADDED Requirements
### Requirement: 目标复制冲突处理
当 target 使用 `mode=copy` 且目标路径已存在时，管理器 MUST 使用目标目录内 `.llman-skill.json` 判断是否为 llman 管理；缺失或与当前内容不一致时视为冲突。交互模式 MUST 提示用户选择 `overwrite` 或 `skip`。非交互模式遇到冲突 MUST 报错并提示使用 `--target-conflict=overwrite|skip`，若提供该参数则按策略执行。

#### Scenario: copy 模式写入新目标
- **WHEN** target 为 `mode=copy` 且目标路径不存在
- **THEN** 管理器复制托管 `current` 内容到目标并写入 `.llman-skill.json`

#### Scenario: 交互冲突选择覆盖
- **WHEN** 交互模式下目标存在且被视为冲突，用户选择 `overwrite`
- **THEN** 管理器覆盖目标并更新 `.llman-skill.json`

#### Scenario: 非交互冲突无策略
- **WHEN** 非交互模式下目标存在且被视为冲突且未传入 `--target-conflict`
- **THEN** 命令返回错误并提示使用 `--target-conflict=overwrite|skip`

## REMOVED Requirements
### Requirement: 交互式重链接确认与跳过
**原因**：来源目录不再重链接，确认流程不再适用。
**迁移**：无；`llman skills` 默认只读同步。

### Requirement: 非交互模式需显式授权
**原因**：来源目录不再重链接，非交互不需要 `--relink-sources` 授权。
**迁移**：无；非交互仍可运行，但冲突需显式策略。

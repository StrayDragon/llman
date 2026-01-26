# skills-management Specification

## Purpose
描述 llman 在不同来源中发现技能、进行托管快照、处理冲突并为目标路径建立链接的整体流程和约束。
## Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 执行技能同步流程且无需子命令；当终端可交互时，完成同步后进入交互式管理器用于切换目标链接。

#### Scenario: 启动交互式管理器
- **WHEN** 用户运行 `llman skills` 且终端可交互
- **THEN** 同步完成后进入交互式管理器并在用户退出后返回成功

#### Scenario: 非交互模式
- **WHEN** 用户运行 `llman skills` 且终端不可交互
- **THEN** 仅执行同步与链接更新，不进入交互式选择

### Requirement: 可配置来源与目标并提供默认值
技能管理器 MUST 从 `LLMAN_CONFIG_DIR/skills/config.toml` 加载 version=1 的 source/target 配置；若配置缺失，则使用默认来源与目标。repo scope MUST 由运行时基于 git 根目录自动发现。配置路径 MUST 支持 `~` 与环境变量展开。

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

### Requirement: 非项目目录需要确认
当前目录不属于 git 仓库时，管理器 MUST 在交互环境中提示用户确认是否继续扫描 global/user 来源；在非交互环境 MUST 返回错误并退出非零。

#### Scenario: 非项目目录提示确认
- **WHEN** 当前目录不属于 git 仓库且终端可交互
- **THEN** 管理器提示确认，用户取消则不执行扫描

#### Scenario: 非交互非项目目录
- **WHEN** 当前目录不属于 git 仓库且终端不可交互
- **THEN** 命令返回错误且不执行扫描

### Requirement: 尊重忽略规则并跳过软链接
管理器 MUST 在扫描与复制时遵循 `.gitignore` 与全局忽略规则，并跳过软链接目录/文件；被忽略的路径不得导入或重链接。

#### Scenario: 忽略路径不被导入
- **WHEN** 来源路径下存在被 `.gitignore` 排除的技能目录
- **THEN** 管理器不导入该技能且不创建托管记录

#### Scenario: 软链接目录跳过
- **WHEN** 来源中的技能目录是软链接
- **THEN** 管理器跳过该目录且不计算哈希

### Requirement: 自动托管与来源重链接
管理器 MUST 扫描所有启用的来源，将技能目录复制到 `LLMAN_CONFIG_DIR/skills/store/<skill_id>/versions/<hash>`，更新 `LLMAN_CONFIG_DIR/skills/registry.json`，并确保 `LLMAN_CONFIG_DIR/skills/store/<skill_id>/current` 指向选定版本。所有来源目录 MUST 被替换为指向 `current` 的软链接。

#### Scenario: 导入并重链接
- **WHEN** 来源目录包含尚未托管的 `SKILL.md` 技能目录
- **THEN** 管理器将其复制到托管仓库并用软链接替换来源目录

### Requirement: 冲突检测与版本选择
如果多个来源提供同名但内容不同的技能，管理器 MUST 选择激活版本并保留全部版本快照。交互环境下 MUST 提示用户选择；非交互环境下 MUST 自动选择已有 `current_hash`，否则按来源优先级选择（claude > codex > agent，repo scope 额外加分）。

#### Scenario: 冲突选择
- **WHEN** 两个来源都包含 `foo` 且哈希不同且终端可交互
- **THEN** 管理器提示选择，并在托管仓库中保留两个版本

#### Scenario: 非交互自动选择
- **WHEN** 冲突发生且终端不可交互
- **THEN** 管理器自动选择已有版本或按来源优先级选择

### Requirement: 基于内容哈希的快照跟踪
管理器 MUST 基于技能目录中真实文件内容计算 md5（排除软链接与被忽略路径），并为每个唯一哈希存储快照记录。

#### Scenario: 检测到新版本
- **WHEN** 托管技能内容变化导致新的哈希
- **THEN** 管理器记录新的快照而不删除旧快照

### Requirement: 按 agent 目标启用/禁用
管理器 MUST 支持按 agent 目标启用或禁用技能；启用在目标目录下创建 `<skill_id>` 软链接指向托管 `current`，禁用仅移除该软链接。目标路径不存在时 MUST 创建；若目标路径存在但不是目录，必须记录错误并跳过该目标。

#### Scenario: 为单个 agent 禁用技能
- **WHEN** 用户禁用某个技能在指定 agent 目标下
- **THEN** 仅移除该目标下的软链接，托管副本仍保留

#### Scenario: 目标路径非法
- **WHEN** 目标路径存在但不是目录
- **THEN** 记录错误并不创建链接

### Requirement: 技能标识规则
管理器 MUST 使用 `SKILL.md` frontmatter `name` 经过 slugify（小写、非字母数字替换为 `-`、去除首尾 `-`、最多 64 个字符）作为 skill_id；若该字段缺失或 slugify 后为空，则回退目录名。

#### Scenario: name 缺失或非法
- **WHEN** `SKILL.md` 缺失 `name` 或包含非法值
- **THEN** 管理器使用目录名作为 skill_id

#### Scenario: name 需要 slugify
- **WHEN** `SKILL.md` 的 `name` 为 `Slint GUI Expert`
- **THEN** skill_id 为 `slint-gui-expert`

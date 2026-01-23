# skills-management Specification

## Purpose
TBD - created by archiving change add-skills-command. Update Purpose after archive.
## Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 启动交互式管理器且无需子命令。

#### Scenario: 启动交互式管理器
- **WHEN** 用户运行 `llman skills`
- **THEN** 交互式管理器启动并在用户退出后返回成功

### Requirement: 可配置来源与目标并提供默认值
技能管理器 MUST 从 `LLMAN_CONFIG_DIR/skills/config.toml` 加载 global/user 级别来源与目标目录，并在配置缺失时回退到默认值；repo/project scope MUST 由运行时基于 git 根目录自动发现。

#### Scenario: 缺省配置时使用默认值并自动发现项目级来源
- **WHEN** 技能配置文件不存在且当前目录位于 git 仓库内
- **THEN** 管理器使用默认的 global/user 来源与目标，并自动加入仓库根目录下的 `.claude/skills` 与 `.codex/skills` 作为 repo scope

### Requirement: 非项目目录需要确认
当前目录不属于 git 仓库时，管理器 MUST 提示用户确认是否继续扫描 global/user 来源。

#### Scenario: 非项目目录提示确认
- **WHEN** 当前目录不属于 git 仓库
- **THEN** 管理器提示确认，用户取消则不执行扫描

### Requirement: 尊重忽略规则并跳过软链接
管理器 MUST 在扫描时遵循 `.gitignore` 与全局忽略规则，并跳过软链接目录/文件；被忽略的路径不得导入或重链接。

#### Scenario: 忽略路径不被导入
- **WHEN** 来源路径下存在被 `.gitignore` 排除的技能目录
- **THEN** 管理器不导入该技能且不创建托管记录

#### Scenario: 软链接目录跳过
- **WHEN** 来源中的技能目录是软链接
- **THEN** 管理器跳过该目录且不计算哈希

### Requirement: 自动导入并重链接未托管技能
进入管理器时，管理器 MUST 扫描所有配置来源与自动发现的项目来源，将未托管技能导入 `LLMAN_CONFIG_DIR/skills`，并将来源目录替换为指向托管副本的软链接。

#### Scenario: 导入并重链接
- **WHEN** 来源目录包含尚未托管的 `SKILL.md` 技能目录
- **THEN** 管理器将其复制到托管仓库并用软链接替换来源目录

### Requirement: 冲突检测与交互式解决
如果多个来源提供同名但内容不同的技能，管理器 MUST 提示用户选择激活版本，并保留未选版本。

#### Scenario: 冲突选择
- **WHEN** 两个来源都包含 `foo` 且哈希不同
- **THEN** 管理器提示选择，并在托管仓库中保留两个版本

### Requirement: 基于内容哈希的快照跟踪
管理器 MUST 基于技能目录中真实文件内容计算 md5（排除软链接与被忽略路径），并为每个唯一哈希存储快照记录。

#### Scenario: 检测到新版本
- **WHEN** 托管技能内容变化导致新的哈希
- **THEN** 管理器记录新的快照而不删除旧快照

### Requirement: 按 agent 目标启用/禁用
管理器 MUST 支持按 agent 目标启用或禁用技能；启用创建软链接，禁用仅移除软链接。

#### Scenario: 为单个 agent 禁用技能
- **WHEN** 用户禁用某个技能在指定 agent 目标下
- **THEN** 仅移除该目标下的软链接，托管副本仍保留

### Requirement: 技能标识规则
管理器 MUST 使用 `SKILL.md` frontmatter `name` 经过 slugify（小写、非字母数字替换为 `-`、去除首尾 `-`、最多 64 个字符）作为 skill_id；若该字段缺失或 slugify 后为空，则回退目录名。

#### Scenario: name 缺失或非法
- **WHEN** `SKILL.md` 缺失 `name` 或包含非法值
- **THEN** 管理器使用目录名作为 skill_id

#### Scenario: name 需要 slugify
- **WHEN** `SKILL.md` 的 `name` 为 `Slint GUI Expert`
- **THEN** skill_id 为 `slint-gui-expert`

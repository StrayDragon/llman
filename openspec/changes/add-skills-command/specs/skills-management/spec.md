# skills-management 规范

## 目的
提供交互式 `llman skills` 命令，集中管理技能存储并分发到支持的 CLI agent。

## ADDED Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 启动交互式管理器且无需子命令。

#### Scenario: 启动交互式管理器
- **WHEN** 用户运行 `llman skills`
- **THEN** 交互式管理器启动并在用户退出后返回成功

### Requirement: 可配置来源与目标并提供默认值
技能管理器 MUST 从 `LLMAN_CONFIG_DIR/skills` 下的配置文件加载来源/目标目录，并在配置缺失时回退到默认值。

#### Scenario: 缺省配置时使用默认值
- **WHEN** 技能配置文件不存在
- **THEN** 管理器使用包含 Claude、Codex（repo/user/admin scope）与通用 Agent Skills 路径的默认来源/目标目录

### Requirement: 自动导入并重链接未托管技能
进入管理器时，管理器 MUST 扫描所有配置来源，将未托管技能导入 `LLMAN_CONFIG_DIR/skills`，并将来源目录替换为指向托管副本的软链接。

#### Scenario: 导入并重链接
- **WHEN** 来源目录包含尚未托管的 `SKILL.md` 技能目录
- **THEN** 管理器将其复制到托管仓库并用软链接替换来源目录

### Requirement: 冲突检测与交互式解决
如果多个来源提供同名但内容不同的技能，管理器 MUST 提示用户选择激活版本，并保留未选版本。

#### Scenario: 冲突选择
- **WHEN** 两个来源都包含 `foo` 且哈希不同
- **THEN** 管理器提示选择，并在托管仓库中保留两个版本

### Requirement: 基于内容哈希的快照跟踪
管理器 MUST 为每个技能目录计算 md5 并为每个唯一哈希存储快照记录。

#### Scenario: 检测到新版本
- **WHEN** 托管技能内容变化导致新的哈希
- **THEN** 管理器记录新的快照而不删除旧快照

### Requirement: 按 agent 目标启用/禁用
管理器 MUST 支持按 agent 目标启用或禁用技能；启用创建软链接，禁用仅移除软链接。

#### Scenario: 为单个 agent 禁用技能
- **WHEN** 用户禁用某个技能在指定 agent 目标下
- **THEN** 仅移除该目标下的软链接，托管副本仍保留

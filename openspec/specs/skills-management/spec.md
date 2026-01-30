# skills-management Specification

## Purpose
描述 llman 在不同来源中发现技能、进行托管快照、处理冲突并为目标路径建立链接的整体流程和约束。
## Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 在交互式终端执行只读同步（不修改来源目录），并在同步完成后进入交互式管理器；不需要 `--relink-sources` 授权。

#### Scenario: 交互式默认进入管理器
- **WHEN** 用户在交互式终端运行 `llman skills` 且未传入任何 flags
- **THEN** 命令执行只读同步并进入交互式管理器

#### Scenario: 交互式传入 --relink-sources
- **WHEN** 用户在交互式终端运行 `llman skills --relink-sources`
- **THEN** 命令返回错误并提示该参数已移除

### Requirement: 可配置来源与目标并提供默认值
技能管理器 MUST 解析 skills 根目录，优先级如下：
1) CLI `--skills-dir`
2) 环境变量 `LLMAN_SKILLS_DIR`
3) llman 配置文件 `config.yaml` 的 `skills.dir`（本地 `.llman/config.yaml` 优先于 `LLMAN_CONFIG_DIR/config.yaml`）
4) 默认值 `LLMAN_CONFIG_DIR/skills`

技能管理器 MUST 从 `<skills_root>/config.toml` 加载 version=1 的 source/target 配置；若配置缺失，则使用默认来源与目标。repo scope MUST 由运行时基于 git 根目录自动发现。配置路径 MUST 支持 `~` 与环境变量展开。target 配置 MAY 指定 `mode`（`link`、`copy`、`skip`），缺省为 `link`；`skip` 目标必须被跳过且在交互式管理器中以只读状态展示。

#### Scenario: CLI 覆盖 skills 根目录
- **WHEN** 用户运行 `llman skills --skills-dir /tmp/llman.skills`
- **THEN** 管理器使用 `/tmp/llman.skills` 作为 skills 根目录并读取 `/tmp/llman.skills/config.toml`

#### Scenario: 环境变量覆盖 skills 根目录
- **WHEN** 未传入 CLI 覆盖且设置 `LLMAN_SKILLS_DIR`
- **THEN** 管理器使用该环境变量作为 skills 根目录

#### Scenario: 配置文件提供 skills 根目录
- **WHEN** 未传入 CLI/ENV 覆盖且 `config.yaml` 设置 `skills.dir`
- **THEN** 管理器使用 `skills.dir` 作为 skills 根目录

#### Scenario: 本地配置优先于全局配置
- **WHEN** 当前目录存在 `.llman/config.yaml` 且其中设置 `skills.dir`，同时全局 `LLMAN_CONFIG_DIR/config.yaml` 也设置 `skills.dir`
- **THEN** 管理器使用本地 `.llman/config.yaml` 的 `skills.dir`

#### Scenario: 缺省回退到默认路径
- **WHEN** CLI/ENV/config 均未提供 skills 根目录
- **THEN** 管理器使用 `LLMAN_CONFIG_DIR/skills` 作为 skills 根目录

#### Scenario: 缺省配置时使用默认值并自动发现项目级来源
- **WHEN** `<skills_root>/config.toml` 不存在且当前目录位于 git 仓库内
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

### Requirement: 技能标识规则
管理器 MUST 使用 `SKILL.md` frontmatter `name` 经过 slugify（小写、非字母数字替换为 `-`、去除首尾 `-`、最多 64 个字符）作为 skill_id；若该字段缺失或 slugify 后为空，则回退目录名。

#### Scenario: name 缺失或非法
- **WHEN** `SKILL.md` 缺失 `name` 或包含非法值
- **THEN** 管理器使用目录名作为 skill_id

#### Scenario: name 需要 slugify
- **WHEN** `SKILL.md` 的 `name` 为 `Slint GUI Expert`
- **THEN** skill_id 为 `slint-gui-expert`

### Requirement: 自动托管与来源只读
管理器 MUST 扫描所有启用的来源，将技能目录复制到 `LLMAN_CONFIG_DIR/skills/store/<skill_id>/versions/<hash>`，更新 `LLMAN_CONFIG_DIR/skills/registry.json`，并确保 `LLMAN_CONFIG_DIR/skills/store/<skill_id>/current` 指向选定版本。管理器 MUST NOT 替换或重链接来源目录。

#### Scenario: 导入并保持来源不变
- **WHEN** 来源目录包含尚未托管的 `SKILL.md` 技能目录
- **THEN** 管理器将其复制到托管仓库并更新 registry，但来源目录保持不变且不被软链接替换

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

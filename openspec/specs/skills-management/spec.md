# skills-management Specification

## Purpose
描述 llman 在不同来源中发现技能、进行托管快照、处理冲突并为目标路径建立链接的整体流程和约束。
## Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 在交互式终端扫描 `<skills_root>` 并进入交互式选择流程：先选择单个 target（`mode=skip` 目标必须展示为不可选），然后为该 target 展示技能多选列表；默认勾选来自该 target 目录内的实际软链接状态。用户确认后，管理器 MUST 对该 target 执行差异同步：新增项创建软链接、取消项移除软链接。命令 MUST NOT 创建或更新 `store/` 快照。

#### Scenario: 交互式先选目标再选技能
- **WHEN** 用户在交互式终端运行 `llman skills`
- **THEN** 管理器先要求选择一个 target，再展示技能多选列表

#### Scenario: 默认勾选来自目标链接
- **WHEN** 目标目录已有指向技能目录的 `<skill_id>` 软链接
- **THEN** 该技能在列表中默认勾选

#### Scenario: 确认后仅同步差异
- **WHEN** 用户确认选择
- **THEN** 管理器仅对该 target 增删变更项

#### Scenario: 取消不产生变更
- **WHEN** 用户在确认前退出或返回
- **THEN** 不修改任何目标链接且不写入 registry

### Requirement: 尊重忽略规则并跳过软链接
管理器 MUST 在扫描时遵循 `.gitignore` 与全局忽略规则；若技能目录或 `SKILL.md` 为软链接，管理器 MUST 解析其目标并在目标包含 `SKILL.md` 时将其视为可管理技能。

#### Scenario: 忽略路径不被导入
- **WHEN** `<skills_root>` 下存在被 `.gitignore` 排除的技能目录
- **THEN** 管理器不导入该技能且不创建托管记录

#### Scenario: 软链接技能目录可被发现
- **WHEN** `<skills_root>` 下的技能目录是软链接且解析后目录包含 `SKILL.md`
- **THEN** 管理器将其作为可管理技能展示

#### Scenario: 软链接 SKILL.md 可被发现
- **WHEN** `<skills_root>/<skill_id>/SKILL.md` 为软链接且解析后存在有效 `SKILL.md`
- **THEN** 管理器将该目录作为可管理技能展示

### Requirement: 按 agent 目标启用/禁用
管理器 MUST 支持按 agent 目标启用或禁用技能；启用在目标目录下创建 `<skill_id>` 软链接（`mode=link`），`mode=skip` 目标必须跳过。禁用仅移除该链接。目标路径不存在时 MUST 创建；若目标路径存在但不是目录，必须记录错误并跳过该目标。

#### Scenario: 为单个 agent 禁用技能（link）
- **WHEN** 用户禁用某个技能在 `mode=link` 的目标下
- **THEN** 仅移除该目标下的软链接，源目录保持不变

#### Scenario: 目标路径非法
- **WHEN** 目标路径存在但不是目录
- **THEN** 记录错误并不创建链接

#### Scenario: 交互模式下 link 目标发生冲突
- **WHEN** 交互模式下 `mode=link` 目标已存在同名条目且不是期望的软链接
- **THEN** 管理器提示用户选择覆盖或跳过，覆盖时删除后重建链接

#### Scenario: 非交互冲突无策略
- **WHEN** 非交互模式下 `mode=link` 目标存在冲突且未传入 `--target-conflict`
- **THEN** 命令返回错误并提示使用 `--target-conflict=overwrite|skip`

#### Scenario: 非交互冲突使用策略
- **WHEN** 非交互模式下 `mode=link` 目标存在冲突且传入 `--target-conflict=overwrite|skip`
- **THEN** 管理器按策略执行覆盖或跳过

### Requirement: 技能标识规则
管理器 MUST 使用 `SKILL.md` frontmatter `name` 经过 slugify（小写、非字母数字替换为 `-`、去除首尾 `-`、最多 64 个字符）作为 skill_id；若该字段缺失或 slugify 后为空，则回退目录名。

#### Scenario: name 缺失或非法
- **WHEN** `SKILL.md` 缺失 `name` 或包含非法值
- **THEN** 管理器使用目录名作为 skill_id

#### Scenario: name 需要 slugify
- **WHEN** `SKILL.md` 的 `name` 为 `Slint GUI Expert`
- **THEN** skill_id 为 `slint-gui-expert`

### Requirement: 可配置目标并提供默认值
技能管理器 MUST 解析 skills 根目录，优先级如下：
1) CLI `--skills-dir`
2) 环境变量 `LLMAN_SKILLS_DIR`
3) llman 全局配置文件 `LLMAN_CONFIG_DIR/config.yaml` 的 `skills.dir`
4) 默认值 `LLMAN_CONFIG_DIR/skills`

本地 `.llman/config.yaml` 中的 `skills.dir` MUST 被忽略，不得覆盖全局配置。

技能管理器 MUST 从 `<skills_root>/config.toml` 加载 version=2 的 target 配置；若配置缺失，则使用默认 targets。`[[source]]` 配置 MUST 被拒绝并提示迁移。配置路径 MUST 支持 `~` 与环境变量展开。target 配置 MAY 指定 `mode`（`link`、`skip`），缺省为 `link`；`skip` 目标必须被跳过且在交互式管理器中以只读状态展示。

#### Scenario: CLI 覆盖 skills 根目录
- **WHEN** 用户运行 `llman skills --skills-dir /tmp/llman.skills`
- **THEN** 管理器使用 `/tmp/llman.skills` 作为 skills 根目录并读取 `/tmp/llman.skills/config.toml`

#### Scenario: 环境变量覆盖 skills 根目录
- **WHEN** 未传入 CLI 覆盖且设置 `LLMAN_SKILLS_DIR`
- **THEN** 管理器使用该环境变量作为 skills 根目录

#### Scenario: 全局配置文件提供 skills 根目录
- **WHEN** 未传入 CLI/ENV 覆盖且全局 `LLMAN_CONFIG_DIR/config.yaml` 设置 `skills.dir`
- **THEN** 管理器使用该 `skills.dir` 作为 skills 根目录

#### Scenario: 本地配置被忽略
- **WHEN** 当前目录存在 `.llman/config.yaml` 且其中设置 `skills.dir`
- **THEN** 管理器忽略本地配置并继续使用全局配置或默认值

#### Scenario: 缺省回退到默认路径
- **WHEN** CLI/ENV/config 均未提供 skills 根目录
- **THEN** 管理器使用 `LLMAN_CONFIG_DIR/skills` 作为 skills 根目录

#### Scenario: 缺省配置时使用默认目标
- **WHEN** `<skills_root>/config.toml` 不存在
- **THEN** 管理器使用默认 targets

#### Scenario: 不支持的配置版本
- **WHEN** `config.toml` 中 `version` 不是 2
- **THEN** 命令返回错误并提示迁移到 v2

#### Scenario: source 配置被拒绝
- **WHEN** `config.toml` 中包含 `[[source]]`
- **THEN** 命令返回错误并提示仅支持 targets-only 的 v2 配置

#### Scenario: 目标 mode 缺省为 link
- **WHEN** target 配置缺失 `mode`
- **THEN** 该目标使用 `link` 模式

#### Scenario: 目标 mode=skip 被跳过
- **WHEN** target 配置为 `mode = "skip"`
- **THEN** 管理器不同步该目标且在交互式管理器中以只读状态展示

#### Scenario: 不支持的目标 mode
- **WHEN** target 配置包含 `mode = "copy"` 或其他未知值
- **THEN** 命令返回错误并提示仅支持 `link/skip`

### Requirement: 单一来源扫描
技能管理器 MUST 以 `<skills_root>` 作为唯一来源扫描技能目录；来源路径不可配置。

#### Scenario: 扫描单一根目录
- **WHEN** `<skills_root>` 下存在包含 `SKILL.md` 的技能目录
- **THEN** 管理器将其作为可管理技能展示

### Requirement: 启用状态持久化
管理器 MUST 在 `<skills_root>/registry.json` 记录用户确认后的技能/目标启用状态。交互模式 MUST 使用目标目录中的链接状态作为默认选择来源，并且在用户确认之前不得写入或更新 registry。非交互模式下，若 `registry.json` 缺失则回退到 `config.toml` 里的 `enabled` 默认值。

#### Scenario: 交互默认来自文件系统
- **WHEN** 交互模式选择某 target，且 `registry.json` 存在不同状态
- **THEN** 默认勾选仍以目标目录链接状态为准

#### Scenario: 交互取消不写入 registry
- **WHEN** 用户退出而未确认
- **THEN** `registry.json` 不被创建或修改

#### Scenario: 确认后持久化状态
- **WHEN** 用户确认应用
- **THEN** `registry.json` 更新为确认后的状态

#### Scenario: 非交互缺省回退配置默认值
- **WHEN** 非交互模式且 `registry.json` 不存在
- **THEN** 使用 `config.toml` 的 `enabled` 默认值

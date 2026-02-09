# skills-management Specification

## Purpose
描述 llman 在不同来源中发现技能、进行托管快照、处理冲突并为目标路径建立链接的整体流程和约束。
## Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 在交互式终端扫描 `<skills_root>` 后，直接进入既有交互主流程：先选择 agent，再选择 scope，最后进入 skills 多选。`mode=skip` 的 target 必须展示为只读不可切换。用户确认后，管理器 MUST 仅对选定 target 执行差异同步：新增项创建软链接、取消项移除软链接。命令 MUST NOT 创建或更新 `store/` 快照。

#### Scenario: 直接进入 agent 菜单
- **WHEN** 用户运行 `llman skills`
- **THEN** 管理器直接展示 `Select which agent tools to manage`，不再出现 `Select mode`

#### Scenario: Select individually 既有流程保留
- **WHEN** 用户进入交互流程
- **THEN** 管理器按 agent → scope → skills 流程执行

#### Scenario: 取消不产生变更
- **WHEN** 用户在确认前退出或返回
- **THEN** 不修改任何目标链接且不写入持久化状态文件

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

默认 targets MUST 同时覆盖个人范围与项目范围：
- Claude：`user` 与 `project`
- Codex：`user` 与 `repo`
- Agent Skills：`global`

项目范围目标路径 MUST 基于 git 根目录解析：
- Claude project: `<repo_root>/.claude/skills`
- Codex repo: `<repo_root>/.agents/skills`

如果当前目录不在 git 仓库内，项目范围目标 MUST 以 `mode=skip` 只读展示。

Codex user 目标路径 MUST 优先使用 `.agents/skills` 体系，并兼容已有 `.codex/skills` 回退路径。

#### Scenario: 缺省配置时使用默认目标
- **WHEN** `<skills_root>/config.toml` 不存在
- **THEN** 管理器使用默认 targets（claude user/project、codex user/repo、agent global）

#### Scenario: 仓库内启用项目目标
- **WHEN** 当前目录位于 git 仓库内且配置文件缺失
- **THEN** Claude `project` 与 Codex `repo` 默认目标为 `mode=link`，路径指向仓库根目录对应 skills 目录

#### Scenario: 非仓库内项目目标只读
- **WHEN** 当前目录不在 git 仓库内且配置文件缺失
- **THEN** Claude `project` 与 Codex `repo` 默认目标为 `mode=skip`

#### Scenario: Codex user 使用 agents 目录
- **WHEN** 配置文件缺失且用户存在 `~/.agents/skills`
- **THEN** Codex user 默认目标路径使用 `~/.agents/skills`

#### Scenario: Codex user 回退 codex 目录
- **WHEN** 配置文件缺失且 `~/.agents/skills` 不可用但 `~/.codex/skills` 可用
- **THEN** Codex user 默认目标路径回退为 `~/.codex/skills`

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

### Requirement: Skills 重构保持行为一致
Skills 模块重构 MUST 保持技能发现、目标链接、冲突处理与 CLI 输出行为一致，且不得改变配置解析优先级。

#### Scenario: Skills 重构后回归
- **WHEN** `src/skills/` 的模块结构被重组
- **THEN** `llman skills` 的扫描与链接行为保持不变

### Requirement: 断链 symlink 必须被视为已存在条目
在同步 targets 时，技能管理器 MUST 将断链 symlink 视为“已存在的文件系统条目”。即使 `exists()` 为 false，只要 `symlink_metadata()` 表明这是一个 symlink，也必须能够按冲突策略执行覆盖或移除。

#### Scenario: 覆盖断链 symlink
- **WHEN** 某 target 目录中存在名为 `<skill_id>` 的断链 symlink，且用户希望为该 target 启用该 skill
- **THEN** 冲突处理流程会运行，并可按选定冲突策略覆盖该断链 symlink

#### Scenario: 移除断链 symlink
- **WHEN** 某 target 目录中存在名为 `<skill_id>` 的断链 symlink，且用户希望为该 target 禁用该 skill
- **THEN** 该断链 symlink 会被移除

### Requirement: 冲突提示取消必须是安全 no-op
在交互模式下，如果用户取消冲突处理提示，管理器 MUST 将其视为安全的整体 abort（安全退出），并 MUST NOT 产生部分变更。

#### Scenario: 取消冲突提示
- **WHEN** target 存在冲突条目且用户在 overwrite/skip 提示中取消
- **THEN** 命令整体 abort 且不应用任何变更，并以成功状态退出

### Requirement: 交互菜单必须展示 agent 与 scope 语义
交互菜单 MUST 展示与工具语义一致的文案，而不是直接暴露内部 target id。

- Agent 菜单必须包含：`claude`、`codex`、`_agentskills_`、`Exit`
- Claude scope 菜单必须包含：`Personal (All your projects)`、`Project (This project only)`、`Exit`
- Codex scope 菜单必须包含：`User (All your projects)`、`Repo (This project only)`、`Exit`

#### Scenario: Claude scope 文案
- **WHEN** 用户选择 `claude`
- **THEN** scope 菜单展示 `Personal (All your projects)` 与 `Project (This project only)`

#### Scenario: Codex scope 文案
- **WHEN** 用户选择 `codex`
- **THEN** scope 菜单展示 `User (All your projects)` 与 `Repo (This project only)`

#### Scenario: Agent skills 文案
- **WHEN** 用户选择 `_agentskills_`
- **THEN** 管理器直接进入其可用目标（global）并继续技能多选

### Requirement: 项目范围菜单应隐藏用户范围已管理技能
当用户在 `project/repo` scope 下管理技能时，技能列表 SHOULD 隐藏“仅由同 agent 的 `user` scope 链接且当前 scope 未链接”的技能，减少重复噪音；若某技能已在当前 scope 链接，则即使也存在于 user scope，仍 MUST 展示。

#### Scenario: 隐藏仅 user scope 已链接技能
- **WHEN** 用户进入 `claude project` 或 `codex repo` 管理，某技能在同 agent 的 `user` scope 已链接且当前 scope 未链接
- **THEN** 该技能不出现在当前 scope 的多选列表中

#### Scenario: 保留当前 scope 已链接技能
- **WHEN** 用户进入 `project/repo` 管理，某技能在当前 scope 已链接（无论 user scope 是否也链接）
- **THEN** 该技能继续显示在多选列表中

### Requirement: 预设来源与默认推断
管理器 MUST 仅支持运行时目录推断预设：MUST 从技能目录名按 `<preset>.<skill>` 规则自动推断分组。推断得到的预设 MUST 仅存在于运行时，不得依赖或写入任何 registry 持久化字段。

#### Scenario: 自动推断默认预设
- **WHEN** 技能目录名为 `superpowers.brainstorming`
- **THEN** 该目录被归入预设 `superpowers`，并以完整目录名作为该预设成员

#### Scenario: 无分段目录归入 ungrouped
- **WHEN** 技能目录名不包含 `.`
- **THEN** 该技能归入 `ungrouped` 分组

### Requirement: 预设功能仅限交互模式
本变更中，预设能力 MUST 仅通过交互流程提供。`llman skills` MUST NOT 新增任何 presets 专用命令参数。

#### Scenario: 命令行帮助不包含预设参数
- **WHEN** 用户查看 `llman skills --help`
- **THEN** 帮助信息不包含 `--preset`、`--save-preset`、`--list-presets` 等 presets 参数

### Requirement: 技能分组推断与展示
管理器 MUST 根据技能目录名中的 `.` 推断分组：`<group>.<name>` 归入 `<group>`，不含 `.` 的目录归入 `ungrouped`。交互式技能列表 MUST 按分组聚合展示。

#### Scenario: 分组推断
- **WHEN** 技能目录名为 `superpowers.brainstorming`
- **THEN** 该技能归入 `superpowers` 分组

#### Scenario: 无分组技能
- **WHEN** 技能目录名为 `mermaid-expert`
- **THEN** 该技能归入 `ungrouped` 分组

#### Scenario: 分组展示
- **WHEN** 用户进入技能多选列表
- **THEN** 技能按分组聚合显示，并展示分组标题

### Requirement: skills 列表中的分组节点
技能多选列表 MUST 以树形结构展示可选项：父节点为分组节点，子节点为该分组覆盖的具体技能。分组来源 MUST 仅为基于目录名 `<group>.<name>` 自动推断的分组预设。

选择分组项时，管理器 MUST 将其展开为对应技能集合并去重，最终按技能集合应用到目标。
分组项的默认勾选状态 MUST 由当前默认技能集合推导：仅当该分组覆盖的技能集合全部已在默认集合中时，才显示为勾选；否则 MUST 不勾选。

分组项的可视状态 MUST 支持三态：`[ ]`（未选）、`[x]`（全集选中）、`[-]`（部分选中）。
树形选择 MUST 支持关键字过滤搜索：用户输入关键字后，列表仅展示匹配的分组与技能（匹配技能时其父分组必须保留显示）。

#### Scenario: 选择分组自动展开
- **WHEN** 用户在 skills 列表中选择 `dakesan (3 skills)`
- **THEN** 管理器将 `dakesan` 对应技能集合加入最终选择集合

#### Scenario: 树形父子联动
- **WHEN** 用户在树形列表中切换分组父节点
- **THEN** 该父节点下所有子技能同步选中或取消

#### Scenario: 重叠技能去重
- **WHEN** 用户同时选择多个分组，且它们包含同一技能
- **THEN** 最终应用集合中该技能只保留一份

#### Scenario: 搜索过滤保留父节点
- **WHEN** 用户在树形选择中输入关键字，仅匹配到某个技能
- **THEN** 该技能所属分组仍显示，且仅显示匹配到的子技能

#### Scenario: preset 默认勾选为“全集命中”
- **WHEN** 当前默认集合仅包含某 preset 的部分技能
- **THEN** 该分组默认状态不勾选

### Requirement: 技能条目展示应同时包含 skill_id 与目录名
交互式技能列表中的每个技能项 MUST 展示为 `skill_id (directory_name)`，用于明确用户可选标识与其目录来源。当技能目录分组与 skill_id 不一致时，管理器 MUST 仍按该格式展示。

#### Scenario: 展示 skill_id 与目录名
- **WHEN** 技能 `skill_id` 为 `brainstorming`，目录名为 `superpowers.brainstorming`
- **THEN** 交互选项显示为 `brainstorming (superpowers.brainstorming)`

### Requirement: 启用状态实时计算
管理器 MUST 不再依赖任何 registry 文件作为技能启用状态来源。交互与非交互流程都 MUST 基于目标目录真实链接状态实时计算技能启用状态，并仅将 `config.toml` 的 `target.enabled` 作为“当前未链接时”的默认回退值。

#### Scenario: 交互默认来自文件系统
- **WHEN** 交互模式选择某 target
- **THEN** 默认勾选基于目标目录的真实链接状态计算

#### Scenario: 非交互已链接优先
- **WHEN** 非交互模式下某技能在某 target 已存在正确链接
- **THEN** 管理器保持该技能在该 target 启用，不因配置默认值覆盖

#### Scenario: 非交互未链接回退配置默认值
- **WHEN** 非交互模式下某技能在某 target 当前未链接
- **THEN** 管理器使用该 target 的 `enabled` 默认值决定是否创建链接

#### Scenario: 运行不写持久化状态文件
- **WHEN** 管理器完成交互或非交互会话
- **THEN** 管理器不会创建或更新任何 registry 状态文件

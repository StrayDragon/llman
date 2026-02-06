## MODIFIED Requirements
### Requirement: 交互式技能管理入口
`llman skills` MUST 在交互式终端扫描 `<skills_root>` 并进入三段式交互流程：先选择 agent tool（如 `claude`、`codex`、`_agentskills_`），再为该 agent 选择 scope，最后展示该 scope 对应 target 的技能多选列表。`mode=skip` 的 target 必须展示为只读不可切换。默认勾选来自该 target 目录内的实际软链接状态。用户确认后，管理器 MUST 仅对该 target 执行差异同步：新增项创建软链接、取消项移除软链接。命令 MUST NOT 创建或更新 `store/` 快照。

#### Scenario: 交互式先选 agent 再选 scope 再选技能
- **WHEN** 用户在交互式终端运行 `llman skills`
- **THEN** 管理器先要求选择 agent tool，再选择 scope，最后展示技能多选列表

#### Scenario: scope 级别目标选择
- **WHEN** 用户选择 `claude` 后再选择 `Project (This project only)`
- **THEN** 管理器仅针对 Claude 的 project target 展示默认勾选并执行后续同步

#### Scenario: 默认勾选来自目标链接
- **WHEN** 目标目录已有指向技能目录的 `<skill_id>` 软链接
- **THEN** 该技能在列表中默认勾选

#### Scenario: 确认后仅同步差异
- **WHEN** 用户确认选择
- **THEN** 管理器仅对该 target 增删变更项

#### Scenario: 取消不产生变更
- **WHEN** 用户在确认前退出或返回
- **THEN** 不修改任何目标链接且不写入 registry

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

## ADDED Requirements
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

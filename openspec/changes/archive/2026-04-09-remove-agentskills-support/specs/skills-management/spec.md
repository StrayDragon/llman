# skills-management Delta Spec

## MODIFIED Requirements

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

项目范围目标路径 MUST 基于 git 根目录解析：
- Claude project: `<repo_root>/.claude/skills`
- Codex repo: `<repo_root>/.agents/skills`

如果当前目录不在 git 仓库内，项目范围目标 MUST 以 `mode=skip` 只读展示。

Codex user 目标路径 MUST 优先使用 `.agents/skills` 体系，并兼容已有 `.codex/skills` 回退路径。

#### Scenario: 缺省配置时使用默认目标
- **WHEN** `<skills_root>/config.toml` 不存在
- **THEN** 管理器使用默认 targets（claude user/project、codex user/repo）

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

---

### Requirement: 交互菜单必须展示 agent 与 scope 语义

交互菜单 MUST 展示与工具语义一致的文案，而不是直接暴露内部 target id。

- Agent 菜单必须包含：`claude`、`codex`、`Exit`
- Claude scope 菜单必须包含：`Personal (All your projects)`、`Project (This project only)`、`Exit`
- Codex scope 菜单必须包含：`User (All your projects)`、`Repo (This project only)`、`Exit`

#### Scenario: Claude scope 文案
- **WHEN** 用户选择 `claude`
- **THEN** scope 菜单展示 `Personal (All your projects)` 与 `Project (This project only)`

#### Scenario: Codex scope 文案
- **WHEN** 用户选择 `codex`
- **THEN** scope 菜单展示 `User (All your projects)` 与 `Repo (This project only)`

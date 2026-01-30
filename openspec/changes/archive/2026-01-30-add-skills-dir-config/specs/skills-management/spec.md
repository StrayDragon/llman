## MODIFIED Requirements
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

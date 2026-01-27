# sdd-workflow Specification

## Purpose
Define the llman SDD workflow and its OpenSpec-compatible behaviors for `llmanspec/`.
## Requirements
### Requirement: SDD 初始化脚手架
`llman sdd init [path]` 命令 MUST 在目标路径创建 `llmanspec/` 目录结构，包括 `llmanspec/AGENTS.md`、`llmanspec/project.md`、`llmanspec/specs/`、`llmanspec/changes/` 与 `llmanspec/changes/archive/`，以及 `llmanspec/templates/spec-driven/` 下的 `proposal.md`、`spec.md`、`design.md`、`tasks.md`。命令 MUST 生成 `llmanspec/config.yaml` 并写入 locale 配置。命令 MUST 创建或刷新 repo 根目录 `AGENTS.md` 的 LLMANSPEC 受管块并指向 `llmanspec/AGENTS.md`。当 `llmanspec/` 已存在时，命令 MUST 报错并且不修改任何文件。生成的 `llmanspec/AGENTS.md` MUST 包含完整 llman sdd 方法论说明且包含 LLMANSPEC 受管提示块。

#### Scenario: 初始化新项目
- **WHEN** 用户在不存在 `llmanspec/` 的目录执行 `llman sdd init`
- **THEN** 必要的目录结构与模板文件被创建

#### Scenario: 初始化指定路径
- **WHEN** 用户执行 `llman sdd init <path>`
- **THEN** 在 `<path>` 下创建 `llmanspec/` 结构与模板文件

#### Scenario: 初始化时生成提示块
- **WHEN** `llman sdd init` 生成 `llmanspec/AGENTS.md`
- **THEN** 文件中包含 `<!-- LLMANSPEC:START -->` 与 `<!-- LLMANSPEC:END -->` 包裹的提示块

#### Scenario: 初始化时写入配置
- **WHEN** `llman sdd init --lang en` 运行
- **THEN** `llmanspec/config.yaml` 被写入且 locale 为 `en`

#### Scenario: 初始化时生成根 AGENTS
- **WHEN** `llman sdd init` 运行
- **THEN** repo 根目录 `AGENTS.md` 被创建或刷新受管块并指向 `llmanspec/AGENTS.md`

#### Scenario: 已存在 llmanspec 目录
- **WHEN** 用户在已有 `llmanspec/` 的目录执行 `llman sdd init`
- **THEN** 命令返回错误且不做任何更改

#### Scenario: openspec 共存
- **WHEN** `openspec/` 已存在但 `llmanspec/` 不存在
- **THEN** `llman sdd init` 仅创建 `llmanspec/` 且不修改 `openspec/`

### Requirement: SDD 指令与提示词刷新
`llman sdd update [path]` MUST 刷新 `llmanspec/AGENTS.md` 与 `llmanspec/templates/spec-driven/` 内置模板，同时 MUST 保持 `llmanspec/specs/**` 与 `llmanspec/changes/**` 不被修改。命令 MUST 刷新 repo 根目录 `AGENTS.md` 的 LLMANSPEC 受管块并保留非受管内容。更新 `llmanspec/AGENTS.md` 时 MUST 仅替换 LLMANSPEC 受管提示块，并保留受管块以外的用户内容。更新时必须使用 `llmanspec/config.yaml` 的 locale 选择模板。

#### Scenario: 更新指令文件
- **WHEN** 用户执行 `llman sdd update`
- **THEN** 指令/模板文件被刷新且现有 specs 与 changes 内容保持不变

#### Scenario: 更新指定路径
- **WHEN** 用户执行 `llman sdd update <path>`
- **THEN** 仅更新 `<path>/llmanspec/` 下的指令与模板文件

#### Scenario: 未初始化时更新
- **WHEN** 目标路径下不存在 `llmanspec/`
- **THEN** 命令返回错误并提示先执行 `llman sdd init`

#### Scenario: 保留用户自定义内容
- **WHEN** `llmanspec/AGENTS.md` 含有用户自定义内容且包含 LLMANSPEC 受管块
- **THEN** update 仅替换受管块并保留其他内容

#### Scenario: 更新根 AGENTS
- **WHEN** repo 根目录 `AGENTS.md` 存在且包含 LLMANSPEC 受管块
- **THEN** update 仅替换受管块并保留其他内容

#### Scenario: openspec 共存
- **WHEN** `openspec/` 与 `llmanspec/` 同时存在
- **THEN** `llman sdd update` 不修改 `openspec/`

### Requirement: SDD 本地化配置与模板加载
`llman sdd` MUST 使用项目级配置 `llmanspec/config.yaml` 解析 locale，并基于 `templates/sdd/<locale>/` 加载 `llmanspec/AGENTS.md`、`llmanspec/templates/**` 与 sdd skills 内容。locale 仅影响模板与 skills 输出，不影响 CLI 文本。

#### Scenario: locale 回退链
- **WHEN** 配置 locale 为 `zh-Hans` 但缺少 `templates/sdd/zh-Hans/...`
- **THEN** 按 `zh-Hans` → `zh` → `en` 顺序回退

#### Scenario: locale 仅影响模板与 skills
- **WHEN** `llmanspec/config.yaml` 设置 locale 为 `zh-Hans`
- **THEN** `llmanspec/AGENTS.md` 与 sdd skills 使用中文模板
- **AND** CLI 输出仍保持英文

### Requirement: SDD Skills 生成与更新
`llman sdd update-skills` MUST 生成并更新 Claude Code 与 Codex 的技能目录（不生成 slash commands）。默认输出路径来自 `llmanspec/config.yaml`，交互模式允许输入覆盖路径；非交互模式必须通过 `--all` 或 `--tool` 指定目标，并可选 `--path` 覆盖输出路径。若目标技能已存在，命令 MUST 刷新托管内容以保持一致性。生成的 skills MUST 包含 sdd 校验修复提示与最小示例。

#### Scenario: 交互式技能生成
- **WHEN** 用户在可交互终端执行 `llman sdd update-skills`
- **THEN** 可选择 Claude Code 或 Codex，并使用默认路径或输入自定义路径生成技能

#### Scenario: 非交互技能生成
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude`
- **THEN** 命令在 Claude Code 技能路径下生成/更新技能

#### Scenario: 非交互更新全部
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** 命令生成/更新 Claude Code 与 Codex 的技能

#### Scenario: 更新已有技能
- **WHEN** 目标路径中存在同名技能目录
- **THEN** `SKILL.md` 被托管内容刷新

### Requirement: SDD 模板区域复用
SDD 模板与 skills MUST 支持 `{{region: <path>#<name>}}` 引用，系统 MUST 使用源文件中与文件类型匹配的 `region` 块替换占位符内容。文件类型匹配规则：Markdown/HTML 使用 `<!-- region: name -->` / `<!-- endregion -->`；YAML/TOML/INI/Shell 使用 `# region: name` / `# endregion`；Rust/JS/TS 使用 `// region: name` / `// endregion`。若 region 缺失或重复，命令 MUST 报错并中止。

#### Scenario: 引用 Markdown region
- **WHEN** 模板包含 `{{region: docs/sdd.md#overview}}` 且目标文件存在 `<!-- region: overview -->` 块
- **THEN** 生成结果中替换为对应 region 内容

#### Scenario: region 缺失
- **WHEN** 模板引用的 region 在目标文件中不存在
- **THEN** 命令报错并退出非零

### Requirement: SDD 命令范围
`llman sdd` MUST 仅暴露 OpenSpec 工作流的核心命令：`init`、`update`、`update-skills`、`list`、`show`、`validate`、`archive`。在 SDD 子命令组中 MUST 不提供 `change`、`spec`、`view`、`completion`、`config` 等额外子命令。

#### Scenario: 帮助文本仅包含核心命令
- **WHEN** 用户执行 `llman sdd --help`
- **THEN** 帮助文本仅包含 `init`、`update`、`update-skills`、`list`、`show`、`validate`、`archive`
- **AND** 不包含 `change`、`spec`、`view`、`completion`、`config`

### Requirement: SDD 列表与查看
`llman sdd list` 默认 MUST 列出 `llmanspec/changes/` 下除 `archive` 外的变更 ID，提供 `--specs` 时 MUST 列出 `llmanspec/specs/` 下的 spec ID，提供 `--changes` 时 MUST 显式列出变更。`llman sdd list` MUST 支持 `--sort`（默认 `recent`，可选 `name`）。`llman sdd show` MUST 输出指定 change/spec 的原始 markdown（非 JSON 模式），并遵循 OpenSpec 的自动识别与 `--type change|spec` 覆盖规则。`list` 与 `show` MUST 支持 `--json` 机器可读输出：change JSON 输出 `id/title/deltaCount/deltas`，spec JSON 输出 `id/title/overview/requirementCount/requirements/metadata`。spec JSON MUST 支持 `--requirements`、`--no-scenarios` 与 `--requirement` 过滤（`--requirements` 与 `--requirement` 冲突时报错）。`--requirements-only` 作为 `--deltas-only` 的弃用别名，仅提示警告且不改变输出。

#### Scenario: 默认列出变更
- **WHEN** 用户执行 `llman sdd list`
- **THEN** 输出包含 `llmanspec/changes/` 下的变更目录（排除 `archive`）

#### Scenario: 列出 specs
- **WHEN** 用户执行 `llman sdd list --specs`
- **THEN** 输出包含 `llmanspec/specs/` 下的 spec 目录

#### Scenario: 列出变更（显式）
- **WHEN** 用户执行 `llman sdd list --changes`
- **THEN** 输出包含 `llmanspec/changes/` 下的变更目录（排除 `archive`）

#### Scenario: 查看变更
- **WHEN** 用户执行 `llman sdd show <change-id> --type change`
- **THEN** 输出 `llmanspec/changes/<change-id>/proposal.md` 的原始内容

#### Scenario: 查看 spec
- **WHEN** 用户执行 `llman sdd show <spec-id> --type spec`
- **THEN** 输出 `llmanspec/specs/<spec-id>/spec.md` 的原始内容

#### Scenario: 自动识别与歧义处理
- **WHEN** 用户执行 `llman sdd show <item-name>` 且未指定 `--type`
- **THEN** 自动识别 change/spec；若同时匹配则报错并提示使用 `--type change|spec`

#### Scenario: JSON 输出（changes）
- **WHEN** 用户执行 `llman sdd list --json`
- **THEN** 输出 JSON 结构：`{ "changes": [{ "name": "...", "completedTasks": 0, "totalTasks": 0, "lastModified": "...", "status": "no-tasks|in-progress|complete" }] }`

#### Scenario: JSON 输出（specs）
- **WHEN** 用户执行 `llman sdd list --specs --json`
- **THEN** 输出 JSON 数组，元素包含 `{ "id": "...", "title": "...", "requirementCount": 0 }`（与 `openspec spec list --json` 一致）

#### Scenario: JSON 输出（show）
- **WHEN** 用户执行 `llman sdd show <id> --json`
- **THEN** 输出 OpenSpec 对齐的 JSON（change: `id/title/deltaCount/deltas...`；spec: `id/title/overview/requirementCount/requirements/metadata...`）

#### Scenario: JSON 输出（spec 过滤）
- **WHEN** 用户执行 `llman sdd show <spec-id> --json --requirement 1 --no-scenarios`
- **THEN** 仅返回指定序号的 requirement 且不包含 scenarios

#### Scenario: JSON 输出（参数冲突）
- **WHEN** 用户执行 `llman sdd show <spec-id> --json --requirements --requirement 1`
- **THEN** 命令返回错误并提示参数冲突

### Requirement: SDD 交互提示与非交互提示
`llman sdd show` 与 `llman sdd validate` MUST 按 OpenSpec 的交互体验实现：在可交互环境下提供选择式流程；在不可交互或显式禁用交互时输出一致的提示语并退出非零。

#### Scenario: show 交互选择
- **WHEN** 用户执行 `llman sdd show` 且终端可交互
- **THEN** 提示选择类型，提示语为 `What would you like to show?`
- **AND** 选中 change 时提示 `Pick a change` 并列出 `llmanspec/changes/` 中的 ID
- **AND** 选中 spec 时提示 `Pick a spec` 并列出 `llmanspec/specs/` 中的 ID

#### Scenario: show 非交互提示语
- **WHEN** 用户执行 `llman sdd show` 且为非交互环境或使用 `--no-interactive`
- **THEN** 输出以下提示并退出码为 1：
  - `Nothing to show. Try one of:`
  - `  llman sdd show <item>`
  - `  llman sdd show --type change`
  - `  llman sdd show --type spec`
  - `Or run in an interactive terminal.`

#### Scenario: validate 交互选择
- **WHEN** 用户执行 `llman sdd validate` 且终端可交互
- **THEN** 提示选择验证范围，提示语为 `What would you like to validate?`
- **AND** 提供选项：`All (changes + specs)`、`All changes`、`All specs`、`Pick a specific change or spec`
- **AND** 当选择 `Pick a specific change or spec` 时提示语为 `Pick an item`，并列出 `change/<id>` 与 `spec/<id>`

#### Scenario: validate 非交互提示语
- **WHEN** 用户执行 `llman sdd validate` 且为非交互环境或使用 `--no-interactive`
- **THEN** 输出以下提示并退出码为 1：
  - `Nothing to validate. Try one of:`
  - `  llman sdd validate --all`
  - `  llman sdd validate --changes`
  - `  llman sdd validate --specs`
  - `  llman sdd validate <item-name>`
  - `Or run in an interactive terminal.`

### Requirement: SDD 校验
`llman sdd validate` MUST 校验 spec 与 delta 的格式：spec 必须包含 `## Purpose` 与 `## Requirements`，每个 requirement 文本必须包含 `SHALL` 或 `MUST`，且至少包含一个 `#### Scenario:`；delta 必须使用 `## ADDED|MODIFIED|REMOVED|RENAMED Requirements`，每个 requirement 块必须包含文本与场景，同一 requirement 不得出现在多个 section，`RENAMED` section 若存在必须包含 `FROM/TO` 对。命令 MUST 支持 `--all --changes --specs --type change|spec --strict --no-interactive --json`。`--json` 输出 MUST 采用 OpenSpec 顶层 validate 结构（`items`/`summary`/`version`），单项校验亦使用同一结构。

#### Scenario: 非法场景标题
- **WHEN** 某个 requirement 使用了不合法的场景标题（非 `#### Scenario:`）
- **THEN** 校验失败并报告具体文件

#### Scenario: 缺少 Purpose 或 Requirements
- **WHEN** spec 缺少 `## Purpose` 或 `## Requirements` 章节
- **THEN** 校验失败并报告具体文件

#### Scenario: Requirement 缺少 SHALL/MUST
- **WHEN** requirement 文本未包含 `SHALL` 或 `MUST`
- **THEN** 校验失败并报告具体 requirement

#### Scenario: RENAMED 缺少配对
- **WHEN** delta 包含 `## RENAMED Requirements` 但未提供 FROM/TO 配对
- **THEN** 校验失败并报告具体文件

#### Scenario: 合法变更
- **WHEN** 变更包含完整且格式正确的 deltas 与必需文件
- **THEN** 校验成功并返回退出码 0

#### Scenario: 校验 JSON 输出
- **WHEN** 用户执行 `llman sdd validate <id> --json`
- **THEN** 输出包含 `items`、`summary` 与 `version` 的机器可读 JSON

#### Scenario: 校验歧义
- **WHEN** `llman sdd validate <item-name>` 同时匹配 change 与 spec
- **THEN** 报错并提示使用 `--type change|spec`

### Requirement: SDD 校验提示增强
`llman sdd validate` 在非 JSON 模式下 MUST 为常见结构错误提供可执行修复提示，包含最小示例片段与推荐命令。

#### Scenario: 缺少必需章节
- **WHEN** spec 缺少 `## Purpose` 或 `## Requirements`
- **THEN** 输出提示包含期望标题与最小示例片段

#### Scenario: 场景标题格式错误
- **WHEN** requirement 使用了非 `#### Scenario:` 标题
- **THEN** 输出提示包含标准 `#### Scenario:` 示例

#### Scenario: 无 delta 变更
- **WHEN** change 目录中未找到任何 delta
- **THEN** 输出提示包含 delta section 示例与文件路径提示

### Requirement: Spec 校验元数据（YAML Frontmatter）
`llmanspec/specs/<id>/spec.md` MUST 以 YAML frontmatter 开头，包含以下键：
- `llman_spec_valid_scope`
- `llman_spec_valid_commands`
- `llman_spec_evidence`

`llman_spec_valid_scope` MUST 是 repo 根目录相对路径的列表或单个字符串；列表项/字符串可以用逗号分隔多个路径。校验范围匹配规则 MUST 为“路径完全相等或以 `<path>/` 为前缀”。`llman_spec_valid_commands` 与 `llman_spec_evidence` MUST 为非空字符串或字符串列表，用于记录最小验证证据。`llman sdd validate` MUST 在校验 spec 时解析并校验这些元数据，缺失或为空 MUST 视为校验失败并提示。

#### Scenario: 缺少 frontmatter
- **WHEN** spec 文件缺少 YAML frontmatter 或缺少任一必需键
- **THEN** `llman sdd validate` 报错并提示补充校验元数据

#### Scenario: Frontmatter 字符串与列表
- **WHEN** `llman_spec_valid_scope` 使用字符串或 YAML 列表
- **THEN** `llman sdd validate` 解析为等价的路径列表

### Requirement: Spec 过期（staleness）校验
`llman sdd validate` MUST 对 specs 进行 staleness 评估：
- 基准 MUST 使用 `git merge-base <base-ref> HEAD`，`<base-ref>` 默认 `origin/main`，不存在时回退 `origin/master`，并允许通过 `LLMANSPEC_BASE_REF` 覆盖。
- 校验范围 MUST 使用 `git diff --name-only <base>..HEAD` 获取变更路径，并与 `llman_spec_valid_scope` 匹配。
- 若变更触及范围且 `llmanspec/specs/<id>/spec.md` 未在 `<base>..HEAD` 中更新，则 MUST 标记为 `STALE`。
- 若 spec 更新但范围未触及，则 MUST 记录为 `INFO`。
- 若 Git 无法解析 base（如无 remote / 无 merge-base），则 MUST 记录 `WARN` 并跳过 staleness 评估；`--strict` MUST 将 `WARN` 升级为 `ERROR`。
- 若工作区存在未提交变更（`git status --porcelain` 非空），则 MUST 记录 `WARN`；`--strict` MUST 将其升级为 `ERROR`。
- 子模块指针变更若路径匹配范围 MUST 视为触及范围。
- staleness 状态 MUST 为 `OK|STALE|INFO|WARN|NOT_APPLICABLE`；非 spec 项（change 校验）使用 `NOT_APPLICABLE`。

`llman sdd validate --json` MUST 在每个 `items[]` 中新增 `staleness` 字段，包含 `status`、`baseRef`、`scope`、`touchedPaths`、`specUpdated`、`dirty` 与 `notes`。文本输出 MUST 在每个 spec 的校验结果中提示 staleness 状态。

#### Scenario: 范围触及但 spec 未更新
- **WHEN** `<base>..HEAD` 变更触及 `llman_spec_valid_scope`，且 spec 文件未更新
- **THEN** staleness 状态为 `STALE` 且在输出中提示

#### Scenario: spec 更新但范围未触及
- **WHEN** spec 文件更新，但变更未触及 `llman_spec_valid_scope`
- **THEN** staleness 状态为 `INFO`

#### Scenario: 无法解析 base
- **WHEN** git 无法解析 merge-base
- **THEN** staleness 记录为 `WARN`，`--strict` 时视为错误

### Requirement: SDD 归档流程
`llman sdd archive` MUST 将 delta 合并到 `llmanspec/specs` 并将变更目录移动到 `llmanspec/changes/archive/YYYY-MM-DD-<change-id>`。当目标 spec 不存在时 MUST 创建包含默认 frontmatter 与 Purpose 占位的 skeleton，且仅允许 `ADDED` requirements。命令 MUST 支持 `--skip-specs` 以在不更新 specs 的情况下归档；当 MODIFIED/REMOVED/RENAMED 引用不存在的 requirement 时 MUST 报错并中止。

#### Scenario: 归档并更新 specs
- **WHEN** 用户执行 `llman sdd archive <change-id>`
- **THEN** specs 被 delta 更新且变更目录被移动到 archive

#### Scenario: 新 spec 创建
- **WHEN** 归档的 delta 指向不存在的 spec 且仅包含 `ADDED` requirements
- **THEN** 归档创建包含默认 frontmatter 与 Purpose 占位文本的 spec skeleton

#### Scenario: 新 spec 含非 ADDED
- **WHEN** 归档的 delta 指向不存在的 spec 且包含 `MODIFIED`/`REMOVED`/`RENAMED`
- **THEN** 归档失败并输出错误提示

#### Scenario: 仅归档目录
- **WHEN** 用户执行 `llman sdd archive <change-id> --skip-specs`
- **THEN** 变更目录被移动到 archive 且 specs 不被修改

#### Scenario: 缺失 requirement 报错
- **WHEN** MODIFIED/REMOVED/RENAMED 指向不存在的 requirement
- **THEN** 归档失败并输出错误提示

### Requirement: SDD 归档 dry-run
`llman sdd archive --dry-run` MUST 输出将要修改/移动的文件与目标路径，并 MUST 不进行任何文件写入。

#### Scenario: 归档 dry-run
- **WHEN** 用户执行 `llman sdd archive <change-id> --dry-run`
- **THEN** 输出预览信息且文件系统无任何改动

### Requirement: SDD 模板版本元信息
SDD locale 模板 MUST 包含 `llman-template-version` 元信息。对于带 YAML frontmatter 的模板，frontmatter MUST 包含 `llman-template-version` 字段；其他模板 MUST 在第一行使用 `<!-- llman-template-version: N -->` 形式。相同相对路径的不同 locale 模板 MUST 使用相同的版本值。仓库 MUST 提供维护者检查命令以验证版本一致性与模板集合一致性。

#### Scenario: 模板版本一致性检查
- **WHEN** 维护者运行 `just check-sdd-templates`
- **THEN** 命令在缺失元信息、缺少 locale 模板或版本不一致时退出非零
- **AND** 在所有模板一致时退出零

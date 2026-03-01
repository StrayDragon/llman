# sdd-workflow Specification

## Purpose
Define the llman SDD workflow and its OpenSpec-compatible behaviors for `llmanspec/`.
## Requirements
### Requirement: SDD 初始化脚手架
`llman sdd init [path]` 命令 MUST 在目标路径创建 `llmanspec/` 目录结构，包括 `llmanspec/AGENTS.md`、`llmanspec/project.md`、`llmanspec/specs/`、`llmanspec/changes/` 与 `llmanspec/changes/archive/`，以及 `llmanspec/templates/spec-driven/` 下的 `proposal.md`、`spec.md`、`design.md`、`tasks.md`。命令 MUST 生成 `llmanspec/config.yaml` 并写入 locale 配置。命令 MUST 在 `llmanspec/config.yaml` 顶部写入 `yaml-language-server` schema 头注释，指向 `llmanspec-config` schema URL。命令 MUST 创建或刷新 repo 根目录下的 `AGENTS.md` 受管块以指向 `llmanspec/AGENTS.md`。当 `llmanspec/` 已存在时，命令 MUST 报错并且不修改任何文件。生成的 `llmanspec/AGENTS.md` MUST 包含 LLMANSPEC 受管提示块且包含完整 llman sdd 方法论说明。

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

#### Scenario: 初始化时写入 schema 头注释
- **WHEN** `llman sdd init` 生成 `llmanspec/config.yaml`
- **THEN** 文件顶部包含 `yaml-language-server` schema 头注释

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

#### Scenario: 初始化写入 locale 配置
- **WHEN** 用户执行 `llman sdd init --lang zh-Hans`
- **THEN** `llmanspec/config.yaml` 写入 `locale: zh-Hans` 且包含版本字段

#### Scenario: locale 回退链
- **WHEN** 配置 locale 为 `zh-Hans` 但缺少 `templates/sdd/zh-Hans/...`
- **THEN** 按 `zh-Hans` → `zh` → `en` 顺序回退

#### Scenario: locale 仅影响模板与 skills
- **WHEN** `llmanspec/config.yaml` 设置 locale 为 `zh-Hans`
- **THEN** `llmanspec/AGENTS.md` 与 sdd skills 使用中文模板
- **AND** CLI 输出仍保持英文

### Requirement: SDD Skills 生成与更新
`llman sdd update-skills` 生成的 workflow skills 集 MUST 新增 `llman-sdd-specs-compact`，并在现有 `llman-sdd-*` skills 中使用统一结构化提示协议（见 `sdd-structured-skill-prompts` capability）。

#### Scenario: 生成包含 specs-compact skill
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool codex`
- **THEN** 目标路径下存在 `llman-sdd-specs-compact/SKILL.md`

#### Scenario: 结构化协议在已生成技能中可见
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --all`
- **THEN** 生成的 `llman-sdd-archive` 与 `llman-sdd-explore` 等技能均包含统一结构化章节

### Requirement: SDD 模板区域复用
SDD 模板与 skills MUST 使用基于 MiniJinja 的模板单元注入机制进行复用。系统 MUST 通过显式模板单元标识符完成注入渲染，并在缺失单元、重复注册或渲染失败时报错并中止。

#### Scenario: 引用模板单元并成功渲染
- **WHEN** 模板声明注入一个已注册的共享单元
- **THEN** 生成结果中包含该单元的渲染内容且无未解析占位符

#### Scenario: 模板单元缺失
- **WHEN** 模板声明的单元标识符在当前 locale 与回退链中都不存在
- **THEN** 命令报错并退出非零

#### Scenario: 模板单元注册冲突
- **WHEN** 同一渲染上下文中存在重复的单元标识符定义
- **THEN** 命令报错并拒绝继续渲染

### Requirement: SDD 命令范围
`llman sdd` MUST 暴露以下命令集合：`init`、`update`、`update-skills`、`list`、`show`、`validate`、`archive`、`import`、`export`。
其中 `import` 与 `export` MUST 作为 `llmanspec` 与外部规范目录互转的唯一入口。实现 MUST NOT 暴露 `migrate --from/--to` 兼容别名。
在 SDD 子命令组中 MUST 不提供 `change`、`spec`、`view`、`completion`、`config` 等额外子命令。

#### Scenario: 帮助文本包含 import/export
- **WHEN** 用户执行 `llman sdd --help`
- **THEN** 帮助文本包含 `import` 与 `export`
- **AND** 帮助文本不包含 `migrate`

#### Scenario: style 参数强约束
- **WHEN** 用户执行 `llman sdd import` 或 `llman sdd export` 且缺少 `--style`
- **THEN** 命令返回非零并提示 `--style openspec` 为必填

#### Scenario: 旧命名不可用
- **WHEN** 用户执行 `llman sdd migrate --from openspec`
- **THEN** 命令返回未知子命令错误

### Requirement: SDD OpenSpec 双向互转
系统 MUST 提供：
- `llman sdd import --style openspec [path]`，将 `openspec/` 迁移到 `llmanspec/`
- `llman sdd export --style openspec [path]`，将 `llmanspec/` 迁移到 `openspec/`

`--style` MUST 为必填，且当前仅允许值 `openspec`。命令 MUST 覆盖 `specs`、active `changes` 与 `changes/archive`。源侧非标准目录（例如 `explorations/`）MUST 输出 warning，并按相对路径复制到目标侧。目标存在同名文件冲突时 MUST 失败且 MUST NOT 覆盖。

#### Scenario: style 参数缺失
- **WHEN** 用户执行 `llman sdd import` 或 `llman sdd export` 且未传 `--style`
- **THEN** 命令返回非零并提示需要 `--style openspec`

#### Scenario: style 参数非法
- **WHEN** 用户执行 `llman sdd import --style unknown`
- **THEN** 命令返回非零并提示仅支持 `openspec`

#### Scenario: 非标准目录复制并警告
- **WHEN** 源目录包含非标准目录（例如 `openspec/explorations/`）
- **THEN** 命令输出 warning
- **AND** 执行写入阶段复制该目录到目标侧相同相对路径

#### Scenario: 目标冲突即失败
- **WHEN** 目标目录存在计划写入的同名文件
- **THEN** 命令返回非零并中止
- **AND** 不覆盖任何冲突文件

### Requirement: SDD 互转执行安全门禁
`import/export` MUST 默认先输出完整 dry-run 计划。真实写入 MUST 仅在交互终端中通过双确认后执行（`Confirm` + 确认短语）。非交互环境 MUST 在输出 dry-run 后拒绝写入并返回非零。迁移写入成功后 MUST 提示是否删除旧迁移目录（源目录），默认 MUST 为“不删除”。

#### Scenario: 非交互环境仅演练
- **WHEN** 用户在非交互环境执行 `llman sdd export --style openspec`
- **THEN** 命令输出 dry-run 计划并返回非零
- **AND** 不写入目标目录

#### Scenario: 交互双确认后执行
- **WHEN** 用户在交互环境执行 `llman sdd import --style openspec` 且通过双确认
- **THEN** 命令执行实际写入

#### Scenario: 默认不删除旧目录
- **WHEN** 迁移写入成功后进入“删除旧目录”提示
- **THEN** 默认选项为“不删除”
- **AND** 用户接受默认选项后源目录保持不变

### Requirement: SDD 互转元数据兼容
`export` MUST 在缺失时自动补齐 OpenSpec 元数据：
- `openspec/config.yaml`（至少包含 `schema: spec-driven`）
- active change 的 `.openspec.yaml`（包含 `schema` 与 `created`）

`import` MUST 在主 spec 缺失 llman frontmatter 时补齐最小必需键：
- `llman_spec_valid_scope`
- `llman_spec_valid_commands`
- `llman_spec_evidence`

#### Scenario: 导出自动创建 openspec config
- **WHEN** 用户执行 `llman sdd export --style openspec` 且目标缺失 `openspec/config.yaml`
- **THEN** 命令创建该文件并写入 `schema: spec-driven`

#### Scenario: 导入补齐 frontmatter
- **WHEN** 用户执行 `llman sdd import --style openspec` 且主 spec 缺失 llman frontmatter
- **THEN** 命令在写入时补齐所需 frontmatter 键

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
`llman sdd archive` MUST 支持子命令组工作流，并保持兼容入口可用：
- 兼容入口：`llman sdd archive <change-id>`（等价于 `llman sdd archive run <change-id>`）
- 标准入口：`llman sdd archive run <change-id>`

`archive run` MUST 延续当前行为：将 delta 合并到 `llmanspec/specs`，并将 change 移动到 `llmanspec/changes/archive/YYYY-MM-DD-<change-id>`；`--skip-specs`、`--dry-run`、隐藏 `--force` 行为 MUST 保持一致。

#### Scenario: 兼容入口仍可归档
- **WHEN** 用户执行 `llman sdd archive add-sample`
- **THEN** 命令成功归档并与 `llman sdd archive run add-sample` 结果一致

#### Scenario: run 子命令行为与历史一致
- **WHEN** 用户执行 `llman sdd archive run <change-id> --skip-specs`
- **THEN** 仅移动目录到 archive 且不修改主 specs

#### Scenario: force 仍保持隐藏
- **WHEN** 用户执行 `llman sdd archive --help` 或 `llman sdd archive run --help`
- **THEN** 帮助文本不显示 `--force`

### Requirement: SDD 归档 dry-run
`llman sdd archive --dry-run` MUST 输出将要修改/移动的文件与目标路径，并 MUST 不进行任何文件写入。

#### Scenario: 归档 dry-run
- **WHEN** 用户执行 `llman sdd archive <change-id> --dry-run`
- **THEN** 输出预览信息且文件系统无任何改动

### Requirement: SDD 归档冻结与解冻
`llman sdd archive freeze` MUST 将 `llmanspec/changes/archive/` 下符合 `YYYY-MM-DD-*` 的目录合并写入单一冷备文件 `llmanspec/changes/archive/freezed_changes.7z.archived`，并在成功后移除本次冻结源目录。`llman sdd archive thaw` MUST 从该冷备文件恢复归档目录到默认 `.thawed/`（或 `--dest` 指定目录），并支持 `--change` 选择性恢复。

#### Scenario: 单文件冻结
- **WHEN** 用户执行 `llman sdd archive freeze`
- **THEN** 生成或更新 `freezed_changes.7z.archived`
- **AND** 被冻结目录从 `archive/` 下移除

#### Scenario: 冻结 dry-run
- **WHEN** 用户执行 `llman sdd archive freeze --dry-run`
- **THEN** 仅输出候选与目标文件
- **AND** 不写入冷备文件且不删除源目录

#### Scenario: 冻结失败保护
- **WHEN** 冻结写入冷备文件失败
- **THEN** 命令返回非零
- **AND** 源归档目录保持不变

#### Scenario: 解冻默认目录
- **WHEN** 用户执行 `llman sdd archive thaw`
- **THEN** 内容恢复到 `llmanspec/changes/archive/.thawed/`

#### Scenario: 解冻过滤与目标目录
- **WHEN** 用户执行 `llman sdd archive thaw --change <id> --dest <path>`
- **THEN** 仅恢复所选目录到 `<path>`

### Requirement: SDD 模板版本元信息
SDD locale 模板 MUST 包含 `llman-template-version` 元信息。对于带 YAML frontmatter 的模板，frontmatter MUST 在 `metadata` 字段中包含 `llman-template-version` 键；其他模板 MUST 在第一行使用 `<!-- llman-template-version: N -->` 形式。相同相对路径的不同 locale 模板 MUST 使用相同的版本值。仓库 MUST 提供维护者检查命令以验证版本一致性与模板集合一致性。

#### Scenario: 模板版本一致性检查
- **WHEN** 维护者运行 `just check-sdd-templates`
- **THEN** 命令在缺失元信息、缺少 locale 模板或版本不一致时退出非零
- **AND** 在所有模板一致时退出零

### Requirement: SDD 归档前置校验
`llman sdd archive` MUST 在修改 specs 或移动 change 目录之前，对本次涉及的 specs 执行与 `llman sdd validate --strict --no-interactive` 等价的校验（包括 frontmatter 与 staleness）。归档校验 MUST 以重建后的 spec 内容为准，并在 staleness 判断中将本次归档涉及的 spec 视为已更新。任何 Error 或 Warn MUST 阻止归档并返回非零。

#### Scenario: 校验失败阻止归档
- **WHEN** 用户执行 `llman sdd archive <change-id>` 且任一 spec 校验失败
- **THEN** 命令退出非零，且不写入/移动任何文件

#### Scenario: staleness 警告视为失败
- **WHEN** staleness 状态为 `STALE` 或 `WARN`
- **THEN** 归档失败并提示修复

#### Scenario: 允许强制绕过
- **WHEN** 用户执行 `llman sdd archive <change-id> --force`
- **THEN** 归档继续执行即使校验失败

#### Scenario: force 参数隐藏
- **WHEN** 用户执行 `llman sdd archive --help`
- **THEN** 帮助输出不包含 `--force`

#### Scenario: skip-specs 跳过校验
- **WHEN** 用户执行 `llman sdd archive <change-id> --skip-specs`
- **THEN** 不执行归档前的 spec 校验

#### Scenario: 错误提示不引导绕过
- **WHEN** 归档因校验失败而中止
- **THEN** 输出仅提示修复校验问题，不提示 `--force`

### Requirement: SDD skills 输出符合 Agent Skills SKILL.md 规范
`llman sdd update-skills` MUST 生成符合 Agent Skills 规范的 `SKILL.md` frontmatter，至少包含 `name` 与 `description`：
- `name` MUST 与技能目录名一致，且仅包含小写字母/数字/连字符、长度 1-64、不得以连字符开头/结尾、不得包含连续连字符。
- `description` MUST 为 1-1024 字符的非空描述文本。
- `license`、`compatibility`、`metadata`、`allowed-tools` MAY 在需要时提供。

#### Scenario: name 与目录一致
- **WHEN** `llman sdd update-skills` 写入 `llman-sdd-archive/SKILL.md`
- **THEN** frontmatter `name` 为 `llman-sdd-archive`

#### Scenario: description 非空
- **WHEN** `llman sdd update-skills` 生成任意 SKILL.md
- **THEN** frontmatter `description` 为非空字符串

### Requirement: SDD skills 不暴露绕过参数
`llman sdd update-skills` 生成的 skills 内容 MUST 不包含 `--force` 绕过提示或示例。

#### Scenario: skills 不包含 --force
- **WHEN** 维护者运行 `llman sdd update-skills`
- **THEN** 生成的 SKILL.md 不提及 `--force`

### Requirement: SDD Future 记录文件为可选
`llmanspec/changes/<change-id>/future.md` MUST 作为可选记录文件存在，用于承载延期项与分叉路线。缺失该文件 MUST NOT 阻塞 `llman sdd validate` 或 `llman sdd archive`。

与 future 相关的 `skills/*.md` 与 `spec-driven/{new,continue,ff,explore}.md` 模板 MUST 提供“future 到执行”的注入式引导：将 future 条目分类为 `now|later|drop`，并为 `now` 条目映射后续 change 与首个可执行动作（如 `/llman-sdd:new`、`/llman-sdd:continue`、`/llman-sdd:ff`、`llman-sdd-apply`）。

#### Scenario: 缺失 future 不阻塞
- **WHEN** change 不包含 `future.md`
- **THEN** `llman sdd validate <change-id> --type change` 仍可通过

#### Scenario: future 条目可转执行
- **WHEN** 生成后的 skill/template 处理 `future.md` 中的延期或分叉条目
- **THEN** 输出包含 `now|later|drop` 分类与后续落地动作建议

### Requirement: SDD 模板结构化提示协议
`templates/sdd/{locale}/skills/*.md` 与 `templates/sdd/{locale}/spec-driven/*.md` MUST 提供统一结构化章节（或等价命名）：`Context`、`Goal`、`Constraints`、`Workflow`、`Decision Policy`、`Output Contract`。模板 MUST 为自包含规则，不得要求先调用外部技能作为前置步骤。

#### Scenario: skills 模板包含结构化章节
- **WHEN** 维护者运行 `llman sdd update-skills`
- **THEN** 生成技能中可见结构化章节

#### Scenario: 模板无外部技能硬依赖
- **WHEN** 检查生成的 `SKILL.md`
- **THEN** 不存在“先调用外部技能再执行”的硬依赖描述

### Requirement: SDD 重构保持行为一致
SDD 模块重构 MUST 保持所有 `llman sdd` 子命令的行为、输出与退出码一致，并且不得改变模板内容与配置生成路径。

#### Scenario: SDD 重构后回归
- **WHEN** `src/sdd/` 的模块结构被重组
- **THEN** `sdd init/update/update-skills/list/show/validate/archive` 的行为与输出保持不变

### Requirement: change/spec ID 必须作为标识符处理而不是路径
所有接受 `change-id` 或 `spec-id` 的 `llman sdd` 子命令 MUST 将其视为标识符。实现 MUST 拒绝包含路径分隔符或穿越片段的值（例如：`/`、`\\`、`..`），并 MUST NOT 因此在 `llmanspec/` 之外执行任何文件系统操作。

#### Scenario: 拒绝路径穿越 ID
- **WHEN** 用户运行 `llman sdd archive ../oops`
- **THEN** 命令返回错误，且不会移动或修改任何文件

### Requirement: list 的冲突 flag 必须显式报错
`llman sdd list` MUST 将 `--specs` 与 `--changes` 视为互斥参数。若同时提供，两者冲突 MUST 返回非零错误并说明冲突原因。

#### Scenario: 同时传入 --specs 与 --changes
- **WHEN** 用户运行 `llman sdd list --specs --changes`
- **THEN** 命令返回错误并以非零退出

### Requirement: update-skills multi-tool 下的 --path 不得造成覆盖
当一次 `llman sdd update-skills` 生成多个 tool 的 skills 时，若仅提供单个 `--path` 覆盖路径，而实现无法保证不同 tool 的输出互不覆盖，则命令 MUST 以非零错误拒绝执行并给出安全用法提示。

#### Scenario: Multi-tool + --path 被拒绝
- **WHEN** 用户运行 `llman sdd update-skills --no-interactive --all --path ./skills-out`
- **THEN** 命令以非零退出并解释如何安全地按 tool 生成（避免覆盖）

### Requirement: SDD Workflow Command Bindings
SDD MUST 提供 llman sdd workflow commands 的工具适配文件，并由 `llman sdd update-skills` 负责生成/刷新。命令绑定内容 MUST 引导用户进入 llman sdd 的工作流（`llmanspec/`）并与 skills 的动作集合保持一致。命令绑定 MUST 仅包含 workflow 命令集合，不得生成 legacy commands（例如旧式 `/openspec:*` 体系）。

当前版本中，workflow command bindings MUST 仅为 Claude Code 生成。实现 MUST NOT 为 Codex 生成 `.codex/prompts/llman-sdd-*.md` 绑定文件。

#### Scenario: 仅生成 workflow commands
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool claude --commands-only`
- **THEN** `.claude/commands/llman-sdd/` 下仅存在命令文件（`new/continue/ff/apply/verify/sync/archive/explore/onboard`）

#### Scenario: 命令绑定指向 llman sdd 工作流
- **WHEN** 用户调用任一 `/llman-sdd:<command>` 触发对应命令绑定
- **THEN** 命令绑定文本引导其在 `llmanspec/` 下执行对应动作（创建 artifacts / 实施 tasks / 归档等），并引用 `llman sdd` 命令用于验证闭环

#### Scenario: Codex 不生成 command prompts
- **WHEN** 用户执行 `llman sdd update-skills --no-interactive --tool codex`
- **THEN** 命令仅生成/刷新 Codex skills，且 MUST NOT 在 `.codex/prompts/` 下生成 `llman-sdd-*.md`

### Requirement: SDD Archive Skill Supports Batch
`llman-sdd-archive` skill MUST 同时覆盖单个与批量归档协议：列出活动 changes、让用户确认一个或多个 change IDs、按顺序执行归档，并在完成后运行一次全量校验。批量归档 MUST 默认遵循与单个归档一致的护栏（需要确认目标、失败时停止并报告）。

#### Scenario: 批量归档多个变更
- **WHEN** 用户调用 `llman-sdd-archive` 并提供多个 change IDs
- **THEN** skill 指导依次运行 `llman sdd archive run <id>`（或兼容入口 `llman sdd archive <id>`），并在结束后运行 `llman sdd validate --strict --no-interactive`

### Requirement: SDD Explore 模式 Skill
`llman-sdd-explore` skill MUST 提供探索模式指导，允许 AI 助手在问题分析、设计思考阶段提供帮助。探索模式 MUST 明确禁止直接实现代码，仅允许阅读代码、创建 artifacts、提出问题。skill 内容 MUST 包含：stance 定义、可执行的操作列表、与 llmanspec 的交互方式、结束探索的引导。

#### Scenario: 探索模式进入
- **WHEN** 用户调用 `llman-sdd-explore`（或通过 `/llman-sdd:explore` 进入）
- **THEN** AI 助手进入探索模式，可阅读代码和创建 artifacts，但不实现功能

#### Scenario: 探索模式退出
- **WHEN** 用户在探索模式中准备开始实现
- **THEN** skill 引导用户使用 `llman-sdd-new-change`、`llman-sdd-ff` 或 `/llman-sdd:new` 开始正式工作流

### Requirement: SDD Continue Skill
`llman-sdd-continue` skill MUST 指导 AI 助手继续未完成的变更，创建下一个待完成的 artifact。skill MUST 检查当前变更状态，识别已完成和待创建的 artifacts，按依赖顺序创建下一个 artifact。若所有 artifacts 已完成，skill MUST 引导用户进入 apply 阶段或 archive。

#### Scenario: 继续创建 artifact
- **WHEN** 用户调用 `llman-sdd-continue` 且变更存在未完成的 artifacts
- **THEN** skill 指导创建下一个按依赖顺序应完成的 artifact

#### Scenario: artifacts 全部完成
- **WHEN** 用户调用 `llman-sdd-continue` 且所有 artifacts 已完成
- **THEN** skill 提示可使用 `llman-sdd-apply` 或 `llman-sdd-archive`

### Requirement: SDD Apply Skill
`llman-sdd-apply` skill MUST 指导 AI 助手实施 tasks.md 中的任务。skill MUST 读取变更的上下文文件（proposal、specs、design、tasks），按顺序实施未完成的任务，每完成一个任务后更新 tasks.md 中的 checkbox 状态。实施过程中遇到问题时 skill MUST 暂停并请求用户指导。

#### Scenario: 实施任务
- **WHEN** 用户调用 `llman-sdd-apply` 且存在未完成的任务
- **THEN** skill 指导按顺序实施任务并更新 checkbox 状态

#### Scenario: 任务全部完成
- **WHEN** 用户调用 `llman-sdd-apply` 且所有任务已完成
- **THEN** skill 提示可使用 `llman-sdd-archive`

#### Scenario: 实施遇阻
- **WHEN** 实施任务过程中遇到不明确的需求或技术问题
- **THEN** skill 指导暂停并请求用户指导

### Requirement: SDD Fast-Forward Skill
`llman-sdd-ff` skill MUST 指导 AI 助手快速创建变更的所有 artifacts（proposal → specs → design → tasks），无需逐步确认。skill MUST 在创建前询问用户对变更的描述，然后依次创建所有 artifacts，最后显示完成状态和后续可用操作。

#### Scenario: 快速创建变更
- **WHEN** 用户调用 `llman-sdd-ff <change-name>`
- **THEN** skill 询问变更描述后依次创建 proposal、specs、design、tasks

#### Scenario: 变更已存在
- **WHEN** 用户调用 `llman-sdd-ff <change-name>` 但该变更已存在
- **THEN** skill 提示使用 `llman-sdd-continue` 继续

### Requirement: SDD Verify Skill
`llman-sdd-verify` skill MUST 指导 AI 助手验证实现与变更 artifacts 的一致性。skill MUST 读取 specs 和 design，检查代码实现是否符合规范，识别不一致之处并提供修复建议。验证通过后 skill MUST 引导用户进行 archive。

#### Scenario: 验证实现
- **WHEN** 用户调用 `llman-sdd-verify`
- **THEN** skill 指导检查实现与 specs/design 的一致性

#### Scenario: 验证发现问题
- **WHEN** 验证发现实现与 artifacts 不一致
- **THEN** skill 列出不一致之处并提供修复建议

#### Scenario: 验证通过
- **WHEN** 验证确认实现与 artifacts 一致
- **THEN** skill 提示可使用 `llman-sdd-archive`

### Requirement: SDD Sync Skill
`llman-sdd-sync` skill MUST 指导 AI 助手以可复现的人工作业协议将变更中的 delta specs 同步到主 specs，而不归档变更目录。skill MUST 指导用户检查 delta、手动应用 ADDED/MODIFIED/REMOVED/RENAMED 到对应主 specs，并在完成后运行验证命令。

#### Scenario: 同步 delta specs
- **WHEN** 用户调用 `llman-sdd-sync <change-name>`
- **THEN** skill 提供可复现步骤，指导手动将 delta specs 同步到主 specs 目录

#### Scenario: 同步后验证
- **WHEN** delta specs 已合并到主 specs
- **THEN** skill 运行 `llman sdd validate --specs` 验证合并结果

### Requirement: Style Routing for SDD Commands
`llman sdd` command flows MUST support explicit style selection for new vs legacy tracks.

#### Scenario: Update commands accept style selection
- **WHEN** a user runs `llman sdd update` or `llman sdd update-skills` with style selector
- **THEN** the command routes template loading and output generation through the selected style track

### Requirement: Default Style Is New
The default SDD style MUST be new when style selector is omitted.

#### Scenario: Show and validate default to new style
- **WHEN** a user runs `llman sdd show` or `llman sdd validate` without style selector
- **THEN** the command evaluates and displays new style outputs by default

### Requirement: Archive Merge Emits ISON Spec Payload
`llman sdd archive` MUST merge delta changes into main specs using structured ISON semantics and emit ISON payload output.

#### Scenario: Archive applies ops and writes merged ISON
- **WHEN** a change contains delta spec ops and a user runs `llman sdd archive <change>`
- **THEN** archive applies add/modify/remove/rename operations over requirement ids
- **AND** the resulting `llmanspec/specs/<capability>/spec.md` contains merged ISON payload as canonical spec content

### Requirement: Legacy Workflow Command Preserves JSON Parsing
The CLI MUST provide a legacy workflow command to preserve the previous JSON-in-` ```ison ` parsing behavior for `llmanspec` specs and delta specs.

This legacy command MUST:
- be exposed as `llman sdd-legacy ...`
- preserve the previous parsing/validation semantics for legacy repositories
- continue to support existing repositories whose ` ```ison ` payload is JSON (including any JSON repair behavior already present)
- include the workflow subcommands referenced by `templates/sdd-legacy/**` (at minimum: `init`, `update`, `update-skills`, `list`, `show`, `validate`, and `archive` including archive subcommands like freeze/thaw as applicable)

`llman sdd` (the new default) MUST be allowed to evolve without needing to accept legacy JSON payloads.

#### Scenario: Legacy repo remains usable via sdd-legacy
- **WHEN** a repository contains JSON-in-` ```ison ` payloads under `llmanspec/specs/**` or `llmanspec/changes/**/specs/**`
- **THEN** `llman sdd-legacy show/validate/archive` continues to operate successfully
- **AND** `llman sdd show/validate/archive` fails fast with a legacy-command hint and canonical rewrite guidance

### Requirement: SDD Provides ISON Authoring Commands
`llman sdd` MUST provide an explicit command group for ISON spec authoring/editing to reduce manual edits of `llmanspec/**/spec.md`.

At minimum, the CLI MUST support:
- generating a main spec skeleton for a capability
- adding a requirement to a main spec
- adding a scenario to a main spec (keyed by `req_id` + `scenario.id`)
- generating a delta spec skeleton for a change + capability
- adding/removing/updating delta ops (add/modify/remove/rename requirement)
- adding scenarios for add/modify ops (keyed by `req_id` + `scenario.id`)

#### Scenario: Generate delta skeleton and add an op
- **WHEN** a maintainer creates a new change directory and needs to add a delta requirement
- **THEN** the maintainer can use the CLI to generate a delta spec skeleton and add an op without manual table editing

### Requirement: CLI Provides Lightweight Spec Metadata for Agents
The CLI MUST provide an agent-friendly way to fetch a spec’s feature name/purpose without retrieving full requirement bodies.

At minimum, `llman sdd show` MUST support a JSON metadata-only mode for specs:
- command shape: `llman sdd show <spec-id> --type spec --json --meta-only`
- output MUST include:
  - `id` (spec id / directory name)
  - `featureId` (from `object.spec.name`)
  - `title` (human-facing name; defaults to `featureId`)
  - `overview` (from `object.spec.purpose`)
  - `requirementCount`
  - `metadata`
- output MUST NOT include `requirements` when `--meta-only` is set.

If the CLI provides a `--compact-json` mode, it MUST emit JSON without pretty formatting whitespace (token-friendly) while keeping field order stable.
This MUST apply to `llman sdd list/show/validate` (and `llman sdd-legacy list/show/validate`) whenever `--json` is used.

#### Scenario: Agent fetches spec feature name cheaply
- **WHEN** an agent needs the spec feature name/purpose for prompt assembly
- **THEN** the agent can call `llman sdd show <spec-id> --type spec --json --meta-only`
- **AND** the output includes only spec metadata (no `requirements` array)

### Requirement: Validation Fails on JSON-in-ISON Payloads
`llman sdd validate` MUST fail when any `llmanspec` main spec or delta spec contains a fenced ` ```ison ` payload that is JSON.

The error MUST be actionable and MUST include:
- a concrete hint to temporarily use the legacy command (`llman sdd-legacy ...`).

#### Scenario: Validate blocks legacy payloads
- **WHEN** a user runs `llman sdd validate` (with or without `--strict`) on a project containing JSON-in-` ```ison ` payloads
- **THEN** validation fails with non-zero exit
- **AND** output includes an explicit legacy-command hint

### Requirement: No Automatic Migration Is Required for Legacy Repos
The system MUST support a “two command” posture:
- `llman sdd` enforces canonical table/object ISON.
- `llman sdd-legacy` preserves the legacy JSON parsing behavior.

The system MUST NOT require an automatic migration command as part of the new-style workflow. Users MAY manually rewrite legacy payloads into the canonical table/object ISON schema when they choose to switch from `llman sdd-legacy` to `llman sdd`.

#### Scenario: User chooses when to switch formats
- **WHEN** a repository remains on legacy JSON payloads
- **THEN** users can continue using `llman sdd-legacy`
- **AND** switching to `llman sdd` requires rewriting payloads into canonical table/object ISON, but no automatic migration command is required


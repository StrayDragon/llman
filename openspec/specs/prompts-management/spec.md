# prompts-management Specification

## Purpose
TBD - created by archiving change add-prompts-injection. Update Purpose after archive.
## Requirements
### Requirement: `prompts` 为主命令并保留 `prompt` 别名
CLI MUST 将 `llman prompts` 作为主命令名，并且 MUST 接受 `llman prompt` 作为等价别名。

#### Scenario: 使用别名调用
- **WHEN** 用户运行 `llman prompt list`
- **THEN** 行为与 `llman prompts list` 等价

### Requirement: 支持 `cursor` / `codex` / `claude-code` 三类 app
`llman prompts` MUST 支持 `--app cursor|codex|claude-code`，并在生成/列出/增删改查时按 app 维度隔离模板。

#### Scenario: app 维度隔离模板
- **WHEN** 用户分别对 `cursor` 与 `codex` 执行 `upsert` 并使用相同 `--name`
- **THEN** 两者模板互不覆盖，且 `list --app cursor` 不显示 `codex` 模板

### Requirement: Codex prompts 同时支持 user/project 两种 scope
当 `--app codex` 且执行生成注入时，命令 MUST 支持将所选模板写入以下两类目录（由 `--scope` 控制）：
- user scope：`~/.codex/prompts/<name>.md`
- project scope：`<repo_root>/.codex/prompts/<name>.md`

命令 MUST 仅写入顶层文件（不创建子目录），且 MUST 使用 Markdown 扩展名 `.md`。

#### Scenario: 生成 Codex prompt 文件
- **WHEN** 用户运行 `llman prompts gen --app codex --scope user --template draftpr`
- **THEN** `~/.codex/prompts/draftpr.md` 被创建或更新

#### Scenario: 生成 Codex prompt 到项目目录
- **WHEN** 用户运行 `llman prompts gen --app codex --scope project --template draftpr`
- **THEN** `<repo_root>/.codex/prompts/draftpr.md` 被创建或更新

### Requirement: Claude Code prompts 同时支持 user/project 两种 scope
当 `--app claude-code` 且执行生成注入时，命令 MUST 支持将模板内容注入到 Claude Code memory file（由 `--scope` 控制）：
- user scope：`~/.claude/CLAUDE.md`
- project scope：`<repo_root>/CLAUDE.md`

命令 MUST 使用托管块策略，仅更新 llman 管理的区段，并保留文件中非托管内容。

#### Scenario: 生成并保留用户自定义内容
- **WHEN** 项目 `CLAUDE.md` 已包含用户手写内容，且用户运行 `llman prompts gen --app claude-code --template project-rules`
- **THEN** 命令仅更新托管块内容，不删除或改写用户手写内容

### Requirement: 通过 `--scope` 选择注入目标范围
当 `--app` 为 `codex` 或 `claude-code` 且执行生成注入时，命令 MUST 支持 `--scope user|project|all`：
- 默认值 MUST 为 `project`
- `all` MUST 同时应用 user 与 project scope

#### Scenario: scope=all 同时写入用户与项目
- **WHEN** 用户运行 `llman prompts gen --app codex --scope all --template draftpr`
- **THEN** `~/.codex/prompts/draftpr.md` 与 `<repo_root>/.codex/prompts/draftpr.md` 均被创建或更新

### Requirement: 冲突与覆盖策略一致
当目标文件存在时，命令 MUST 默认在交互环境中提示确认是否覆盖；在非交互环境 MUST 拒绝覆盖并返回错误，除非用户显式启用 `--force`（或等价覆盖策略）。

#### Scenario: 非交互默认拒绝覆盖
- **WHEN** 目标文件已存在且终端不可交互
- **THEN** 命令返回错误并提示需要显式覆盖策略

### Requirement: 模板列举与读取在扩展名维度一致
对每个 `--app`，`llman prompts list` MUST 只展示 `llman prompts gen` 能够以确定方式读取的模板。命令 MUST 忽略该 app 模板目录中不属于该 app 支持扩展名集合的文件（例如备份文件、不同扩展名的同名文件）。

#### Scenario: 模板目录包含混合扩展名
- **WHEN** 模板目录同时包含支持的模板文件与不相关文件（例如不同扩展名的备份）
- **THEN** `llman prompts list --app <app>` 仅展示可读取模板
- **AND** 对任意被展示的模板执行 `gen` 都能成功读取（不出现 “rule not found”）

### Requirement: 非交互删除必须显式确认
`llman prompts rm` MUST 支持 `--yes` 用于跳过确认提示。若处于非交互环境，命令 MUST 拒绝删除并返回错误，除非用户显式提供 `--yes`。

#### Scenario: 非交互 rm 未提供 --yes
- **WHEN** 终端不可交互且用户运行 `llman prompts rm --app cursor --name foo`
- **THEN** 命令返回非零错误并提示需要传 `--yes`

#### Scenario: 非交互 rm 提供 --yes
- **WHEN** 终端不可交互且用户运行 `llman prompts rm --app cursor --name foo --yes`
- **THEN** 模板被删除且不会出现交互提示

### Requirement: Claude memory 注入在读取失败时不得静默覆盖
当向 `CLAUDE.md` 注入托管块时，如果目标文件已存在但无法作为 UTF-8 文本读取，命令 MUST 直接失败并 MUST NOT 对该路径进行写入。

#### Scenario: 既有 CLAUDE.md 不可读
- **WHEN** `<repo_root>/CLAUDE.md` 存在但无法作为 UTF-8 读取
- **THEN** `llman prompts gen --app claude-code --scope project --template <name>` 返回错误且文件未被修改

### Requirement: Project scope 必须通过 repo root 发现解析
对于 project-scope 目标路径，命令 MUST 通过向上查找父目录定位 git repo root，并 MUST 将 repo 内任意子目录视为有效 project 上下文。

#### Scenario: 在 repo 子目录中运行
- **WHEN** 用户在 repo 子目录中运行 `llman prompts gen --app codex --scope project --template <name>`
- **THEN** 输出被写入 `<repo_root>/.codex/prompts/`

### Requirement: Project scope 在无 git root 时必须显式 force
当用户请求 project-scope 但命令无法发现 git repo root 时，实现 MUST 默认拒绝写入并返回错误，除非用户显式确认/显式提供 `--force` 以允许将当前工作目录视为 project root。

#### Scenario: 非交互无 git root 且未提供 --force
- **WHEN** 终端不可交互、当前目录不在 git repo 内且用户运行 `llman prompts gen --scope project --app codex --template <name>`
- **THEN** 命令返回非零错误并提示需要 `--force`，且不会写入任何文件

#### Scenario: 非交互无 git root 但提供 --force
- **WHEN** 终端不可交互、当前目录不在 git repo 内且用户运行 `llman prompts gen --scope project --force --app codex --template <name>`
- **THEN** 命令将 `cwd` 视为 project root 并写入到 `<cwd>/.codex/prompts/`

#### Scenario: 交互无 git root 且用户拒绝强制执行
- **WHEN** 终端可交互、当前目录不在 git repo 内且用户运行 `llman prompts gen --scope project --app codex --template <name>` 并在提示中选择不强制执行
- **THEN** 命令安全退出且不会写入任何文件

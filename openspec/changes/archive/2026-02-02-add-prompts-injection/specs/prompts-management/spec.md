# prompts-management Specification (Change: add-prompts-injection)

## ADDED Requirements
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

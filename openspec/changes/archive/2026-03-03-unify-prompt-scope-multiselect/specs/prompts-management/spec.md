# prompts-management Specification (Change: unify-prompt-scope-multiselect)

## MODIFIED Requirements

### Requirement: Codex prompts 同时支持 global/project 两种 scope
当 `--app codex` 且执行生成注入时，命令 MUST 支持两类注入目标（由 `--target` 控制），并在 `global|project` 两层分别生效：

- `--target prompts`：写入 Codex custom prompts 文件
  - global scope：`$CODEX_HOME/prompts/<name>.md`（未设置 `CODEX_HOME` 时为 `~/.codex/prompts/<name>.md`）
  - project scope：`<repo_root>/.codex/prompts/<name>.md`
- `--target agents`：注入到 Codex project doc 文件（托管块聚合；输出文件名固定，不随模板名变化）
  - global scope：`$CODEX_HOME/AGENTS.md`（未设置 `CODEX_HOME` 时为 `~/.codex/AGENTS.md`）
  - project scope：`<repo_root>/AGENTS.md`

当 `--target agents` 且用户启用 `--override` 时，命令 MUST 改为写入：
- global scope：`$CODEX_HOME/AGENTS.override.md`
- project scope：`<repo_root>/AGENTS.override.md`

命令 MUST 使用托管块策略注入 `agents` 目标，仅更新 llman 管理区段并保留非托管内容。

#### Scenario: 生成 Codex 全局 custom prompt 文件
- **WHEN** 用户运行 `llman prompts gen --app codex --target prompts --scope global --template draftpr`
- **THEN** `$CODEX_HOME/prompts/draftpr.md` 被创建或更新

#### Scenario: 生成 Codex 项目 custom prompt 文件
- **WHEN** 用户运行 `llman prompts gen --app codex --target prompts --scope project --template draftpr`
- **THEN** `<repo_root>/.codex/prompts/draftpr.md` 被创建或更新

#### Scenario: 生成 Codex 全局 AGENTS 文档
- **WHEN** 用户运行 `llman prompts gen --app codex --target agents --scope global --template common.en`
- **THEN** `$CODEX_HOME/AGENTS.md` 被创建或更新

#### Scenario: 生成 Codex override AGENTS 文档
- **WHEN** 用户运行 `llman prompts gen --app codex --target agents --scope global --override --template common.en`
- **THEN** `$CODEX_HOME/AGENTS.override.md` 被创建或更新

### Requirement: Claude Code prompts 同时支持 global/project 两种 scope
当 `--app claude-code` 且执行生成注入时，命令 MUST 支持以下 scope 目标：
- global scope：`~/.claude/CLAUDE.md`
- project scope：`<repo_root>/CLAUDE.md`

命令 MUST 使用托管块策略，仅更新 llman 管理的区段，并保留文件中非托管内容。

#### Scenario: 生成 Claude Code 全局 memory 文档
- **WHEN** 用户运行 `llman prompts gen --app claude-code --scope global --template project-rules`
- **THEN** `~/.claude/CLAUDE.md` 被创建或更新

#### Scenario: 生成并保留用户自定义内容
- **WHEN** 项目 `CLAUDE.md` 已包含用户手写内容，且用户运行 `llman prompts gen --app claude-code --scope project --template project-rules`
- **THEN** 命令仅更新托管块内容，不删除或改写用户手写内容

### Requirement: 通过 `--scope` 选择注入目标范围
当执行 `llman prompts gen` 时，命令 MUST 将 `--scope` 解析为“目标集合”，并支持以下输入形式：
- 重复参数：`--scope global --scope project`
- 逗号列表：`--scope global,project`

scope 关键字 MUST 为 `global|project`，并按 app 的支持范围进行校验：
- `codex`：支持 `global` 与 `project`
- `claude-code`：支持 `global` 与 `project`
- `cursor`：仅支持 `project`

命令 MUST 不再接受 `user` 与 `all`。
若用户未提供 `--scope`，默认 MUST 为 `project`。

#### Scenario: 重复参数选择双 scope
- **WHEN** 用户运行 `llman prompts gen --app codex --scope global --scope project --template draftpr`
- **THEN** 命令同时处理全局与项目目标

#### Scenario: 逗号列表选择双 scope
- **WHEN** 用户运行 `llman prompts gen --app claude-code --scope global,project --template project-rules`
- **THEN** 命令同时处理全局与项目目标

#### Scenario: Cursor 传入不支持 scope
- **WHEN** 用户运行 `llman prompts gen --app cursor --scope global --template demo-project`
- **THEN** 命令返回错误并提示 cursor 仅支持 `project`

#### Scenario: 传入已移除的旧 scope
- **WHEN** 用户运行 `llman prompts gen --app codex --scope user --template draftpr`
- **THEN** 命令返回错误
- **AND** 错误输出不包含迁移建议

### Requirement: 冲突与覆盖策略一致
当目标文件存在时，命令 MUST 使用一致的冲突与覆盖策略：
- 对完全托管的目标（例如 codex prompts、cursor rules），命令 MUST 在交互环境中提示确认是否覆盖；在非交互环境 MUST 拒绝覆盖并返回错误，除非用户显式启用 `--force`。
- 对托管块注入目标（例如 codex agents、claude-code memory），若目标文件存在且不包含 llman 托管块标记：
  - 交互模式 MUST 执行二次确认，任一确认未通过都 MUST 放弃该目标写入。
  - 非交互模式 MUST 拒绝该目标写入，除非启用 `--force`。

#### Scenario: 交互模式下 codex prompts 覆盖确认
- **WHEN** 目标 custom prompt 文件已存在且终端可交互
- **THEN** 命令提示确认是否覆盖

#### Scenario: 交互模式下非托管文件触发二次确认
- **WHEN** 目标文件存在且不包含 llman 托管块，用户在交互模式运行 `llman prompts gen --app codex --target agents`
- **THEN** 命令执行二次确认
- **AND** 仅当两次确认都通过时才写入

#### Scenario: 非交互模式下非托管文件未提供 force
- **WHEN** 目标文件存在且不包含 llman 托管块，终端不可交互且命令未提供 `--force`
- **THEN** 命令拒绝该目标写入并返回错误

### Requirement: Project scope 必须通过 repo root 发现解析
对于选择了 project scope 的目标路径，命令 MUST 通过向上查找父目录定位 git repo root，并 MUST 将 repo 内任意子目录视为有效 project 上下文。

project scope 的写入目标 MUST 为：
- codex + `--target prompts`：`<repo_root>/.codex/prompts/<name>.md`
- codex + `--target agents`：`<repo_root>/AGENTS.md` 或 `<repo_root>/AGENTS.override.md`
- claude-code：`<repo_root>/CLAUDE.md`
- cursor：`<repo_root>/.cursor/rules/<name>.mdc`

#### Scenario: 在 repo 子目录中运行 codex project prompts
- **WHEN** 用户在 repo 子目录中运行 `llman prompts gen --app codex --target prompts --scope project --template draftpr`
- **THEN** 输出被写入 `<repo_root>/.codex/prompts/draftpr.md`

#### Scenario: 在 repo 子目录中运行 codex project agents
- **WHEN** 用户在 repo 子目录中运行 `llman prompts gen --app codex --target agents --scope project --template common.en`
- **THEN** 输出被写入 `<repo_root>/AGENTS.md`

#### Scenario: 在 repo 子目录中运行 cursor project scope
- **WHEN** 用户在 repo 子目录中运行 `llman prompts gen --app cursor --scope project --template demo-project`
- **THEN** 输出被写入 `<repo_root>/.cursor/rules/`

### Requirement: Project scope 在无 git root 时必须显式 force
当用户请求 project scope 但命令无法发现 git repo root 时，实现 MUST 默认拒绝该 project 目标并返回错误，除非用户显式确认/显式提供 `--force` 以允许将当前工作目录视为 project root。

对于同时选择了多个 scope 的执行，命令 MUST 逐目标处理，且 MUST NOT 因 project 目标失败而跳过其他 scope 目标的尝试。

#### Scenario: 非交互无 git root 且仅 project scope
- **WHEN** 终端不可交互、当前目录不在 git repo 内且用户运行 `llman prompts gen --scope project --app codex --template draftpr`
- **THEN** 命令返回非零错误并提示需要 `--force`，且不会写入 project 目标

#### Scenario: 非交互无 git root 且选择 global+project
- **WHEN** 终端不可交互、当前目录不在 git repo 内且用户运行 `llman prompts gen --scope global --scope project --app codex --template draftpr`
- **THEN** 命令会尝试写入 global 目标
- **AND** 命令不会因为 project 目标失败而跳过 global 目标

## ADDED Requirements

### Requirement: Codex 支持选择注入目标类型
当 `--app codex` 且执行生成注入时，命令 MUST 支持 `--target agents|prompts` 用于选择注入目标类型。

若用户未提供 `--target`，默认 MUST 为 `prompts`。

当 `--override` 被提供但 `--target` 不包含 `agents` 时，命令 MUST 返回错误。

#### Scenario: 默认 target 为 prompts
- **WHEN** 用户运行 `llman prompts gen --app codex --template draftpr`
- **THEN** 命令按 `--target prompts` 处理

#### Scenario: override 仅对 agents 生效
- **WHEN** 用户运行 `llman prompts gen --app codex --target prompts --override --template draftpr`
- **THEN** 命令返回错误

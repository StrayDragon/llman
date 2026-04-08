# unified-ignore-sync — Delta Specification（add-sync-ignore-tool）

## ADDED Requirements

### Requirement: 统一解析与 union 同步
系统 MUST 提供 `llman tool sync-ignore` 命令，用于在项目内对以下 ignore 配置进行统一解析，并以 union（并集）方式同步到选定 targets：

- OpenCode：`.ignore`
- Cursor：`.cursorignore`
- Claude Code：`.claude/settings.json`、`.claude/settings.local.json`（仅处理 `permissions.deny` 的 `Read(...)`）

#### Scenario: 默认 dry-run 预览（不写入）
- **GIVEN** 当前目录位于一个 git repo 内
- **WHEN** 用户执行 `llman tool sync-ignore`
- **THEN** 系统自动发现项目内存在的 sources（`.ignore` / `.cursorignore` / `.claude/settings*.json`）
- **AND** 将 sources 解析为统一结构 `{ignore, include}`
- **AND** 对 `ignore` 与 `include` 分别取 union 并去重
- **AND** 输出预览（包括 targets 的 create/update/unchanged 计划）
- **AND** 不修改任何文件

#### Scenario: `--yes` 应用写入并自动创建缺失 targets
- **GIVEN** 当前目录位于一个 git repo 内
- **WHEN** 用户执行 `llman tool sync-ignore --yes`
- **THEN** 系统把 union 结果写入/创建默认 targets：`.ignore`、`.cursorignore`、`.claude/settings.json`
- **AND** 系统自动创建必要父目录（例如 `.claude/`）
- **AND** 若 `.claude/settings.local.json` 已存在，则同步更新；若不存在则默认不创建

#### Scenario: `--target` 限制输出目标
- **WHEN** 用户执行 `llman tool sync-ignore --target cursor`
- **THEN** 系统仅写入/创建 `.cursorignore`
- **AND** 不修改其他目标文件

### Requirement: `.ignore` / `.cursorignore` 的 include（`!pattern`）解析
系统 MUST 支持解析 gitignore 风格的 include 规则，并写回到 gitignore-like 文件中。

#### Scenario: `.ignore` 中的 include
- **GIVEN** `.ignore` 内容包含 `!dist/`
- **WHEN** 系统解析 `.ignore`
- **THEN** 必须把 `dist/` 记录为 `include` 规则（而不是 ignore）

#### Scenario: 稳定输出顺序
- **WHEN** 系统写回 `.ignore` 或 `.cursorignore`
- **THEN** 必须以稳定顺序输出：先输出所有 ignore，再输出所有 include（以 `!` 前缀）

### Requirement: Claude Code settings 的读写与保留策略
系统 MUST 能解析并更新项目内 Claude Code settings，并尽量保留 JSONC 注释（best-effort）。

#### Scenario: 仅转换 `permissions.deny` 的 `Read(...)`
- **GIVEN** `.claude/settings.json` 的 `permissions.deny` 中包含 `Read(./secrets/**)`
- **WHEN** 系统解析 Claude Code settings
- **THEN** 必须提取 `secrets/**` 作为 ignore 规则

#### Scenario: include 规则无法写入 Claude Code 时告警
- **GIVEN** union 结果包含至少一条 include（例如 `!dist/`）
- **WHEN** 系统写入 `.claude/settings.json`
- **THEN** 必须跳过 include 规则
- **AND** 必须输出警告，说明 include 无法映射到 Claude Code（deny-only）

#### Scenario: 保留非 Read deny 规则
- **GIVEN** `.claude/settings.json` 的 `permissions.deny` 中包含非 `Read(...)` 项（例如 `WebFetch(...)`）
- **WHEN** 系统写入 Claude Code settings
- **THEN** 必须保留这些非 Read 项（不得删除）

### Requirement: git repo 强制检查 + `--force`
系统 MUST 强制检查 git root，避免在非项目目录误写入文件。

#### Scenario: 非 git 目录报错
- **GIVEN** 当前目录向上遍历找不到 `.git`
- **WHEN** 用户执行 `llman tool sync-ignore`
- **THEN** 系统必须报错并返回非零退出码
- **AND** 提示用户使用 `--force`

#### Scenario: `--force` 允许在非 git 目录运行
- **GIVEN** 当前目录向上遍历找不到 `.git`
- **WHEN** 用户执行 `llman tool sync-ignore --force`
- **THEN** 系统将当前目录视为 root 并继续执行（仍默认 dry-run）

### Requirement: 交互式模式（inquire）
系统 MUST 提供交互式模式用于选择 targets、预览与确认执行。

#### Scenario: MultiSelect 选择 targets + 反选删除提示
- **WHEN** 用户执行 `llman tool sync-ignore --interactive`
- **THEN** 系统显示 MultiSelect 列表（包含 `.ignore` / `.cursorignore` / `.claude/settings.json` / `.claude/settings.local.json`，并标注 exists/missing）
- **AND** 若用户反选某个已存在文件，系统必须询问是否删除该文件（默认不删除）

#### Scenario: 交互式预览与确认
- **WHEN** 用户在交互模式下完成 targets 选择
- **THEN** 系统必须显示预览（包含每个 target 的 create/update/unchanged/delete 计划与规则数量）
- **AND** 系统必须要求用户确认
- **AND** 仅在确认后才实际写入/删除

### Requirement: x 子命令快捷方式（可选但推荐）
系统 SHOULD 通过 `llman x` 子命令提供便捷的快捷方式；若提供该快捷方式，则系统 MUST 使用目标 app 的合理默认 target（例如 cc→claude-shared，cursor→cursor）。

#### Scenario: 通过 cc 子命令同步到 Claude Code
- **WHEN** 用户执行 `llman x cc sync-ignore`
- **THEN** 系统默认将 targets 限制为 `claude-shared`（`.claude/settings.json`）

#### Scenario: 通过 cursor 子命令同步到 Cursor
- **WHEN** 用户执行 `llman x cursor sync-ignore`
- **THEN** 系统默认将 targets 限制为 `cursor`（`.cursorignore`）

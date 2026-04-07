## ADDED Requirements

### Requirement: 命令行转换忽略规则
系统必须提供 `llman tool sync-ignore` 命令，用于在 Cursor `.cursorignore` 和 Claude Code `permissions.deny` 格式之间转换忽略规则。

#### Scenario: 从 cursorignore 转换到 Claude Code
- **WHEN** 用户执行 `llman tool sync-ignore --from cursor -i .cursorignore`
- **THEN** 系统读取 `.cursorignore` 文件
- **AND** 将每个模式转换为 `Read(./pattern)` 格式
- **AND** 输出转换后的规则到标准输出或指定文件

#### Scenario: 从 Claude Code 转换到 cursorignore
- **WHEN** 用户执行 `llman tool sync-ignore --from claude-code -i .claude/settings.json`
- **THEN** 系统解析 settings.json 中的 `permissions.deny` 数组
- **AND** 提取 `Read()` 规则并转换为 gitignore 格式
- **AND** 输出到 `.cursorignore` 或指定文件

#### Scenario: 自动检测源格式
- **WHEN** 用户执行 `llman tool sync-ignore` 而不指定 `--from`
- **THEN** 系统按优先级检查 `.cursorignore` 和 `.claude/settings.json`
- **AND** 使用找到的第一个文件作为源
- **AND** 自动检测文件格式

### Requirement: 交互式模式
系统必须提供交互式模式，通过 inquirer 引导用户完成转换过程。

#### Scenario: 选择转换方向
- **WHEN** 用户执行 `llman tool sync-ignore --interactive`
- **THEN** 系统显示选项列表：
  - Cursor → Claude Code
  - Claude Code → Cursor
  - 双向同步
- **AND** 用户选择一个方向

#### Scenario: 确认并预览
- **WHEN** 用户在交互模式下选择转换方向
- **THEN** 系统显示将进行的更改预览
- **AND** 要求用户确认
- **AND** 仅在用户确认后执行转换

### Requirement: 模式转换规则
系统必须正确转换忽略规则模式，处理常见情况和边缘情况。

#### Scenario: 基本通配符转换
- **WHEN** 转换 `*.log` 从 cursorignore 到 Claude Code
- **THEN** 输出 `Read(./*.log)`

#### Scenario: 目录通配符转换
- **WHEN** 转换 `secrets/**` 从 cursorignore 到 Claude Code
- **THEN** 输出 `Read(./secrets/**)`

#### Scenario: 反向转换 Read 规则
- **WHEN** 转换 `Read(./.env)` 从 Claude Code 到 cursorignore
- **THEN** 输出 `.env`

#### Scenario: 否定模式警告
- **WHEN** cursorignore 包含 `!public/index.html`
- **THEN** 系统跳过该模式
- **AND** 输出警告说明否定模式不受支持

#### Scenario: 非 Read 规则警告
- **WHEN** Claude Code permissions.deny 包含 `WebFetch(domain:example.com)`
- **THEN** 系统跳过该规则
- **AND** 输出警告说明仅支持 Read 规则

### Requirement: 试运行模式
系统必须提供试运行模式，预览更改而不实际修改文件。

#### Scenario: 试运行显示预览
- **WHEN** 用户执行 `llman tool sync-ignore --dry-run`
- **THEN** 系统显示将进行的所有更改
- **AND** 不修改任何文件
- **AND** 显示源文件和目标文件路径

#### Scenario: 试运行配合详细输出
- **WHEN** 用户执行 `llman tool sync-ignore --dry-run --verbose`
- **THEN** 系统显示每个模式的转换详情
- **AND** 显示所有警告和跳过的规则

### Requirement: x 子命令快捷方式
系统必须通过 `llman x` 子命令提供便捷的转换快捷方式。

#### Scenario: 通过 cc 子命令转换到 Claude Code
- **WHEN** 用户执行 `llman x cc sync-ignore`
- **THEN** 系统默认使用 cursorignore 作为源
- **AND** 默认输出到 Claude Code settings.json
- **AND** 执行转换

#### Scenario: 通过 cursor 子命令转换到 Cursor
- **WHEN** 用户执行 `llman x cursor sync-ignore`
- **THEN** 系统默认使用 Claude Code settings.json 作为源
- **AND** 默认输出到 `.cursorignore`
- **AND** 执行转换

### Requirement: 错误处理
系统必须提供清晰的错误消息和适当的退出码。

#### Scenario: 源文件不存在
- **WHEN** 指定的输入文件不存在
- **THEN** 系统输出错误消息说明文件未找到
- **AND** 返回非零退出码
- **AND** 建议检查文件路径

#### Scenario: 无法解析 JSON
- **WHEN** settings.json 包含无效的 JSON
- **THEN** 系统输出解析错误详情
- **AND** 返回非零退出码

#### Scenario: 目标目录不可写
- **WHEN** 输出路径的目录不可写
- **THEN** 系统输出权限错误
- **AND** 返回非零退出码

### Requirement: 双向同步
系统必须支持双向同步模式，合并两个方向的规则。

#### Scenario: 双向同步合并规则
- **WHEN** 用户执行 `llman tool sync-ignore --bidirectional`
- **THEN** 系统读取 `.cursorignore` 和 `.claude/settings.json`
- **AND** 转换 cursorignore → Claude Code 格式
- **AND** 转换 Claude Code → cursorignore 格式
- **AND** 合并去重后的规则
- **AND** 写回两个文件（除非使用 dry-run）

#### Scenario: 双向同步冲突处理
- **WHEN** 双向同步时存在冲突的规则
- **THEN** 系统保留两个来源的规则（并集）
- **AND** 输出警告说明存在差异

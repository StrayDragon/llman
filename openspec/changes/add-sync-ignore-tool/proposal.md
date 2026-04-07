## Why

使用多个 AI 编码助手（Cursor、Claude Code）的开发者需要为每个工具单独维护忽略配置。这既繁琐又容易出错——当开发者在一个工具中添加敏感文件到忽略列表时，必须记住在另一个工具中也做同样的操作。两个工具都会自动遵守 `.gitignore`，但工具特定的忽略规则（Cursor 的 `.cursorignore`、Claude Code 的 `permissions.deny`）无法互操作。

## What Changes

为 `llman tool` 添加 `sync-ignore` 工具，用于在 Cursor 和 Claude Code 格式之间转换忽略规则：

1. **新工具命令**: `llman tool sync-ignore` (别名: `si`)
   - `--from, -f`: 源格式 (cursor, claude-code, 或自动检测)
   - `--to, -t`: 目标格式 (仅非 x 命令需要)
   - `--input, -i`: 输入文件路径 (自动检测常见位置)
   - `--output, -o`: 输出文件路径 (根据目标格式默认确定)
   - `--bidirectional, -b`: 双向合并
   - `--interactive, -I`: 基于 inquirer 的交互式提示流程
   - `--dry-run, -d`: 预览更改而不实际应用
   - `--verbose, -v`: 详细输出

2. **新增 x 子命令** (便捷快捷方式):
   - `llman x cc sync-ignore`: 转换 Cursor → Claude Code
   - `llman x cursor sync-ignore`: 转换 Claude Code → Cursor

3. **模式转换逻辑**:
   - Cursor `.cursorignore` → Claude Code `permissions.deny`:
     - `*.log` → `Read(./*.log)`
     - `secrets/**` → `Read(./secrets/**)`
     - `!pattern` → 跳过并警告 (不支持否定模式)
   - Claude Code `permissions.deny` → Cursor `.cursorignore`:
     - `Read(./.env)` → `.env`
     - `Read(./secrets/**)` → `secrets/**`
     - 非 Read 规则 → 跳过并警告

## Capabilities

### New Capabilities
- `cursor-claude-ignore-sync`: Cursor `.cursorignore` 模式与 Claude Code `permissions.deny` 规则之间的双向转换，支持自动检测、交互模式和试运行。

### Modified Capabilities
- 无 (新工具，无现有功能变更)

## Impact

**代码变更:**
- `src/tool/command.rs`: 为 `ToolCommands` 枚举添加 `SyncIgnore` 变体
- `src/tool/sync_ignore.rs`: 新增转换逻辑模块
- `src/tool/mod.rs`: 导出新模块
- `src/x/claude_code/command.rs`: 添加 `SyncIgnore` 子命令
- `src/x/cursor/command.rs`: 添加 `SyncIgnore` 子命令
- `src/cli.rs`: 添加新命令的处理程序
- `locales/app.yml`: 添加 i18n 字符串
- `tests/sync_ignore_tests.rs`: 添加集成测试

**新依赖:** 无 (使用现有的 `inquire`, `serde_json`, `anyhow`)

**用户可见变更:**
- 新 CLI 命令: `llman tool sync-ignore`
- 新 x 子命令: `llman x cc sync-ignore`, `llman x cursor sync-ignore`
- 可能创建新文件: `.cursorignore`, `.claude/settings.json`

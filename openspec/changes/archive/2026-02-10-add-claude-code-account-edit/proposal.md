## Why

目前 `llman x codex account edit` 已支持用 `$VISUAL/$EDITOR` 直接打开用户配置文件，但 `llman x claude-code account` 缺少等价的编辑入口。为 Claude Code 增加 `account edit` 能减少配置摩擦，并让两套集成的 CLI 体验更一致。

## What Changes

- 新增命令 `llman x claude-code account edit`（同时支持别名 `llman x cc account edit`），使用 `$VISUAL` 或 `$EDITOR` 打开 `claude-code.toml`。
- 编辑器命令解析与执行行为对齐 `llman x codex account edit`：支持包含参数（例如 `code --wait`），并将配置文件路径作为最后一个参数追加。
- 当配置文件不存在时，创建包含最小可解析结构的默认模板（避免出现空文件导致后续解析失败）。
- 补齐必要的本地化文案与错误提示，并增加覆盖该命令的测试。

## Capabilities

### New Capabilities
- `claude-code-account-management`: 为 Claude Code 提供 `account edit` 以便用 `$VISUAL/$EDITOR` 直接编辑 `claude-code.toml`，并定义缺省创建与错误处理行为。

### Modified Capabilities
- (none)

## Impact

- CLI：`src/x/claude_code/command.rs` 增加 `AccountAction::Edit` 及相关处理逻辑；可能抽取与 Codex 共享的 editor 选择/解析辅助函数。
- Templates：新增 Claude Code 默认配置模板文件（或内置字符串），用于首次创建 `claude-code.toml`。
- i18n：`locales/app.yml` 增加 `claude_code.account.*`/`claude_code.error.*` 相关条目。
- Tests：新增/更新测试以验证 editor 命令解析、文件创建与错误分支（使用 `LLMAN_CONFIG_DIR` 测试夹具，避免触碰真实用户配置）。

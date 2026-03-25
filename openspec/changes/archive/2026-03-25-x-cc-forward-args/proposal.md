## Why

`llman x codex` 支持通过 `--` 将剩余参数原样传递给底层 `codex` 命令（例如 `llman x codex -- --version`）。但目前 `llman x cc`（`llman x claude-code`）不接受 `--` 后的参数，导致无法用同样的方式把参数交给 `claude`，体验不一致且使用不便。

## What Changes

- 为 `llman x cc` / `llman x claude-code` 主命令新增 trailing args（使用 `--` 分隔），并在完成配置选择后将其传递给 `claude` 执行。
- 更新 CLI help 文案，明确 `--` 的参数透传用法。
- 添加测试，验证 `--` 后的参数能被 clap 捕获并最终传递到 `claude`。

## Capabilities

### New Capabilities
<!-- 无新增 capability；此变更是对现有行为的补全与一致性提升。 -->

### Modified Capabilities
- `claude-code-runner`: 支持 `llman x cc -- <args...>` 将参数透传给 `claude`（与 `llman x codex -- <args...>` 对齐）。

## Impact

- 受影响代码：`src/x/claude_code/command.rs`（CLI 解析与执行）
- 测试：新增/调整 `tests/*` 覆盖参数透传
- 用户可见变化：`llman x cc --help` / `llman x claude-code --help` 文案与主命令行为

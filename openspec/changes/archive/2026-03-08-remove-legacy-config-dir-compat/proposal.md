## Why

当前 `config-paths` 仍保留一段 macOS 发布过渡逻辑：在未提供 CLI/env override 时，会探测 `~/Library/Application Support/...` 旧配置目录，并在命中时自动回退到旧目录，同时输出迁移提示。经过一段时间发布后，这段兼容层已经不再值得继续维护；它让默认行为依赖历史残留目录，增加测试矩阵、实现复杂度和排障成本。

现在可以将 Linux/macOS 的默认全局配置路径直接收敛为单一路径 `~/.config/llman`，并移除旧目录兼容与提示代码，只保留显式覆盖入口。

## What Changes

- **BREAKING**：在 Linux/macOS 上，当用户未显式传入 `--config-dir` 且未设置 `LLMAN_CONFIG_DIR` 时，默认全局配置目录一律解析为 `~/.config/llman`。
- 删除 macOS legacy 目录自动探测、自动回退与 stderr 迁移提示。
- 保留显式覆盖能力：`--config-dir` 与 `LLMAN_CONFIG_DIR` 仍可指向任意自定义目录（包括旧路径）。
- 更新测试与文案，移除仅服务于 legacy fallback 的 helper、分支与 locale 文案。

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `config-paths`: 移除 macOS legacy 配置目录兼容与迁移提示，并将无 override 时的默认解析固定为 `~/.config/llman`。

## Impact

- 代码：`src/config.rs` 及其调用链（例如 `src/cli.rs`、`src/agents/command.rs`、`src/x/codex/config.rs`、`src/x/claude_code/config.rs`）的默认路径行为。
- 文案：`locales/app.yml` 中仅用于 macOS legacy warning 的消息键。
- 测试：`src/config.rs` 单元测试，以及与默认配置目录解析相关的 CLI / 集成测试。
- 发布说明：需要明确说明这是对旧 macOS 自动兼容路径的收敛，仍可通过显式 override 访问旧目录。

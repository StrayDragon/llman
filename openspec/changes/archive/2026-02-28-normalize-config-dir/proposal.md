## Why

当前 `llman` 的默认配置目录解析依赖 `ProjectDirs`，导致 macOS 默认落在 `~/Library/Application Support/...`，而团队预期与 CLI help 文案却是 `~/.config/llman`。由于 v1 已经发布并产生真实用户数据，这会造成：

- 用户配置在 macOS/Linux 不一致，排障与文档成本高
- 同一台 macOS 机器可能存在两套路径（新/旧），行为不可预测
- 依赖 `directories/dirs` 的平台差异与边界行为增加维护难度

需要在不破坏旧用户的前提下，统一 macOS/Linux 的默认配置目录为 `~/.config/llman`，并为 macOS 历史路径提供兼容与迁移提示。

## What Changes

- 默认配置目录（无 CLI/env override）在 macOS/Linux 统一为 `~/.config/llman`（仍支持显式覆盖）
- 解析优先级保持不变：CLI `--config-dir` > `LLMAN_CONFIG_DIR` > 默认路径
- macOS 特化兼容：
  - 若检测到旧路径存在 llman 配置（例如 `~/Library/Application Support/llman` 或 `~/Library/Application Support/com.StrayDragon.llman`），CLI 会给出“建议迁移”的警告
  - 同时保持可用性：在需要时仍会自动从旧路径解析配置（避免 v1 用户被破坏）
- 移除用于“获取用户配置目录”的相关依赖 crates（`directories/dirs`），改为手写 resolver，确保行为可预测且与文案一致

## Capabilities

### New Capabilities
- （无）

### Modified Capabilities
- `config-paths`: 默认回退路径从 `ProjectDirs` 改为 `~/.config/llman`（macOS/Linux），并新增 macOS legacy 路径检测 + 警告 + 兼容解析规则。

## Impact

- 代码：`src/config.rs`、`src/cli.rs`、`src/x/codex/config.rs`、`src/x/claude_code/config.rs` 以及所有依赖 `directories/dirs` 的路径解析点
- 依赖：移除 `directories/dirs`（以及其传递依赖 `dirs-sys` 等）
- 测试：更新/补充配置路径解析的单测与集成测试（覆盖 macOS legacy 场景）
- 文档与提示：CLI help 与实际默认行为对齐；新增迁移警告文案（stderr）

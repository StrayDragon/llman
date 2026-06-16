# Proposal: refactor-config-dir-guard

## Why

当前 `cli::run()` 在**所有子命令执行前**都调用 `determine_config_dir()`，导致在 llman 项目目录下运行 `llman sdd list` 这种不需要全局配置的命令也会触发 dev-project 检测报错。

这影响了开发体验：开发者在项目目录下工作时，即使只是查看 SDD 变更列表，也必须指定 `--config-dir` 或设置环境变量。

## What Changes

1. 定义 `RequiresGlobalConfig` trait，用于标记需要全局配置的子命令
2. 为 `Commands` 实现该 trait：`Sdd` 返回 `false`，其他返回 `true`
3. 修改 `run()` 函数，只有当子命令需要全局配置时才调用 `determine_config_dir()` 和 `ensure_global_sample_config()`

## Capabilities

- `cli`：命令行入口和配置目录解析

## Impact

- **用户体验**：在 llman 项目目录下运行 `sdd` 子命令不再需要指定 `--config-dir`
- **兼容性**：其他子命令行为不变，无破坏性变更
- **可维护性**：通过 trait 显式声明配置依赖，便于未来扩展

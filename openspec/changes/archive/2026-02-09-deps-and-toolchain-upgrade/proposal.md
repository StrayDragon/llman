## Why

当前仓库依赖与 toolchain 已出现可用更新，但现有约束与 CI 配置存在不一致：本地通过 `rust-toolchain.toml` 固定 nightly，CI 却安装 stable。我们需要在保持 nightly-only 策略的前提下，建立可重复、可验证的升级路径，降低依赖滞后与构建漂移风险。

## What Changes

- 明确并固化 **nightly-only** 构建策略：开发、CI、文档与注释保持一致，不引入 stable 作为主构建路径。
- 升级 `rust-toolchain.toml` 中的 nightly 固定版本到更新日期，并同步验证 `just check-all` 与 release 构建流程。
- 升级 `Cargo.lock` 及必要的 `Cargo.toml` 依赖约束到与目标 nightly 兼容的较新版本（以兼容与稳定优先，不追求一次性大规模破坏性升级）。
- 建立依赖与 toolchain 升级的执行与回归准则（升级命令、验证命令、失败回滚路径）。
- 清理与现实不符的说明（例如“CI 使用 nightly/stable”的陈旧描述），避免后续维护误导。

## Capabilities

### New Capabilities
- `nightly-toolchain-governance`: 定义 nightly 固定版本的选择、升级触发条件、验证步骤与回滚要求，确保团队在同一 toolchain 基线上构建。
- `dependency-upgrade-workflow`: 定义依赖升级的最小可行流程（锁文件更新、必要版本约束调整、nightly 下质量门禁验证），保证升级可重复且可审计。

### Modified Capabilities
- `tests-ci`: 更新 CI 要求，使其显式以项目固定 nightly 运行检查与构建，并与本地 `just`/cargo 校验路径一致。

## Impact

- 受影响代码与配置：`rust-toolchain.toml`、`Cargo.lock`、`Cargo.toml`、`justfile`、`.github/workflows/ci.yaml`，以及相关文档说明。
- 对外行为：CLI 命令与用户运行方式保持不变；主要变化在构建与维护流程。
- 风险：依赖升级可能引入编译告警/行为变化；通过 nightly 门禁（fmt/clippy/tests/build）和小步提交策略控制风险。

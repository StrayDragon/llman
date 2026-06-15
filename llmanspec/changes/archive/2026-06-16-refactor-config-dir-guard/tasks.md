# Tasks: refactor-config-dir-guard

## Implementation

- [x] 1. 定义 `RequiresGlobalConfig` trait 和为 `Commands` 实现
- [x] 2. 修改 `run()` 函数，按需调用配置目录校验
- [x] 3. 添加单元测试验证 trait 行为
- [x] 4. 运行完整测试套件
- [x] 5. 手动验证：在项目目录下运行 `llman sdd list`

## Validation

- [x] `cargo +nightly fmt -- --check` 通过
- [x] `cargo +nightly clippy --all-targets --all-features -- -D warnings` 通过
- [x] `cargo +nightly test --all` 通过

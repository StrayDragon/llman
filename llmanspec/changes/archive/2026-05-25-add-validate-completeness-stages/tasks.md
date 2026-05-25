# Tasks: add-validate-completeness-stages

## Implementation

- [x] 1. 在 `src/sdd/spec/validation.rs` 中新增 `ChangeStage` 枚举和 `determine_stage()` 函数
- [x] 2. 在 `src/sdd/spec/validation.rs` 中新增 `check_completeness_stage()` 函数，实现分级消息逻辑
- [x] 3. 新增 `check_design_tasks_constraint()` 函数，强制 design → tasks 依赖
- [x] 4. 在 `src/sdd/shared/validate.rs` 的 `validate_change_full()` 中集成 completeness check
- [x] 5. 修改 `src/sdd/shared/list.rs`，为变更列表新增 stage 列（文本和 JSON 输出）
- [x] 6. 在 `locales/` 中添加所需的 i18n 翻译键
- [x] 7. 在 `tests/sdd_integration_tests.rs` 中添加测试用例覆盖各阶段检测和约束校验

## Validation

- [x] `cargo +nightly fmt -- --check` 通过
- [x] `cargo +nightly clippy --all-targets --all-features -- -D warnings` 通过

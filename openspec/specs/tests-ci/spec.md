# tests-ci Specification

## Purpose
Describe CI quality gates and required checks for llman.
## Requirements
### Requirement: CI runs fmt and clippy with warnings denied
CI MUST run format checks and clippy with `-D warnings` as part of the standard check pipeline.

#### Scenario: CI check job
- **WHEN** CI runs on main branch
- **THEN** the job executes `just check` (fmt-check + clippy + tests)

### Requirement: CI runs a release build check
CI MUST run a release build to ensure the project builds with the nightly toolchain.

#### Scenario: CI build job
- **WHEN** CI runs on main branch
- **THEN** the job executes `just build-release`

### Requirement: Clippy warnings are addressed in tests
Test code MUST avoid clippy warnings that would fail `-D warnings` (e.g., prefer `is_empty()` over `len() > 0`).

#### Scenario: Clippy run
- **WHEN** `cargo +nightly clippy -- -D warnings` runs
- **THEN** test code does not emit len_zero or similar warnings

### Requirement: check-all 包含 schema 校验
`just check-all` MUST 包含 schema 校验步骤，确保生成的 JSON schema 与样例配置有效且可用。

#### Scenario: 运行 check-all
- **WHEN** 开发者运行 `just check-all`
- **THEN** `just check-schemas` 会被执行


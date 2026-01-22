# Capability: tests-ci

## ADDED Requirements
### Requirement: CI runs fmt and clippy with warnings denied
CI MUST run format checks and clippy with `-D warnings` as part of the standard check pipeline.

#### Scenario: CI check job
- **WHEN** CI runs on main branch
- **THEN** the job executes `just check` (fmt-check + clippy + tests)

### Requirement: Clippy warnings are addressed in tests
Test code MUST avoid clippy warnings that would fail `-D warnings` (e.g., prefer `is_empty()` over `len() > 0`).

#### Scenario: Clippy run
- **WHEN** `cargo +nightly clippy --all-targets --all-features -- -D warnings` runs
- **THEN** test code does not emit len_zero or similar warnings

# Tasks: harden-security-checker-masking-profile

## 1. Artifacts

- [x] proposal / design / deltas
- [x] `llman sdd validate harden-security-checker-masking-profile --no-interactive`

## 2. Implement

- [x] SecurityChecker 告警非空时中止（所有启动 claude 的路径）
- [x] 扩展 `get_display_vars` / mask 敏感键判定 + 单测
- [x] PowerShell `PROFILE` home 约束 + 单测

## 3. Gates

- [x] 相关 `cargo +nightly test`
- [x] `llman sdd validate harden-security-checker-masking-profile --strict --no-interactive`

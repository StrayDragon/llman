# Tasks: harden-security-checker-masking-profile

## 1. Artifacts

- [x] proposal / design / deltas
- [x] `llman sdd validate harden-security-checker-masking-profile --no-interactive`

## 2. Implement

- [ ] SecurityChecker 告警非空时中止（所有启动 claude 的路径）
- [ ] 扩展 `get_display_vars` / mask 敏感键判定 + 单测
- [ ] PowerShell `PROFILE` home 约束 + 单测

## 3. Gates

- [ ] 相关 `cargo +nightly test`
- [ ] `llman sdd validate harden-security-checker-masking-profile --strict --no-interactive`

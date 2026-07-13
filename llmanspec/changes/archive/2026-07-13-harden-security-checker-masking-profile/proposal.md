# Proposal: harden-security-checker-masking-profile

## Why

QA 剩余中等安全项：
1. SecurityChecker 仅警告后仍启动 `claude`，告警形同虚设。
2. `account list` 脱敏只覆盖 KEY/TOKEN/SECRET，`PASSWORD` 等仍可能明文。
3. PowerShell `$PROFILE` / `PROFILE` 无 home 约束，可能写入任意路径。

## What Changes

- **claude-code-runner**：告警非空 → 失败且不启动子进程。
- **claude-code-account-management**：扩展脱敏子串（PASSWORD/PASSWD/CREDENTIAL 等，大小写不敏感）。
- **cli-experience**：PowerShell PROFILE 路径必须位于 home 下。

## Capabilities

| Capability | Delta |
|------------|--------|
| `claude-code-runner` | add r5 |
| `claude-code-account-management` | add r11 |
| `cli-experience` | add r11 |

## Impact

- Breaking（有意）：存在安全告警的环境将无法再“警告后继续”跑 Claude。
- list 输出对更多键脱敏。
- 外部 PROFILE 将被拒绝。

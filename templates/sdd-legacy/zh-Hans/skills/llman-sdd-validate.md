---
name: "llman-sdd-validate"
description: "校验 llmanspec 变更与 specs 并提供修复提示。"
metadata:
  llman-template-version: 2
---

# LLMAN SDD 校验

使用此 skill 校验变更/spec 格式与过期状态。

## 步骤
1. 校验单个条目：`llman sdd-legacy validate <id>`。
2. 批量校验：`llman sdd-legacy validate --all`（或 `--changes` / `--specs`）。
3. 在 CI 或自动化场景中使用 `--strict` 与 `--no-interactive`。
4. 若校验失败，汇总错误并给出最小、可执行的修复建议。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}

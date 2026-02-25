---
name: "llman-sdd-validate"
description: "校验 llmanspec 变更与 specs 并提供修复提示。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD 校验

使用此 skill 校验变更/spec 格式与过期状态。

## 步骤
1. 校验单个条目：`llman sdd validate <id>`。
2. 批量校验：`llman sdd validate --all`（或 `--changes` / `--specs`）。
3. 在 CI 或自动化场景中使用 `--strict` 与 `--no-interactive`。

{{region: templates/sdd/zh-Hans/skills/shared.md#sdd-commands}}

{{region: templates/sdd/zh-Hans/skills/shared.md#validation-hints}}

{{region: templates/sdd/zh-Hans/skills/shared.md#structured-protocol}}

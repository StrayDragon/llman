---
llman-template-version: 1
name: "LLMAN SDD Archive"
description: "Archive a change and merge deltas into specs."
---

# LLMAN SDD 归档

使用此 skill 归档已完成的变更。

## 步骤
1. 确认变更已部署或已接受。
2. 运行 `llman sdd archive <change-id>`。
3. 仅工具类变更使用 `--skip-specs`。
4. 使用 `--dry-run` 预览操作。
5. 重新执行 `llman sdd validate --strict --no-interactive`。

{{region: templates/sdd/zh-Hans/skills/shared.md#sdd-commands}}

{{region: templates/sdd/zh-Hans/skills/shared.md#validation-hints}}

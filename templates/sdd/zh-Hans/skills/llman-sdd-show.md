---
name: "llman-sdd-show"
description: "快速查看 llmanspec 变更与 specs。"
metadata:
  llman-template-version: 1
---

# LLMAN SDD 查看

使用此 skill 快速查看变更与 specs。

## 步骤
1. 列出条目：`llman sdd list` 或 `llman sdd list --specs`。
2. 查看详情：`llman sdd show <id>`。
3. 需要时使用 `--type change|spec` 消除歧义。
4. 使用 `--json` 获取结构化输出。

{{region: templates/sdd/zh-Hans/skills/shared.md#sdd-commands}}

{{region: templates/sdd/zh-Hans/skills/shared.md#validation-hints}}

{{region: templates/sdd/zh-Hans/skills/shared.md#structured-protocol}}

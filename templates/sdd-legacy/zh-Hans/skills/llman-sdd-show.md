---
name: "llman-sdd-show"
description: "快速查看 llmanspec 变更与 specs。"
metadata:
  llman-template-version: 2
---

# LLMAN SDD 查看

使用此 skill 快速查看变更与 specs。

## 步骤
1. 列出条目：`llman sdd-legacy list` 或 `llman sdd-legacy list --specs`。
2. 如果 id 不明确，展示列表并让用户选择。
3. 查看详情：`llman sdd-legacy show <id>`。
4. 需要时使用 `--type change|spec` 消除歧义。
5. 使用 `--json` 获取结构化输出。

{{ unit("skills/sdd-commands") }}

{{ unit("skills/validation-hints") }}

{{ unit("skills/structured-protocol") }}
